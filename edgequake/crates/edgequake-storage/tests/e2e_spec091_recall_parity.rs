//! SPEC-091 W3 recall parity (M-3.1): typed `chunk_embeddings` ANN recall@10
//! matches an exact brute-force cosine baseline on a seeded corpus.
//!
//! Run:
//!   DATABASE_URL=postgresql://edgequake:edgequake_secret@localhost:5432/edgequake \
//!     cargo test -p edgequake-storage --features postgres --test e2e_spec091_recall_parity -- --nocapture
#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;
#[path = "support/spec091_w3.rs"]
mod w3;

use edgequake_storage::traits::domain::{
    EmbeddingIndex, EmbeddingRow, ModelId, VectorQuery, WorkspaceId,
};
use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};
use std::collections::HashSet;
use uuid::Uuid;

const DIM: usize = 1536;
const CORPUS: usize = 40;
const TOP_K: usize = 10;

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

#[tokio::test]
async fn e2e_spec091_recall_parity_typed_vs_exact() {
    let Some(cfg) = require_or_skip_postgres("spec091_w3_recall") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;
    let ws = w3::seed_workspace(&pool, "recall").await;
    let doc = w3::seed_document(&pool, ws).await;

    let index = edgequake_storage::PgChunkEmbeddingIndex::new(pool.clone(), "w3-recall-model");

    // Seed a corpus of typed chunk embeddings.
    let mut corpus: Vec<(Uuid, Vec<f32>)> = Vec::new();
    let mut rows: Vec<EmbeddingRow> = Vec::new();
    for i in 0..CORPUS {
        let cid = w3::seed_chunk(&pool, doc, ws, i as i32, &format!("chunk {i}")).await;
        let emb = w3::make_embedding(DIM, 500 + i as u32);
        corpus.push((cid, emb.clone()));
        rows.push(EmbeddingRow {
            chunk_id: cid.into(),
            workspace_id: WorkspaceId(ws),
            dimensions: DIM as i32,
            embedding: emb,
        });
    }
    index
        .upsert_batch(ModelId(Uuid::nil()), &rows)
        .await
        .expect("upsert");

    // Run several probes; aggregate recall@10 vs exact brute force.
    let probes = [0u32, 7, 13, 21, 33];
    let mut total_recall = 0f32;
    for (p, probe_seed) in probes.iter().enumerate() {
        let query_emb = w3::make_embedding(DIM, 900 + probe_seed);

        // Exact brute-force top-K.
        let mut exact: Vec<(Uuid, f32)> = corpus
            .iter()
            .map(|(cid, emb)| (*cid, cosine(&query_emb, emb)))
            .collect();
        exact.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let exact_top: HashSet<Uuid> = exact.iter().take(TOP_K).map(|(c, _)| *c).collect();

        // Typed ANN top-K.
        let hits = index
            .search(&VectorQuery {
                model_id: ModelId(Uuid::nil()),
                workspace_id: Some(WorkspaceId(ws)),
                embedding: query_emb.clone(),
                limit: TOP_K as u32,
                        allowed_document_ids: None,
})
            .await
            .expect("typed search");
        let typed_top: HashSet<Uuid> = hits.iter().map(|h| h.chunk_id.0).collect();

        let overlap = exact_top.intersection(&typed_top).count() as f32;
        let recall = overlap / TOP_K as f32;
        total_recall += recall;
        eprintln!(
            "probe {p}: recall@10 = {recall:.2} (typed {} hits)",
            hits.len()
        );
    }
    let mean_recall = total_recall / probes.len() as f32;
    eprintln!("mean recall@10 = {mean_recall:.3}");
    // M-3.1 gate: on a small corpus ANN must be essentially exact.
    assert!(
        mean_recall >= 0.9,
        "typed recall parity gate failed: {mean_recall:.3} < 0.9"
    );

    w3::cleanup_workspace(&pool, ws).await;
}
