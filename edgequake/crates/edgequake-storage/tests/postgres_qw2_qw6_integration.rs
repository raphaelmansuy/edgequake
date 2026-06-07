//! DB-gated integration tests for QW2 (single-transaction batch upsert) and
//! QW6 (workspace-scoped vector clear) against a live PostgreSQL + pgvector.
//!
//! These exercise behavior that cannot be observed against the in-memory
//! adapter: the `UNNEST` batch path, intra-batch ID de-duplication, all-or-
//! nothing dimension validation, and the `workspace_id` column OR JSONB
//! fallback delete predicate (which guards the SPEC-007 backfill window).
//!
//! Run: `cargo test -p edgequake-storage --features postgres --test postgres_qw2_qw6_integration`
//!
//! SKIPS cleanly when DATABASE_URL / POSTGRES_PASSWORD is unset (set
//! `EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1` to turn the skip into a hard failure
//! in CI environments that must have a database).

#![cfg(feature = "postgres")]

use std::env;

use edgequake_storage::adapters::postgres::{PgVectorStorage, PostgresConfig, PostgresPool};
use edgequake_storage::traits::VectorStorage;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

const DIM: usize = 8;

/// Build a PostgresConfig from DATABASE_URL (preferred) or discrete POSTGRES_*
/// vars, mirroring the harness used by `postgres_workspace_vector_stats.rs`.
fn test_config(namespace: &str) -> Option<PostgresConfig> {
    if let Ok(url) = env::var("DATABASE_URL") {
        let without_scheme = url.split("://").nth(1)?;
        let (auth, host_path) = without_scheme.split_once('@')?;
        let (user, password) = auth.split_once(':')?;
        let (host_port, db_path) = host_path.split_once('/')?;
        let db = db_path.split('?').next().unwrap_or(db_path).to_string();
        let (host, port) = match host_port.split_once(':') {
            Some((h, p)) => (h.to_string(), p.parse().ok()?),
            None => (host_port.to_string(), 5432),
        };
        return Some(
            PostgresConfig::new(host, port, db, user.to_string(), password.to_string())
                .with_namespace(namespace),
        );
    }

    let password = env::var("POSTGRES_PASSWORD").ok()?;
    Some(
        PostgresConfig::new(
            env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string()),
            env::var("POSTGRES_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5432),
            env::var("POSTGRES_DB").unwrap_or_else(|_| "edgequake".to_string()),
            env::var("POSTGRES_USER").unwrap_or_else(|_| "edgequake".to_string()),
            password,
        )
        .with_namespace(namespace),
    )
}

/// Resolve config or skip (panic only under EDGEQUAKE_REQUIRE_POSTGRES_TESTS).
fn config_or_skip(namespace: &str) -> Option<PostgresConfig> {
    if let Some(cfg) = test_config(namespace) {
        return Some(cfg);
    }
    let strict = env::var("EDGEQUAKE_REQUIRE_POSTGRES_TESTS")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if strict {
        panic!("EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1 but DATABASE_URL/POSTGRES_PASSWORD missing");
    }
    eprintln!("SKIP: DATABASE_URL or POSTGRES_PASSWORD not set");
    None
}

async fn shared_pool(config: &PostgresConfig) -> sqlx::PgPool {
    PgPoolOptions::new()
        .max_connections(4)
        .after_connect(|conn, _| {
            Box::pin(async move {
                sqlx::query("SET search_path TO public")
                    .execute(conn)
                    .await
                    .map(|_| ())
            })
        })
        .connect(&config.connection_url())
        .await
        .expect("postgres pool")
}

fn embedding(seed: f32) -> Vec<f32> {
    vec![seed; DIM]
}

/// QW2: a single `upsert` call with many rows must commit atomically via
/// `UNNEST`, de-duplicate IDs within the batch (last-write-wins), and reject
/// the whole batch if ANY embedding has the wrong dimension.
#[tokio::test]
async fn qw2_batch_upsert_dedup_and_dimension_validation() {
    let Some(config) = config_or_skip("default") else {
        return;
    };

    // Unique namespace → isolated table per run (avoids cross-test bleed).
    let suffix = Uuid::new_v4().to_string().replace('-', "")[..12].to_string();
    let config = config.with_namespace(format!("qw2_{suffix}"));
    let raw_pool = shared_pool(&config).await;
    let pool = PostgresPool::from_existing(raw_pool.clone(), config.clone());
    let store = PgVectorStorage::with_pool_and_dimension(pool, config.clone(), DIM);
    store.initialize().await.expect("initialize");
    let table = format!("public.eq_{}_vectors", config.table_prefix());

    // --- Edge case A: large batch + an in-batch duplicate id (last wins) ---
    let mut batch: Vec<(String, Vec<f32>, serde_json::Value)> = Vec::new();
    for i in 0..2_500u32 {
        batch.push((format!("id-{i}"), embedding(0.1), serde_json::json!({})));
    }
    // Duplicate "id-0": the LATER occurrence (marker=true) must win.
    batch.push((
        "id-0".to_string(),
        embedding(0.9),
        serde_json::json!({"marker": true}),
    ));

    store.upsert(&batch).await.expect("batch upsert");

    // 2_500 unique ids (the duplicate collapses to one row).
    assert_eq!(store.count().await.expect("count"), 2_500);

    // Confirm last-write-wins for the duplicated id by reading its metadata.
    let marker: Option<bool> = sqlx::query_scalar(&format!(
        "SELECT (metadata->>'marker')::bool FROM {table} WHERE id = 'id-0'"
    ))
    .fetch_one(&raw_pool)
    .await
    .expect("read marker");
    assert_eq!(marker, Some(true), "later duplicate occurrence must win");

    // --- Edge case B: all-or-nothing dimension validation ---
    let bad = vec![
        ("good".to_string(), embedding(0.2), serde_json::json!({})),
        (
            "bad".to_string(),
            vec![0.3; DIM + 1], // wrong dimension
            serde_json::json!({}),
        ),
    ];
    let err = store
        .upsert(&bad)
        .await
        .expect_err("dimension mismatch must error");
    assert!(
        err.to_string().contains("dimension"),
        "error should mention dimension: {err}"
    );
    // Neither row from the rejected batch may be committed.
    assert_eq!(
        store.count().await.expect("count after bad batch"),
        2_500,
        "failed batch must not partially commit"
    );

    // --- Edge case C: empty batch is a no-op ---
    store.upsert(&[]).await.expect("empty upsert");
    assert_eq!(store.count().await.expect("count after empty"), 2_500);

    // Cleanup isolated table.
    sqlx::query(&format!("DROP TABLE IF EXISTS {table} CASCADE"))
        .execute(&raw_pool)
        .await
        .ok();
    sqlx::query(&format!(
        "DROP TABLE IF EXISTS public.eq_{}_vectors_stats CASCADE",
        config.table_prefix()
    ))
    .execute(&raw_pool)
    .await
    .ok();
}

/// QW6: `clear_workspace` must delete rows whose `workspace_id` column matches
/// AND legacy rows where only `metadata->>'workspace_id'` matches, while
/// leaving other workspaces untouched.
#[tokio::test]
async fn qw6_clear_workspace_column_and_jsonb_fallback() {
    let Some(config) = config_or_skip("default") else {
        return;
    };

    let suffix = Uuid::new_v4().to_string().replace('-', "")[..12].to_string();
    let config = config.with_namespace(format!("qw6_{suffix}"));
    let raw_pool = shared_pool(&config).await;
    let pool = PostgresPool::from_existing(raw_pool.clone(), config.clone());
    let store = PgVectorStorage::with_pool_and_dimension(pool, config.clone(), DIM);
    store.initialize().await.expect("initialize");
    let table = format!("public.eq_{}_vectors", config.table_prefix());

    let ws_a = Uuid::new_v4();
    let ws_b = Uuid::new_v4();

    // Modern rows: upsert maps metadata->>'workspace_id' into the column.
    store
        .upsert(&[
            (
                "a1".to_string(),
                embedding(0.1),
                serde_json::json!({"workspace_id": ws_a.to_string()}),
            ),
            (
                "a2".to_string(),
                embedding(0.1),
                serde_json::json!({"workspace_id": ws_a.to_string()}),
            ),
            (
                "b1".to_string(),
                embedding(0.1),
                serde_json::json!({"workspace_id": ws_b.to_string()}),
            ),
        ])
        .await
        .expect("upsert modern rows");

    // Legacy row for ws_a: JSONB key present but column left NULL (simulates a
    // pre-backfill row). clear_workspace must still delete it via the OR clause.
    sqlx::query(&format!(
        "INSERT INTO {table} (id, embedding, metadata, workspace_id) \
         VALUES ($1, $2::vector, $3, NULL)"
    ))
    .bind("a_legacy")
    .bind(format!("[{}]", ["0.1"; DIM].join(",")))
    .bind(serde_json::json!({"workspace_id": ws_a.to_string()}))
    .execute(&raw_pool)
    .await
    .expect("insert legacy row");

    assert_eq!(store.count().await.expect("count before clear"), 4);

    // Clear ws_a: must remove a1, a2, AND a_legacy (3 rows), keep b1.
    let deleted = store.clear_workspace(&ws_a).await.expect("clear ws_a");
    assert_eq!(deleted, 3, "must delete 2 column rows + 1 legacy JSONB row");
    assert_eq!(store.count().await.expect("count after clear"), 1);

    // Surviving row is b1 (the other workspace).
    let remaining: String = sqlx::query_scalar(&format!("SELECT id FROM {table}"))
        .fetch_one(&raw_pool)
        .await
        .expect("read remaining id");
    assert_eq!(remaining, "b1");

    // Clearing an unrelated workspace is a no-op (idempotent / safe).
    let again = store.clear_workspace(&ws_a).await.expect("re-clear ws_a");
    assert_eq!(again, 0, "second clear deletes nothing");

    // Cleanup isolated tables.
    sqlx::query(&format!("DROP TABLE IF EXISTS {table} CASCADE"))
        .execute(&raw_pool)
        .await
        .ok();
    sqlx::query(&format!(
        "DROP TABLE IF EXISTS public.eq_{}_vectors_stats CASCADE",
        config.table_prefix()
    ))
    .execute(&raw_pool)
    .await
    .ok();
}
