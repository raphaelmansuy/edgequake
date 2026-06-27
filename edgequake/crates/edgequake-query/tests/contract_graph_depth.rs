//! SPEC-024 2.7 — graph_depth multi-hop BFS contract.

use std::collections::HashMap;

use edgequake_storage::adapters::memory::MemoryGraphStorage;
use edgequake_storage::traits::{GraphReadView, GraphStorage, GraphStorageMutateOps};

#[tokio::test]
async fn contract_graph_depth_two_reaches_second_hop() {
    let graph = MemoryGraphStorage::new("test");
    graph.initialize().await.unwrap();
    graph.upsert_edge("A", "B", HashMap::new()).await.unwrap();
    graph.upsert_edge("B", "C", HashMap::new()).await.unwrap();

    let view = GraphReadView::new(&graph);
    let depth_one =
        edgequake_query::graph_hops::edges_within_depth(&view, &["A".to_string()], 1, 10)
            .await
            .unwrap();
    assert_eq!(depth_one.len(), 1);

    let depth_two =
        edgequake_query::graph_hops::edges_within_depth(&view, &["A".to_string()], 2, 10)
            .await
            .unwrap();
    assert_eq!(depth_two.len(), 2, "depth=2 must include A→B and B→C");
}

#[tokio::test]
async fn contract_graph_depth_respects_max_edges() {
    let graph = MemoryGraphStorage::new("test");
    graph.initialize().await.unwrap();
    for (src, tgt) in [("A", "B"), ("A", "C"), ("A", "D")] {
        graph.upsert_edge(src, tgt, HashMap::new()).await.unwrap();
    }

    let view = GraphReadView::new(&graph);
    let edges = edgequake_query::graph_hops::edges_within_depth(&view, &["A".to_string()], 1, 2)
        .await
        .unwrap();
    assert_eq!(edges.len(), 2);
}
