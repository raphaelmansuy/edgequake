//! SPEC-025 6.3 — community_id push-down filter contract.

use std::collections::HashMap;

use edgequake_storage::adapters::memory::MemoryGraphStorage;
use edgequake_storage::traits::{
    GraphScanOps, GraphStorage, GraphStorageMutateOps, NodeListFilter,
};

#[tokio::test]
async fn contract_spec025_nodes_filtered_by_community_id() {
    let graph = MemoryGraphStorage::new("community-filter");
    graph.initialize().await.unwrap();

    for (id, cid) in [("A", 1_u64), ("B", 1_u64), ("C", 2_u64)] {
        let mut props = HashMap::new();
        props.insert("community_id".to_string(), serde_json::json!(cid));
        graph.upsert_node(id, props).await.unwrap();
    }

    let filter = NodeListFilter {
        community_ids: Some(vec![1]),
        ..Default::default()
    };
    let page = graph.list_nodes_filtered(&filter, 0, 10).await.unwrap();
    let ids: Vec<_> = page.items.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, vec!["A", "B"]);
    assert_eq!(page.total, 2);
}
