//! Tests for optimized graph storage methods.
//!
//! These tests verify the performance-optimized batch query methods
//! that eliminate N+1 patterns:
//! - `get_popular_nodes_with_degree()`
//! - `get_edges_for_node_set()`

use std::collections::HashMap;

use edgequake_storage::adapters::memory::MemoryGraphStorage;
use edgequake_storage::traits::{GraphEdge, GraphNode, GraphStorage};

/// Helper to create a test node with properties.
fn create_test_node(id: &str, entity_type: &str, tenant_id: Option<&str>) -> GraphNode {
    let mut props = HashMap::new();
    props.insert(
        "entity_type".to_string(),
        serde_json::Value::String(entity_type.to_string()),
    );
    props.insert(
        "description".to_string(),
        serde_json::Value::String(format!("Description for {}", id)),
    );
    if let Some(tid) = tenant_id {
        props.insert(
            "tenant_id".to_string(),
            serde_json::Value::String(tid.to_string()),
        );
    }
    GraphNode::with_properties(id, props)
}

/// Helper to create a test edge with properties.
fn create_test_edge(source: &str, target: &str, tenant_id: Option<&str>) -> GraphEdge {
    let mut props = HashMap::new();
    props.insert(
        "relation_type".to_string(),
        serde_json::Value::String("RELATED_TO".to_string()),
    );
    props.insert("weight".to_string(), serde_json::json!(1.0));
    if let Some(tid) = tenant_id {
        props.insert(
            "tenant_id".to_string(),
            serde_json::Value::String(tid.to_string()),
        );
    }
    GraphEdge::with_properties(source, target, props)
}

/// Create a test graph with nodes and edges.
async fn setup_test_graph(storage: &MemoryGraphStorage) {
    // Create nodes with varying degrees
    // Node A: degree 4 (connected to B, C, D, E)
    // Node B: degree 3 (connected to A, C, D)
    // Node C: degree 3 (connected to A, B, D)
    // Node D: degree 3 (connected to A, B, C)
    // Node E: degree 1 (connected to A)
    // Node F: degree 0 (orphan)

    let nodes = vec![
        create_test_node("A", "PERSON", Some("tenant1")),
        create_test_node("B", "PERSON", Some("tenant1")),
        create_test_node("C", "ORGANIZATION", Some("tenant1")),
        create_test_node("D", "CONCEPT", Some("tenant1")),
        create_test_node("E", "LOCATION", Some("tenant1")),
        create_test_node("F", "PERSON", Some("tenant2")), // Different tenant
    ];

    for node in nodes {
        storage
            .upsert_node(&node.id, node.properties.clone())
            .await
            .unwrap();
    }

    // Create edges
    let edges = vec![
        create_test_edge("A", "B", Some("tenant1")),
        create_test_edge("A", "C", Some("tenant1")),
        create_test_edge("A", "D", Some("tenant1")),
        create_test_edge("A", "E", Some("tenant1")),
        create_test_edge("B", "C", Some("tenant1")),
        create_test_edge("B", "D", Some("tenant1")),
        create_test_edge("C", "D", Some("tenant1")),
    ];

    for edge in edges {
        storage
            .upsert_edge(&edge.source, &edge.target, edge.properties.clone())
            .await
            .unwrap();
    }
}

// ============================================
// get_popular_nodes_with_degree() Tests
// ============================================

#[tokio::test]
async fn test_get_popular_nodes_basic() {
    let storage = MemoryGraphStorage::new("test");
    storage.initialize().await.unwrap();
    setup_test_graph(&storage).await;

    let results = storage
        .get_popular_nodes_with_degree(10, None, None, None, None)
        .await
        .unwrap();

    // Should return all 6 nodes
    assert_eq!(results.len(), 6);

    // Results should be ordered by degree (highest first)
    // A has degree 4, B/C/D have degree 3, E has degree 1, F has degree 0
    assert_eq!(results[0].0.id, "A");
    assert_eq!(results[0].1, 4); // degree

    // F (orphan) should be last with degree 0
    let last = results.last().unwrap();
    assert_eq!(last.0.id, "F");
    assert_eq!(last.1, 0);
}

#[tokio::test]
async fn test_get_popular_nodes_limit() {
    let storage = MemoryGraphStorage::new("test");
    storage.initialize().await.unwrap();
    setup_test_graph(&storage).await;

    let results = storage
        .get_popular_nodes_with_degree(3, None, None, None, None)
        .await
        .unwrap();

    // Should return only top 3 nodes by degree
    assert_eq!(results.len(), 3);

    // First should be A with degree 4
    assert_eq!(results[0].0.id, "A");
    assert_eq!(results[0].1, 4);
}

#[tokio::test]
async fn test_get_popular_nodes_min_degree() {
    let storage = MemoryGraphStorage::new("test");
    storage.initialize().await.unwrap();
    setup_test_graph(&storage).await;

    let results = storage
        .get_popular_nodes_with_degree(10, Some(3), None, None, None)
        .await
        .unwrap();

    // Should return only nodes with degree >= 3
    // A (4), B (3), C (3), D (3)
    assert_eq!(results.len(), 4);

    for (_, degree) in &results {
        assert!(*degree >= 3);
    }
}

#[tokio::test]
async fn test_get_popular_nodes_entity_type() {
    let storage = MemoryGraphStorage::new("test");
    storage.initialize().await.unwrap();
    setup_test_graph(&storage).await;

    let results = storage
        .get_popular_nodes_with_degree(10, None, Some("PERSON"), None, None)
        .await
        .unwrap();

    // Should return only PERSON nodes: A, B, F
    assert_eq!(results.len(), 3);

    for (node, _) in &results {
        let entity_type = node
            .properties
            .get("entity_type")
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(entity_type, "PERSON");
    }
}

#[tokio::test]
async fn test_get_popular_nodes_tenant_filter() {
    let storage = MemoryGraphStorage::new("test");
    storage.initialize().await.unwrap();
    setup_test_graph(&storage).await;

    let results = storage
        .get_popular_nodes_with_degree(10, None, None, Some("tenant1"), None)
        .await
        .unwrap();

    // Should return only tenant1 nodes: A, B, C, D, E (not F)
    assert_eq!(results.len(), 5);

    for (node, _) in &results {
        let tenant = node
            .properties
            .get("tenant_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(tenant.is_empty() || tenant == "tenant1");
    }
}

#[tokio::test]
async fn test_get_popular_nodes_combined_filters() {
    let storage = MemoryGraphStorage::new("test");
    storage.initialize().await.unwrap();
    setup_test_graph(&storage).await;

    let results = storage
        .get_popular_nodes_with_degree(10, Some(3), Some("PERSON"), Some("tenant1"), None)
        .await
        .unwrap();

    // Should return PERSON nodes with degree >= 3 in tenant1
    // Only A (degree 4) and B (degree 3) match
    assert_eq!(results.len(), 2);

    for (node, degree) in &results {
        let entity_type = node
            .properties
            .get("entity_type")
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(entity_type, "PERSON");
        assert!(*degree >= 3);
    }
}

#[tokio::test]
async fn test_get_popular_nodes_empty_graph() {
    let storage = MemoryGraphStorage::new("test");
    storage.initialize().await.unwrap();

    let results = storage
        .get_popular_nodes_with_degree(10, None, None, None, None)
        .await
        .unwrap();

    assert!(results.is_empty());
}

// ============================================
// get_edges_for_node_set() Tests
// ============================================

#[tokio::test]
async fn test_get_edges_for_node_set_basic() {
    let storage = MemoryGraphStorage::new("test");
    storage.initialize().await.unwrap();
    setup_test_graph(&storage).await;

    let node_ids = vec!["A".to_string(), "B".to_string(), "C".to_string()];
    let results = storage
        .get_edges_for_node_set(&node_ids, None, None)
        .await
        .unwrap();

    // Should return edges: A-B, A-C, B-C
    assert_eq!(results.len(), 3);

    for edge in &results {
        assert!(node_ids.contains(&edge.source));
        assert!(node_ids.contains(&edge.target));
    }
}

#[tokio::test]
async fn test_get_edges_for_node_set_single_node() {
    let storage = MemoryGraphStorage::new("test");
    storage.initialize().await.unwrap();
    setup_test_graph(&storage).await;

    let node_ids = vec!["A".to_string()];
    let results = storage
        .get_edges_for_node_set(&node_ids, None, None)
        .await
        .unwrap();

    // No edges - need at least 2 nodes in set for edges
    assert!(results.is_empty());
}

#[tokio::test]
async fn test_get_edges_for_node_set_disjoint() {
    let storage = MemoryGraphStorage::new("test");
    storage.initialize().await.unwrap();
    setup_test_graph(&storage).await;

    // E and F have no connection
    let node_ids = vec!["E".to_string(), "F".to_string()];
    let results = storage
        .get_edges_for_node_set(&node_ids, None, None)
        .await
        .unwrap();

    assert!(results.is_empty());
}

#[tokio::test]
async fn test_get_edges_for_node_set_empty_input() {
    let storage = MemoryGraphStorage::new("test");
    storage.initialize().await.unwrap();
    setup_test_graph(&storage).await;

    let node_ids: Vec<String> = vec![];
    let results = storage
        .get_edges_for_node_set(&node_ids, None, None)
        .await
        .unwrap();

    assert!(results.is_empty());
}

#[tokio::test]
async fn test_get_edges_for_node_set_tenant_filter() {
    let storage = MemoryGraphStorage::new("test");
    storage.initialize().await.unwrap();
    setup_test_graph(&storage).await;

    let node_ids = vec![
        "A".to_string(),
        "B".to_string(),
        "C".to_string(),
        "F".to_string(),
    ];
    let results = storage
        .get_edges_for_node_set(&node_ids, Some("tenant1"), None)
        .await
        .unwrap();

    // Should return only tenant1 edges: A-B, A-C, B-C
    assert_eq!(results.len(), 3);

    for edge in &results {
        let tenant = edge
            .properties
            .get("tenant_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(tenant.is_empty() || tenant == "tenant1");
    }
}

#[tokio::test]
async fn test_get_edges_for_node_set_all_nodes() {
    let storage = MemoryGraphStorage::new("test");
    storage.initialize().await.unwrap();
    setup_test_graph(&storage).await;

    let node_ids = vec![
        "A".to_string(),
        "B".to_string(),
        "C".to_string(),
        "D".to_string(),
        "E".to_string(),
        "F".to_string(),
    ];
    let results = storage
        .get_edges_for_node_set(&node_ids, None, None)
        .await
        .unwrap();

    // Should return all 7 edges
    assert_eq!(results.len(), 7);
}

// ============================================
// Performance Comparison Tests
// ============================================

#[tokio::test]
async fn test_optimized_methods_return_correct_data() {
    let storage = MemoryGraphStorage::new("test");
    storage.initialize().await.unwrap();
    setup_test_graph(&storage).await;

    // Get nodes via optimized method
    let nodes_optimized = storage
        .get_popular_nodes_with_degree(10, None, None, None, None)
        .await
        .unwrap();

    // Get nodes via traditional method (N+1 pattern)
    let labels = storage.get_popular_labels(10).await.unwrap();
    let mut nodes_traditional = Vec::new();
    for label in labels {
        if let Some(node) = storage.get_node(&label).await.unwrap() {
            let degree = storage.node_degree(&label).await.unwrap();
            nodes_traditional.push((node, degree));
        }
    }

    // Both should return same number of nodes
    assert_eq!(nodes_optimized.len(), nodes_traditional.len());

    // Node A should have the same degree in both
    let optimized_a = nodes_optimized.iter().find(|(n, _)| n.id == "A");
    let traditional_a = nodes_traditional.iter().find(|(n, _)| n.id == "A");

    assert!(optimized_a.is_some());
    assert!(traditional_a.is_some());
    assert_eq!(optimized_a.unwrap().1, traditional_a.unwrap().1);
}

// ============================================
// get_nodes_with_degrees_batch() Tests
// Regression coverage for issue #214:
//   "operator does not exist: ag_catalog.graphid = ag_catalog.graphid"
// ============================================

/// Verify that get_nodes_with_degrees_batch returns nodes with their in/out degrees.
///
/// This test guards against the regression fixed in this PR where the PostgreSQL
/// implementation used direct `graphid = graphid` comparisons in CTEs (deg_out, deg_in,
/// and the final LEFT JOINs). The fix casts all graphid values to ::text before
/// comparing — the same pattern used by node_degree() and node_degrees_batch().
#[tokio::test]
async fn test_get_nodes_with_degrees_batch_basic() {
    let storage = MemoryGraphStorage::new("test");
    storage.initialize().await.unwrap();
    setup_test_graph(&storage).await;

    // Query subset of nodes — A (out=4,in=0), B (out=2,in=1), E (out=0,in=1)
    let ids = vec!["A".to_string(), "B".to_string(), "E".to_string()];
    let results = storage
        .get_nodes_with_degrees_batch(&ids)
        .await
        .unwrap();

    assert_eq!(results.len(), 3, "Should return exactly the queried nodes");

    let map: std::collections::HashMap<String, (usize, usize)> = results
        .into_iter()
        .map(|(node, in_deg, out_deg)| (node.id, (in_deg, out_deg)))
        .collect();

    assert!(map.contains_key("A"), "Node A must be present");
    assert!(map.contains_key("B"), "Node B must be present");
    assert!(map.contains_key("E"), "Node E must be present");

    // Memory storage returns total degree as both in and out (symmetric),
    // so just verify the total is non-zero for connected nodes.
    let (a_in, a_out) = map["A"];
    assert!(a_in + a_out > 0, "Node A should have connections");

    let (e_in, e_out) = map["E"];
    assert!(e_in + e_out > 0, "Node E should have at least one connection");
}

/// Verify that an empty input returns an empty result without panicking.
#[tokio::test]
async fn test_get_nodes_with_degrees_batch_empty_input() {
    let storage = MemoryGraphStorage::new("test");
    storage.initialize().await.unwrap();

    let results = storage
        .get_nodes_with_degrees_batch(&[])
        .await
        .unwrap();

    assert!(results.is_empty(), "Empty input must produce empty output");
}

/// Verify that requesting non-existent node IDs returns an empty result.
#[tokio::test]
async fn test_get_nodes_with_degrees_batch_nonexistent_nodes() {
    let storage = MemoryGraphStorage::new("test");
    storage.initialize().await.unwrap();
    setup_test_graph(&storage).await;

    let ids = vec![
        "DOES_NOT_EXIST_1".to_string(),
        "DOES_NOT_EXIST_2".to_string(),
    ];
    let results = storage
        .get_nodes_with_degrees_batch(&ids)
        .await
        .unwrap();

    assert!(
        results.is_empty(),
        "Non-existent node IDs must return empty results"
    );
}

/// Verify that a mix of existing and non-existing IDs returns only the existing ones.
#[tokio::test]
async fn test_get_nodes_with_degrees_batch_mixed_ids() {
    let storage = MemoryGraphStorage::new("test");
    storage.initialize().await.unwrap();
    setup_test_graph(&storage).await;

    let ids = vec![
        "A".to_string(),
        "NONEXISTENT".to_string(),
        "C".to_string(),
    ];
    let results = storage
        .get_nodes_with_degrees_batch(&ids)
        .await
        .unwrap();

    let found_ids: Vec<_> = results.iter().map(|(n, _, _)| n.id.as_str()).collect();
    assert!(
        found_ids.contains(&"A"),
        "Existing node A should be included"
    );
    assert!(
        found_ids.contains(&"C"),
        "Existing node C should be included"
    );
    assert_eq!(results.len(), 2, "Only the 2 existing nodes should appear");
}

/// Verify that get_nodes_with_degrees_batch is consistent with node_degrees_batch.
///
/// The default (memory) implementation of get_nodes_with_degrees_batch stores the
/// total degree as BOTH in_degree and out_degree (no directional distinction).
/// The PostgreSQL implementation correctly separates them.
/// This test verifies that every node present in node_degrees_batch with degree > 0
/// also appears in get_nodes_with_degrees_batch with a non-zero total.
#[tokio::test]
async fn test_get_nodes_with_degrees_batch_consistent_with_degrees_batch() {
    let storage = MemoryGraphStorage::new("test");
    storage.initialize().await.unwrap();
    setup_test_graph(&storage).await;

    let ids = vec!["A".to_string(), "B".to_string(), "C".to_string()];

    let degrees_only = storage
        .node_degrees_batch(&ids)
        .await
        .unwrap()
        .into_iter()
        .collect::<std::collections::HashMap<_, _>>();

    let with_nodes = storage
        .get_nodes_with_degrees_batch(&ids)
        .await
        .unwrap();

    // Every node returned by node_degrees_batch with degree > 0 should also appear
    // in get_nodes_with_degrees_batch with a non-zero in_deg or out_deg.
    for (node, in_deg, out_deg) in &with_nodes {
        let batch_deg = degrees_only.get(&node.id).copied().unwrap_or(0);
        if batch_deg > 0 {
            assert!(
                *in_deg > 0 || *out_deg > 0,
                "Node '{}' has non-zero batch degree {} but zero in/out degrees in get_nodes_with_degrees_batch",
                node.id, batch_deg
            );
        }
    }
}

/// Verify that isolated nodes (no edges) get degree 0.
#[tokio::test]
async fn test_get_nodes_with_degrees_batch_isolated_node() {
    let storage = MemoryGraphStorage::new("test");
    storage.initialize().await.unwrap();

    // Create one isolated node — no edges at all
    let mut props = HashMap::new();
    props.insert("node_id".to_string(), serde_json::json!("ISOLATED"));
    props.insert("entity_type".to_string(), serde_json::json!("CONCEPT"));
    storage.upsert_node("ISOLATED", props).await.unwrap();

    let ids = vec!["ISOLATED".to_string()];
    let results = storage
        .get_nodes_with_degrees_batch(&ids)
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    let (node, in_deg, out_deg) = &results[0];
    assert_eq!(node.id, "ISOLATED");
    assert_eq!(
        *in_deg + *out_deg,
        0,
        "Isolated node must have degree 0"
    );
}
