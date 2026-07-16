//! SPEC-054 / specs/054 — verified AGE + pgvector performance e2e.
//!
//! Run:
//! ```bash
//! export DATABASE_URL="$(cat /tmp/edgequake-db-url)"
//! cargo test -p edgequake-storage --features postgres --test e2e_spec054_age_pgvector_perf -- --nocapture
//! ```
//!
//! Budgets (warm local Postgres / AGE / pgvector 0.8+) — measured 2026-07-16:
//! - Q1: filtered HNSW top_k=20 under workspace filter → results==20 and max wall < 100ms
//!   (observed ~3ms on 2k-row table)
//! - Q3: native upsert 500 nodes < 500ms (observed ~50ms)
//! - Q2: get_nodes_batch 100 ids < 50ms (observed ~2ms)
//! - Index: EXPLAIN on UNIQUE node_id uses Index Scan

#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

use edgequake_storage::traits::{
    GraphStorage, GraphStorageAnalyticsOps, GraphStorageMutateOps, GraphStorageReadOps,
    MetadataFilter, VectorStorage,
};
use edgequake_storage::{PgVectorStorage, PostgresAGEGraphStorage, PostgresConfig};
use std::collections::HashMap;
use std::time::{Duration, Instant};

const DIM: usize = 64;
const FILTERED_TOP_K: usize = 20;
const MATCHING_WS: &str = "ws-perf-a";
const OTHER_WS: &str = "ws-perf-b";

fn emb(seed: f32) -> Vec<f32> {
    (0..DIM)
        .map(|i| ((i as f32 + seed) * 0.017).sin())
        .collect()
}

#[tokio::test]
async fn e2e_filtered_hnsw_meets_topk_under_workspace_filter() {
    let Some(config) = postgres_test_config::contract_postgres_config("perf054_vec") else {
        eprintln!("SKIP: no DATABASE_URL / POSTGRES_PASSWORD");
        return;
    };

    let storage = PgVectorStorage::with_dimension(config, DIM);
    storage.initialize().await.expect("vector init");

    // 100 matching + 1900 distractors — selective filter (~5%).
    let mut batch = Vec::with_capacity(2000);
    for i in 0..100 {
        batch.push((
            format!("match-{i}"),
            emb(i as f32),
            serde_json::json!({
                "workspace_id": MATCHING_WS,
                "tenant_id": "t-perf",
                "type": "chunk",
                "document_id": format!("doc-a-{i}"),
            }),
        ));
    }
    for i in 0..1900 {
        batch.push((
            format!("other-{i}"),
            emb(1000.0 + i as f32),
            serde_json::json!({
                "workspace_id": OTHER_WS,
                "tenant_id": "t-perf",
                "type": "chunk",
                "document_id": format!("doc-b-{i}"),
            }),
        ));
    }
    storage.upsert(&batch).await.expect("bulk upsert");

    let mf = MetadataFilter {
        workspace_id: Some(MATCHING_WS.to_string()),
        tenant_id: Some("t-perf".to_string()),
        vector_type: Some("chunk".to_string()),
        document_ids: None,
        modalities: None,
    };

    // Warm-up (index / caches)
    let _ = storage
        .query_filtered(&emb(0.0), FILTERED_TOP_K, None, Some(&mf))
        .await
        .expect("warmup");

    let mut samples = Vec::new();
    for _ in 0..5 {
        let start = Instant::now();
        let results = storage
            .query_filtered(&emb(1.0), FILTERED_TOP_K, None, Some(&mf))
            .await
            .expect("filtered query");
        samples.push(start.elapsed());
        assert_eq!(
            results.len(),
            FILTERED_TOP_K,
            "Q1 recall: filtered HNSW+iterative_scan must fill top_k under workspace filter"
        );
        assert!(
            results.iter().all(|r| {
                r.metadata.get("workspace_id").and_then(|v| v.as_str()) == Some(MATCHING_WS)
            }),
            "all hits must match workspace filter"
        );
    }

    samples.sort();
    let worst = samples[samples.len() - 1];
    assert!(
        worst < Duration::from_millis(100),
        "Q1 FAIL: filtered ANN worst {worst:?} exceeds 100ms budget (samples={samples:?})"
    );
    eprintln!(
        "OK Q1: filtered HNSW top_k={FILTERED_TOP_K} filled; wall samples={samples:?} max={worst:?}"
    );

    let _ = storage.clear().await;
}

#[tokio::test]
async fn e2e_native_upsert_batch_get_and_unique_index_plan() {
    let Some(config) = postgres_test_config::contract_postgres_config("perf054_graph") else {
        eprintln!("SKIP: no DATABASE_URL / POSTGRES_PASSWORD");
        return;
    };

    let prev = std::env::var("EDGEQUAKE_NATIVE_GRAPH_WRITES").ok();
    std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", "1");

    let storage = PostgresAGEGraphStorage::new(config.clone());
    storage.initialize().await.expect("graph init");
    let graph = storage.graph_name().to_string();

    let nodes: Vec<(String, HashMap<String, serde_json::Value>)> = (0..500)
        .map(|i| {
            let mut props = HashMap::new();
            props.insert("entity_type".to_string(), serde_json::json!("CONCEPT"));
            props.insert(
                "description".to_string(),
                serde_json::json!(format!("perf node {i}")),
            );
            props.insert(
                "workspace_id".to_string(),
                serde_json::json!("ws-perf-graph"),
            );
            (format!("PERF_NODE_{i}"), props)
        })
        .collect();

    let start = Instant::now();
    storage
        .upsert_nodes_batch(&nodes)
        .await
        .expect("native upsert 500");
    let upsert_elapsed = start.elapsed();
    assert!(
        upsert_elapsed < Duration::from_millis(500),
        "Q3 FAIL: native upsert 500 nodes took {upsert_elapsed:?} (budget 500ms)"
    );

    let ids: Vec<String> = (0..100).map(|i| format!("PERF_NODE_{i}")).collect();
    let start = Instant::now();
    let fetched = storage.get_nodes_batch(&ids).await.expect("batch get");
    let batch_elapsed = start.elapsed();
    assert_eq!(fetched.len(), 100, "batch get must return all 100 ids");
    assert!(
        batch_elapsed < Duration::from_millis(50),
        "Q2 FAIL: get_nodes_batch(100) took {batch_elapsed:?} (budget 50ms)"
    );
    eprintln!(
        "OK Q2/Q3: native upsert 500 in {upsert_elapsed:?}; get_nodes_batch(100) in {batch_elapsed:?}"
    );

    assert_unique_index_plan(&config, &graph, "PERF_NODE_0").await;

    match prev {
        Some(v) => std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", v),
        None => std::env::remove_var("EDGEQUAKE_NATIVE_GRAPH_WRITES"),
    }

    let _ = storage.clear().await;
}

/// L1-a AGE portion: batched source-prefix counts must beat N sequential
/// round-trips and finish under 200ms for ~20 prefixes on a warm graph.
#[tokio::test]
async fn e2e_batched_source_prefix_counts_under_budget() {
    let Some(config) = postgres_test_config::contract_postgres_config("perf054_lineage") else {
        eprintln!("SKIP: no DATABASE_URL / POSTGRES_PASSWORD");
        return;
    };

    let prev = std::env::var("EDGEQUAKE_NATIVE_GRAPH_WRITES").ok();
    std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", "1");

    let storage = PostgresAGEGraphStorage::new(config);
    storage.initialize().await.expect("graph init");

    let doc_count = 20usize;
    let mut nodes = Vec::with_capacity(doc_count * 3);
    let mut prefixes = Vec::with_capacity(doc_count);
    for d in 0..doc_count {
        let doc_id = format!("l1a-doc-{d}");
        let prefix = format!("{doc_id}-chunk-");
        prefixes.push(prefix.clone());
        for c in 0..3 {
            let mut props = HashMap::new();
            props.insert("entity_type".to_string(), serde_json::json!("CONCEPT"));
            props.insert(
                "source_ids".to_string(),
                serde_json::json!([format!("{prefix}{c}")]),
            );
            props.insert(
                "workspace_id".to_string(),
                serde_json::json!("ws-l1a-lineage"),
            );
            nodes.push((format!("L1A_NODE_{d}_{c}"), props));
        }
    }
    storage
        .upsert_nodes_batch(&nodes)
        .await
        .expect("seed lineage nodes");

    // Warm
    let _ = storage
        .node_counts_by_source_prefixes(&prefixes)
        .await
        .expect("warm batch");

    let start = Instant::now();
    let batch = storage
        .node_counts_by_source_prefixes(&prefixes)
        .await
        .expect("batched counts");
    let batch_elapsed = start.elapsed();

    assert_eq!(batch.len(), doc_count);
    for p in &prefixes {
        assert_eq!(
            batch.get(p).copied().unwrap_or(0),
            3,
            "prefix {p} should count 3 nodes"
        );
    }
    assert!(
        batch_elapsed < Duration::from_millis(200),
        "L1-a FAIL: batched prefix counts took {batch_elapsed:?} (budget 200ms)"
    );

    // Sequential path should be slower or equal — assert batch is not a
    // disguised N+1 (wall time roughly comparable to one prefix × small factor).
    let start = Instant::now();
    let one = storage
        .node_count_by_source_prefix(&prefixes[0])
        .await
        .expect("single");
    let one_elapsed = start.elapsed();
    assert_eq!(one, 3);
    assert!(
        batch_elapsed < one_elapsed * 10 + Duration::from_millis(50),
        "L1-a FAIL: batch {batch_elapsed:?} looks like N+1 vs single {one_elapsed:?}"
    );
    eprintln!(
        "OK L1-a AGE: batched {doc_count} prefixes in {batch_elapsed:?} (single={one_elapsed:?})"
    );

    match prev {
        Some(v) => std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", v),
        None => std::env::remove_var("EDGEQUAKE_NATIVE_GRAPH_WRITES"),
    }
    let _ = storage.clear().await;
}

async fn assert_unique_index_plan(config: &PostgresConfig, graph: &str, node_id: &str) {
    let pool = postgres_test_config::contract_pg_pool(config).await;
    // Expression matches idx_node_prop_node_id_unique definition.
    let sql = format!(
        r#"EXPLAIN (FORMAT TEXT)
           SELECT id FROM {graph}."Node"
           WHERE ag_catalog.agtype_to_json(properties)->>'node_id' = $1
           LIMIT 1"#
    );
    let plan_rows: Vec<(String,)> = sqlx::query_as(&sql)
        .bind(node_id)
        .fetch_all(&pool)
        .await
        .expect("EXPLAIN");
    let plan = plan_rows
        .into_iter()
        .map(|r| r.0)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        plan.contains("Index Scan")
            || plan.contains("Index Only Scan")
            || plan.contains("idx_node_prop_node_id_unique"),
        "UNIQUE expression index must be used for node_id lookup; plan was:\n{plan}"
    );
    eprintln!("OK Index: node_id EXPLAIN uses index path:\n{plan}");
}
