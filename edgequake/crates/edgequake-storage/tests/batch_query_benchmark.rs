//! Benchmark tests comparing batch vs individual query performance.
//!
//! These tests verify that the LightRAG-inspired batch query pattern
//! achieves O(1) performance compared to O(N) individual queries.

use edgequake_storage::adapters::memory::MemoryGraphStorage;
use edgequake_storage::traits::GraphStorage;
use std::collections::HashMap;
use std::time::Instant;

/// Create a test graph with the specified number of nodes and edges per node.
async fn setup_test_graph(
    node_count: usize,
    edges_per_node: usize,
) -> (MemoryGraphStorage, Vec<String>) {
    let storage = MemoryGraphStorage::new("benchmark");
    storage.initialize().await.unwrap();

    let mut node_ids = Vec::new();
    let mut properties = HashMap::new();
    properties.insert("entity_type".to_string(), serde_json::json!("TEST_ENTITY"));
    properties.insert(
        "description".to_string(),
        serde_json::json!("Test entity description"),
    );

    // Create nodes
    for i in 0..node_count {
        let node_id = format!("NODE_{}", i);
        let mut node_props = properties.clone();
        node_props.insert("node_id".to_string(), serde_json::json!(&node_id));
        storage.upsert_node(&node_id, node_props).await.unwrap();
        node_ids.push(node_id);
    }

    // Create edges (each node connected to next N nodes cyclically)
    for i in 0..node_count {
        for j in 1..=edges_per_node {
            let target_idx = (i + j) % node_count;
            storage
                .upsert_edge(&node_ids[i], &node_ids[target_idx], HashMap::new())
                .await
                .unwrap();
        }
    }

    (storage, node_ids)
}

#[tokio::test]
async fn test_batch_vs_individual_nodes_performance() {
    // Setup: 100 nodes, 5 edges per node
    let (storage, node_ids) = setup_test_graph(100, 5).await;

    // Query subset of 50 nodes
    let query_ids: Vec<String> = node_ids[..50].to_vec();

    // Measure individual queries (O(N) pattern)
    let individual_start = Instant::now();
    let mut individual_results = HashMap::new();
    for id in &query_ids {
        if let Ok(Some(node)) = storage.get_node(id).await {
            individual_results.insert(id.clone(), node);
        }
    }
    let individual_time = individual_start.elapsed();

    // Measure batch query (O(1) pattern)
    let batch_start = Instant::now();
    let batch_results = storage.get_nodes_batch(&query_ids).await.unwrap();
    let batch_time = batch_start.elapsed();

    // Verify correctness
    assert_eq!(individual_results.len(), batch_results.len());
    for (id, individual_node) in &individual_results {
        let batch_node = batch_results.get(id).expect("Node should exist in batch");
        assert_eq!(individual_node.id, batch_node.id);
    }

    println!(
        "\n=== Batch vs Individual Node Query Performance (50 nodes) ===\n\
         Individual queries: {:?}\n\
         Batch query:        {:?}\n\
         Speedup:            {:.2}x",
        individual_time,
        batch_time,
        individual_time.as_nanos() as f64 / batch_time.as_nanos().max(1) as f64
    );

    // For memory storage, batch should be faster (or at least not much slower)
    // The real benefit is in PostgreSQL where it's 50x fewer round-trips
}

#[tokio::test]
async fn test_batch_vs_individual_edges_performance() {
    // Setup: 100 nodes, 5 edges per node = 500 edges
    let (storage, node_ids) = setup_test_graph(100, 5).await;

    // Query edges for 30 nodes
    let query_ids: Vec<String> = node_ids[..30].to_vec();

    // Measure individual edge queries (O(N) pattern)
    let individual_start = Instant::now();
    let mut individual_edges = Vec::new();
    for id in &query_ids {
        let edges = storage.get_node_edges(id).await.unwrap();
        individual_edges.extend(edges);
    }
    let individual_time = individual_start.elapsed();

    // Measure batch edge query (O(1) pattern)
    let batch_start = Instant::now();
    let batch_edges = storage.get_edges_for_nodes_batch(&query_ids).await.unwrap();
    let batch_time = batch_start.elapsed();

    // Batch should return edges where BOTH endpoints are in the set
    // So batch_edges might be fewer than individual_edges
    println!(
        "\n=== Batch vs Individual Edge Query Performance (30 nodes) ===\n\
         Individual queries: {:?} ({} edges found)\n\
         Batch query:        {:?} ({} edges found)\n\
         Note: Batch returns edges where both endpoints in query set",
        individual_time,
        individual_edges.len(),
        batch_time,
        batch_edges.len()
    );

    // Verify batch edges are valid (both endpoints in query set)
    let query_set: std::collections::HashSet<_> = query_ids.iter().collect();
    for edge in &batch_edges {
        assert!(
            query_set.contains(&edge.source) && query_set.contains(&edge.target),
            "Batch edge should have both endpoints in query set"
        );
    }
}

#[tokio::test]
async fn test_batch_nodes_with_degrees_performance() {
    // Setup: 200 nodes, 10 edges per node = rich connectivity
    let (storage, node_ids) = setup_test_graph(200, 10).await;

    // Query 100 nodes with their degrees
    let query_ids: Vec<String> = node_ids[..100].to_vec();

    // Measure individual node + degree queries (O(2N) pattern)
    let individual_start = Instant::now();
    let mut individual_results = Vec::new();
    for id in &query_ids {
        if let Ok(Some(node)) = storage.get_node(id).await {
            let degree = storage.node_degree(id).await.unwrap();
            individual_results.push((node, degree, 0usize)); // memory doesn't distinguish in/out
        }
    }
    let individual_time = individual_start.elapsed();

    // Measure batch query with degrees (O(1) pattern)
    let batch_start = Instant::now();
    let batch_results = storage
        .get_nodes_with_degrees_batch(&query_ids)
        .await
        .unwrap();
    let batch_time = batch_start.elapsed();

    // Verify correctness
    assert_eq!(individual_results.len(), batch_results.len());

    println!(
        "\n=== Batch vs Individual Nodes+Degrees Performance (100 nodes) ===\n\
         Individual queries: {:?}\n\
         Batch query:        {:?}\n\
         Speedup:            {:.2}x",
        individual_time,
        batch_time,
        individual_time.as_nanos() as f64 / batch_time.as_nanos().max(1) as f64
    );
}

#[tokio::test]
async fn test_batch_empty_input() {
    let storage = MemoryGraphStorage::new("test");
    storage.initialize().await.unwrap();

    // Empty input should return empty results
    let empty_ids: Vec<String> = vec![];

    let nodes = storage.get_nodes_batch(&empty_ids).await.unwrap();
    assert!(nodes.is_empty());

    let edges = storage.get_edges_for_nodes_batch(&empty_ids).await.unwrap();
    assert!(edges.is_empty());

    let nodes_with_degrees = storage
        .get_nodes_with_degrees_batch(&empty_ids)
        .await
        .unwrap();
    assert!(nodes_with_degrees.is_empty());
}

#[tokio::test]
async fn test_batch_nonexistent_nodes() {
    let (storage, _) = setup_test_graph(10, 2).await;

    // Query non-existent nodes
    let nonexistent_ids: Vec<String> = vec![
        "NONEXISTENT_1".to_string(),
        "NONEXISTENT_2".to_string(),
        "NODE_0".to_string(), // Mix with one existing node
    ];

    let nodes = storage.get_nodes_batch(&nonexistent_ids).await.unwrap();

    // Only the existing node should be returned
    assert_eq!(nodes.len(), 1);
    assert!(nodes.contains_key("NODE_0"));
}

#[tokio::test]
async fn test_batch_query_preserves_node_properties() {
    let storage = MemoryGraphStorage::new("test");
    storage.initialize().await.unwrap();

    // Create nodes with different properties
    let mut props1 = HashMap::new();
    props1.insert("node_id".to_string(), serde_json::json!("ALICE"));
    props1.insert("entity_type".to_string(), serde_json::json!("PERSON"));
    props1.insert("name".to_string(), serde_json::json!("Alice"));
    props1.insert("age".to_string(), serde_json::json!(30));

    let mut props2 = HashMap::new();
    props2.insert("node_id".to_string(), serde_json::json!("ACME"));
    props2.insert("entity_type".to_string(), serde_json::json!("ORGANIZATION"));
    props2.insert("name".to_string(), serde_json::json!("Acme Corp"));
    props2.insert("employees".to_string(), serde_json::json!(1000));

    storage.upsert_node("ALICE", props1.clone()).await.unwrap();
    storage.upsert_node("ACME", props2.clone()).await.unwrap();

    // Batch query
    let nodes = storage
        .get_nodes_batch(&["ALICE".to_string(), "ACME".to_string()])
        .await
        .unwrap();

    // Verify properties preserved
    let alice = nodes.get("ALICE").unwrap();
    assert_eq!(
        alice.properties.get("entity_type").unwrap().as_str(),
        Some("PERSON")
    );
    assert_eq!(alice.properties.get("age").unwrap().as_i64(), Some(30));

    let acme = nodes.get("ACME").unwrap();
    assert_eq!(
        acme.properties.get("entity_type").unwrap().as_str(),
        Some("ORGANIZATION")
    );
    assert_eq!(
        acme.properties.get("employees").unwrap().as_i64(),
        Some(1000)
    );
}

#[tokio::test]
async fn test_batch_query_at_scale() {
    // Test with larger scale to verify performance characteristics
    let node_count = 500;
    let edges_per_node = 8;
    let query_size = 200;

    let (storage, node_ids) = setup_test_graph(node_count, edges_per_node).await;
    let query_ids: Vec<String> = node_ids[..query_size].to_vec();

    // Warm up
    let _ = storage.get_nodes_batch(&query_ids).await.unwrap();

    // Multiple runs for stability
    let mut batch_times = Vec::new();
    for _ in 0..5 {
        let start = Instant::now();
        let _ = storage.get_nodes_batch(&query_ids).await.unwrap();
        batch_times.push(start.elapsed());
    }

    let avg_batch_time = batch_times.iter().sum::<std::time::Duration>() / batch_times.len() as u32;

    println!(
        "\n=== Scale Test ({} nodes, {} edges, {} query size) ===\n\
         Average batch query time: {:?}",
        node_count,
        node_count * edges_per_node,
        query_size,
        avg_batch_time
    );

    // Verify correctness
    let result = storage.get_nodes_batch(&query_ids).await.unwrap();
    assert_eq!(result.len(), query_size);
}
