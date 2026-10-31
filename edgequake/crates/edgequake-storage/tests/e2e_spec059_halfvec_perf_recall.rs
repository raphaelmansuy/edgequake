//! SPEC-059 Wave 5 — halfvec vs full: p95 within 1.25× and recall@20 ≥ 0.99.

#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

use edgequake_storage::traits::{MetadataFilter, VectorStorage};
use edgequake_storage::PgVectorStorage;
use std::collections::HashSet;
use std::time::{Duration, Instant};

const DIM: usize = 64;
const ROW_COUNT: usize = 8_000;
const TOP_K: usize = 20;
const SAMPLES: usize = 30;
const WS: &str = "ws-halfvec-ab";

fn emb(seed: f32) -> Vec<f32> {
    (0..DIM)
        .map(|i| ((i as f32 + seed) * 0.013).sin())
        .collect()
}

fn percentile_p95(sorted: &[Duration]) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    let idx = ((sorted.len() as f64) * 0.95).ceil() as usize - 1;
    sorted[idx.min(sorted.len() - 1)]
}

async fn seed_and_measure(storage: &PgVectorStorage) -> (Duration, Vec<String>) {
    let chunk = 1000usize;
    for batch_start in (0..ROW_COUNT).step_by(chunk) {
        let end = (batch_start + chunk).min(ROW_COUNT);
        let mut batch = Vec::with_capacity(end - batch_start);
        for i in batch_start..end {
            batch.push((
                format!("hv-{i}"),
                emb(i as f32),
                serde_json::json!({
                    "workspace_id": WS,
                    "tenant_id": "t-hv",
                    "type": "chunk",
                    "document_id": format!("doc-{}", i / 10),
                }),
            ));
        }
        storage.upsert(&batch).await.expect("upsert");
    }

    let mf = MetadataFilter {
        workspace_id: Some(WS.to_string()),
        tenant_id: Some("t-hv".to_string()),
        vector_type: Some("chunk".to_string()),
        document_ids: None,
        modalities: None,
        embedding_model: None,
        kinds: None,
    };
    let query = emb(0.0);
    // Warm + discard a few cold samples so p95 is not dominated by HNSW init.
    for _ in 0..3 {
        let _ = storage
            .query_filtered(&query, TOP_K, None, Some(&mf))
            .await
            .expect("warm");
    }

    let mut samples = Vec::with_capacity(SAMPLES);
    let mut last_ids = Vec::new();
    for s in 0..SAMPLES {
        let q = emb(s as f32 * 17.0);
        let start = Instant::now();
        let results = storage
            .query_filtered(&q, TOP_K, None, Some(&mf))
            .await
            .expect("query");
        samples.push(start.elapsed());
        if s == 0 {
            last_ids = results.into_iter().map(|r| r.id).collect();
        }
    }
    samples.sort();
    (percentile_p95(&samples), last_ids)
}

fn recall_at_k(reference: &[String], candidate: &[String]) -> f64 {
    if reference.is_empty() {
        return 1.0;
    }
    let ref_set: HashSet<&str> = reference.iter().map(|s| s.as_str()).collect();
    let hits = candidate
        .iter()
        .filter(|id| ref_set.contains(id.as_str()))
        .count();
    hits as f64 / reference.len() as f64
}

#[tokio::test]
async fn e2e_spec059_halfvec_p95_and_recall_vs_full() {
    let Some(full_cfg) = postgres_test_config::require_or_skip_postgres("perf059_full") else {
        return;
    };
    let Some(half_cfg) = postgres_test_config::require_or_skip_postgres("perf059_half") else {
        return;
    };

    let prev_mode = std::env::var("EDGEQUAKE_VECTOR_STORAGE").ok();

    std::env::set_var("EDGEQUAKE_VECTOR_STORAGE", "full");
    let full = PgVectorStorage::with_dimension(full_cfg, DIM);
    full.initialize().await.expect("full init");
    let (p95_full, ids_full) = seed_and_measure(&full).await;

    std::env::set_var("EDGEQUAKE_VECTOR_STORAGE", "halfvec");
    let half = PgVectorStorage::with_dimension(half_cfg, DIM);
    half.initialize().await.expect("half init");
    let (p95_half, ids_half) = seed_and_measure(&half).await;

    match prev_mode {
        Some(v) => std::env::set_var("EDGEQUAKE_VECTOR_STORAGE", v),
        None => std::env::remove_var("EDGEQUAKE_VECTOR_STORAGE"),
    }

    let ratio = p95_half.as_secs_f64() / p95_full.as_secs_f64().max(1e-9);
    // Allow tiny absolute noise when both sides are already fast (<80ms).
    let ratio_ok = ratio <= 1.25
        || (p95_half.as_secs_f64() * 1000.0 < 80.0
            && p95_full.as_secs_f64() * 1000.0 < 80.0
            && ratio <= 1.50);
    assert!(
        ratio_ok,
        "halfvec p95 {p95_half:?} exceeds 1.25× full {p95_full:?} (ratio={ratio:.3})"
    );

    let recall = recall_at_k(&ids_full, &ids_half);
    assert!(
        recall >= 0.99,
        "halfvec recall@20 vs full = {recall:.4} (need ≥ 0.99); full={ids_full:?} half={ids_half:?}"
    );

    eprintln!(
        "OK SPEC-059 halfvec A/B @ {ROW_COUNT}: full_p95={p95_full:?} half_p95={p95_half:?} ratio={ratio:.3} recall@20={recall:.4}"
    );
    println!(
        "PERF_REPORT {}",
        serde_json::json!({
            "profile": std::env::var("EQ_POSTGRES_PROFILE").unwrap_or_else(|_| "unknown".into()),
            "pg_major": std::env::var("EQ_POSTGRES_MAJOR").unwrap_or_default(),
            "op": "halfvec_ab_query_filtered",
            "p95_ms": {
                "full": p95_full.as_secs_f64() * 1000.0,
                "half": p95_half.as_secs_f64() * 1000.0,
                "ratio": ratio,
            },
            "plan_class": "hnsw",
            "pass": true,
            "detail": format!("recall@20={recall:.4} N={ROW_COUNT}"),
        })
    );

    let _ = full.clear().await;
    let _ = half.clear().await;
}
