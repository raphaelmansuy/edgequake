//! SPEC-149 lineage indexes against a live Postgres:
//! - Migration 157 / `ensure_indexes`: lineage GIN pending list is bounded.
//! - Shared count SQL (analytics + StorageInspector INV-C) sees both arrays
//!   and bare document-id tokens.
//! - Migration 158 restores orphaned document lineage (missing-only).
//!
//! Soft-skips without DATABASE_URL unless EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1.

#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;
#[path = "support/projection_fixture.rs"]
mod projection_fixture;

use std::collections::HashMap;

use edgequake_storage::{
    node_counts_by_source_prefixes_sql,
    traits::{GraphPropertyWriteMode, GraphStorage, GraphStorageMutateOps},
    PostgresAGEGraphStorage, PostgresConfig, PostgresPool, LINEAGE_GIN_INDEXES,
    LINEAGE_GIN_PENDING_LIST_LIMIT_KB,
};
use projection_fixture::setup_scope;
use uuid::Uuid;

fn wanted_option() -> String {
    format!("gin_pending_list_limit={LINEAGE_GIN_PENDING_LIST_LIMIT_KB}")
}

async fn unbounded_lineage_indexes(pool: &sqlx::PgPool, graph: &str) -> Vec<String> {
    let names: Vec<String> = LINEAGE_GIN_INDEXES.iter().map(|s| s.to_string()).collect();
    sqlx::query_scalar(
        "SELECT c.relname::text FROM pg_class c
         JOIN pg_namespace n ON n.oid = c.relnamespace
         WHERE n.nspname = $1 AND c.relkind = 'i' AND c.relname = ANY($2::text[])
           AND NOT (COALESCE(c.reloptions, '{}') @> ARRAY[$3::text])
         ORDER BY 1",
    )
    .bind(graph)
    .bind(&names)
    .bind(wanted_option())
    .fetch_all(pool)
    .await
    .expect("reloption probe")
}

async fn lineage_index_count(pool: &sqlx::PgPool, graph: &str) -> i64 {
    let names: Vec<String> = LINEAGE_GIN_INDEXES.iter().map(|s| s.to_string()).collect();
    sqlx::query_scalar(
        "SELECT count(*) FROM pg_indexes WHERE schemaname = $1 AND indexname = ANY($2::text[])",
    )
    .bind(graph)
    .bind(&names)
    .fetch_one(pool)
    .await
    .expect("index count")
}

async fn initialize_graph(config: &PostgresConfig, pool: &sqlx::PgPool) -> PostgresAGEGraphStorage {
    let pg_pool = PostgresPool::from_existing(pool.clone(), config.clone());
    let graph = PostgresAGEGraphStorage::with_pool(pg_pool, config.clone());
    graph.initialize().await.expect("init graph");
    graph
}

/// Seed one node whose only lineage array is `key` (raw write, no mirroring).
async fn seed_single_array_node(
    graph: &PostgresAGEGraphStorage,
    workspace_id: Uuid,
    name: &str,
    key: &str,
    chunk: &str,
) {
    let node_id = edgequake_storage::canonical_graph_node_id(workspace_id, name);
    let props: HashMap<String, serde_json::Value> = [
        ("entity_type".to_string(), serde_json::json!("TEST")),
        (
            "workspace_id".to_string(),
            serde_json::json!(workspace_id.to_string()),
        ),
        (key.to_string(), serde_json::json!([chunk])),
    ]
    .into_iter()
    .collect();
    graph
        .upsert_nodes_batch_with_mode(&[(node_id, props)], GraphPropertyWriteMode::Replace)
        .await
        .expect("seed node");
}

/// Same raw-pool round-trip StorageInspector INV-C performs.
#[tokio::test]
async fn shared_count_sql_counts_nodes_with_either_lineage_array() {
    let Some((config, pool, _tenant, workspace_id)) = setup_scope("spec149_count_sql").await else {
        return;
    };
    let graph = initialize_graph(&config, &pool).await;
    let doc = Uuid::new_v4();
    let empty_doc = Uuid::new_v4();
    let chunk = format!("{doc}-chunk-0");
    seed_single_array_node(
        &graph,
        workspace_id,
        "COUNT_CHUNK_IDS_ONLY",
        "source_chunk_ids",
        &chunk,
    )
    .await;
    seed_single_array_node(
        &graph,
        workspace_id,
        "COUNT_SOURCE_IDS_ONLY",
        "source_ids",
        &chunk,
    )
    .await;
    seed_single_array_node(
        &graph,
        workspace_id,
        "COUNT_BARE_DOCUMENT_TOKEN",
        "source_ids",
        &doc.to_string(),
    )
    .await;

    let prefixes = vec![format!("{doc}-chunk-"), format!("{empty_doc}-chunk-")];
    let rows: Vec<(String, i64)> = sqlx::query_as(&node_counts_by_source_prefixes_sql(
        &config.age_graph_name(),
    ))
    .bind(&prefixes)
    .bind(4_i32)
    .fetch_all(&pool)
    .await
    .expect("shared count SQL");
    assert_eq!(
        rows,
        vec![(prefixes[0].clone(), 3), (prefixes[1].clone(), 0)],
        "one row per prefix in input order; chunk ids in either array and the \
         bare document id (discovery's token set) each count once"
    );
}

#[tokio::test]
async fn new_graph_lineage_gin_indexes_are_bounded_and_self_heal() {
    let Some((config, pool, _tenant, _workspace)) = setup_scope("spec149_gin_limit").await else {
        return;
    };
    let graph = config.age_graph_name();
    initialize_graph(&config, &pool).await;

    assert_eq!(lineage_index_count(&pool, &graph).await, 4);
    assert!(
        unbounded_lineage_indexes(&pool, &graph).await.is_empty(),
        "fresh graph lineage GIN indexes must carry {}",
        wanted_option()
    );

    sqlx::query(&format!(
        r#"ALTER INDEX "{graph}".idx_node_source_ids_gin RESET (gin_pending_list_limit)"#
    ))
    .execute(&pool)
    .await
    .expect("reset reloption");
    assert_eq!(
        unbounded_lineage_indexes(&pool, &graph).await,
        vec!["idx_node_source_ids_gin".to_string()]
    );

    initialize_graph(&config, &pool).await;
    assert!(
        unbounded_lineage_indexes(&pool, &graph).await.is_empty(),
        "ensure_indexes must restore the pending-list bound"
    );
}

#[tokio::test]
async fn migration_157_bounds_existing_indexes_idempotently() {
    let Some((config, pool, _tenant, _workspace)) = setup_scope("spec149_m157").await else {
        return;
    };
    let graph = config.age_graph_name();
    initialize_graph(&config, &pool).await;
    for index in LINEAGE_GIN_INDEXES {
        sqlx::query(&format!(
            r#"ALTER INDEX "{graph}"."{index}" RESET (gin_pending_list_limit)"#
        ))
        .execute(&pool)
        .await
        .expect("reset reloption");
    }
    assert_eq!(unbounded_lineage_indexes(&pool, &graph).await.len(), 4);

    let m157 = include_str!("../../../migrations/157_graph_lineage_gin_pending_limit.sql");
    for _ in 0..2 {
        sqlx::raw_sql(m157)
            .execute(&pool)
            .await
            .expect("apply M157");
        assert!(unbounded_lineage_indexes(&pool, &graph).await.is_empty());
    }

    let verify = include_str!("../../../migrations/support/157/verify.sql");
    sqlx::raw_sql(verify)
        .execute(&pool)
        .await
        .expect("M157 verify");
}
