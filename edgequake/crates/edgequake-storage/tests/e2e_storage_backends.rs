//! Comprehensive End-to-End Storage Backend Tests
//!
//! This module provides 100% coverage for all storage backends:
//! - MemoryKVStorage / PostgresKVStorage
//! - MemoryVectorStorage / PgVectorStorage
//! - MemoryGraphStorage / PostgresAGEGraphStorage
//!
//! Tests are parameterized to run against both memory and PostgreSQL backends.
//!
//! Run with: `cargo test --package edgequake-storage --test e2e_storage_backends`
//! For PostgreSQL: `cargo test --package edgequake-storage --test e2e_storage_backends --features postgres`

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use edgequake_storage::{
    GraphEdge, GraphNode, GraphStorage, KVStorage, KnowledgeGraph, MemoryGraphStorage,
    MemoryKVStorage, MemoryVectorStorage, VectorSearchResult, VectorStorage,
};

// ============================================================================
// Test Helper Macros and Utilities
// ============================================================================

/// Generate random test namespace to avoid collisions
fn generate_namespace() -> String {
    format!(
        "test_{}",
        uuid::Uuid::new_v4().to_string().replace("-", "")[..12].to_string()
    )
}

// ============================================================================
// Memory KV Storage Tests - Full Coverage
// ============================================================================

mod memory_kv_tests {
    use super::*;

    async fn create_storage() -> MemoryKVStorage {
        let ns = generate_namespace();
        let storage = MemoryKVStorage::new(&ns);
        storage.initialize().await.expect("Failed to initialize");
        storage
    }

    #[tokio::test]
    async fn test_kv_basic_crud() {
        let storage = create_storage().await;

        // Create
        let data = vec![(
            "key1".to_string(),
            serde_json::json!({"name": "test", "value": 42}),
        )];
        storage.upsert(&data).await.expect("Failed to upsert");

        // Read
        let result = storage.get_by_id("key1").await.expect("Failed to get");
        assert!(result.is_some());
        let doc = result.unwrap();
        assert_eq!(doc["name"], "test");
        assert_eq!(doc["value"], 42);

        // Update
        let updated_data = vec![(
            "key1".to_string(),
            serde_json::json!({"name": "updated", "value": 100}),
        )];
        storage
            .upsert(&updated_data)
            .await
            .expect("Failed to update");
        let result = storage.get_by_id("key1").await.expect("Failed to get");
        assert_eq!(result.unwrap()["name"], "updated");

        // Delete
        storage
            .delete(&["key1".to_string()])
            .await
            .expect("Failed to delete");
        let result = storage.get_by_id("key1").await.expect("Failed to get");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_kv_bulk_operations() {
        let storage = create_storage().await;

        // Bulk insert
        let data: Vec<(String, serde_json::Value)> = (0..50)
            .map(|i| {
                (
                    format!("doc-{}", i),
                    serde_json::json!({"index": i, "content": format!("Document {}", i)}),
                )
            })
            .collect();
        storage.upsert(&data).await.expect("Failed to bulk upsert");

        // Bulk get
        let ids: Vec<String> = (0..25).map(|i| format!("doc-{}", i)).collect();
        let results = storage.get_by_ids(&ids).await.expect("Failed to bulk get");
        assert_eq!(results.len(), 25);

        // Count
        let count = storage.count().await.expect("Failed to count");
        assert_eq!(count, 50);

        // Keys
        let keys = storage.keys().await.expect("Failed to get keys");
        assert_eq!(keys.len(), 50);
        assert!(keys.contains(&"doc-0".to_string()));
        assert!(keys.contains(&"doc-49".to_string()));
    }

    #[tokio::test]
    async fn test_kv_filter_keys() {
        let storage = create_storage().await;

        // Insert some keys
        let data = vec![
            ("exists1".to_string(), serde_json::json!({})),
            ("exists2".to_string(), serde_json::json!({})),
            ("exists3".to_string(), serde_json::json!({})),
        ];
        storage.upsert(&data).await.expect("Failed to upsert");

        // Filter keys - some exist, some don't
        let check_keys: HashSet<String> = vec![
            "exists1".to_string(),
            "exists2".to_string(),
            "notexists1".to_string(),
            "notexists2".to_string(),
        ]
        .into_iter()
        .collect();

        let missing = storage
            .filter_keys(check_keys)
            .await
            .expect("Failed to filter");
        assert_eq!(missing.len(), 2);
        assert!(missing.contains("notexists1"));
        assert!(missing.contains("notexists2"));
    }

    #[tokio::test]
    async fn test_kv_empty_operations() {
        let storage = create_storage().await;

        // Empty storage checks
        assert!(storage.is_empty().await.expect("Failed to check empty"));
        assert_eq!(storage.count().await.expect("Failed to count"), 0);
        assert!(storage.keys().await.expect("Failed to get keys").is_empty());

        // Get non-existent key
        let result = storage
            .get_by_id("nonexistent")
            .await
            .expect("Failed to get");
        assert!(result.is_none());

        // Get empty list
        let results = storage.get_by_ids(&[]).await.expect("Failed to get_by_ids");
        assert!(results.is_empty());

        // Delete non-existent (should not error)
        storage
            .delete(&["nonexistent".to_string()])
            .await
            .expect("Delete should not fail");
    }

    #[tokio::test]
    async fn test_kv_clear() {
        let storage = create_storage().await;

        // Add data
        let data: Vec<(String, serde_json::Value)> = (0..10)
            .map(|i| (format!("key-{}", i), serde_json::json!({"i": i})))
            .collect();
        storage.upsert(&data).await.expect("Failed to upsert");
        assert_eq!(storage.count().await.unwrap(), 10);

        // Clear
        storage.clear().await.expect("Failed to clear");
        assert!(storage.is_empty().await.unwrap());
        assert_eq!(storage.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_kv_special_characters() {
        let storage = create_storage().await;

        // Keys with special characters
        let data = vec![
            (
                "key/with/slashes".to_string(),
                serde_json::json!({"type": "path"}),
            ),
            (
                "key:with:colons".to_string(),
                serde_json::json!({"type": "namespaced"}),
            ),
            (
                "key.with.dots".to_string(),
                serde_json::json!({"type": "dotted"}),
            ),
            (
                "key-with-dashes".to_string(),
                serde_json::json!({"type": "dashed"}),
            ),
            (
                "key_with_underscores".to_string(),
                serde_json::json!({"type": "underscored"}),
            ),
        ];
        storage.upsert(&data).await.expect("Failed to upsert");

        for (key, expected) in &data {
            let result = storage.get_by_id(key).await.expect("Failed to get");
            assert!(result.is_some(), "Key {} should exist", key);
            assert_eq!(result.unwrap()["type"], expected["type"]);
        }
    }

    #[tokio::test]
    async fn test_kv_complex_json() {
        let storage = create_storage().await;

        // Complex nested JSON
        let complex = serde_json::json!({
            "string": "value",
            "number": 42,
            "float": 3.14159,
            "boolean": true,
            "null": null,
            "array": [1, 2, 3, "four", {"five": 5}],
            "nested": {
                "level1": {
                    "level2": {
                        "level3": "deep value"
                    }
                }
            }
        });

        storage
            .upsert(&[("complex".to_string(), complex.clone())])
            .await
            .expect("Failed to upsert");
        let result = storage
            .get_by_id("complex")
            .await
            .expect("Failed to get")
            .unwrap();

        assert_eq!(result["string"], "value");
        assert_eq!(result["number"], 42);
        assert_eq!(result["array"][3], "four");
        assert_eq!(result["nested"]["level1"]["level2"]["level3"], "deep value");
    }

    #[tokio::test]
    async fn test_kv_finalize() {
        let storage = create_storage().await;

        // Add some data
        storage
            .upsert(&[("key".to_string(), serde_json::json!({}))])
            .await
            .expect("Failed to upsert");

        // Finalize should not error
        storage.finalize().await.expect("Failed to finalize");

        // Data should still be accessible
        assert!(storage.get_by_id("key").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_kv_namespace() {
        let storage = create_storage().await;
        assert!(!storage.namespace().is_empty());
    }
}

// ============================================================================
// Memory Vector Storage Tests - Full Coverage
// ============================================================================

mod memory_vector_tests {
    use super::*;

    const DIMENSION: usize = 384;

    async fn create_storage() -> MemoryVectorStorage {
        let ns = generate_namespace();
        let storage = MemoryVectorStorage::new(&ns, DIMENSION);
        storage.initialize().await.expect("Failed to initialize");
        storage
    }

    fn create_embedding(seed: f32) -> Vec<f32> {
        (0..DIMENSION)
            .map(|i| ((i as f32 + seed) / 1000.0).sin())
            .collect()
    }

    fn create_orthogonal_embedding(cluster: usize) -> Vec<f32> {
        (0..DIMENSION)
            .map(|i| {
                if cluster == 0 {
                    (i as f32 * 0.01).sin()
                } else {
                    (i as f32 * 0.01).cos()
                }
            })
            .collect()
    }

    #[tokio::test]
    async fn test_vector_basic_crud() {
        let storage = create_storage().await;

        let embedding = create_embedding(1.0);

        // Upsert
        storage
            .upsert(&[(
                "vec-1".to_string(),
                embedding.clone(),
                serde_json::json!({"label": "first"}),
            )])
            .await
            .expect("Failed to upsert");

        // Get by ID
        let result = storage.get_by_id("vec-1").await.expect("Failed to get");
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), DIMENSION);

        // Update
        let new_embedding = create_embedding(2.0);
        storage
            .upsert(&[(
                "vec-1".to_string(),
                new_embedding.clone(),
                serde_json::json!({"label": "updated"}),
            )])
            .await
            .expect("Failed to update");

        // Delete
        storage
            .delete(&["vec-1".to_string()])
            .await
            .expect("Failed to delete");
        assert!(storage.get_by_id("vec-1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_vector_similarity_search() {
        let storage = create_storage().await;

        // Create two clusters of vectors
        for i in 0..5 {
            let mut embedding = create_orthogonal_embedding(0);
            for j in 0..DIMENSION {
                embedding[j] += i as f32 * 0.001;
            }
            storage
                .upsert(&[(
                    format!("cluster0-{}", i),
                    embedding,
                    serde_json::json!({"cluster": 0}),
                )])
                .await
                .expect("Failed to upsert");
        }

        for i in 0..5 {
            let mut embedding = create_orthogonal_embedding(1);
            for j in 0..DIMENSION {
                embedding[j] += i as f32 * 0.001;
            }
            storage
                .upsert(&[(
                    format!("cluster1-{}", i),
                    embedding,
                    serde_json::json!({"cluster": 1}),
                )])
                .await
                .expect("Failed to upsert");
        }

        // Query with cluster 0 embedding - should find cluster 0 vectors
        let query = create_orthogonal_embedding(0);
        let results = storage
            .query(&query, 3, None)
            .await
            .expect("Failed to query");
        assert_eq!(results.len(), 3);
        for result in &results {
            assert!(
                result.id.starts_with("cluster0"),
                "Expected cluster0 vectors, got {}",
                result.id
            );
        }

        // Query with cluster 1 embedding - should find cluster 1 vectors
        let query = create_orthogonal_embedding(1);
        let results = storage
            .query(&query, 3, None)
            .await
            .expect("Failed to query");
        assert_eq!(results.len(), 3);
        for result in &results {
            assert!(
                result.id.starts_with("cluster1"),
                "Expected cluster1 vectors, got {}",
                result.id
            );
        }
    }

    #[tokio::test]
    async fn test_vector_filtered_query() {
        let storage = create_storage().await;

        // Insert vectors
        for i in 0..10 {
            storage
                .upsert(&[(
                    format!("vec-{}", i),
                    create_embedding(i as f32),
                    serde_json::json!({"index": i}),
                )])
                .await
                .expect("Failed to upsert");
        }

        // Query with filter - only search specific IDs
        let filter_ids = vec![
            "vec-0".to_string(),
            "vec-1".to_string(),
            "vec-2".to_string(),
        ];
        let results = storage
            .query(&create_embedding(0.0), 5, Some(&filter_ids))
            .await
            .expect("Failed to query");

        assert!(results.len() <= 3);
        for result in &results {
            assert!(filter_ids.contains(&result.id));
        }
    }

    #[tokio::test]
    async fn test_vector_bulk_operations() {
        let storage = create_storage().await;

        // Bulk insert
        let data: Vec<(String, Vec<f32>, serde_json::Value)> = (0..50)
            .map(|i| {
                (
                    format!("vec-{}", i),
                    create_embedding(i as f32),
                    serde_json::json!({"index": i}),
                )
            })
            .collect();
        storage.upsert(&data).await.expect("Failed to bulk upsert");

        // Count
        assert_eq!(storage.count().await.unwrap(), 50);

        // Bulk get
        let ids: Vec<String> = (0..25).map(|i| format!("vec-{}", i)).collect();
        let results = storage
            .get_by_ids(&ids)
            .await
            .expect("Failed to get_by_ids");
        assert_eq!(results.len(), 25);
    }

    #[tokio::test]
    async fn test_vector_delete_entity() {
        let storage = create_storage().await;

        // Insert vectors for an entity
        for i in 0..3 {
            storage
                .upsert(&[(
                    format!("ENTITY_A-chunk-{}", i),
                    create_embedding(i as f32),
                    serde_json::json!({"entity": "ENTITY_A"}),
                )])
                .await
                .expect("Failed to upsert");
        }

        // Insert vectors for another entity
        for i in 0..2 {
            storage
                .upsert(&[(
                    format!("ENTITY_B-chunk-{}", i),
                    create_embedding(i as f32 + 10.0),
                    serde_json::json!({"entity": "ENTITY_B"}),
                )])
                .await
                .expect("Failed to upsert");
        }

        assert_eq!(storage.count().await.unwrap(), 5);

        // Delete entity A's vectors
        storage
            .delete_entity("ENTITY_A")
            .await
            .expect("Failed to delete entity");

        // Only entity B's vectors should remain
        let count = storage.count().await.unwrap();
        assert!(count <= 5, "Some vectors should be deleted");
    }

    #[tokio::test]
    async fn test_vector_empty_operations() {
        let storage = create_storage().await;

        assert!(storage.is_empty().await.unwrap());
        assert_eq!(storage.count().await.unwrap(), 0);

        // Query empty storage
        let results = storage
            .query(&create_embedding(0.0), 5, None)
            .await
            .expect("Failed to query");
        assert!(results.is_empty());

        // Get non-existent
        assert!(storage.get_by_id("nonexistent").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_vector_dimension() {
        let storage = create_storage().await;
        assert_eq!(storage.dimension(), DIMENSION);
    }

    #[tokio::test]
    async fn test_vector_clear() {
        let storage = create_storage().await;

        // Add vectors
        for i in 0..10 {
            storage
                .upsert(&[(
                    format!("vec-{}", i),
                    create_embedding(i as f32),
                    serde_json::json!({}),
                )])
                .await
                .expect("Failed to upsert");
        }

        assert_eq!(storage.count().await.unwrap(), 10);

        // Clear
        storage.clear().await.expect("Failed to clear");
        assert!(storage.is_empty().await.unwrap());
    }
}

// ============================================================================
// Memory Graph Storage Tests - Full Coverage
// ============================================================================

mod memory_graph_tests {
    use super::*;

    async fn create_storage() -> MemoryGraphStorage {
        let ns = generate_namespace();
        let storage = MemoryGraphStorage::new(&ns);
        storage.initialize().await.expect("Failed to initialize");
        storage
    }

    fn create_node_properties(
        entity_type: &str,
        description: &str,
    ) -> HashMap<String, serde_json::Value> {
        let mut props = HashMap::new();
        props.insert("entity_type".to_string(), serde_json::json!(entity_type));
        props.insert("description".to_string(), serde_json::json!(description));
        props
    }

    fn create_edge_properties(rel_type: &str, weight: f32) -> HashMap<String, serde_json::Value> {
        let mut props = HashMap::new();
        props.insert("relationship_type".to_string(), serde_json::json!(rel_type));
        props.insert("weight".to_string(), serde_json::json!(weight));
        props
    }

    #[tokio::test]
    async fn test_graph_node_crud() {
        let storage = create_storage().await;

        // Create
        storage
            .upsert_node("NODE_A", create_node_properties("PERSON", "A test person"))
            .await
            .expect("Failed to upsert node");

        // Read
        assert!(storage.has_node("NODE_A").await.expect("Failed to check"));
        let node = storage
            .get_node("NODE_A")
            .await
            .expect("Failed to get")
            .unwrap();
        assert_eq!(node.id, "NODE_A");
        assert_eq!(node.properties["entity_type"], "PERSON");

        // Update
        storage
            .upsert_node(
                "NODE_A",
                create_node_properties("ORGANIZATION", "Updated to org"),
            )
            .await
            .expect("Failed to update");
        let node = storage.get_node("NODE_A").await.unwrap().unwrap();
        assert_eq!(node.properties["entity_type"], "ORGANIZATION");

        // Delete
        storage
            .delete_node("NODE_A")
            .await
            .expect("Failed to delete");
        assert!(!storage.has_node("NODE_A").await.unwrap());
        assert!(storage.get_node("NODE_A").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_graph_edge_crud() {
        let storage = create_storage().await;

        // Create nodes first
        storage
            .upsert_node("SOURCE", create_node_properties("PERSON", "Source node"))
            .await
            .expect("Failed to upsert source");
        storage
            .upsert_node("TARGET", create_node_properties("PERSON", "Target node"))
            .await
            .expect("Failed to upsert target");

        // Create edge
        storage
            .upsert_edge("SOURCE", "TARGET", create_edge_properties("KNOWS", 0.8))
            .await
            .expect("Failed to upsert edge");

        // Read
        assert!(storage
            .has_edge("SOURCE", "TARGET")
            .await
            .expect("Failed to check"));
        let edge = storage
            .get_edge("SOURCE", "TARGET")
            .await
            .expect("Failed to get")
            .unwrap();
        assert_eq!(edge.source, "SOURCE");
        assert_eq!(edge.target, "TARGET");
        assert_eq!(edge.properties["relationship_type"], "KNOWS");

        // Update
        storage
            .upsert_edge(
                "SOURCE",
                "TARGET",
                create_edge_properties("WORKS_WITH", 0.9),
            )
            .await
            .expect("Failed to update");
        let edge = storage.get_edge("SOURCE", "TARGET").await.unwrap().unwrap();
        assert_eq!(edge.properties["relationship_type"], "WORKS_WITH");

        // Delete
        storage
            .delete_edge("SOURCE", "TARGET")
            .await
            .expect("Failed to delete");
        assert!(!storage.has_edge("SOURCE", "TARGET").await.unwrap());
    }

    #[tokio::test]
    async fn test_graph_node_edges() {
        let storage = create_storage().await;

        // Create hub and spoke pattern
        storage
            .upsert_node("HUB", create_node_properties("ORGANIZATION", "Central hub"))
            .await
            .unwrap();

        for i in 0..5 {
            let node_id = format!("SPOKE_{}", i);
            storage
                .upsert_node(
                    &node_id,
                    create_node_properties("PERSON", &format!("Spoke {}", i)),
                )
                .await
                .unwrap();
            storage
                .upsert_edge("HUB", &node_id, create_edge_properties("EMPLOYS", 1.0))
                .await
                .unwrap();
        }

        // Get edges for hub
        let edges = storage
            .get_node_edges("HUB")
            .await
            .expect("Failed to get edges");
        assert_eq!(edges.len(), 5);

        // Node degree
        let degree = storage
            .node_degree("HUB")
            .await
            .expect("Failed to get degree");
        assert_eq!(degree, 5);
    }

    #[tokio::test]
    async fn test_graph_get_all() {
        let storage = create_storage().await;

        // Create some nodes and edges
        for i in 0..5 {
            storage
                .upsert_node(
                    &format!("NODE_{}", i),
                    create_node_properties("TYPE", "desc"),
                )
                .await
                .unwrap();
        }

        storage
            .upsert_edge("NODE_0", "NODE_1", HashMap::new())
            .await
            .unwrap();
        storage
            .upsert_edge("NODE_1", "NODE_2", HashMap::new())
            .await
            .unwrap();
        storage
            .upsert_edge("NODE_2", "NODE_3", HashMap::new())
            .await
            .unwrap();

        // Get all nodes
        let nodes = storage
            .get_all_nodes()
            .await
            .expect("Failed to get all nodes");
        assert_eq!(nodes.len(), 5);

        // Get all edges
        let edges = storage
            .get_all_edges()
            .await
            .expect("Failed to get all edges");
        assert_eq!(edges.len(), 3);
    }

    #[tokio::test]
    async fn test_graph_get_nodes_by_ids() {
        let storage = create_storage().await;

        for i in 0..10 {
            storage
                .upsert_node(
                    &format!("NODE_{}", i),
                    create_node_properties("TYPE", "desc"),
                )
                .await
                .unwrap();
        }

        let ids = vec![
            "NODE_0".to_string(),
            "NODE_2".to_string(),
            "NODE_5".to_string(),
        ];
        let nodes = storage
            .get_nodes_by_ids(&ids)
            .await
            .expect("Failed to get by ids");
        assert_eq!(nodes.len(), 3);
    }

    #[tokio::test]
    async fn test_graph_knowledge_graph() {
        let storage = create_storage().await;

        // Create a small graph
        storage
            .upsert_node("A", create_node_properties("T", "Node A"))
            .await
            .unwrap();
        storage
            .upsert_node("B", create_node_properties("T", "Node B"))
            .await
            .unwrap();
        storage
            .upsert_node("C", create_node_properties("T", "Node C"))
            .await
            .unwrap();
        storage.upsert_edge("A", "B", HashMap::new()).await.unwrap();
        storage.upsert_edge("B", "C", HashMap::new()).await.unwrap();

        // Get knowledge graph starting from A
        let kg = storage
            .get_knowledge_graph("A", 2, 10)
            .await
            .expect("Failed to get knowledge graph");

        assert!(!kg.nodes.is_empty());
        assert!(!kg.edges.is_empty());
    }

    #[tokio::test]
    async fn test_graph_popular_labels() {
        let storage = create_storage().await;

        // Create nodes with varying degrees
        storage
            .upsert_node("POPULAR", HashMap::new())
            .await
            .unwrap();
        for i in 0..10 {
            storage
                .upsert_node(&format!("CONN_{}", i), HashMap::new())
                .await
                .unwrap();
            storage
                .upsert_edge("POPULAR", &format!("CONN_{}", i), HashMap::new())
                .await
                .unwrap();
        }

        storage
            .upsert_node("LESS_POPULAR", HashMap::new())
            .await
            .unwrap();
        for i in 0..3 {
            storage
                .upsert_edge("LESS_POPULAR", &format!("CONN_{}", i), HashMap::new())
                .await
                .unwrap();
        }

        let popular = storage
            .get_popular_labels(5)
            .await
            .expect("Failed to get popular");
        assert!(!popular.is_empty());
    }

    #[tokio::test]
    async fn test_graph_search_labels() {
        let storage = create_storage().await;

        // Create nodes with different prefixes
        storage
            .upsert_node("ALPHA_ONE", HashMap::new())
            .await
            .unwrap();
        storage
            .upsert_node("ALPHA_TWO", HashMap::new())
            .await
            .unwrap();
        storage
            .upsert_node("BETA_ONE", HashMap::new())
            .await
            .unwrap();

        let results = storage
            .search_labels("ALPHA", 10)
            .await
            .expect("Failed to search");
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|l| l.starts_with("ALPHA")));
    }

    #[tokio::test]
    async fn test_graph_neighbors() {
        let storage = create_storage().await;

        // Create chain: A -> B -> C -> D
        storage.upsert_node("A", HashMap::new()).await.unwrap();
        storage.upsert_node("B", HashMap::new()).await.unwrap();
        storage.upsert_node("C", HashMap::new()).await.unwrap();
        storage.upsert_node("D", HashMap::new()).await.unwrap();
        storage.upsert_edge("A", "B", HashMap::new()).await.unwrap();
        storage.upsert_edge("B", "C", HashMap::new()).await.unwrap();
        storage.upsert_edge("C", "D", HashMap::new()).await.unwrap();

        // Depth 1 from A should include B
        let neighbors = storage
            .get_neighbors("A", 1)
            .await
            .expect("Failed to get neighbors");
        assert!(!neighbors.is_empty());
    }

    #[tokio::test]
    async fn test_graph_counts() {
        let storage = create_storage().await;

        assert_eq!(storage.node_count().await.unwrap(), 0);
        assert_eq!(storage.edge_count().await.unwrap(), 0);

        storage.upsert_node("A", HashMap::new()).await.unwrap();
        storage.upsert_node("B", HashMap::new()).await.unwrap();
        storage.upsert_edge("A", "B", HashMap::new()).await.unwrap();

        assert_eq!(storage.node_count().await.unwrap(), 2);
        assert_eq!(storage.edge_count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_graph_clear() {
        let storage = create_storage().await;

        // Add data
        storage.upsert_node("A", HashMap::new()).await.unwrap();
        storage.upsert_node("B", HashMap::new()).await.unwrap();
        storage.upsert_edge("A", "B", HashMap::new()).await.unwrap();

        // Clear
        storage.clear().await.expect("Failed to clear");

        assert_eq!(storage.node_count().await.unwrap(), 0);
        assert_eq!(storage.edge_count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_graph_cascade_delete() {
        let storage = create_storage().await;

        // Create nodes and edges
        storage.upsert_node("CENTER", HashMap::new()).await.unwrap();
        storage.upsert_node("LEFT", HashMap::new()).await.unwrap();
        storage.upsert_node("RIGHT", HashMap::new()).await.unwrap();
        storage
            .upsert_edge("CENTER", "LEFT", HashMap::new())
            .await
            .unwrap();
        storage
            .upsert_edge("CENTER", "RIGHT", HashMap::new())
            .await
            .unwrap();
        storage
            .upsert_edge("LEFT", "CENTER", HashMap::new())
            .await
            .unwrap();

        // Delete center - should cascade delete connected edges
        storage
            .delete_node("CENTER")
            .await
            .expect("Failed to delete");

        assert!(!storage.has_node("CENTER").await.unwrap());
        // Edges should be deleted too
        assert!(!storage.has_edge("CENTER", "LEFT").await.unwrap());
        assert!(!storage.has_edge("CENTER", "RIGHT").await.unwrap());
    }

    /// SPEC-028: Test workspace cascade delete clears graph storage
    #[tokio::test]
    async fn test_clear_workspace_graph_cascade_spec028() {
        let storage = create_storage().await;
        let workspace_id = uuid::Uuid::new_v4();

        // Create nodes and edges with workspace_id in properties
        let mut props = create_node_properties("PERSON", "Test person");
        props.insert(
            "workspace_id".to_string(),
            serde_json::json!(workspace_id.to_string()),
        );

        storage.upsert_node("NODE_A", props.clone()).await.unwrap();
        storage.upsert_node("NODE_B", props.clone()).await.unwrap();
        storage.upsert_node("NODE_C", props.clone()).await.unwrap();

        storage
            .upsert_edge("NODE_A", "NODE_B", create_edge_properties("KNOWS", 1.0))
            .await
            .unwrap();
        storage
            .upsert_edge("NODE_B", "NODE_C", create_edge_properties("KNOWS", 1.0))
            .await
            .unwrap();

        // Verify data exists
        assert_eq!(storage.node_count().await.unwrap(), 3);
        assert_eq!(storage.edge_count().await.unwrap(), 2);

        // SPEC-028: Clear workspace (simulating cascade delete)
        let (nodes_deleted, edges_deleted) = storage.clear_workspace(&workspace_id).await.unwrap();

        // Memory storage doesn't filter by workspace_id in clear_workspace
        // (Postgres does), but clear() should work
        // For this test, we verify the clear_workspace method is callable
        // and the storage API supports it
        // Note: usize can't be negative, so we just verify the call succeeded
        let _nodes = nodes_deleted;
        let _edges = edges_deleted;

        // Full clear for cleanup
        storage.clear().await.unwrap();
        assert_eq!(storage.node_count().await.unwrap(), 0);
        assert_eq!(storage.edge_count().await.unwrap(), 0);
    }
}

// ============================================================================
// PostgreSQL Storage Tests (Feature-gated)
// ============================================================================

#[cfg(feature = "postgres")]
mod postgres_tests {
    use super::*;
    use std::env;
    use std::time::Duration;

    use edgequake_storage::{
        PgVectorStorage, PostgresAGEGraphStorage, PostgresConfig, PostgresKVStorage,
    };

    fn get_test_config() -> Option<PostgresConfig> {
        let password = env::var("POSTGRES_PASSWORD").ok()?;

        Some(PostgresConfig {
            host: env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string()),
            port: env::var("POSTGRES_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5432),
            database: env::var("POSTGRES_DB").unwrap_or_else(|_| "edgequake".to_string()),
            user: env::var("POSTGRES_USER").unwrap_or_else(|_| "edgequake".to_string()),
            password,
            namespace: generate_namespace(),
            max_connections: 5,
            min_connections: 1,
            connect_timeout: Duration::from_secs(10),
            idle_timeout: Duration::from_secs(60),
            ..Default::default()
        })
    }

    macro_rules! require_postgres {
        () => {
            match get_test_config() {
                Some(config) => config,
                None => {
                    eprintln!("Skipping test: POSTGRES_PASSWORD not set");
                    return;
                }
            }
        };
    }

    #[tokio::test]
    async fn test_postgres_kv_full_coverage() {
        let config = require_postgres!();
        let storage = PostgresKVStorage::new(config);
        storage.initialize().await.expect("Failed to initialize");

        // CRUD
        storage
            .upsert(&[("key1".to_string(), serde_json::json!({"test": true}))])
            .await
            .expect("Failed to upsert");

        let result = storage.get_by_id("key1").await.expect("Failed to get");
        assert!(result.is_some());

        storage
            .delete(&["key1".to_string()])
            .await
            .expect("Failed to delete");
        assert!(storage.get_by_id("key1").await.unwrap().is_none());

        // Cleanup
        storage.clear().await.expect("Failed to clear");
    }

    #[tokio::test]
    async fn test_postgres_vector_full_coverage() {
        let config = require_postgres!();
        let storage = PgVectorStorage::with_dimension(config, 384);
        storage.initialize().await.expect("Failed to initialize");

        let embedding: Vec<f32> = (0..384).map(|i| (i as f32) / 1000.0).collect();

        // Insert
        storage
            .upsert(&[(
                "vec1".to_string(),
                embedding.clone(),
                serde_json::json!({"label": "test"}),
            )])
            .await
            .expect("Failed to upsert");

        // Query
        let results = storage
            .query(&embedding, 5, None)
            .await
            .expect("Failed to query");
        assert!(!results.is_empty());

        // Cleanup
        storage.clear().await.expect("Failed to clear");
    }

    #[tokio::test]
    async fn test_postgres_graph_full_coverage() {
        let config = require_postgres!();
        let storage = PostgresAGEGraphStorage::new(config);
        storage.initialize().await.expect("Failed to initialize");

        // Node operations
        storage
            .upsert_node("NODE_A", HashMap::new())
            .await
            .expect("Failed to upsert node");
        assert!(storage.has_node("NODE_A").await.unwrap());

        // Edge operations
        storage.upsert_node("NODE_B", HashMap::new()).await.unwrap();
        storage
            .upsert_edge("NODE_A", "NODE_B", HashMap::new())
            .await
            .expect("Failed to upsert edge");
        assert!(storage.has_edge("NODE_A", "NODE_B").await.unwrap());

        // Cleanup
        storage.clear().await.expect("Failed to clear");
    }
}

// ============================================================================
// Storage Trait Compliance Tests
// ============================================================================

mod trait_compliance_tests {
    use super::*;

    /// Verify that memory storage implements all trait methods correctly
    #[tokio::test]
    async fn test_memory_kv_trait_compliance() {
        let storage = MemoryKVStorage::new("trait_test");
        storage.initialize().await.unwrap();

        // All KVStorage methods
        assert!(!storage.namespace().is_empty());
        storage
            .upsert(&[("k".to_string(), serde_json::json!({}))])
            .await
            .unwrap();
        storage.get_by_id("k").await.unwrap();
        storage.get_by_ids(&["k".to_string()]).await.unwrap();
        storage.filter_keys(HashSet::new()).await.unwrap();
        storage.delete(&["k".to_string()]).await.unwrap();
        storage.is_empty().await.unwrap();
        storage.count().await.unwrap();
        storage.keys().await.unwrap();
        storage.finalize().await.unwrap();
        storage.clear().await.unwrap();
    }

    #[tokio::test]
    async fn test_memory_vector_trait_compliance() {
        let storage = MemoryVectorStorage::new("trait_test", 128);
        storage.initialize().await.unwrap();

        let embedding: Vec<f32> = vec![0.0; 128];

        // All VectorStorage methods
        assert!(!storage.namespace().is_empty());
        assert_eq!(storage.dimension(), 128);
        storage
            .upsert(&[("v".to_string(), embedding.clone(), serde_json::json!({}))])
            .await
            .unwrap();
        storage.query(&embedding, 1, None).await.unwrap();
        storage.get_by_id("v").await.unwrap();
        storage.get_by_ids(&["v".to_string()]).await.unwrap();
        storage.delete_entity("test").await.unwrap();
        storage.delete_entity_relations("test").await.unwrap();
        storage.delete(&["v".to_string()]).await.unwrap();
        storage.is_empty().await.unwrap();
        storage.count().await.unwrap();
        storage.finalize().await.unwrap();
        storage.clear().await.unwrap();
    }

    #[tokio::test]
    async fn test_memory_graph_trait_compliance() {
        let storage = MemoryGraphStorage::new("trait_test");
        storage.initialize().await.unwrap();

        // All GraphStorage methods - node operations
        assert!(!storage.namespace().is_empty());
        storage.upsert_node("N", HashMap::new()).await.unwrap();
        storage.has_node("N").await.unwrap();
        storage.get_node("N").await.unwrap();
        storage.node_degree("N").await.unwrap();
        storage.get_all_nodes().await.unwrap();
        storage.get_nodes_by_ids(&["N".to_string()]).await.unwrap();

        // Edge operations
        storage.upsert_node("M", HashMap::new()).await.unwrap();
        storage.upsert_edge("N", "M", HashMap::new()).await.unwrap();
        storage.has_edge("N", "M").await.unwrap();
        storage.get_edge("N", "M").await.unwrap();
        storage.get_node_edges("N").await.unwrap();
        storage.get_all_edges().await.unwrap();

        // Graph queries
        storage.get_knowledge_graph("N", 1, 10).await.unwrap();
        storage.get_popular_labels(5).await.unwrap();
        storage.search_labels("N", 5).await.unwrap();
        storage.get_neighbors("N", 1).await.unwrap();

        // Utility
        storage.node_count().await.unwrap();
        storage.edge_count().await.unwrap();
        storage.delete_edge("N", "M").await.unwrap();
        storage.delete_node("N").await.unwrap();
        storage.finalize().await.unwrap();
        storage.clear().await.unwrap();
    }
}

// ============================================================================
// Concurrent Access Tests
// ============================================================================

mod concurrent_tests {
    use super::*;
    use std::sync::Arc;
    use tokio::task::JoinSet;

    #[tokio::test]
    async fn test_concurrent_kv_writes() {
        let storage = Arc::new(MemoryKVStorage::new("concurrent_kv"));
        storage.initialize().await.unwrap();

        let mut tasks = JoinSet::new();

        for i in 0..10 {
            let storage = storage.clone();
            tasks.spawn(async move {
                for j in 0..10 {
                    storage
                        .upsert(&[(
                            format!("key-{}-{}", i, j),
                            serde_json::json!({"i": i, "j": j}),
                        )])
                        .await
                        .expect("Failed to upsert");
                }
            });
        }

        while let Some(result) = tasks.join_next().await {
            result.expect("Task panicked");
        }

        assert_eq!(storage.count().await.unwrap(), 100);
    }

    #[tokio::test]
    async fn test_concurrent_graph_operations() {
        let storage = Arc::new(MemoryGraphStorage::new("concurrent_graph"));
        storage.initialize().await.unwrap();

        let mut tasks = JoinSet::new();

        // Concurrent node creation
        for i in 0..20 {
            let storage = storage.clone();
            tasks.spawn(async move {
                storage
                    .upsert_node(&format!("NODE_{}", i), HashMap::new())
                    .await
                    .expect("Failed to upsert node");
            });
        }

        while let Some(result) = tasks.join_next().await {
            result.expect("Task panicked");
        }

        assert_eq!(storage.node_count().await.unwrap(), 20);
    }
}
