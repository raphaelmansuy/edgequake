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
    GraphScanOps, GraphStorage, GraphStorageAnalyticsOps, GraphStorageMutateOps,
    GraphStorageReadOps, NodeListFilter,
};
use edgequake_storage::{PostgresAGEGraphStorage, StorageError};
use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};
use std::collections::HashMap;

fn props(map: &[(&str, &str)]) -> HashMap<String, serde_json::Value> {
    graph_workspace_contract::props(map)
}

#[tokio::test]
async fn degree_search_preserves_native_and_legacy_endpoint_counts() {
    let Some(cfg) = require_or_skip_postgres("degree_endpoint_parity") else {
        return;
    };
    let pool = contract_pg_pool(&cfg).await;
    let graph = PostgresAGEGraphStorage::new(cfg);
    graph.initialize().await.unwrap();
    let workspace = w3::seed_workspace(&pool, "degree-parity").await;
    let tenant = w3::workspace_tenant(&pool, workspace).await.to_string();
    let workspace = workspace.to_string();
    for id in ["DEGREE_PARITY_A", "DEGREE_PARITY_B", "DEGREE_PARITY_C"] {
        graph
            .upsert_node(
                id,
                props(&[("tenant_id", &tenant), ("workspace_id", &workspace)]),
            )
            .await
            .unwrap();
    }
    for (source, target) in [
        ("DEGREE_PARITY_A", "DEGREE_PARITY_B"),
        ("DEGREE_PARITY_A", "DEGREE_PARITY_C"),
        ("DEGREE_PARITY_B", "DEGREE_PARITY_C"),
    ] {
        graph
            .upsert_edge(
                source,
                target,
                props(&[
                    ("tenant_id", &tenant),
                    ("workspace_id", &workspace),
                    ("relation_type", "RELATED"),
                ]),
            )
            .await
            .unwrap();
    }
    // One null endpoint requires the property fallback; another deliberately
    // disagrees with its property to verify that the native value takes priority.
    sqlx::query(&format!(
        "UPDATE {}.\"EDGE\" SET eq_source_id=NULL, eq_target_id=NULL \
        WHERE eq_source_id='DEGREE_PARITY_A' AND eq_target_id='DEGREE_PARITY_C'",
        graph.graph_name()
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(&format!(
        "UPDATE {}.\"EDGE\" SET eq_source_id='DEGREE_PARITY_A' \
        WHERE eq_source_id='DEGREE_PARITY_B' AND eq_target_id='DEGREE_PARITY_C'",
        graph.graph_name()
    ))
    .execute(&pool)
    .await
    .unwrap();
    let popular: HashMap<_, _> = graph
        .get_popular_nodes_with_degree(10, None, None, Some(&tenant), Some(&workspace))
        .await
        .unwrap()
        .into_iter()
        .map(|(node, degree)| (node.id, degree))
        .collect();
    assert_eq!(popular["DEGREE_PARITY_A"], 3);
    assert_eq!(popular["DEGREE_PARITY_B"], 0);
    assert_eq!(popular["DEGREE_PARITY_C"], 0);
    let search: HashMap<_, _> = graph
        .search_nodes("DEGREE_PARITY", 10, None, Some(&tenant), Some(&workspace))
        .await
        .unwrap()
        .into_iter()
        .map(|(node, degree)| (node.id, degree))
        .collect();
    assert_eq!(search["DEGREE_PARITY_A"], 3);
    assert_eq!(search["DEGREE_PARITY_B"], 1);
    assert_eq!(search["DEGREE_PARITY_C"], 2);
    let ids = vec![
        "DEGREE_PARITY_A".into(),
        "DEGREE_PARITY_B".into(),
        "DEGREE_PARITY_C".into(),
    ];
    let batch: HashMap<_, _> = graph
        .node_degrees_batch(&ids)
        .await
        .unwrap()
        .into_iter()
        .collect();
    assert_eq!(
        batch, search,
        "batch and search must preserve the same endpoint counts"
    );
    let isolated = graph
        .get_popular_nodes_with_degree(10, Some(4), None, Some(&tenant), Some(&workspace))
        .await
        .unwrap();
    assert!(isolated.is_empty());
}

#[tokio::test]
async fn fuzzy_label_search_uses_installed_trigram_schema_and_preserves_scope() {
    let Some(cfg) = require_or_skip_postgres("fuzzy_label_scope") else {
        return;
    };
    let pool = contract_pg_pool(&cfg).await;
    let schema: Option<String> = sqlx::query_scalar(
        "SELECT n.nspname FROM pg_catalog.pg_extension e \
         JOIN pg_catalog.pg_namespace n ON n.oid=e.extnamespace WHERE e.extname='pg_trgm'",
    )
    .fetch_optional(&pool)
    .await
    .unwrap();
    assert!(
        schema.is_some(),
        "the database fixture must install pg_trgm"
    );
    let graph = PostgresAGEGraphStorage::new(cfg);
    graph.initialize().await.unwrap();
    let ws_a = w3::seed_workspace(&pool, "fuzzy-a").await;
    let ws_b = w3::seed_workspace(&pool, "fuzzy-b").await;
    let tenant_a = w3::workspace_tenant(&pool, ws_a).await.to_string();
    let tenant_b = w3::workspace_tenant(&pool, ws_b).await.to_string();
    for (label, workspace, tenant) in [
        ("ENTANGLEMENT_ALPHA", ws_a, tenant_a.as_str()),
        ("ENTANGLEMENT_BETA", ws_b, tenant_b.as_str()),
    ] {
        graph
            .upsert_node(
                label,
                props(&[
                    ("label", label),
                    ("tenant_id", tenant),
                    ("workspace_id", &workspace.to_string()),
                ]),
            )
            .await
            .unwrap();
    }
    // Neither FTS nor prefix matching can recover this interior typo.
    // An apostrophe also verifies that the fuzzy query uses a bound value.
    for query in ["ENTANGLEMNT", "ENTANGLEMNT'"] {
        assert_eq!(
            graph
                .search_labels(query, 10, Some(&tenant_a), Some(&ws_a.to_string()))
                .await
                .unwrap(),
            vec!["ENTANGLEMENT_ALPHA"],
            "fuzzy search must recover typos without leaking the other workspace"
        );
    }
    assert!(graph
        .search_labels(
            "ZXQ_NO_MATCH_7391",
            10,
            Some(&tenant_a),
            Some(&ws_a.to_string())
        )
        .await
        .unwrap()
        .is_empty());
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
