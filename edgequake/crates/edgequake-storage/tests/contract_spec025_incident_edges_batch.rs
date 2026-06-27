//! SPEC-025 6.2 — batched incident edge lookup contract.

use std::collections::HashMap;

use edgequake_storage::adapters::memory::MemoryGraphStorage;
use edgequake_storage::traits::{GraphReadView, GraphStorage, GraphStorageMutateOps};

#[test]
fn contract_incident_edges_batch_uses_sql_not_cypher() {
    let edges = include_str!("../src/adapters/postgres/graph/edges_ops.rs");
    assert!(
        edges.contains("start_id::text = sv.id::text"),
        "incident edges batch must cast graphid to text for AGE joins"
    );
    assert!(
        !edges.contains("e.start_id = sv.id"),
        "direct graphid = graphid joins fail on AGE 1.6.0"
    );
    assert!(
        !edges.contains("UNWIND [{}] AS nid MATCH"),
        "pg_get_incident_edges_batch must not use Cypher UNWIND"
    );
}

#[tokio::test]
async fn contract_spec025_incident_edges_batch_matches_per_node_union() {
    let graph = MemoryGraphStorage::new("batch-contract");
    graph.initialize().await.unwrap();
    for (src, tgt) in [("A", "B"), ("B", "C"), ("A", "D")] {
        graph.upsert_edge(src, tgt, HashMap::new()).await.unwrap();
    }

    let view = GraphReadView::new(&graph);
    let node_ids = vec!["A".to_string(), "B".to_string()];
    let batch = view.get_incident_edges_batch(&node_ids).await.unwrap();

    let mut per_node = Vec::new();
    for id in &node_ids {
        per_node.extend(view.get_node_edges(id).await.unwrap());
    }

    let batch_keys: std::collections::HashSet<_> = batch
        .iter()
        .map(|e| (e.source.clone(), e.target.clone()))
        .collect();
    let per_node_keys: std::collections::HashSet<_> = per_node
        .iter()
        .map(|e| (e.source.clone(), e.target.clone()))
        .collect();

    assert_eq!(batch_keys, per_node_keys);
    assert_eq!(batch.len(), 3, "A-B, A-D, B-C");
}
