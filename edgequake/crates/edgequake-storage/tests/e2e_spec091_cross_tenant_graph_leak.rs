//! SPEC-091 IW5: cross-tenant graph isolation (Postgres AGE).
//!
//! Strict list/count APIs must not leak workspace B into workspace A queries.
//! Scoped discovery excludes legacy NULL ownership; maintenance discovery remains explicit.
//!
//! Run:
//!   cargo test -p edgequake-storage --features postgres \
//!     --test e2e_spec091_cross_tenant_graph_leak -- --test-threads=1
#![cfg(feature = "postgres")]

#[path = "support/graph_workspace_contract.rs"]
#[allow(dead_code)]
mod graph_workspace_contract;
#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;
#[path = "support/spec091_w3.rs"]
mod w3;

use edgequake_storage::traits::{
    GraphScanOps, GraphStorage, GraphStorageAnalyticsOps, GraphStorageMutateOps, NodeListFilter,
};
use edgequake_storage::{PostgresAGEGraphStorage, StorageError};
use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};
use std::collections::HashMap;

fn props(map: &[(&str, &str)]) -> HashMap<String, serde_json::Value> {
    graph_workspace_contract::props(map)
}

#[tokio::test]
async fn e2e_spec091_cross_tenant_graph_strict_filter_no_leak() {
    let Some(cfg) = require_or_skip_postgres("spec091_graph_leak") else {
        return;
    };
    let pool = contract_pg_pool(&cfg).await;
    let graph = PostgresAGEGraphStorage::new(cfg);
    graph.initialize().await.expect("graph init");

    let ws_a = w3::seed_workspace(&pool, "graph-leak-a").await;
    let ws_b = w3::seed_workspace(&pool, "graph-leak-b").await;
    let tenant_a = w3::workspace_tenant(&pool, ws_a).await.to_string();
    let tenant_b = w3::workspace_tenant(&pool, ws_b).await.to_string();

    for (id, ws, tenant) in [
        ("NODE_A1", ws_a, tenant_a.as_str()),
        ("NODE_A2", ws_a, tenant_a.as_str()),
        ("NODE_B1", ws_b, tenant_b.as_str()),
    ] {
        graph
            .upsert_node(
                id,
                props(&[
                    ("entity_type", "person"),
                    ("tenant_id", tenant),
                    ("workspace_id", &ws.to_string()),
                ]),
            )
            .await
            .expect("upsert node");
    }

    // Unscoped legacy vertex — must NOT appear in strict workspace A list.
    graph
        .upsert_node("LEGACY_NULL_WS", HashMap::new())
        .await
        .expect("legacy node");

    assert_eq!(
        graph.node_count_by_workspace(&ws_a).await.expect("count a"),
        2,
        "workspace A counts only its nodes"
    );
    assert_eq!(
        graph.node_count_by_workspace(&ws_b).await.expect("count b"),
        1,
        "workspace B counts only its nodes"
    );

    let filter_a = NodeListFilter {
        tenant_id: Some(tenant_a),
        workspace_id: Some(ws_a.to_string()),
        ..Default::default()
    };
    let page_a = graph
        .list_nodes_filtered(&filter_a, 0, 100)
        .await
        .expect("list a");
    assert_eq!(page_a.total, 2);
    let ids_a: Vec<_> = page_a.items.iter().map(|n| n.id.as_str()).collect();
    assert!(ids_a.contains(&"NODE_A1"));
    assert!(ids_a.contains(&"NODE_A2"));
    assert!(!ids_a.contains(&"NODE_B1"));
    assert!(
        !ids_a.contains(&"LEGACY_NULL_WS"),
        "strict filter excludes NULL workspace_id vertices"
    );

    let filter_b = NodeListFilter {
        tenant_id: Some(tenant_b),
        workspace_id: Some(ws_b.to_string()),
        ..Default::default()
    };
    let page_b = graph
        .list_nodes_filtered(&filter_b, 0, 100)
        .await
        .expect("list b");
    assert_eq!(page_b.total, 1);
    assert_eq!(page_b.items[0].id, "NODE_B1");
}

#[tokio::test]
async fn e2e_spec091_cross_tenant_graph_scoped_discovery_excludes_legacy_null() {
    let Some(cfg) = require_or_skip_postgres("spec091_graph_legacy") else {
        return;
    };
    let pool = contract_pg_pool(&cfg).await;
    let graph = PostgresAGEGraphStorage::new(cfg);
    graph.initialize().await.expect("graph init");

    let ws_a = w3::seed_workspace(&pool, "graph-discovery-a").await;
    let ws_b = w3::seed_workspace(&pool, "graph-discovery-b").await;
    let tenant_a = w3::workspace_tenant(&pool, ws_a).await.to_string();
    let tenant_b = w3::workspace_tenant(&pool, ws_b).await.to_string();
    let doc_a = format!("doc-a-{}", ws_a.as_simple());

    // Legacy-null node linked to workspace A document via source_ids.
    let mut legacy = HashMap::new();
    legacy.insert(
        "source_ids".into(),
        serde_json::json!([format!("{doc_a}-chunk-0")]),
    );
    legacy.insert("tenant_id".into(), serde_json::json!(tenant_a));
    graph
        .upsert_node("LEGACY_FOR_A", legacy)
        .await
        .expect("legacy upsert");

    for (id, workspace, tenant) in [
        ("SCOPED_A", ws_a, tenant_a.as_str()),
        ("SCOPED_B", ws_b, tenant_b.as_str()),
    ] {
        graph
            .upsert_node(id, {
                let mut p = props(&[
                    ("entity_type", "org"),
                    ("tenant_id", tenant),
                    ("workspace_id", &workspace.to_string()),
                ]);
                p.insert(
                    "source_ids".into(),
                    serde_json::json!([format!("{doc_a}-chunk-0")]),
                );
                p
            })
            .await
            .expect("scoped node");
    }

    let filter_a = NodeListFilter {
        tenant_id: Some(tenant_a.clone()),
        workspace_id: Some(ws_a.to_string()),
        ..Default::default()
    };
    let found_a = graph
        .find_nodes_by_source_prefixes(&filter_a, std::slice::from_ref(&doc_a))
        .await
        .expect("discover a");
    assert!(
        found_a.iter().any(|n| n.id == "SCOPED_A"),
        "workspace A discovers its owned node"
    );
    assert!(
        !found_a.iter().any(|n| n.id == "LEGACY_FOR_A"),
        "RLS excludes unowned legacy rows from scoped discovery"
    );
    assert!(
        !found_a.iter().any(|n| n.id == "SCOPED_B"),
        "workspace B scoped node must not leak into A discovery"
    );

    let filter_b = NodeListFilter {
        tenant_id: Some(tenant_b),
        workspace_id: Some(ws_b.to_string()),
        ..Default::default()
    };
    let found_b = graph
        .find_nodes_by_source_prefixes(&filter_b, std::slice::from_ref(&doc_a))
        .await
        .expect("discover b");
    assert!(
        found_b.iter().any(|n| n.id == "SCOPED_B"),
        "workspace B sees its scoped node"
    );
    assert_eq!(found_a.len(), 1);
    assert_eq!(found_b.len(), 1);

    for tenant in ["malformed-tenant".to_owned(), tenant_a] {
        let foreign = NodeListFilter {
            tenant_id: Some(tenant),
            workspace_id: Some(ws_b.to_string()),
            ..Default::default()
        };
        let error = graph
            .find_nodes_by_source_prefixes(&foreign, std::slice::from_ref(&doc_a))
            .await
            .expect_err("invalid tenant/workspace pair must fail closed");
        assert!(matches!(error, StorageError::InvalidInput(_)));
    }
    let maintenance = graph
        .find_nodes_by_source_prefixes(&NodeListFilter::default(), &[doc_a])
        .await
        .expect("explicit unscoped maintenance discovery");
    assert!(maintenance.iter().any(|node| node.id == "LEGACY_FOR_A"));
}
