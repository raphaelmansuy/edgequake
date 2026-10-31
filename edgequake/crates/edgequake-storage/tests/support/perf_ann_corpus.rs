//! SPEC-065 — shared ANN corpus seed / measure helpers (DRY for 063/064).
#![allow(dead_code)]

use edgequake_storage::traits::{MetadataFilter, VectorStorage};
use edgequake_storage::PgVectorStorage;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::perf_harness::percentile_p95_ms;

pub const DEFAULT_HOT_WS: &str = "ws-a";
pub const DEFAULT_COLD_WS: &str = "ws-b";

pub fn emb(dim: usize, seed: f32) -> Vec<f32> {
    (0..dim)
        .map(|i| ((i as f32 + seed) * 0.019).sin())
        .collect()
}

/// Seed all rows into a single workspace (dedicated-table / hot-set shape).
pub async fn seed_single_ws(
    storage: &PgVectorStorage,
    rows: usize,
    dim: usize,
    batch_size: usize,
    id_prefix: &str,
    tenant_id: &str,
    workspace_id: &str,
) -> f64 {
    let wall = Instant::now();
    for batch_start in (0..rows).step_by(batch_size) {
        let end = (batch_start + batch_size).min(rows);
        let batch: Vec<_> = (batch_start..end)
            .map(|i| {
                (
                    format!("{id_prefix}-{i}"),
                    emb(dim, i as f32),
                    serde_json::json!({
                        "type": "chunk",
                        "workspace_id": workspace_id,
                        "tenant_id": tenant_id,
                    }),
                )
            })
            .collect();
        storage.upsert(&batch).await.expect("upsert");
    }
    wall.elapsed().as_secs_f64() * 1000.0
}

/// Seed rows with 20% `hot_ws` / 80% `cold_ws` (ladder filter shape).
#[allow(clippy::too_many_arguments)] // corpus seed knobs stay flat for call-site clarity
pub async fn seed_ws_split(
    storage: &PgVectorStorage,
    rows: usize,
    dim: usize,
    batch_size: usize,
    id_prefix: &str,
    tenant_id: &str,
    hot_ws: &str,
    cold_ws: &str,
) -> f64 {
    let wall = Instant::now();
    for batch_start in (0..rows).step_by(batch_size) {
        let end = (batch_start + batch_size).min(rows);
        let batch: Vec<_> = (batch_start..end)
            .map(|i| {
                let ws = if i % 5 == 0 { hot_ws } else { cold_ws };
                (
                    format!("{id_prefix}-{i}"),
                    emb(dim, i as f32),
                    serde_json::json!({
                        "type": "chunk",
                        "workspace_id": ws,
                        "tenant_id": tenant_id,
                    }),
                )
            })
            .collect();
        storage.upsert(&batch).await.expect("upsert");
    }
    wall.elapsed().as_secs_f64() * 1000.0
}

pub fn workspace_filter(hot_ws: &str, tenant_id: &str) -> MetadataFilter {
    MetadataFilter {
        workspace_id: Some(hot_ws.into()),
        tenant_id: Some(tenant_id.into()),
        vector_type: Some("chunk".into()),
        document_ids: None,
        modalities: None,
        embedding_model: None,
        kinds: None,
    }
}

pub async fn measure_single_n(
    storage: &PgVectorStorage,
    dim: usize,
    filter: &MetadataFilter,
    n: usize,
    top_k: usize,
) -> (f64, Vec<Duration>) {
    for s in 0..5 {
        let _ = storage
            .query_filtered(&emb(dim, s as f32), top_k, None, Some(filter))
            .await
            .expect("warm");
    }
    let mut samples = Vec::with_capacity(n);
    for s in 0..n {
        let start = Instant::now();
        let _ = storage
            .query_filtered(&emb(dim, (s + 10) as f32), top_k, None, Some(filter))
            .await
            .expect("single");
        samples.push(start.elapsed());
    }
    (percentile_p95_ms(&samples), samples)
}

pub async fn measure_single(
    storage: &PgVectorStorage,
    dim: usize,
    filter: &MetadataFilter,
    top_k: usize,
) -> (f64, Vec<Duration>) {
    measure_single_n(storage, dim, filter, 30, top_k).await
}

pub async fn measure_stress(
    storage: Arc<PgVectorStorage>,
    dim: usize,
    filter: MetadataFilter,
    clients: usize,
    qpc: usize,
    top_k: usize,
) -> (f64, Vec<Duration>, Duration) {
    let start_all = Instant::now();
    let mut handles = Vec::new();
    for c in 0..clients {
        let storage = Arc::clone(&storage);
        let filter = filter.clone();
        handles.push(tokio::spawn(async move {
            let mut samples = Vec::with_capacity(qpc);
            for q in 0..qpc {
                let start = Instant::now();
                let hits = storage
                    .query_filtered(&emb(dim, (c * 100 + q) as f32), top_k, None, Some(&filter))
                    .await
                    .expect("concurrent");
                samples.push(start.elapsed());
                assert!(hits.len() <= top_k);
            }
            samples
        }));
    }
    let mut all = Vec::new();
    for h in handles {
        all.extend(h.await.expect("join"));
    }
    (percentile_p95_ms(&all), all, start_all.elapsed())
}
