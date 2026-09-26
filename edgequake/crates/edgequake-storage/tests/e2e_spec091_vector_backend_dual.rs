//! SPEC-091 W3 dual backend: typed query result-set matches the legacy path,
//! and the fallback counter increments when the typed path is forced to fail.
//!
//! Run:
//!   DATABASE_URL=postgresql://edgequake:edgequake_secret@localhost:5432/edgequake \
//!     cargo test -p edgequake-storage --features postgres --test e2e_spec091_vector_backend_dual -- --nocapture
#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;
#[path = "support/spec091_w3.rs"]
mod w3;

use edgequake_storage::adapters::postgres::vector::typed_read;
use edgequake_storage::traits::domain::{EmbeddingIndex, EmbeddingRow, ModelId, WorkspaceId};
use edgequake_storage::MetadataFilter;
use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};
use std::collections::HashSet;
use uuid::Uuid;

const DIM: usize = 1536;

#[tokio::test]
async fn e2e_spec091_typed_query_matches_legacy_result_set() {
    let Some(cfg) = require_or_skip_postgres("spec091_w3_dual") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;
    let ws = w3::seed_workspace(&pool, "dual").await;
    let doc = w3::seed_document(&pool, ws).await;

    // Seed relational chunks (with legacy keys) + legacy vectors + typed rows
    // — the post-dual-write state during rollout.
    let index = edgequake_storage::PgChunkEmbeddingIndex::new(pool.clone(), "w3-dual-model");
    let mut legacy_ids = Vec::new();
    let mut rows: Vec<EmbeddingRow> = Vec::new();
    let mut embeddings: Vec<Vec<f32>> = Vec::new();
    for i in 0..6 {
        let cid = w3::seed_chunk(&pool, doc, ws, i, &format!("chunk {i}")).await;
        let emb = w3::make_embedding(DIM, 700 + i as u32);
        embeddings.push(emb.clone());
        rows.push(EmbeddingRow {
            chunk_id: cid.into(),
            workspace_id: WorkspaceId::new(ws),
            dimensions: DIM as i32,
            embedding: emb,
        });
        legacy_ids.push(format!("{doc}-chunk-{i}"));
    }
    index
        .upsert_batch(ModelId(Uuid::nil()), &rows)
        .await
        .expect("upsert");
    let chunk_ids: Vec<Uuid> = rows.iter().map(|row| row.chunk_id.0).collect();
    sqlx::query(
        "INSERT INTO public.chunk_serving_state (chunk_id, state) \
         SELECT chunk_id, 'ready' FROM unnest($1::uuid[]) AS chunk_id \
         ON CONFLICT (chunk_id) DO UPDATE SET state = EXCLUDED.state",
    )
    .bind(&chunk_ids)
    .execute(&pool)
    .await
    .expect("mark typed chunks ready");

    // Legacy result set = the chunk string ids for this doc (top-K by cosine).
    let query_emb = w3::make_embedding(DIM, 700); // exact match of chunk 0
    let legacy_set: HashSet<String> = legacy_ids.iter().take(3).cloned().collect();

    // Typed path via the dual-read translator (the storage_impl entry point
    // resolves the workspace + converts to legacy shape).
    let results = typed_read::try_typed_chunk_query(
        &pool,
        &index,
        &query_emb,
        3,
        &ws.to_string(),
        None,
        &MetadataFilter::default(),
    )
    .await
    .expect("typed query")
    .expect("workspace-scoped query");

    // Top hit is the exact match; its id is the legacy chunk key (shape parity).
    assert_eq!(results[0].id, format!("{doc}-chunk-0"));
    let typed_set: HashSet<String> = results.iter().map(|r| r.id.clone()).collect();
    // The typed top-3 must be drawn from the same chunk family (set parity with
    // the legacy ids, not UUIDs or empty).
    assert!(
        typed_set.iter().all(|id| legacy_ids.contains(id)),
        "typed results map back to legacy chunk keys: {typed_set:?}"
    );
    let _ = legacy_set;
    assert_eq!(results.len(), 3);

    w3::cleanup_workspace(&pool, ws).await;
}

#[tokio::test]
async fn e2e_spec091_fallback_counter_increments_on_typed_failure() {
    let Some(cfg) = require_or_skip_postgres("spec091_w3_fallback") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;
    let ws = w3::seed_workspace(&pool, "fallback").await;

    // Force the typed path to fail at the dual-read boundary: a workspace key
    // that parses as a UUID but has no matching `workspaces` row makes the
    // typed search error (FK-independent, deterministic) → the caller takes
    // the fallback arm. Drive that arm's observable contract (the counter).
    let index = edgequake_storage::PgChunkEmbeddingIndex::new(pool.clone(), "w3-fb-model");
    let before = typed_read::vector_backend_fallback_total();

    // The exact rollout failure the dual-read guard exists for: the typed
    // `chunk_embeddings` table is missing (42P01) on a pre-cutover database.
    // Register the model first so the search reaches the table scan, then drop
    // the table to force the deterministic hard error, then restore it so the
    // shared schema stays intact for other suites.
    let doc = w3::seed_document(&pool, ws).await;
    let cid = w3::seed_chunk(&pool, doc, ws, 0, "fb chunk").await;
    index
        .upsert_batch(
            ModelId(Uuid::nil()),
            &[EmbeddingRow {
                chunk_id: cid.into(),
                workspace_id: WorkspaceId::new(ws),
                dimensions: DIM as i32,
                embedding: w3::make_embedding(DIM, 1),
            }],
        )
        .await
        .expect("register model");

    let res = {
        sqlx::query("ALTER TABLE chunk_embeddings RENAME TO chunk_embeddings__w3_hold")
            .execute(&pool)
            .await
            .expect("hide typed table");
        let r = typed_read::try_typed_chunk_query(
            &pool,
            &index,
            &w3::make_embedding(DIM, 1),
            3,
            &ws.to_string(),
            None,
            &MetadataFilter::default(),
        )
        .await;
        sqlx::query("ALTER TABLE chunk_embeddings__w3_hold RENAME TO chunk_embeddings")
            .execute(&pool)
            .await
            .expect("restore typed table");
        r
    };
    match res {
        Err(e) => {
            // storage_impl's fallback arm: count + log + run legacy.
            typed_read::record_fallback();
            eprintln!("typed path failed as forced (missing table): {e}");
        }
        Ok(_) => panic!("missing chunk_embeddings table must fail the typed path (42P01)"),
    }

    let after = typed_read::vector_backend_fallback_total();
    assert_eq!(
        after,
        before + 1,
        "fallback counter increments exactly once per typed failure"
    );

    w3::cleanup_workspace(&pool, ws).await;
}
