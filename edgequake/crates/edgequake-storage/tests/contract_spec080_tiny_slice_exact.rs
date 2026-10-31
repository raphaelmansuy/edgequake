//! SPEC-080 — tiny-slice exact: Wave-2 planner bias skipped below threshold.

#![cfg(feature = "postgres")]

use edgequake_storage::adapters::postgres::PgVectorStorage;
use edgequake_storage::traits::MetadataFilter;
use edgequake_storage::{ann_exact_max_rows, DEFAULT_ANN_EXACT_MAX_ROWS};
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn default_threshold_is_2000() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::remove_var("EDGEQUAKE_ANN_EXACT_MAX_ROWS");
    assert_eq!(ann_exact_max_rows(), DEFAULT_ANN_EXACT_MAX_ROWS);
    assert_eq!(DEFAULT_ANN_EXACT_MAX_ROWS, 2_000);
}

#[test]
fn bias_skipped_when_workspace_rows_at_or_below_threshold() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("EDGEQUAKE_ANN_EXACT_MAX_ROWS", "2000");
    let mf = MetadataFilter {
        workspace_id: Some("ws-tiny".into()),
        tenant_id: Some("t1".into()),
        vector_type: Some("chunk".into()),
        document_ids: None,
        modalities: None,
        embedding_model: None,
        kinds: None,
    };
    let tiny = PgVectorStorage::wave2_planner_bias_statements(true, true, &mf, Some(500));
    assert!(tiny.is_empty(), "tiny slice must skip bias: {tiny:?}");
    let large = PgVectorStorage::wave2_planner_bias_statements(true, true, &mf, Some(50_000));
    assert!(
        large.iter().any(|s| s.contains("enable_seqscan = off")),
        "large slice keeps bias: {large:?}"
    );
    std::env::remove_var("EDGEQUAKE_ANN_EXACT_MAX_ROWS");
}

#[test]
fn storage_impl_counts_before_bias() {
    let src = include_str!("../src/adapters/postgres/vector/storage_impl.rs");
    assert!(
        src.contains("workspace_row_count"),
        "query_filtered must pass workspace row count into bias"
    );
    assert!(
        src.contains("count_workspace_rows"),
        "must count workspace rows for B3"
    );
}
