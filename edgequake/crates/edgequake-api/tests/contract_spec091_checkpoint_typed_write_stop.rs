//! SPEC-091 WP1 / WP-AC-05: relational checkpoint authority skips KV write.
//!
//! Requires DATABASE_URL + migrations (pipeline_checkpoints). Strict CI forbids
//! a missing-database skip.
//!
//! Run:
//!   cargo test -p edgequake-api --features postgres --test contract_spec091_checkpoint_typed_write_stop -- --test-threads=1

#![cfg(feature = "postgres")]

#[path = "../../edgequake-storage/tests/support/postgres_access_pool.rs"]
#[allow(dead_code)]
mod postgres_access_pool;

use std::sync::Arc;

use edgequake_api::processor::pipeline_checkpoint::{
    checkpoint_key, load_pipeline_checkpoint, save_pipeline_checkpoint,
};
use edgequake_api::services::postgres_checkpoint_artifact_store::PostgresCheckpointArtifactStore;
use edgequake_api::services::relational_sidecar_store::{
    typed_checkpoint_get, CHECKPOINT_KIND_CRASH,
};
use edgequake_pipeline::{ProcessingResult, ProcessingStats};
use edgequake_storage::traits::KVStorage;
use edgequake_storage::MemoryKVStorage;
use uuid::Uuid;

#[tokio::test]
async fn relational_checkpoint_write_stops_kv() {
    let Some(pool) =
        postgres_access_pool::test_pool_with_acquire_timeout(std::time::Duration::from_secs(10))
            .await
    else {
        return;
    };
    let store = PostgresCheckpointArtifactStore::new(pool.clone());

    let doc_id = Uuid::new_v4().to_string();

    std::env::set_var("EDGEQUAKE_KV_FAMILY_CHECKPOINT", "relational");
    assert!(edgequake_api::services::relational_sidecar_store::checkpoints_prefer_relational());

    let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("documents"));
    let result = ProcessingResult {
        document_id: doc_id.clone(),
        chunks: vec![],
        extractions: vec![],
        stats: ProcessingStats::default(),
        lineage: None,
    };
    let text = "wp1 checkpoint write-stop body";
    let workspace = "cccccccc-0019-0019-0019-cccccccccccc";
    save_pipeline_checkpoint(
        &kv,
        Some(&store),
        &doc_id,
        &result,
        workspace,
        "openai",
        "ollama",
        text,
    )
    .await
    .expect("save");

    let key = checkpoint_key(&doc_id);
    let kv_val = kv.get_by_id(&key).await.expect("kv get");
    assert!(
        kv_val.is_none(),
        "KV must not receive checkpoint when relational typed write succeeds; got {kv_val:?}"
    );

    let typed = typed_checkpoint_get(Some(&store), &doc_id, CHECKPOINT_KIND_CRASH)
        .await
        .expect("typed row present");
    assert!(
        typed.get("content_hash").is_some() || typed.get("result").is_some(),
        "typed payload shape: {typed}"
    );

    let loaded = load_pipeline_checkpoint(
        &kv,
        Some(&store),
        &doc_id,
        workspace,
        "openai",
        "ollama",
        text,
    )
    .await;
    assert!(
        loaded.is_some(),
        "resume must load from typed when KV empty"
    );

    std::env::remove_var("EDGEQUAKE_KV_FAMILY_CHECKPOINT");
    sqlx::query("DELETE FROM documents WHERE id=$1")
        .bind(doc_id.parse::<Uuid>().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}
