//! SPEC-070 — concurrent delete/discovery hot path must not run mid-task DDL.
//!
//! Run:
//! ```bash
//! export DATABASE_URL="$(cat /tmp/edgequake-db-url)"
//! cargo test -p edgequake-storage --features postgres --test e2e_spec070_delete_no_ddl -- --nocapture
//! ```
//!
//! Budgets (warm local Postgres):
//! - Discovery wall for small fixture < 2000ms
//! - Concurrent shared upserts after boot must not create new eq_* triggers

#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

use edgequake_storage::traits::{
    GraphScanOps, GraphStorage, GraphStorageMutateOps, NodeListFilter,
};
use edgequake_storage::PostgresAGEGraphStorage;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

const DOC_A: &str = "spec070-doc-a";
const DOC_B: &str = "spec070-doc-b";
const WALL_BUDGET_MS: u128 = 2000;

fn node_props(id: &str, sources: &[&str]) -> (String, HashMap<String, serde_json::Value>) {
    let mut props = HashMap::new();
    props.insert("node_id".into(), json!(id));
    props.insert("entity_type".into(), json!("CONCEPT"));
    props.insert(
        "source_ids".into(),
        json!(sources.iter().map(|s| s.to_string()).collect::<Vec<_>>()),
    );
    props.insert("tenant_id".into(), json!("t-spec070"));
    props.insert("workspace_id".into(), json!("ws-spec070"));
    (id.to_string(), props)
}

async fn count_eq_triggers(pool: &sqlx::PgPool, graph: &str) -> i64 {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint
        FROM pg_trigger t
        JOIN pg_class c ON c.oid = t.tgrelid
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = $1
          AND NOT t.tgisinternal
          AND t.tgname IN ('trg_eq_sync_node_id', 'trg_eq_sync_edge_ids')
        "#,
    )
    .bind(graph)
    .fetch_one(pool)
    .await
    .unwrap_or(0)
}

#[tokio::test]
async fn e2e_concurrent_hot_path_no_new_eq_triggers_and_gin_discovery() {
    let Some(config) = postgres_test_config::require_or_skip_postgres("spec070_del") else {
        return;
    };

    let storage = Arc::new(PostgresAGEGraphStorage::new(config.clone()));
    storage.initialize().await.expect("graph init");

    // Seed exclusive + shared entities (delete cascade shape).
    let nodes = vec![
        node_props("SPEC070_A", &[DOC_A, &format!("{DOC_A}-chunk-0")]),
        node_props("SPEC070_B", &[DOC_B, &format!("{DOC_B}-chunk-0")]),
        node_props(
            "SPEC070_SHARED",
            &[
                DOC_A,
                DOC_B,
                &format!("{DOC_A}-chunk-0"),
                &format!("{DOC_B}-chunk-0"),
            ],
        ),
    ];
    storage
        .upsert_nodes_batch(&nodes)
        .await
        .expect("seed nodes");

    let pool = postgres_test_config::contract_pg_pool(&config).await;
    let graph = storage.graph_name().to_string();
    let triggers_before = count_eq_triggers(&pool, &graph).await;

    let filter = NodeListFilter {
        tenant_id: Some("t-spec070".into()),
        workspace_id: Some("ws-spec070".into()),
        entity_type: None,
        search: None,
        community_ids: None,
    };
    let prefixes_a = vec![DOC_A.to_string()];
    let prefixes_b = vec![DOC_B.to_string()];

    let start = Instant::now();
    let s1 = Arc::clone(&storage);
    let s2 = Arc::clone(&storage);
    let f1 = filter.clone();
    let f2 = filter.clone();
    let (r1, r2) = tokio::join!(
        async move {
            s1.find_nodes_by_source_prefixes(&f1, &prefixes_a)
                .await
                .expect("discover A");
            // Shared upsert (historically re-entered ensure_indexes / DDL).
            let shared = vec![node_props(
                "SPEC070_SHARED",
                &[DOC_A, DOC_B, &format!("{DOC_A}-chunk-0")],
            )];
            s1.upsert_nodes_batch(&shared).await.expect("upsert A path");
        },
        async move {
            s2.find_nodes_by_source_prefixes(&f2, &prefixes_b)
                .await
                .expect("discover B");
            let shared = vec![node_props(
                "SPEC070_SHARED",
                &[DOC_A, DOC_B, &format!("{DOC_B}-chunk-0")],
            )];
            s2.upsert_nodes_batch(&shared).await.expect("upsert B path");
        }
    );
    let _ = (r1, r2);
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_millis(WALL_BUDGET_MS as u64),
        "concurrent discover+upsert wall {:?} exceeds {}ms budget",
        elapsed,
        WALL_BUDGET_MS
    );

    let triggers_after = count_eq_triggers(&pool, &graph).await;
    assert_eq!(
        triggers_before, triggers_after,
        "hot path must not CREATE/DROP eq_* triggers (before={triggers_before} after={triggers_after})"
    );

    eprintln!(
        "OK SPEC-070: concurrent hot path {:.0}ms, eq triggers stable={}",
        elapsed.as_secs_f64() * 1000.0,
        triggers_after
    );
}
