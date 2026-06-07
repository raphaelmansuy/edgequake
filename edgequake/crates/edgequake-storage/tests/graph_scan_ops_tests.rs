//! SPEC-006 GraphScanOps proof tests.

use edgequake_storage::adapters::memory::MemoryGraphStorage;
use edgequake_storage::traits::{
    EdgeListFilter, GraphScanOps, GraphStorageMutateOps, NodeListFilter,
};
use serde_json::json;
use std::collections::HashMap;

const TENANT: &str = "scan-tenant";
const WORKSPACE: &str = "scan-workspace";

async fn seed_nodes(storage: &MemoryGraphStorage, count: usize) {
    for i in 0..count {
        let mut props = HashMap::new();
        props.insert("tenant_id".to_string(), json!(TENANT));
        props.insert("workspace_id".to_string(), json!(WORKSPACE));
        props.insert(
            "entity_type".to_string(),
            json!(if i % 2 == 0 { "PERSON" } else { "ORG" }),
        );
        storage
            .upsert_node(&format!("NODE_{:05}", i), props)
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn graph_scan_ops_list_nodes_pagination() {
    let storage = MemoryGraphStorage::new("scan-test");
    seed_nodes(&storage, 500).await;

    let filter = NodeListFilter {
        tenant_id: Some(TENANT.to_string()),
        workspace_id: Some(WORKSPACE.to_string()),
        entity_type: Some("PERSON".to_string()),
        search: None,
    };

    let page = storage.list_nodes_filtered(&filter, 0, 20).await.unwrap();

    assert_eq!(page.total, 250);
    assert_eq!(page.items.len(), 20);
}

#[tokio::test]
async fn graph_scan_ops_list_edges_empty_workspace() {
    let storage = MemoryGraphStorage::new("scan-test");
    let filter = EdgeListFilter {
        tenant_id: Some(TENANT.to_string()),
        workspace_id: Some(WORKSPACE.to_string()),
        relationship_type: None,
    };

    let page = storage.list_edges_filtered(&filter, 0, 10).await.unwrap();
    assert_eq!(page.total, 0);
    assert!(page.items.is_empty());
}

#[tokio::test]
async fn graph_scan_ops_find_by_source_prefix() {
    let storage = MemoryGraphStorage::new("scan-test");
    let mut props = HashMap::new();
    props.insert("tenant_id".to_string(), json!(TENANT));
    props.insert("workspace_id".to_string(), json!(WORKSPACE));
    props.insert(
        "source_ids".to_string(),
        json!(["doc-abc-chunk-1", "doc-xyz-chunk-2"]),
    );
    storage.upsert_node("SOURCED_NODE", props).await.unwrap();

    let filter = NodeListFilter {
        tenant_id: Some(TENANT.to_string()),
        workspace_id: Some(WORKSPACE.to_string()),
        entity_type: None,
        search: None,
    };

    let found = storage
        .find_nodes_by_source_prefixes(&filter, &["doc-abc".to_string()])
        .await
        .unwrap();

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, "SOURCED_NODE");
}

#[tokio::test]
async fn graph_scan_ops_find_edge_by_relationship_id() {
    let storage = MemoryGraphStorage::new("scan-test");
    let mut props = HashMap::new();
    props.insert("tenant_id".to_string(), json!(TENANT));
    props.insert("workspace_id".to_string(), json!(WORKSPACE));
    props.insert("keywords".to_string(), json!("works_at"));
    storage
        .upsert_node(
            "ALICE",
            HashMap::from([
                ("tenant_id".to_string(), json!(TENANT)),
                ("workspace_id".to_string(), json!(WORKSPACE)),
            ]),
        )
        .await
        .unwrap();
    storage
        .upsert_node(
            "GOOGLE",
            HashMap::from([
                ("tenant_id".to_string(), json!(TENANT)),
                ("workspace_id".to_string(), json!(WORKSPACE)),
            ]),
        )
        .await
        .unwrap();
    storage.upsert_edge("ALICE", "GOOGLE", props).await.unwrap();

    let filter = EdgeListFilter {
        tenant_id: Some(TENANT.to_string()),
        workspace_id: Some(WORKSPACE.to_string()),
        relationship_type: None,
    };

    let found = storage
        .find_edge_by_relationship_id(&filter, "ALICE_GOOGLE")
        .await
        .unwrap()
        .expect("edge");

    assert_eq!(found.source, "ALICE");
    assert_eq!(found.target, "GOOGLE");
}
