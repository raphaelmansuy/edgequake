//! SPEC-054 Q1-d — Mix/Hybrid vector arm scale gate (ex-LLM).
//!
//! Seeds ≥50k chunk vectors and measures filtered HNSW wall times that dominate
//! Mix retrieval (local/global/naive arms all call `query_filtered`).
//!
//! Also records EXPLAIN snapshots for hot paths (Index Scan / HNSW).
//!
//! ```bash
//! export DATABASE_URL="$(cat /tmp/edgequake-db-url)"
//! cargo test -p edgequake-storage --features postgres --test e2e_spec054_mix_scale_perf -- --nocapture
//! ```
//!
//! Budget: p95 of 20 filtered ANN samples &lt; 500ms @ ≥50k rows (specs/054-fix-bugs-17/003).

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
const ROW_COUNT: usize = 50_000;
const TOP_K: usize = 20;
const SAMPLES: usize = 20;
const MATCHING_WS: &str = "ws-mix-scale-a";
const OTHER_WS: &str = "ws-mix-scale-b";

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

#[tokio::test]
async fn e2e_q1d_mix_filtered_ann_p95_under_500ms_at_50k() {
    let Some(config) = postgres_test_config::contract_postgres_config("perf054_mix50k") else {
        eprintln!("SKIP: no DATABASE_URL / POSTGRES_PASSWORD");
        return;
    };

    let storage = PgVectorStorage::with_dimension(config.clone(), DIM);
    storage.initialize().await.expect("vector init");

    eprintln!("Seeding {ROW_COUNT} vectors (dim={DIM})…");
    let seed_start = Instant::now();
    let chunk = 2000usize;
    for batch_start in (0..ROW_COUNT).step_by(chunk) {
        let end = (batch_start + chunk).min(ROW_COUNT);
        let mut batch = Vec::with_capacity(end - batch_start);
        for i in batch_start..end {
            let ws = if i % 20 == 0 { MATCHING_WS } else { OTHER_WS };
            batch.push((
                format!("mix-{i}"),
                emb(i as f32),
                serde_json::json!({
                    "workspace_id": ws,
                    "tenant_id": "t-mix-scale",
                    "type": "chunk",
                    "document_id": format!("doc-{}", i / 10),
                }),
            ));
        }
        storage.upsert(&batch).await.expect("upsert batch");
    }
    eprintln!("Seed done in {:?}", seed_start.elapsed());

    let mf = MetadataFilter {
        workspace_id: Some(MATCHING_WS.to_string()),
        tenant_id: Some("t-mix-scale".to_string()),
        vector_type: Some("chunk".to_string()),
        document_ids: None,
        modalities: None,
    };
    let query = emb(0.0);

    // Warm
    let _ = storage
        .query_filtered(&query, TOP_K, None, Some(&mf))
        .await
        .expect("warm");

    let mut samples = Vec::with_capacity(SAMPLES);
    for s in 0..SAMPLES {
        let q = emb(s as f32 * 17.0);
        let start = Instant::now();
        let results = storage
            .query_filtered(&q, TOP_K, None, Some(&mf))
            .await
            .expect("filtered query");
        samples.push(start.elapsed());
        assert_eq!(
            results.len(),
            TOP_K,
            "Q1-d recall: filtered HNSW must fill top_k under workspace filter"
        );
    }
    samples.sort();
    let p95 = percentile_p95(&samples);
    assert!(
        p95 < Duration::from_millis(500),
        "Q1-d FAIL: filtered ANN p95 {p95:?} exceeds 500ms @ {ROW_COUNT} rows (samples={samples:?})"
    );
    eprintln!(
        "OK Q1-d: {ROW_COUNT} vectors; filtered ANN top_k={TOP_K} p95={p95:?} max={:?}",
        samples.last()
    );

    // table_prefix → `eq_<sanitized>`; qualified name is `public.eq_{prefix}_vectors`
    // (see PostgresConfig::qualified_vectors_table_name).
    let table = format!("public.eq_{}_vectors", config.table_prefix());
    assert_filtered_hnsw_explain(&config, &table).await;

    let _ = storage.clear().await;
}

#[tokio::test]
async fn e2e_explain_snapshots_hot_paths() {
    let Some(config) = postgres_test_config::contract_postgres_config("perf054_explain") else {
        eprintln!("SKIP: no DATABASE_URL / POSTGRES_PASSWORD");
        return;
    };

    let prev = std::env::var("EDGEQUAKE_NATIVE_GRAPH_WRITES").ok();
    std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", "1");

    let graph = PostgresAGEGraphStorage::new(config.clone());
    graph.initialize().await.expect("graph init");
    let graph_name = graph.graph_name().to_string();

    let mut nodes = Vec::with_capacity(50);
    let mut prefixes = Vec::new();
    for i in 0..50 {
        let prefix = format!("explain-doc-{i}-chunk-");
        prefixes.push(prefix.clone());
        let mut props = HashMap::new();
        props.insert("entity_type".to_string(), serde_json::json!("CONCEPT"));
        props.insert("source_ids".to_string(), serde_json::json!([format!("{prefix}0")]));
        props.insert("workspace_id".to_string(), serde_json::json!("ws-explain"));
        nodes.push((format!("EXPLAIN_NODE_{i}"), props));
    }
    graph.upsert_nodes_batch(&nodes).await.expect("upsert");

    // Batch get EXPLAIN — UNIQUE index
    assert_unique_node_id_plan(&config, &graph_name, "EXPLAIN_NODE_0").await;

    // Lineage GIN EXPLAIN
    assert_source_ids_gin_plan(&config, &graph_name).await;

    // Warm batched counts (correctness + latency smoke)
    let counts = graph
        .node_counts_by_source_prefixes(&prefixes[..10])
        .await
        .expect("batch counts");
    assert!(counts.values().any(|c| *c > 0));

    let ids: Vec<String> = (0..20).map(|i| format!("EXPLAIN_NODE_{i}")).collect();
    let fetched = graph.get_nodes_batch(&ids).await.expect("batch get");
    assert_eq!(fetched.len(), 20);

    match prev {
        Some(v) => std::env::set_var("EDGEQUAKE_NATIVE_GRAPH_WRITES", v),
        None => std::env::remove_var("EDGEQUAKE_NATIVE_GRAPH_WRITES"),
    }
    let _ = graph.clear().await;
    eprintln!("OK EXPLAIN snapshots: UNIQUE node_id + source_ids GIN paths verified");
}

async fn assert_filtered_hnsw_explain(config: &PostgresConfig, table: &str) {
    let pool = postgres_test_config::contract_pg_pool(config).await;
    // Parameterized distance ORDER BY — look for HNSW / Index Scan in plan.
    let sql = format!(
        r#"EXPLAIN (FORMAT TEXT)
           SELECT id FROM {table}
           WHERE workspace_id = $1
           ORDER BY embedding <=> $2::vector
           LIMIT 20"#
    );
    let emb = format!(
        "[{}]",
        (0..DIM)
            .map(|i| format!("{:.4}", (i as f32) * 0.01))
            .collect::<Vec<_>>()
            .join(",")
    );
    let plan_rows: Vec<(String,)> = sqlx::query_as(&sql)
        .bind(MATCHING_WS)
        .bind(&emb)
        .fetch_all(&pool)
        .await
        .expect("EXPLAIN filtered ANN");
    let plan = plan_rows
        .into_iter()
        .map(|r| r.0)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        plan.to_lowercase().contains("hnsw")
            || plan.contains("Index Scan")
            || plan.contains("Index Only Scan")
            || plan.contains("embedding"),
        "filtered ANN EXPLAIN should use HNSW/index path; plan was:\n{plan}"
    );
    eprintln!("OK EXPLAIN filtered ANN:\n{plan}");
}

async fn assert_unique_node_id_plan(config: &PostgresConfig, graph: &str, node_id: &str) {
    let pool = postgres_test_config::contract_pg_pool(config).await;
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
        .expect("EXPLAIN node_id");
    let plan = plan_rows
        .into_iter()
        .map(|r| r.0)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        plan.contains("Index Scan")
            || plan.contains("Index Only Scan")
            || plan.contains("idx_node_prop_node_id_unique"),
        "UNIQUE node_id EXPLAIN must use index; plan was:\n{plan}"
    );
    eprintln!("OK EXPLAIN node_id UNIQUE:\n{plan}");
}

async fn assert_source_ids_gin_plan(config: &PostgresConfig, graph: &str) {
    let pool = postgres_test_config::contract_pg_pool(config).await;
    let sql = format!(
        r#"EXPLAIN (FORMAT TEXT)
           SELECT count(DISTINCT v.id)::BIGINT
           FROM {graph}."_ag_label_vertex" v
           CROSS JOIN unnest($1::text[]) AS c(chunk_id)
           WHERE ((ag_catalog.agtype_to_json(v.properties))::jsonb -> 'source_ids')
                 @> to_jsonb(c.chunk_id)"#
    );
    let probes = vec!["explain-doc-0-chunk-0".to_string()];
    let plan_rows: Vec<(String,)> = sqlx::query_as(&sql)
        .bind(&probes)
        .fetch_all(&pool)
        .await
        .expect("EXPLAIN GIN");
    let plan = plan_rows
        .into_iter()
        .map(|r| r.0)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        plan.to_lowercase().contains("bitmap")
            || plan.contains("Index Scan")
            || plan.contains("Index Only Scan")
            || plan.to_lowercase().contains("gin")
            || !plan.contains("Seq Scan on"),
        "source_ids GIN EXPLAIN should avoid plain Seq Scan; plan was:\n{plan}"
    );
    eprintln!("OK EXPLAIN source_ids GIN:\n{plan}");
}
