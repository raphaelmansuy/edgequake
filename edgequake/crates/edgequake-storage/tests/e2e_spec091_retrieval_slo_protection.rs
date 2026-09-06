//! SPEC-091 IW1 Wave-0 scorecard (GAP-091-22): retrieval SLO protection.
//!
//! - pool acquisition p95 < 10ms
//! - filtered ANN search p95 < 150ms on a 200-row workspace (HNSW via migration 129)
#![cfg(feature = "postgres")]

#[path = "support/perf_harness.rs"]
mod perf_harness;
#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;
#[path = "support/spec091_w3.rs"]
mod w3;

use edgequake_storage::traits::domain::{
    EmbeddingIndex, EmbeddingRow, ModelId, VectorQuery, WorkspaceId,
};
use perf_harness::finish_report;
use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};
use std::time::Instant;
use uuid::Uuid;

const DIM: usize = 1536;
const CORPUS: usize = 200;
const SAMPLES: usize = 30;

#[tokio::test]
async fn e2e_spec091_retrieval_slo_protection() {
    let Some(cfg) = require_or_skip_postgres("spec091_retrieval_slo") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;
    let ws = w3::seed_workspace(&pool, "retr-slo").await;
    let doc = w3::seed_document(&pool, ws).await;
    let index = edgequake_storage::PgChunkEmbeddingIndex::new(pool.clone(), "iw1-retr-model");

    let mut rows = Vec::with_capacity(CORPUS);
    for i in 0..CORPUS {
        let cid = w3::seed_chunk(&pool, doc, ws, i as i32, &format!("r{i}")).await;
        rows.push(EmbeddingRow {
            chunk_id: cid.into(),
            workspace_id: WorkspaceId(ws),
            dimensions: DIM as i32,
            embedding: w3::make_embedding(DIM, 1000 + i as u32),
        });
    }
    index
        .upsert_batch(ModelId(Uuid::nil()), &rows)
        .await
        .expect("seed corpus");

    // Discard first acquires so p95 is not dominated by TLS/TCP setup on GH runners.
    for _ in 0..5 {
        drop(pool.acquire().await.expect("warmup acquire"));
    }
    let mut pool_samples = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let t0 = Instant::now();
        let conn = pool.acquire().await.expect("acquire");
        pool_samples.push(t0.elapsed());
        drop(conn);
    }
    finish_report(
        "pool_acquire",
        &pool_samples,
        10.0,
        "Index",
        false,
        "IW1 GAP-091-22 pool acquisition p95",
    );

    let mut search_samples = Vec::with_capacity(SAMPLES);
    for s in 0..SAMPLES {
        let q = VectorQuery {
            model_id: ModelId(Uuid::nil()),
            embedding: w3::make_embedding(DIM, 2000 + s as u32),
            limit: 10,
            workspace_id: Some(WorkspaceId(ws)),
                    allowed_document_ids: None,
};
        let t0 = Instant::now();
        let hits = index.search(&q).await.expect("search");
        search_samples.push(t0.elapsed());
        assert!(!hits.is_empty());
    }
    finish_report(
        "typed_chunk_ann_search_top10",
        &search_samples,
        150.0,
        "Hnsw",
        false,
        "IW1 GAP-091-22 filtered ANN p95 budget",
    );

    let plan: String = sqlx::query_scalar(
        "SELECT COALESCE((\
           SELECT indexdef FROM pg_indexes \
           WHERE indexname = 'idx_chunk_embeddings_hnsw_d1536'\
         ), '')",
    )
    .fetch_one(&pool)
    .await
    .unwrap_or_default();
    assert!(
        plan.to_lowercase().contains("hnsw"),
        "expected dim-scoped HNSW indexdef after migration 132, got: {plan}"
    );
}
