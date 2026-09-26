//! SPEC-091 WP1 / WP-AC-05: relational checkpoint authority skips KV write.
//!
//! Requires DATABASE_URL + migrations (pipeline_checkpoints). Skips when unset
//! or when Postgres rejects the connection.
//!
//! Run:
//!   cargo test -p edgequake-api --features postgres --test contract_spec091_checkpoint_typed_write_stop -- --test-threads=1

#![cfg(feature = "postgres")]

#[path = "common/test_db.rs"]
mod test_db;

use std::sync::Arc;

use edgequake_api::processor::pipeline_checkpoint::{
    checkpoint_key, load_pipeline_checkpoint, save_pipeline_checkpoint,
};
use edgequake_api::services::relational_sidecar_store::{
    register_sidecar_pool, sidecar_store, typed_checkpoint_get, CHECKPOINT_KIND_CRASH,
};
use edgequake_pipeline::{ProcessingResult, ProcessingStats};
use edgequake_storage::traits::KVStorage;
use edgequake_storage::MemoryKVStorage;
use uuid::Uuid;

fn require_db() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .or_else(|| {
            std::fs::read_to_string("/tmp/edgequake-db-url")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .or_else(|| {
            Some("postgresql://edgequake:edgequake_secret@localhost:5432/edgequake".to_string())
        })
}

#[tokio::test]
async fn relational_checkpoint_write_stops_kv() {
    let Some(base) = require_db() else {
        eprintln!("skip: DATABASE_URL unset");
        return;
    };
    let url = test_db::isolated_test_url(&base);
    let pool = match sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("skip: cannot connect to {url}: {e}");
            return;
        }
    };

    register_sidecar_pool(pool.clone());

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
        &kv, None, &doc_id, &result, workspace, "openai", "ollama", text,
    )
    .await
    .expect("save");

    let key = checkpoint_key(&doc_id);
    let kv_val = kv.get_by_id(&key).await.expect("kv get");
    assert!(
        kv_val.is_none(),
        "KV must not receive checkpoint when relational typed write succeeds; got {kv_val:?}"
    );

    let typed = typed_checkpoint_get(sidecar_store().as_deref(), &doc_id, CHECKPOINT_KIND_CRASH)
        .await
        .expect("typed row present");
    assert!(
        typed.get("content_hash").is_some() || typed.get("result").is_some(),
        "typed payload shape: {typed}"
    );

    let loaded =
        load_pipeline_checkpoint(&kv, None, &doc_id, workspace, "openai", "ollama", text).await;
    assert!(
        loaded.is_some(),
        "resume must load from typed when KV empty"
    );

    std::env::remove_var("EDGEQUAKE_KV_FAMILY_CHECKPOINT");
}
