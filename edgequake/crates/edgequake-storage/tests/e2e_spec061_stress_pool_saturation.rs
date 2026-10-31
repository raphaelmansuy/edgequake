//! SPEC-061/062 — intentional pool saturation (clients ≫ pool).
//!
//! Pool=5, clients=16, modest row count. Queueing is expected; hang-like p95 fails.
//! Also guards against pool deadlock (capability probe must not nest under open txs).
#![cfg(feature = "postgres")]

#[path = "support/perf_harness.rs"]
mod perf_harness;
#[path = "support/perf_stress.rs"]
mod perf_stress;
#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

use edgequake_storage::traits::{MetadataFilter, VectorStorage};
use edgequake_storage::PgVectorStorage;
use perf_harness::{finish_report, samples_after_warmup};
use perf_stress::{with_saturation_pool, POOL_SATURATION_BUDGET_MS};
use std::sync::Arc;
use std::time::{Duration, Instant};

const DIM: usize = 64;
const ROW_COUNT: usize = 2_000;
const CLIENTS: usize = 16;
const QUERIES_PER: usize = 20;
const TOP_K: usize = 10;
const POOL: u32 = 5;

fn emb(seed: f32) -> Vec<f32> {
    (0..DIM)
        .map(|i| ((i as f32 + seed) * 0.021).sin())
        .collect()
}

#[tokio::test]
async fn e2e_spec061_stress_pool_saturation() {
    let Some(base) = postgres_test_config::require_or_skip_postgres("stress061_poolsat") else {
        return;
    };
    let config = with_saturation_pool(base);
    assert_eq!(config.max_connections, POOL);
    let storage = Arc::new(PgVectorStorage::with_dimension(config, DIM));
    storage.initialize().await.expect("init");

    // Warm iterative_scan OnceCell so concurrent txs do not race capability probes.
    let warm_mf = MetadataFilter {
        workspace_id: Some("ws-sat".into()),
        tenant_id: Some("t-sat".into()),
        vector_type: Some("chunk".into()),
        document_ids: None,
        modalities: None,
        embedding_model: None,
        kinds: None,
    };
    let _ = storage
        .upsert(&[(
            "psat-warm".into(),
            emb(0.0),
            serde_json::json!({
                "type": "chunk",
                "workspace_id": "ws-sat",
                "tenant_id": "t-sat",
            }),
        )])
        .await;
    let _ = storage
        .query_filtered(&emb(0.0), TOP_K, None, Some(&warm_mf))
        .await
        .expect("warm capability + plan");

    for batch_start in (0..ROW_COUNT).step_by(500) {
        let end = (batch_start + 500).min(ROW_COUNT);
        let batch: Vec<_> = (batch_start..end)
            .map(|i| {
                (
                    format!("psat-{i}"),
                    emb(i as f32),
                    serde_json::json!({
                        "type": "chunk",
                        "workspace_id": "ws-sat",
                        "tenant_id": "t-sat",
                    }),
                )
            })
            .collect();
        storage.upsert(&batch).await.expect("upsert");
    }

    let mf = warm_mf;

    let mut handles = Vec::new();
    let wall = Instant::now();
    for c in 0..CLIENTS {
        let storage = Arc::clone(&storage);
        let mf = mf.clone();
        handles.push(tokio::spawn(async move {
            let mut samples = Vec::with_capacity(QUERIES_PER);
            for q in 0..QUERIES_PER {
                let start = Instant::now();
                let hits = storage
                    .query_filtered(&emb((c * 50 + q) as f32), TOP_K, None, Some(&mf))
                    .await
                    .map_err(|e| e.to_string())?;
                samples.push(start.elapsed());
                assert!(hits.len() <= TOP_K);
            }
            Ok::<_, String>(samples)
        }));
    }
    let mut all = Vec::new();
    let mut errors = 0usize;
    for h in handles {
        match h.await {
            Ok(Ok(samples)) => all.extend(samples),
            Ok(Err(_)) | Err(_) => errors += 1,
        }
    }
    assert_eq!(
        errors, 0,
        "pool saturation must queue, not error-storm (errors={errors})"
    );
    // Drop cold/queue spike; hang would still dominate remaining samples.
    let measured = samples_after_warmup(&all, 30);
    assert!(
        wall.elapsed() < Duration::from_secs(60),
        "saturation wall {:?} looks like pool deadlock (expected <<60s)",
        wall.elapsed()
    );
    finish_report(
        "stress_pool_saturation",
        &measured,
        POOL_SATURATION_BUDGET_MS,
        "hnsw_pool_queue",
        false,
        format!(
            "clients={CLIENTS} pool={POOL} rows={ROW_COUNT} q/client={QUERIES_PER} wall={:?} noise_ok",
            wall.elapsed()
        ),
    );
    let _ = storage.clear().await;
}
