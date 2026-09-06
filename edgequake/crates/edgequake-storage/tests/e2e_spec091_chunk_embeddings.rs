//! SPEC-091 W3: `PgChunkEmbeddingIndex` (`EmbeddingIndex` port) conformance.
//!
//! Run:
//!   DATABASE_URL=postgresql://edgequake:edgequake_secret@localhost:5432/edgequake \
//!     cargo test -p edgequake-storage --features postgres --test e2e_spec091_chunk_embeddings -- --nocapture
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

fn row(chunk_id: Uuid, workspace: Uuid, embedding: Vec<f32>) -> EmbeddingRow {
    EmbeddingRow {
        chunk_id: chunk_id.into(),
        workspace_id: WorkspaceId(workspace),
        dimensions: embedding.len() as i32,
        embedding,
    }
}

#[tokio::test]
async fn e2e_spec091_chunk_embeddings_upsert_search_delete() {
    let Some(cfg) = require_or_skip_postgres("spec091_w3_port") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;
    let ws = w3::seed_workspace(&pool, "port").await;
    let doc = w3::seed_document(&pool, ws).await;
    // chunk_embeddings.embedding is unconstrained halfvec (migration 132);
    // 1536 remains the OpenAI path covered here.
    let dim: usize = 1536;

    let index = edgequake_storage::PgChunkEmbeddingIndex::new(pool.clone(), "w3-model");

    // Capabilities: cosine, filterable, no rerank.
    let caps = index.capabilities();
    assert_eq!(caps.metric, "cosine");
    assert!(caps.supports_filters);

    // Seed 3 relational chunks and upsert typed embeddings for them.
    let mut chunk_ids = Vec::new();
    let mut rows = Vec::new();
    for i in 0..3 {
        let cid = w3::seed_chunk(&pool, doc, ws, i, &format!("content {i}")).await;
        chunk_ids.push(cid);
        rows.push(row(cid, ws, w3::make_embedding(dim, 10 + i as u32)));
    }
    let report = index
        .upsert_batch(ModelId(Uuid::nil()), &rows)
        .await
        .expect("upsert");
    assert_eq!(report.upserted, 3);

    // Idempotent retry: same batch again → 0 new rows (ON CONFLICT DO NOTHING).
    let retry = index
        .upsert_batch(ModelId(Uuid::nil()), &rows)
        .await
        .expect("retry");
    assert_eq!(retry.upserted, 0, "upsert must be idempotent");

    // Model registry dedupe: one (name, dimensions) row reused across upserts.
    let models: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM embedding_models WHERE name = 'w3-model' AND dimensions = $1",
    )
    .bind(dim as i32)
    .fetch_one(&pool)
    .await
    .expect("model count");
    assert_eq!(models, 1, "model registry deduped by (name, dimensions)");

    // Search returns the nearest chunk for an exact-match query.
    let query = VectorQuery {
        model_id: ModelId(Uuid::nil()),
        workspace_id: Some(WorkspaceId(ws)),
        embedding: w3::make_embedding(dim, 11),
        limit: 1,
                allowed_document_ids: None,
};
    let hits = index.search(&query).await.expect("search");
    assert_eq!(hits.len(), 1);
    assert_eq!(
        hits[0].chunk_id.0, chunk_ids[1],
        "exact-match query returns its own chunk"
    );

    // Workspace filter: a different workspace sees nothing.
    let other_ws = w3::seed_workspace(&pool, "port-other").await;
    let other = VectorQuery {
        model_id: ModelId(Uuid::nil()),
        workspace_id: Some(WorkspaceId(other_ws)),
        embedding: w3::make_embedding(dim, 11),
        limit: 10,
                allowed_document_ids: None,
};
    let none = index.search(&other).await.expect("other ws search");
    assert!(none.is_empty(), "workspace filter isolates rows");

    // delete_for_workspace removes only this workspace's rows.
    let deleted = index
        .delete_for_workspace(WorkspaceId(ws))
        .await
        .expect("delete");
    assert_eq!(deleted, 3);
    let remaining: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM chunk_embeddings WHERE workspace_id = $1")
            .bind(ws)
            .fetch_one(&pool)
            .await
            .expect("remaining");
    assert_eq!(remaining, 0);

    w3::cleanup_workspace(&pool, ws).await;
    w3::cleanup_workspace(&pool, other_ws).await;
}

#[tokio::test]
async fn e2e_spec091_typed_dim_1024_upsert_search() {
    let Some(cfg) = require_or_skip_postgres("spec091_w3_port_1024") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;

    // Schema proof: column typmod unconstrained after migration 132.
    let typmod: Option<i32> = sqlx::query_scalar(
        "SELECT a.atttypmod FROM pg_attribute a \
         JOIN pg_class c ON c.oid = a.attrelid \
         JOIN pg_namespace n ON n.oid = c.relnamespace \
         WHERE n.nspname = 'public' AND c.relname = 'chunk_embeddings' \
           AND a.attname = 'embedding' AND NOT a.attisdropped",
    )
    .fetch_optional(&pool)
    .await
    .expect("typmod probe");
    // Unconstrained halfvec has atttypmod = -1 (no typmod).
    assert_eq!(
        typmod,
        Some(-1),
        "migration 132 must unconstrain chunk_embeddings.embedding (got {typmod:?})"
    );

    let ws = w3::seed_workspace(&pool, "port-1024").await;
    let doc = w3::seed_document(&pool, ws).await;
    let dim: usize = 1024;
    let index = edgequake_storage::PgChunkEmbeddingIndex::new(pool.clone(), "w3-model-1024");

    let mut chunk_ids = Vec::new();
    let mut rows = Vec::new();
    for i in 0..3 {
        let cid = w3::seed_chunk(&pool, doc, ws, i, &format!("content1024 {i}")).await;
        chunk_ids.push(cid);
        rows.push(row(cid, ws, w3::make_embedding(dim, 20 + i as u32)));
    }
    let report = index
        .upsert_batch(ModelId(Uuid::nil()), &rows)
        .await
        .expect("1024 upsert must succeed after unconstrained halfvec");
    assert_eq!(report.upserted, 3);

    let query = VectorQuery {
        model_id: ModelId(Uuid::nil()),
        workspace_id: Some(WorkspaceId(ws)),
        embedding: w3::make_embedding(dim, 21),
        limit: 1,
                allowed_document_ids: None,
};
    let hits = index.search(&query).await.expect("1024 search");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].chunk_id.0, chunk_ids[1]);

    // Mixed-dimension batch rejected before SQL.
    let mixed = vec![
        row(chunk_ids[0], ws, w3::make_embedding(1024, 1)),
        row(Uuid::new_v4(), ws, w3::make_embedding(1536, 2)),
    ];
    let err = index
        .upsert_batch(ModelId(Uuid::nil()), &mixed)
        .await
        .expect_err("mixed dims must fail closed");
    assert!(err.to_string().contains("mixed dimensions"), "got: {err}");

    w3::cleanup_workspace(&pool, ws).await;
}
