//! SPEC-061 — `node_degrees_batch` p95 on the plain graph tables.
#![cfg(feature = "postgres")]

#[path = "support/perf_harness.rs"]
mod perf_harness;
#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

use edgequake_storage::traits::{GraphStorage, GraphStorageMutateOps, GraphStorageReadOps};
use edgequake_storage::PostgresAGEGraphStorage;
use perf_harness::{finish_report, samples_after_warmup};
use std::collections::HashMap;
use std::time::Instant;

const NODE_N: usize = 1_000;
const SAMPLES: usize = 32;

#[tokio::test]
async fn e2e_spec061_node_degrees_batch_p95() {
    let Some(config) = postgres_test_config::require_or_skip_postgres("perf061_deg") else {
        return;
    };
    let prev = std::env::var("EDGEQUAKE_NATIVE_GRAPH_WRITES").ok();
    std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", "1");

    let graph = PostgresAGEGraphStorage::new(config.clone());
    graph.initialize().await.expect("init");

    // Star: hub-0..9 each connect to 100 leaves → enough degree signal
    let mut nodes = Vec::new();
    for i in 0..NODE_N {
        let mut p = HashMap::new();
        p.insert("entity_type".to_string(), serde_json::json!("CONCEPT"));
        nodes.push((format!("DEG_{i}"), p));
    }
    for chunk in nodes.chunks(250) {
        graph.upsert_nodes_batch(chunk).await.expect("nodes");
    }
    let mut edges = Vec::new();
    for i in 0..NODE_N {
        if i % 10 == 0 {
            continue;
        }
        let hub = format!("DEG_{}", (i / 10) * 10);
        let mut p = HashMap::new();
        p.insert("relation_type".to_string(), serde_json::json!("RELATED"));
        edges.push((hub, format!("DEG_{i}"), p));
    }
    for chunk in edges.chunks(250) {
        graph.upsert_edges_batch(chunk).await.expect("edges");
    }

    let ids: Vec<String> = (0..NODE_N).map(|i| format!("DEG_{i}")).collect();
    let _ = graph.node_degrees_batch(&ids).await.expect("warm");

    let mut samples = Vec::new();
    for _ in 0..SAMPLES {
        let start = Instant::now();
        let deg = graph.node_degrees_batch(&ids).await.expect("degrees");
        samples.push(start.elapsed());
        assert_eq!(deg.len(), NODE_N);
    }
    let hygiene = samples_after_warmup(&samples, 20);
    finish_report(
        "graph_node_degrees_batch",
        &hygiene,
        100.0,
        "eq_source_id_btree",
        false,
        format!("N={NODE_N}"),
    );

    match prev {
        Some(v) => std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", v),
        None => std::env::remove_var("EDGEQUAKE_NATIVE_GRAPH_WRITES"),
    }
    let _ = graph.clear().await;
}
