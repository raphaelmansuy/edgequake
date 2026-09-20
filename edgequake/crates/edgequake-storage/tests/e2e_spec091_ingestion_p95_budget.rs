//! SPEC-091 IW1 Wave-0 scorecard (GAP-091-22): typed ingest p95 budget.
//!
//! typed chunk upsert batch (16 rows) p95 < 2000ms (ingest tx budget proxy).
#![cfg(feature = "postgres")]

#[path = "support/perf_harness.rs"]
mod perf_harness;
#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;
#[path = "support/spec091_w3.rs"]
mod w3;

use edgequake_storage::traits::domain::{EmbeddingIndex, EmbeddingRow, ModelId, WorkspaceId};
use perf_harness::finish_report;
use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};
use std::time::Instant;
use uuid::Uuid;

const DIM: usize = 1536;
const SAMPLES: usize = 30;

#[tokio::test]
async fn e2e_spec091_ingestion_p95_budget() {
    let Some(cfg) = require_or_skip_postgres("spec091_ingest_p95") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;
    let ws = w3::seed_workspace(&pool, "ingest-p95").await;
    let doc = w3::seed_document(&pool, ws).await;
    let index = edgequake_storage::PgChunkEmbeddingIndex::new(pool.clone(), "iw1-ingest-model");

    let warm_cid = w3::seed_chunk(&pool, doc, ws, 0, "warm").await;
    index
        .upsert_batch(
            ModelId(Uuid::nil()),
            &[EmbeddingRow {
                chunk_id: warm_cid.into(),
                workspace_id: WorkspaceId::new(ws),
                dimensions: DIM as i32,
                embedding: w3::make_embedding(DIM, 1),
            }],
        )
        .await
        .expect("warm upsert");

    let mut samples = Vec::with_capacity(SAMPLES);
    for s in 0..SAMPLES {
        let mut rows = Vec::with_capacity(16);
        for i in 0..16 {
            let idx = (s * 16 + i + 1) as i32;
            let cid = w3::seed_chunk(&pool, doc, ws, idx, &format!("c{idx}")).await;
            rows.push(EmbeddingRow {
                chunk_id: cid.into(),
                workspace_id: WorkspaceId::new(ws),
                dimensions: DIM as i32,
                embedding: w3::make_embedding(DIM, (s * 16 + i) as u32 + 10),
            });
        }
        let t0 = Instant::now();
        index
            .upsert_batch(ModelId(Uuid::nil()), &rows)
            .await
            .expect("upsert batch");
        samples.push(t0.elapsed());
    }

    finish_report(
        "typed_chunk_embedding_upsert_batch16",
        &samples,
        2000.0,
        "Index",
        false,
        "IW1 GAP-091-22 ingest p95 budget",
    );
}
