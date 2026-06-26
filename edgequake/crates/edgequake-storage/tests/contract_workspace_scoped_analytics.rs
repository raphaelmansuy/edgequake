//! P-G12 contract test (RC-17 / SPEC-021): analytics must be workspace-scoped.
//!
//! Acceptance (plan-19 §5 P-G12): `node_count_by_workspace(W_A)` must NOT count
//! nodes belonging to `W_B`. A workspace with zero nodes must return 0 (E32),
//! not the global count. This locks in that the cross-workspace count leak
//! documented in file 18 cannot recur: `node_count_by_workspace` is now a
//! *required* trait method (no workspace-ignoring default).

use std::sync::Arc;

use edgequake_storage::traits::GraphStorageAnalyticsOps;
use edgequake_storage::traits::GraphStorageMutateOps;
use edgequake_storage::{GraphStorage, MemoryGraphStorage};

#[tokio::test]
async fn node_count_by_workspace_does_not_leak_across_workspaces() {
    let graph = Arc::new(MemoryGraphStorage::new("test"));
    graph.initialize().await.unwrap();

    let ws_a = uuid::Uuid::new_v4();
    let ws_b = uuid::Uuid::new_v4();

    // Seed 2 nodes in workspace A, 1 node in workspace B, and 1 unscoped node.
    let mut a1 = std::collections::HashMap::new();
    a1.insert(
        "workspace_id".to_string(),
        serde_json::json!(ws_a.to_string()),
    );
    graph.upsert_node("A1", a1).await.unwrap();

    let mut a2 = std::collections::HashMap::new();
    a2.insert(
        "workspace_id".to_string(),
        serde_json::json!(ws_a.to_string()),
    );
    graph.upsert_node("A2", a2).await.unwrap();

    let mut b1 = std::collections::HashMap::new();
    b1.insert(
        "workspace_id".to_string(),
        serde_json::json!(ws_b.to_string()),
    );
    graph.upsert_node("B1", b1).await.unwrap();

    let unscoped = std::collections::HashMap::new();
    graph.upsert_node("UNSCOPED", unscoped).await.unwrap();

    // E32: workspace A sees exactly its 2 nodes (not B's, not unscoped).
    assert_eq!(
        graph.node_count_by_workspace(&ws_a).await.unwrap(),
        2,
        "workspace A must count only its own nodes"
    );
    assert_eq!(
        graph.node_count_by_workspace(&ws_b).await.unwrap(),
        1,
        "workspace B must count only its own nodes"
    );

    // An empty (non-existent) workspace must return 0, not the global count.
    let ws_empty = uuid::Uuid::new_v4();
    assert_eq!(
        graph.node_count_by_workspace(&ws_empty).await.unwrap(),
        0,
        "an empty workspace must return 0, not the global node count (RC-17)"
    );
}

#[tokio::test]
async fn edge_count_by_workspace_does_not_leak_across_workspaces() {
    let graph = Arc::new(MemoryGraphStorage::new("test"));
    graph.initialize().await.unwrap();

    let ws_a = uuid::Uuid::new_v4();
    let ws_b = uuid::Uuid::new_v4();

    let mut props = std::collections::HashMap::new();
    props.insert(
        "workspace_id".to_string(),
        serde_json::json!(ws_a.to_string()),
    );
    graph.upsert_edge("A1", "A2", props).await.unwrap();

    let mut props_b = std::collections::HashMap::new();
    props_b.insert(
        "workspace_id".to_string(),
        serde_json::json!(ws_b.to_string()),
    );
    graph.upsert_edge("B1", "B2", props_b).await.unwrap();

    assert_eq!(
        graph.edge_count_by_workspace(&ws_a).await.unwrap(),
        1,
        "workspace A must count only its own edges"
    );
    assert_eq!(
        graph.edge_count_by_workspace(&ws_b).await.unwrap(),
        1,
        "workspace B must count only its own edges"
    );

    let ws_empty = uuid::Uuid::new_v4();
    assert_eq!(
        graph.edge_count_by_workspace(&ws_empty).await.unwrap(),
        0,
        "an empty workspace must return 0 edges, not the global count"
    );
}
