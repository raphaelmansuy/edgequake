// Unit tests for get_popular_nodes_with_degree query performance and correctness
//
// These tests verify:
// 1. Query completes within timeout (< 500ms for 1000 nodes)
// 2. Correct tenant/workspace filtering
// 3. Degree calculation accuracy
// 4. Index usage verification

use edgequake_core::types::{GraphEdge, GraphNode};
use edgequake_storage::{
    adapters::postgres::PostgresStorage, traits::graph::GraphStorage, StorageConfig,
};
use std::collections::HashMap;
use std::time::Instant;

/// Test that get_popular_nodes_with_degree completes quickly with indexes
#[tokio::test]
#[ignore = "requires PostgreSQL with AGE and indexes"]
async fn test_get_popular_nodes_performance_with_indexes() {
    let config = StorageConfig::from_env_or_defaults();
    let storage = PostgresStorage::new(config)
        .await
        .expect("Failed to create storage");

    // Create test graph with 1000 nodes and ~5000 edges (avg degree 5)
    let tenant_id = "test_tenant";
    let workspace_id = "test_workspace";

    // Insert nodes
    for i in 0..1000 {
        let mut props = HashMap::new();
        props.insert(
            "node_id".to_string(),
            serde_json::Value::String(format!("node_{}", i)),
        );
        props.insert(
            "tenant_id".to_string(),
            serde_json::Value::String(tenant_id.to_string()),
        );
        props.insert(
            "workspace_id".to_string(),
            serde_json::Value::String(workspace_id.to_string()),
        );
        props.insert(
            "entity_type".to_string(),
            serde_json::Value::String(format!("type_{}", i % 5)),
        );

        storage
            .upsert_node(&format!("node_{}", i), props)
            .await
            .expect("Failed to insert node");
    }

    // Insert edges (create hub-and-spoke pattern)
    // Node 0 is a hub with 100 connections
    // Nodes 1-10 are mid-level hubs with 50 connections each
    // Rest have random connections
    for i in 0..100 {
        let mut edge_props = HashMap::new();
        edge_props.insert("weight".to_string(), serde_json::Value::Number(1.into()));
        storage
            .upsert_edge("node_0", &format!("node_{}", i + 1), edge_props.clone())
            .await
            .expect("Failed to insert edge");
    }

    for hub in 1..=10 {
        for i in 0..50 {
            let target = (hub * 100 + i) % 1000;
            let mut edge_props = HashMap::new();
            edge_props.insert("weight".to_string(), serde_json::Value::Number(1.into()));
            storage
                .upsert_edge(&format!("node_{}", hub), &format!("node_{}", target), edge_props)
                .await
                .expect("Failed to insert edge");
        }
    }

    // Measure query performance
    let start = Instant::now();
    let results = storage
        .get_popular_nodes_with_degree(
            100,
            None,
            None,
            Some(tenant_id),
            Some(workspace_id),
        )
        .await
        .expect("Query failed");
    let duration = start.elapsed();

    // Verify performance: should complete in < 500ms with indexes
    assert!(
        duration.as_millis() < 500,
        "Query took {:?}, expected < 500ms. Check if indexes are created!",
        duration
    );

    // Verify results
    assert_eq!(results.len(), 100, "Should return exactly 100 nodes");

    // Node 0 should be first (highest degree)
    assert_eq!(results[0].0.id, "node_0");
    assert!(
        results[0].1 >= 100,
        "Hub node should have degree >= 100, got {}",
        results[0].1
    );

    // Verify sorting by degree (descending)
    for i in 0..results.len() - 1 {
        assert!(
            results[i].1 >= results[i + 1].1,
            "Results not sorted by degree: {} < {}",
            results[i].1,
            results[i + 1].1
        );
    }

    println!(
        "✓ Query completed in {:?} for 1000 nodes with {} results",
        duration,
        results.len()
    );
}

/// Test that tenant filtering works correctly
#[tokio::test]
#[ignore = "requires PostgreSQL with AGE"]
async fn test_tenant_filtering_correctness() {
    let config = StorageConfig::from_env_or_defaults();
    let storage = PostgresStorage::new(config)
        .await
        .expect("Failed to create storage");

    // Create nodes in different tenants
    let tenant1 = "tenant_1";
    let tenant2 = "tenant_2";

    for tenant in &[tenant1, tenant2] {
        for i in 0..50 {
            let mut props = HashMap::new();
            props.insert(
                "node_id".to_string(),
                serde_json::Value::String(format!("{}_{}", tenant, i)),
            );
            props.insert(
                "tenant_id".to_string(),
                serde_json::Value::String(tenant.to_string()),
            );

            storage
                .upsert_node(&format!("{}_{}", tenant, i), props)
                .await
                .expect("Failed to insert node");
        }
    }

    // Query tenant1 only
    let results = storage
        .get_popular_nodes_with_degree(100, None, None, Some(tenant1), None)
        .await
        .expect("Query failed");

    // Verify all results belong to tenant1
    for (node, _degree) in &results {
        let tenant_value = node.properties.get("tenant_id").unwrap();
        assert_eq!(
            tenant_value.as_str().unwrap(),
            tenant1,
            "Found node from wrong tenant"
        );
    }

    assert_eq!(
        results.len(),
        50,
        "Should return all 50 nodes from tenant1"
    );

    println!("✓ Tenant filtering works correctly");
}

/// Test that entity type filtering works
#[tokio::test]
#[ignore = "requires PostgreSQL with AGE"]
async fn test_entity_type_filtering() {
    let config = StorageConfig::from_env_or_defaults();
    let storage = PostgresStorage::new(config)
        .await
        .expect("Failed to create storage");

    // Create nodes with different entity types
    for i in 0..100 {
        let mut props = HashMap::new();
        props.insert(
            "node_id".to_string(),
            serde_json::Value::String(format!("node_{}", i)),
        );
        props.insert(
            "entity_type".to_string(),
            serde_json::Value::String(if i < 50 { "PERSON" } else { "ORG" }.to_string()),
        );

        storage
            .upsert_node(&format!("node_{}", i), props)
            .await
            .expect("Failed to insert node");
    }

    // Query PERSON type only
    let results = storage
        .get_popular_nodes_with_degree(100, None, Some("PERSON"), None, None)
        .await
        .expect("Query failed");

    // Verify all results are PERSON type
    for (node, _degree) in &results {
        let entity_type = node.properties.get("entity_type").unwrap();
        assert_eq!(
            entity_type.as_str().unwrap(),
            "PERSON",
            "Found node with wrong entity type"
        );
    }

    assert_eq!(results.len(), 50, "Should return 50 PERSON nodes");

    println!("✓ Entity type filtering works correctly");
}

/// Test that degree calculation is accurate
#[tokio::test]
#[ignore = "requires PostgreSQL with AGE"]
async fn test_degree_calculation_accuracy() {
    let config = StorageConfig::from_env_or_defaults();
    let storage = PostgresStorage::new(config)
        .await
        .expect("Failed to create storage");

    // Create a simple graph where we know exact degrees
    // Node A: 3 outgoing edges
    // Node B: 2 outgoing edges
    // Node C: 1 outgoing edge
    // Node D: 0 outgoing edges

    for node_id in &["A", "B", "C", "D"] {
        let mut props = HashMap::new();
        props.insert(
            "node_id".to_string(),
            serde_json::Value::String(node_id.to_string()),
        );
        storage
            .upsert_node(node_id, props)
            .await
            .expect("Failed to insert node");
    }

    // Create edges
    let edges = vec![("A", "B"), ("A", "C"), ("A", "D"), ("B", "C"), ("B", "D"), ("C", "D")];

    for (source, target) in edges {
        let props = HashMap::new();
        storage
            .upsert_edge(source, target, props)
            .await
            .expect("Failed to insert edge");
    }

    // Query all nodes
    let results = storage
        .get_popular_nodes_with_degree(10, None, None, None, None)
        .await
        .expect("Query failed");

    // Verify degrees
    let mut degree_map: HashMap<String, usize> = HashMap::new();
    for (node, degree) in results {
        degree_map.insert(node.id, degree);
    }

    assert_eq!(*degree_map.get("A").unwrap(), 3, "Node A should have degree 3");
    assert_eq!(*degree_map.get("B").unwrap(), 2, "Node B should have degree 2");
    assert_eq!(*degree_map.get("C").unwrap(), 1, "Node C should have degree 1");
    assert_eq!(*degree_map.get("D").unwrap(), 0, "Node D should have degree 0");

    println!("✓ Degree calculation is accurate");
}

/// Test min_degree filtering
#[tokio::test]
#[ignore = "requires PostgreSQL with AGE"]
async fn test_min_degree_filtering() {
    let config = StorageConfig::from_env_or_defaults();
    let storage = PostgresStorage::new(config)
        .await
        .expect("Failed to create storage");

    // Create nodes with known degrees (reuse graph from previous test)
    for node_id in &["A", "B", "C", "D"] {
        let mut props = HashMap::new();
        props.insert(
            "node_id".to_string(),
            serde_json::Value::String(node_id.to_string()),
        );
        storage
            .upsert_node(node_id, props)
            .await
            .expect("Failed to insert node");
    }

    let edges = vec![("A", "B"), ("A", "C"), ("A", "D"), ("B", "C"), ("B", "D")];
    for (source, target) in edges {
        storage
            .upsert_edge(source, target, HashMap::new())
            .await
            .expect("Failed to insert edge");
    }

    // Query with min_degree = 2
    let results = storage
        .get_popular_nodes_with_degree(10, Some(2), None, None, None)
        .await
        .expect("Query failed");

    // Should only return A (degree 3) and B (degree 2)
    assert!(
        results.len() <= 2,
        "Should return at most 2 nodes with degree >= 2"
    );

    for (node, degree) in &results {
        assert!(
            *degree >= 2,
            "Node {} has degree {}, expected >= 2",
            node.id,
            degree
        );
    }

    println!("✓ Min degree filtering works correctly");
}
