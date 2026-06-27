//! Graph batch upsert contract (SPEC-017 P2).
//!
//! Verifies `upsert_nodes_batch` / `upsert_edges_batch` semantic parity across backends.

use std::collections::HashMap;

use edgequake_storage::traits::GraphStorage;

/// Batch-insert nodes and edges; assert all entities exist and counts match.
pub async fn assert_graph_batch_upsert<G: GraphStorage + ?Sized>(storage: &G) {
    let nodes: Vec<(String, HashMap<String, serde_json::Value>)> = (0..5)
        .map(|i| {
            let mut props = HashMap::new();
            props.insert("batch_idx".to_string(), serde_json::json!(i));
            (format!("BATCH_NODE_{i}"), props)
        })
        .collect();

    storage.upsert_nodes_batch(&nodes).await.unwrap();

    for (id, _) in &nodes {
        assert!(
            storage.has_node(id).await.unwrap(),
            "batch node missing: {id}"
        );
        let node = storage.get_node(id).await.unwrap().unwrap();
        assert_eq!(node.id, *id);
    }

    let edges: Vec<(String, String, HashMap<String, serde_json::Value>)> = (0..4)
        .map(|i| {
            (
                format!("BATCH_NODE_{i}"),
                format!("BATCH_NODE_{}", i + 1),
                HashMap::new(),
            )
        })
        .collect();

    storage.upsert_edges_batch(&edges).await.unwrap();

    for (src, tgt, _) in &edges {
        assert!(
            storage.has_edge(src, tgt).await.unwrap(),
            "batch edge missing: {src} -> {tgt}"
        );
    }

    assert_eq!(storage.node_count().await.unwrap(), 5);
    assert_eq!(storage.edge_count().await.unwrap(), 4);
}
