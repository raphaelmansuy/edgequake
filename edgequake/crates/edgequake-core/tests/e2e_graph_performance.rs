#![cfg(feature = "pipeline")]

//! End-to-End Performance Tests for Graph API
//!
//! These tests verify the performance improvements from N+1 query elimination:
//! - Batch query methods return correct results
//! - Graph loading scales efficiently with node count
//! - Edge filtering happens at database level
//!
//! Run with: `cargo test --package edgequake-core --test e2e_graph_performance`

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use edgequake_storage::{GraphStorage, MemoryGraphStorage};

/// Create a test graph storage with known data.
async fn create_populated_graph_storage(node_count: usize) -> Arc<MemoryGraphStorage> {
    let storage = Arc::new(MemoryGraphStorage::new("test"));
    storage.initialize().await.expect("Failed to initialize");

    // Create nodes with varying degrees
    // Central hub nodes (high degree)
    let hub_count = (node_count / 20).max(3); // 5% are hubs
    for i in 0..hub_count {
        let node_id = format!("HUB_{}", i);
        let mut props = std::collections::HashMap::new();
        props.insert(
            "entity_type".to_string(),
            serde_json::Value::String("HUB".to_string()),
        );
        props.insert(
            "description".to_string(),
            serde_json::Value::String(format!("Hub node {}", i)),
        );
        props.insert(
            "tenant_id".to_string(),
            serde_json::Value::String("default".to_string()),
        );
        storage.upsert_node(&node_id, props).await.unwrap();
    }

    // Regular nodes (connected to hubs)
    for i in 0..(node_count - hub_count) {
        let node_id = format!("NODE_{}", i);
        let mut props = std::collections::HashMap::new();
        props.insert(
            "entity_type".to_string(),
            serde_json::Value::String("ENTITY".to_string()),
        );
        props.insert(
            "description".to_string(),
            serde_json::Value::String(format!("Regular node {}", i)),
        );
        props.insert(
            "tenant_id".to_string(),
            serde_json::Value::String("default".to_string()),
        );
        storage.upsert_node(&node_id, props).await.unwrap();
    }

    // Create edges - connect regular nodes to hubs
    for i in 0..(node_count - hub_count) {
        let node_id = format!("NODE_{}", i);
        let hub_id = format!("HUB_{}", i % hub_count);
        let mut props = std::collections::HashMap::new();
        props.insert(
            "relation_type".to_string(),
            serde_json::Value::String("CONNECTED_TO".to_string()),
        );
        props.insert("weight".to_string(), serde_json::json!(1.0));
        props.insert(
            "tenant_id".to_string(),
            serde_json::Value::String("default".to_string()),
        );
        storage.upsert_edge(&node_id, &hub_id, props).await.unwrap();
    }

    // Connect hubs to each other
    for i in 0..hub_count {
        for j in (i + 1)..hub_count {
            let hub_a = format!("HUB_{}", i);
            let hub_b = format!("HUB_{}", j);
            let mut props = std::collections::HashMap::new();
            props.insert(
                "relation_type".to_string(),
                serde_json::Value::String("HUB_CONNECTION".to_string()),
            );
            props.insert("weight".to_string(), serde_json::json!(2.0));
            props.insert(
                "tenant_id".to_string(),
                serde_json::Value::String("default".to_string()),
            );
            storage.upsert_edge(&hub_a, &hub_b, props).await.unwrap();
        }
    }

    storage
}

// ============================================================================
// Performance Verification Tests
// ============================================================================

#[tokio::test]
async fn test_get_popular_nodes_with_degree_performance() {
    let storage = create_populated_graph_storage(200).await;

    let start = Instant::now();

    // Use optimized batch method
    let nodes_with_degrees = storage
        .get_popular_nodes_with_degree(100, None, None, None, None)
        .await
        .unwrap();

    let elapsed = start.elapsed();

    // Verify correctness
    assert!(!nodes_with_degrees.is_empty());
    assert!(nodes_with_degrees.len() <= 100);

    // Verify ordering (highest degree first)
    for i in 1..nodes_with_degrees.len() {
        assert!(
            nodes_with_degrees[i - 1].1 >= nodes_with_degrees[i].1,
            "Nodes should be ordered by degree descending"
        );
    }

    // Performance check - should be fast (< 100ms for 200 nodes)
    assert!(
        elapsed.as_millis() < 100,
        "Batch query should complete in < 100ms, took {:?}",
        elapsed
    );

    println!(
        "get_popular_nodes_with_degree: {} nodes in {:?}",
        nodes_with_degrees.len(),
        elapsed
    );
}

#[tokio::test]
async fn test_get_edges_for_node_set_performance() {
    let storage = create_populated_graph_storage(200).await;

    // Get top nodes first
    let nodes_with_degrees = storage
        .get_popular_nodes_with_degree(100, None, None, None, None)
        .await
        .unwrap();

    let node_ids: Vec<String> = nodes_with_degrees
        .iter()
        .map(|(n, _)| n.id.clone())
        .collect();

    let start = Instant::now();

    // Use optimized batch method
    let edges = storage
        .get_edges_for_node_set(&node_ids, None, None)
        .await
        .unwrap();

    let elapsed = start.elapsed();

    // Verify correctness - all edges should have both endpoints in node set
    let node_set: HashSet<&str> = node_ids.iter().map(|s| s.as_str()).collect();
    for edge in &edges {
        assert!(
            node_set.contains(edge.source.as_str()),
            "Edge source should be in node set"
        );
        assert!(
            node_set.contains(edge.target.as_str()),
            "Edge target should be in node set"
        );
    }

    // Performance check - should be fast (< 100ms for 100 nodes)
    assert!(
        elapsed.as_millis() < 100,
        "Batch query should complete in < 100ms, took {:?}",
        elapsed
    );

    println!(
        "get_edges_for_node_set: {} edges in {:?}",
        edges.len(),
        elapsed
    );
}

#[tokio::test]
async fn test_optimized_vs_n_plus_one_comparison() {
    let storage = create_populated_graph_storage(100).await;

    // Method 1: Optimized batch query
    let start_optimized = Instant::now();
    let nodes_optimized = storage
        .get_popular_nodes_with_degree(50, None, None, None, None)
        .await
        .unwrap();
    let elapsed_optimized = start_optimized.elapsed();

    // Method 2: Simulate N+1 pattern
    let start_n_plus_one = Instant::now();
    let labels = storage.get_popular_labels(50).await.unwrap();
    let mut nodes_n_plus_one = Vec::new();
    for label in &labels {
        if let Some(node) = storage.get_node(label).await.unwrap() {
            let degree = storage.node_degree(label).await.unwrap();
            nodes_n_plus_one.push((node, degree));
        }
    }
    let elapsed_n_plus_one = start_n_plus_one.elapsed();

    // Both should return same number of nodes
    assert_eq!(nodes_optimized.len(), nodes_n_plus_one.len());

    println!(
        "Performance comparison:\n  Optimized: {:?}\n  N+1 pattern: {:?}\n  Speedup: {:.2}x",
        elapsed_optimized,
        elapsed_n_plus_one,
        elapsed_n_plus_one.as_nanos() as f64 / elapsed_optimized.as_nanos().max(1) as f64
    );

    // Note: With in-memory storage, the difference might be minimal.
    // The real benefit shows with PostgreSQL where network round-trips add latency.
}

// ============================================================================
// Correctness Tests
// ============================================================================

#[tokio::test]
async fn test_hub_nodes_have_highest_degree() {
    let storage = create_populated_graph_storage(100).await;

    let nodes = storage
        .get_popular_nodes_with_degree(10, None, None, None, None)
        .await
        .unwrap();

    // Top nodes should be hubs (they have the most connections)
    for (node, degree) in nodes.iter().take(5) {
        assert!(
            node.id.starts_with("HUB_"),
            "Top nodes should be HUBs, got {}",
            node.id
        );
        assert!(*degree > 0, "Hub should have positive degree");
    }
}

#[tokio::test]
async fn test_entity_type_filter() {
    let storage = create_populated_graph_storage(50).await;

    // Get only HUB entities
    let hubs = storage
        .get_popular_nodes_with_degree(100, None, Some("HUB"), None, None)
        .await
        .unwrap();

    for (node, _) in &hubs {
        let entity_type = node
            .properties
            .get("entity_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert_eq!(entity_type, "HUB");
    }

    // Get only ENTITY type
    let entities = storage
        .get_popular_nodes_with_degree(100, None, Some("ENTITY"), None, None)
        .await
        .unwrap();

    for (node, _) in &entities {
        let entity_type = node
            .properties
            .get("entity_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert_eq!(entity_type, "ENTITY");
    }
}

#[tokio::test]
async fn test_minimum_degree_filter() {
    let storage = create_populated_graph_storage(100).await;

    // Get nodes with at least 5 connections
    let nodes = storage
        .get_popular_nodes_with_degree(100, Some(5), None, None, None)
        .await
        .unwrap();

    for (_, degree) in &nodes {
        assert!(*degree >= 5, "Node should have degree >= 5, got {}", degree);
    }
}

#[tokio::test]
async fn test_tenant_filtering() {
    let storage = create_populated_graph_storage(50).await;

    // All test data has tenant_id "default"
    let nodes = storage
        .get_popular_nodes_with_degree(100, None, None, Some("default"), None)
        .await
        .unwrap();

    assert!(!nodes.is_empty(), "Should find nodes for default tenant");

    // Non-existent tenant should return empty
    let nodes_other = storage
        .get_popular_nodes_with_degree(100, None, None, Some("other_tenant"), None)
        .await
        .unwrap();

    assert!(
        nodes_other.is_empty(),
        "Should not find nodes for other tenant"
    );
}

#[tokio::test]
async fn test_edge_set_filtering_correctness() {
    let storage = create_populated_graph_storage(50).await;

    // Get subset of nodes
    let all_nodes = storage
        .get_popular_nodes_with_degree(50, None, None, None, None)
        .await
        .unwrap();

    // Take only first 10 nodes
    let subset: Vec<String> = all_nodes
        .iter()
        .take(10)
        .map(|(n, _)| n.id.clone())
        .collect();

    let edges = storage
        .get_edges_for_node_set(&subset, None, None)
        .await
        .unwrap();

    // All edges should have both endpoints in the subset
    let subset_set: HashSet<&str> = subset.iter().map(|s| s.as_str()).collect();

    for edge in &edges {
        assert!(
            subset_set.contains(edge.source.as_str()) && subset_set.contains(edge.target.as_str()),
            "Edge ({} -> {}) has endpoint not in node set",
            edge.source,
            edge.target
        );
    }
}

// ============================================================================
// Scale Tests
// ============================================================================

#[tokio::test]
async fn test_scale_200_nodes() {
    let storage = create_populated_graph_storage(200).await;

    let start = Instant::now();

    let nodes = storage
        .get_popular_nodes_with_degree(200, None, None, None, None)
        .await
        .unwrap();

    let node_ids: Vec<String> = nodes.iter().map(|(n, _)| n.id.clone()).collect();

    let edges = storage
        .get_edges_for_node_set(&node_ids, None, None)
        .await
        .unwrap();

    let elapsed = start.elapsed();

    println!(
        "Scale test 200 nodes: {} nodes, {} edges in {:?}",
        nodes.len(),
        edges.len(),
        elapsed
    );

    assert!(nodes.len() == 200);
    assert!(!edges.is_empty());
    assert!(elapsed.as_millis() < 200, "Should complete in < 200ms");
}

#[tokio::test]
async fn test_scale_500_nodes() {
    let storage = create_populated_graph_storage(500).await;

    let start = Instant::now();

    let nodes = storage
        .get_popular_nodes_with_degree(500, None, None, None, None)
        .await
        .unwrap();

    let node_ids: Vec<String> = nodes.iter().map(|(n, _)| n.id.clone()).collect();

    let edges = storage
        .get_edges_for_node_set(&node_ids, None, None)
        .await
        .unwrap();

    let elapsed = start.elapsed();

    println!(
        "Scale test 500 nodes: {} nodes, {} edges in {:?}",
        nodes.len(),
        edges.len(),
        elapsed
    );

    assert!(nodes.len() == 500);
    assert!(!edges.is_empty());
    assert!(elapsed.as_millis() < 500, "Should complete in < 500ms");
}

#[tokio::test]
async fn test_scale_1000_nodes() {
    let storage = create_populated_graph_storage(1000).await;

    let start = Instant::now();

    // Only fetch top 200 (realistic UI limit)
    let nodes = storage
        .get_popular_nodes_with_degree(200, None, None, None, None)
        .await
        .unwrap();

    let node_ids: Vec<String> = nodes.iter().map(|(n, _)| n.id.clone()).collect();

    let edges = storage
        .get_edges_for_node_set(&node_ids, None, None)
        .await
        .unwrap();

    let elapsed = start.elapsed();

    println!(
        "Scale test 1000 nodes (top 200): {} nodes, {} edges in {:?}",
        nodes.len(),
        edges.len(),
        elapsed
    );

    assert!(nodes.len() == 200);
    assert!(!edges.is_empty());
    assert!(elapsed.as_millis() < 300, "Should complete in < 300ms");
}
