//! SPEC-091 IW4 (GAP-091-33) — minimal typed CRUD smoke for PG16/PG18 CI matrix.
//!
//! Fast gate: one workspace + document + chunk + typed embedding round-trip.
//! Full data-layer suites remain on the primary spec091-data-layer job / nightly.
//!
//! Run:
//!   DATABASE_URL=postgresql://edgequake_test:test_password_123@localhost:5432/edgequake_test \
//!     cargo test -p edgequake-storage --features postgres --test e2e_spec091_pg_matrix_smoke -- --nocapture

#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;
#[path = "support/spec091_w3.rs"]
mod w3;

use edgequake_storage::traits::domain::{
    EmbeddingIndex, EmbeddingRow, ModelId, VectorQuery, WorkspaceId,
};
use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};
use uuid::Uuid;

#[tokio::test]
async fn e2e_spec091_pg_matrix_smoke_typed_crud() {
    let Some(cfg) = require_or_skip_postgres("spec091_pg_matrix_smoke") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;

    let major: i32 =
        sqlx::query_scalar("SELECT current_setting('server_version_num')::int / 10000")
            .fetch_one(&pool)
            .await
            .expect("server_version_num");
    assert!(
        (16..=18).contains(&major),
        "matrix smoke expects PG16–18, got {major}"
    );

    let ws = w3::seed_workspace(&pool, "matrix-smoke").await;
    let doc = w3::seed_document(&pool, ws).await;
    let dim: usize = 1536;
    let cid = w3::seed_chunk(&pool, doc, ws, 0, "matrix smoke chunk").await;

    let index = edgequake_storage::PgChunkEmbeddingIndex::new(pool.clone(), "matrix-smoke-model");
    let row = EmbeddingRow {
        chunk_id: cid.into(),
        workspace_id: WorkspaceId(ws),
        dimensions: dim as i32,
        embedding: w3::make_embedding(dim, 42),
    };
    let upsert = index
        .upsert_batch(ModelId(Uuid::nil()), std::slice::from_ref(&row))
        .await
        .expect("typed upsert");
    assert_eq!(upsert.upserted, 1);

    let hits = index
        .search(&VectorQuery {
            model_id: ModelId(Uuid::nil()),
            workspace_id: Some(WorkspaceId(ws)),
            embedding: row.embedding.clone(),
            limit: 1,
                    allowed_document_ids: None,
})
        .await
        .expect("typed search");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].chunk_id.0, cid);

    let deleted = index
        .delete_for_workspace(WorkspaceId(ws))
        .await
        .expect("typed delete");
    assert_eq!(deleted, 1);

    w3::cleanup_workspace(&pool, ws).await;
}
