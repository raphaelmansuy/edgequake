//! SPEC-149 P0 pgvector ANN recall@10 + p95 artifact.
//!
//! Retains numbers under
//! `specs/149-data-access-improvements/evidence/p0-ann-recall.json`.
//! Not a 7-day soak and not a P1 comparison.
//!
//! Run:
//!   EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1 \
//!     cargo test -p edgequake-storage --features postgres \
//!     --test e2e_spec149_p0_ann_recall -- --nocapture

#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;
#[path = "support/spec091_w3.rs"]
mod w3;

use edgequake_storage::traits::domain::{
    EmbeddingIndex, EmbeddingRow, ModelId, VectorQuery, WorkspaceId,
};
use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};
use serde_json::json;
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;
use uuid::Uuid;

const DIM: usize = 64;
const CORPUS: usize = 1_000;
const TOP_K: usize = 10;
const PROBES: usize = 20;

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0f32;
    let mut na = 0f32;
    let mut nb = 0f32;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

fn percentile(sorted_ms: &[f64], p: f64) -> f64 {
    if sorted_ms.is_empty() {
        return 0.0;
    }
    let rank = ((p / 100.0) * (sorted_ms.len() as f64 - 1.0)).round() as usize;
    sorted_ms[rank.min(sorted_ms.len() - 1)]
}

fn git_sha() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".into())
}

fn evidence_path() -> PathBuf {
    // edgequake/crates/edgequake-storage/tests -> repo specs/
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join("specs/149-data-access-improvements/evidence/p0-ann-recall.json")
}

#[tokio::test]
async fn e2e_spec149_p0_ann_recall_and_p95() {
    let Some(cfg) = require_or_skip_postgres("spec149_p0_ann") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;
    let ws = w3::seed_workspace(&pool, "p0-ann").await;
    let doc = w3::seed_document(&pool, ws).await;

    let index = edgequake_storage::PgChunkEmbeddingIndex::new(pool.clone(), "spec149-p0-ann-model");

    let mut corpus: Vec<(Uuid, Vec<f32>)> = Vec::with_capacity(CORPUS);
    let mut rows: Vec<EmbeddingRow> = Vec::with_capacity(CORPUS);
    for i in 0..CORPUS {
        let cid = w3::seed_chunk(&pool, doc, ws, i as i32, &format!("ann {i}")).await;
        let emb = w3::make_embedding(DIM, 1_000 + i as u32);
        corpus.push((cid, emb.clone()));
        rows.push(EmbeddingRow {
            chunk_id: cid.into(),
            workspace_id: WorkspaceId::new(ws),
            dimensions: DIM as i32,
            embedding: emb,
        });
    }
    // Upsert in batches to stay under statement limits.
    for chunk in rows.chunks(100) {
        index
            .upsert_batch(ModelId(Uuid::nil()), chunk)
            .await
            .expect("upsert batch");
    }

    let mut total_recall = 0f32;
    let mut latencies_ms = Vec::with_capacity(PROBES);
    for p in 0..PROBES {
        let query_emb = w3::make_embedding(DIM, 50_000 + p as u32);
        let mut exact: Vec<(Uuid, f32)> = corpus
            .iter()
            .map(|(cid, emb)| (*cid, cosine(&query_emb, emb)))
            .collect();
        exact.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let exact_top: HashSet<Uuid> = exact.iter().take(TOP_K).map(|(c, _)| *c).collect();

        let started = Instant::now();
        let hits = index
            .search(&VectorQuery {
                model_id: ModelId(Uuid::nil()),
                model_revision: "test-current".into(),
                workspace_id: Some(WorkspaceId::new(ws)),
                document_ids: None,
                tenant_id: None,
                modalities: None,
                filter_ids: None,
                vector_type: None,
                embedding: query_emb.clone(),
                limit: TOP_K as u32,
            })
            .await
            .expect("ann search");
        latencies_ms.push(started.elapsed().as_secs_f64() * 1000.0);

        let typed_top: HashSet<Uuid> = hits.iter().map(|h| h.chunk_id.0).collect();
        let overlap = exact_top.intersection(&typed_top).count() as f32;
        total_recall += overlap / TOP_K as f32;
    }

    let mean_recall = total_recall / PROBES as f32;
    latencies_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50 = percentile(&latencies_ms, 50.0);
    let p95 = percentile(&latencies_ms, 95.0);

    assert!(
        mean_recall >= 0.95,
        "P0 recall@10 gate failed: {mean_recall:.3} < 0.95"
    );

    let artifact = json!({
        "schema_version": 1,
        "profile": "P0",
        "index": "chunk_embeddings",
        "model": "spec149-p0-ann-model",
        "corpus_size": CORPUS,
        "dimension": DIM,
        "top_k": TOP_K,
        "probes": PROBES,
        "recall_at_10": mean_recall,
        "p50_ms": p50,
        "p95_ms": p95,
        "git_sha": git_sha(),
        "gate_recall_at_10": 0.95,
    });
    let path = evidence_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(
        &path,
        format!("{}\n", serde_json::to_string_pretty(&artifact).unwrap()),
    )
    .unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
    eprintln!(
        "P0 ANN artifact written to {} recall@10={mean_recall:.3} p50={p50:.2}ms p95={p95:.2}ms",
        path.display()
    );

    w3::cleanup_workspace(&pool, ws).await;
}
