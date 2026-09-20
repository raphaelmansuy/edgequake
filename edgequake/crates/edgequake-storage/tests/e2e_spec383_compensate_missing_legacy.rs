//! SPEC-383 / GitHub #383: saga compensation must not DELETE a dropped
//! per-workspace `eq_*_vectors` table. Typed chunk rows roll back via
//! `chunks` ON DELETE CASCADE into `chunk_embeddings`.
//!
//! Run:
//!   DATABASE_URL=postgresql://edgequake:edgequake_secret@localhost:5432/edgequake \
//!     cargo test -p edgequake-storage --features postgres --test e2e_spec383_compensate_missing_legacy -- --nocapture
#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;
#[path = "support/spec091_w3.rs"]
mod w3;

use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use edgequake_storage::adapters::postgres::{
    PgVectorStorage, PostgresChunkRepository, PostgresPool,
};
use edgequake_storage::traits::domain::{
    ChunkRepository, DocumentId, EmbeddingIndex, EmbeddingRow, ModelId, UnitOfWork, WorkspaceId,
};
use edgequake_storage::{compensate_orphan_vectors, VECTOR_BACKEND_ENV};
use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};
use tracing_subscriber::fmt::MakeWriter;
use uuid::Uuid;

const DIM: usize = 1536;

#[derive(Clone, Default)]
struct LogBuf(Arc<Mutex<Vec<u8>>>);

impl Write for LogBuf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().expect("log buf").extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for LogBuf {
    type Writer = LogBuf;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

async fn table_exists(pool: &sqlx::PgPool, table: &str) -> bool {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_name = $1)",
    )
    .bind(table)
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

#[tokio::test(flavor = "current_thread")]
#[allow(clippy::await_holding_lock)]
async fn e2e_spec383_compensate_skips_missing_legacy_and_cascades_typed() {
    let Some(cfg) = require_or_skip_postgres("spec383_compensate") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;

    for table in ["chunk_embeddings", "chunks", "documents", "workspaces"] {
        if !table_exists(&pool, table).await {
            eprintln!("skip: typed table {table} missing — run migrations through 108");
            return;
        }
    }

    std::env::set_var(VECTOR_BACKEND_ENV, "typed_embeddings");
    std::env::set_var("EDGEQUAKE_EMBEDDING_MODEL", "spec383-e2e");

    let ws = w3::seed_workspace(&pool, "spec383").await;
    let doc = w3::seed_document(&pool, ws).await;
    let chunk_id = w3::seed_chunk(&pool, doc, ws, 0, "spec383 chunk").await;

    let mut pg_cfg = cfg.clone();
    pg_cfg.namespace = format!("default_ws_{}", ws.to_string().replace('-', "_"));
    let storage = PgVectorStorage::with_pool(
        PostgresPool::from_existing(pool.clone(), pg_cfg.clone()),
        pg_cfg,
        DIM,
    );
    let legacy_table = storage
        .vectors_table_name()
        .trim_start_matches("public.")
        .to_string();
    assert!(
        legacy_table.starts_with("eq_eq_default_ws_"),
        "production workspace table shape, got {legacy_table}"
    );
    assert!(
        !table_exists(&pool, &legacy_table).await,
        "precondition: {legacy_table} must be absent"
    );

    let index = edgequake_storage::PgChunkEmbeddingIndex::new(pool.clone(), "spec383-e2e");
    let emb = w3::make_embedding(DIM, 383);
    index
        .upsert_batch(
            ModelId(Uuid::nil()),
            &[EmbeddingRow {
                chunk_id: chunk_id.into(),
                workspace_id: WorkspaceId::new(ws),
                embedding: emb,
                dimensions: DIM as i32,
            }],
        )
        .await
        .expect("typed chunk upsert");
    let before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM chunk_embeddings WHERE chunk_id = $1")
            .bind(chunk_id)
            .fetch_one(&pool)
            .await
            .expect("count embeddings");
    assert_eq!(before, 1, "typed SSOT row must exist before compensate");

    let logs = LogBuf::default();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(logs.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let chunk_vector_id = format!("{doc}-chunk-0");
    compensate_orphan_vectors(
        &storage,
        &doc.to_string(),
        &[chunk_vector_id],
        &[],
        "spec383 merge failed",
    )
    .await;

    drop(_guard);
    let captured = String::from_utf8_lossy(&logs.0.lock().expect("log buf")).into_owned();
    assert!(
        captured.contains("SPEC-383: skip mutate — legacy vectors relation absent"),
        "must skip DELETE, not 42P01-swallow. logs:\n{captured}"
    );
    assert!(
        !captured.contains("legacy vectors relation gone — mutate write-stop"),
        "42P01 swallow path must not fire when the table was never queried. logs:\n{captured}"
    );
    assert!(
        !table_exists(&pool, &legacy_table).await,
        "compensate must not CREATE {legacy_table}"
    );

    let repo = PostgresChunkRepository::new(pool.clone());
    repo.delete_for_document(&mut UnitOfWork::default(), DocumentId::new(doc))
        .await
        .expect("relational chunk compensation");

    let chunks_left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chunks WHERE document_id = $1")
        .bind(doc)
        .fetch_one(&pool)
        .await
        .expect("count chunks");
    let embeddings_left: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM chunk_embeddings WHERE chunk_id = $1")
            .bind(chunk_id)
            .fetch_one(&pool)
            .await
            .expect("count embeddings after cascade");
    assert_eq!(chunks_left, 0, "delete_for_document must remove chunks");
    assert_eq!(
        embeddings_left, 0,
        "chunk_embeddings must CASCADE from chunks delete"
    );

    w3::cleanup_workspace(&pool, ws).await;
    std::env::remove_var(VECTOR_BACKEND_ENV);
    std::env::remove_var("EDGEQUAKE_EMBEDDING_MODEL");
}
