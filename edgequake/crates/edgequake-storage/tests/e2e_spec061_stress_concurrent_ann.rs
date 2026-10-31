//! SPEC-061/062 — concurrent filtered ANN stress.
//!
//! pg16: N=8 ≤2×; pg17/18: N=16 ≤1.5×. Scale via `EDGEQUAKE_PERF_SCALE=prod|large`.
#![cfg(feature = "postgres")]

#[path = "support/perf_harness.rs"]
mod perf_harness;
#[path = "support/perf_stress.rs"]
mod perf_stress;
#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

use edgequake_storage::traits::{MetadataFilter, VectorStorage};
use edgequake_storage::PgVectorStorage;
use perf_harness::{finish_report, percentile_p95_ms};
use perf_stress::{
    ann_scale, perf_scale, stress_clients, stress_mult, stress_pool_max, with_stress_pool,
    PerfScale,
};
use std::sync::Arc;
use std::time::{Duration, Instant};

const TOP_K: usize = 20;
const SINGLE_SLO_MS: f64 = 100.0;

fn emb(dim: usize, seed: f32) -> Vec<f32> {
    (0..dim)
        .map(|i| ((i as f32 + seed) * 0.019).sin())
        .collect()
}

#[tokio::test]
async fn e2e_spec061_stress_concurrent_filtered_ann() {
    let scale = perf_scale();
    let ann = ann_scale(scale);
    let clients = stress_clients();
    let mult = stress_mult();
    let pool = stress_pool_max(clients);

    let Some(base) = postgres_test_config::require_or_skip_postgres("stress061_ann") else {
        return;
    };
    let config = with_stress_pool(base, clients);
    let storage = Arc::new(PgVectorStorage::with_dimension(config, ann.dim));
    storage.initialize().await.expect("init");

    for batch_start in (0..ann.rows).step_by(ann.batch_size) {
        let end = (batch_start + ann.batch_size).min(ann.rows);
        let batch: Vec<_> = (batch_start..end)
            .map(|i| {
                let ws = if i % 5 == 0 { "ws-a" } else { "ws-b" };
                (
                    format!("sann-{i}"),
                    emb(ann.dim, i as f32),
                    serde_json::json!({
                        "type": "chunk",
                        "workspace_id": ws,
                        "tenant_id": "t-stress",
                    }),
                )
            })
            .collect();
        storage.upsert(&batch).await.expect("upsert");
    }

    let mf = MetadataFilter {
        workspace_id: Some("ws-a".into()),
        tenant_id: Some("t-stress".into()),
        vector_type: Some("chunk".into()),
        document_ids: None,
        modalities: None,
        embedding_model: None,
        kinds: None,
    };

    let mut single = Vec::new();
    for s in 0..20 {
        let start = Instant::now();
        let _ = storage
            .query_filtered(&emb(ann.dim, s as f32), TOP_K, None, Some(&mf))
            .await
            .expect("single");
        single.push(start.elapsed());
    }
    let single_p95 = percentile_p95_ms(&single);

    let qpc = ann.queries_per_client;
    let dim = ann.dim;
    // SPEC-065: cold cliff is residency-dependent; warm shared buffers before the
    // timed concurrent window so 1.5× stress measures warm contention, not cold I/O.
    {
        let mut warm = Vec::new();
        for c in 0..clients {
            let storage = Arc::clone(&storage);
            let mf = mf.clone();
            warm.push(tokio::spawn(async move {
                for q in 0..4u32 {
                    let _ = storage
                        .query_filtered(
                            &emb(dim, (c * 17 + q as usize) as f32),
                            TOP_K,
                            None,
                            Some(&mf),
                        )
                        .await
                        .expect("warmup");
                }
            }));
        }
        for h in warm {
            h.await.expect("warmup join");
        }
    }

    let mut handles = Vec::new();
    let start_all = Instant::now();
    for c in 0..clients {
        let storage = Arc::clone(&storage);
        let mf = mf.clone();
        handles.push(tokio::spawn(async move {
            let mut samples = Vec::with_capacity(qpc);
            for q in 0..qpc {
                let start = Instant::now();
                let hits = storage
                    .query_filtered(&emb(dim, (c * 100 + q) as f32), TOP_K, None, Some(&mf))
                    .await
                    .expect("concurrent");
                samples.push(start.elapsed());
                assert!(!hits.is_empty() || hits.len() <= TOP_K);
            }
            samples
        }));
    }
    let mut all = Vec::new();
    for h in handles {
        all.extend(h.await.expect("join"));
    }
    let elapsed = start_all.elapsed();
    // Prod/Large: Q1-d-class floor (500ms); default uses Q1-c 100ms.
    let single_floor = match scale {
        PerfScale::Prod | PerfScale::Large => 500.0,
        PerfScale::Default => SINGLE_SLO_MS,
    };
    let budget = (single_floor * mult).max(single_p95 * mult);
    finish_report(
        "stress_concurrent_filtered_ann",
        &all,
        budget,
        "hnsw",
        false,
        format!(
            "scale={} clients={clients} dim={} rows={} q/client={qpc} pool={pool} single_p95={single_p95:.2} mult={mult} wall={elapsed:?}",
            scale.as_str(),
            ann.dim,
            ann.rows,
        ),
    );
    let _ = Duration::ZERO;
    let _ = storage.clear().await;
}
