//! SPEC-065 — partial HNSW lifecycle (opt-in productization).
//!
//! - Flag off → no partial created
//! - Flag on + shared table + enough rows → partial created via query_filtered
//! - Empty workspace_id → error
//! - Dedicated `_ws_` table → skip (no-op)
//!
//! Tests that mutate process env must hold `ENV_LOCK` (cargo runs them in parallel).
#![cfg(feature = "postgres")]
// Env serialization requires the mutex across the whole async test body.
#![allow(clippy::await_holding_lock)]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

use edgequake_storage::traits::{MetadataFilter, VectorStorage};
use edgequake_storage::{PgVectorStorage, VectorIndexType};
use std::sync::{Mutex, MutexGuard, OnceLock};

const DIM: usize = 32;
const WS: &str = "ws-hot-065";
const ROWS: usize = 1_200; // above default partial_min_rows=1000

fn env_lock() -> MutexGuard<'static, ()> {
    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn emb(seed: f32) -> Vec<f32> {
    (0..DIM)
        .map(|i| ((i as f32 + seed) * 0.017).sin())
        .collect()
}

async fn seed_shared(storage: &PgVectorStorage) {
    let chunk = 200usize;
    for batch_start in (0..ROWS).step_by(chunk) {
        let end = (batch_start + chunk).min(ROWS);
        let batch: Vec<_> = (batch_start..end)
            .map(|i| {
                (
                    format!("s065-{i}"),
                    emb(i as f32),
                    serde_json::json!({
                        "type": "chunk",
                        "workspace_id": WS,
                        "tenant_id": "t-065",
                    }),
                )
            })
            .collect();
        storage.upsert(&batch).await.expect("upsert");
    }
}

#[tokio::test]
async fn e2e_spec065_partial_off_creates_nothing() {
    let _guard = env_lock();
    let Some(cfg) = postgres_test_config::require_or_skip_postgres("spec065_off") else {
        return;
    };
    std::env::remove_var("EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE");
    std::env::remove_var("EDGEQUAKE_HNSW_PARTIAL_MIN_ROWS");

    let storage =
        PgVectorStorage::with_dimension(cfg.with_vector_index(VectorIndexType::HNSW), DIM);
    storage.initialize().await.expect("init");
    seed_shared(&storage).await;

    let created = storage.ensure_hot_workspace_ann(WS).await.expect("ensure");
    assert!(!created, "flag off must not create partial");
    assert!(
        !storage.partial_ann_index_exists(WS).await.expect("probe"),
        "partial index must not exist when flag off"
    );
}

#[tokio::test]
async fn e2e_spec065_partial_on_via_query_filtered() {
    let _guard = env_lock();
    let Some(cfg) = postgres_test_config::require_or_skip_postgres("spec065_on") else {
        return;
    };
    std::env::set_var("EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE", "1");
    std::env::set_var("EDGEQUAKE_HNSW_PARTIAL_MIN_ROWS", "500");

    let storage =
        PgVectorStorage::with_dimension(cfg.with_vector_index(VectorIndexType::HNSW), DIM);
    storage.initialize().await.expect("init");
    seed_shared(&storage).await;

    let mf = MetadataFilter {
        workspace_id: Some(WS.into()),
        tenant_id: Some("t-065".into()),
        vector_type: Some("chunk".into()),
        document_ids: None,
        modalities: None,
        embedding_model: None,
        kinds: None,
    };
    let hits = storage
        .query_filtered(&emb(0.0), 10, None, Some(&mf))
        .await
        .expect("query_filtered should create partial then search");
    assert!(!hits.is_empty());

    assert!(
        storage.partial_ann_index_exists(WS).await.expect("probe"),
        "partial index must exist after hot query"
    );
    assert!(
        storage.ann_index_exists().await.expect("ann probe"),
        "readiness accepts global or partial"
    );

    // Empty workspace_id fails closed
    let err = storage.ensure_hot_workspace_ann("").await;
    assert!(err.is_err(), "empty workspace_id must error");

    std::env::remove_var("EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE");
    std::env::remove_var("EDGEQUAKE_HNSW_PARTIAL_MIN_ROWS");
}

#[tokio::test]
async fn e2e_spec065_dedicated_ws_table_skips_partial() {
    let _guard = env_lock();
    let Some(cfg) = postgres_test_config::require_or_skip_postgres("default_ws_abcd1234") else {
        return;
    };
    // Namespace prefix from postgres_test_config is `{prefix}_{uuid8}` —
    // force a dedicated-looking table via namespace containing `_ws_`.
    let mut cfg = cfg;
    cfg.namespace = format!("eq_test_ws_{}", &uuid::Uuid::new_v4().to_string()[..8]);
    std::env::set_var("EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE", "1");
    std::env::set_var("EDGEQUAKE_HNSW_PARTIAL_MIN_ROWS", "1");

    let storage =
        PgVectorStorage::with_dimension(cfg.with_vector_index(VectorIndexType::HNSW), DIM);
    storage.initialize().await.expect("init");
    assert!(
        storage.is_dedicated_workspace_table(),
        "fixture must look like dedicated ws table"
    );
    seed_shared(&storage).await;
    let created = storage.ensure_hot_workspace_ann(WS).await.expect("ensure");
    assert!(!created, "dedicated table must skip partial");
    assert!(!storage.partial_ann_index_exists(WS).await.expect("probe"));

    std::env::remove_var("EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE");
    std::env::remove_var("EDGEQUAKE_HNSW_PARTIAL_MIN_ROWS");
}
