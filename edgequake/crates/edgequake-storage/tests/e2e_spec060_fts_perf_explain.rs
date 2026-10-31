//! SPEC-060 — FTS p95 + GIN/`content_tsv` EXPLAIN gate.
//!
//! ```bash
//! export DATABASE_URL="$(cat /tmp/edgequake-db-url)"
//! export EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1
//! cargo test -p edgequake-storage --features postgres --test e2e_spec060_fts_perf_explain -- --nocapture
//! ```

#![cfg(feature = "postgres")]

#[path = "support/perf_harness.rs"]
mod perf_harness;
#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;
#[path = "support/spec091_w3.rs"]
mod w3;

use edgequake_storage::adapters::postgres::PostgresPool;
use edgequake_storage::traits::{KVStorage, MetadataFilter, VectorStorage};
use edgequake_storage::{PgVectorStorage, PostgresKVStorage, VECTOR_BACKEND_ENV};
use perf_harness::finish_report;
use std::time::{Duration, Instant};

const DIM: usize = 8;
const ROW_COUNT: usize = 10_000;
const SAMPLES: usize = 20;
const TOP_K: usize = 20;
const WS: &str = "ws-fts060";

fn percentile_p95(sorted: &[Duration]) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    let idx = ((sorted.len() as f64) * 0.95).ceil() as usize - 1;
    sorted[idx.min(sorted.len() - 1)]
}

fn emb(i: usize) -> Vec<f32> {
    (0..DIM).map(|d| ((d + i) as f32 * 0.011).sin()).collect()
}

#[tokio::test(flavor = "current_thread")]
#[allow(clippy::await_holding_lock)]
async fn e2e_spec060_fts_p95_and_gin_explain() {
    let Some(config) = postgres_test_config::require_or_skip_postgres("perf060_fts") else {
        return;
    };

    let _env = w3::w3_env_guard().await;
    // Legacy eq_*_vectors + content_tsv GIN perf path (not typed chunks FTS / #405).
    let prev_backend = std::env::var(VECTOR_BACKEND_ENV).ok();
    std::env::set_var(VECTOR_BACKEND_ENV, "legacy_tables");

    let kv = PostgresKVStorage::new(config.clone());
    kv.initialize().await.expect("kv init");
    let pool = PostgresPool::new(config.clone());
    pool.initialize().await.expect("pool init");
    let vectors = PgVectorStorage::with_pool_and_dimension(pool, config.clone(), DIM)
        .with_chunk_kv_table(config.qualified_kv_table());
    vectors.initialize().await.expect("vector init");

    eprintln!("SPEC-060 FTS: seeding {ROW_COUNT} content_ref chunks…");
    let seed_start = Instant::now();
    let chunk = 500usize;
    for batch_start in (0..ROW_COUNT).step_by(chunk) {
        let end = (batch_start + chunk).min(ROW_COUNT);
        let mut kv_batch = Vec::with_capacity(end - batch_start);
        let mut vec_batch = Vec::with_capacity(end - batch_start);
        for i in batch_start..end {
            let id = format!("fts060-{i}");
            let phrase = if i % 50 == 0 {
                "uniquephrase060 quantum entanglement"
            } else {
                "ordinary document body filler text"
            };
            kv_batch.push((
                id.clone(),
                serde_json::json!({"content": phrase, "type": "chunk"}),
            ));
            vec_batch.push((
                id.clone(),
                emb(i),
                serde_json::json!({
                    "type": "chunk",
                    "content_ref": id,
                    "document_id": format!("doc-{}", i / 10),
                    "workspace_id": WS,
                    "tenant_id": "t-fts060",
                }),
            ));
        }
        kv.upsert(&kv_batch).await.expect("kv upsert");
        vectors.upsert(&vec_batch).await.expect("vector upsert");
    }
    eprintln!("Seed done in {:?}", seed_start.elapsed());

    let mf = MetadataFilter {
        workspace_id: Some(WS.to_string()),
        tenant_id: Some("t-fts060".to_string()),
        vector_type: Some("chunk".to_string()),
        document_ids: None,
        modalities: None,
        embedding_model: None,
        kinds: None,
    };

    let _ = vectors
        .text_search_filtered("uniquephrase060", TOP_K, None, Some(&mf))
        .await
        .expect("warmup");

    let mut samples = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let start = Instant::now();
        let hits = vectors
            .text_search_filtered("uniquephrase060", TOP_K, None, Some(&mf))
            .await
            .expect("fts");
        samples.push(start.elapsed());
        assert!(
            !hits.is_empty(),
            "FTS must return hits for uniquephrase060 on content_ref corpus"
        );
    }
    finish_report(
        "fts_text_search_filtered",
        &samples,
        200.0,
        "gin",
        true,
        format!("N={ROW_COUNT}"),
    );
    samples.sort();
    let p95 = percentile_p95(&samples);
    eprintln!("OK Q-FTS: p95={p95:?} max={:?}", samples.last());

    assert_fts_gin_explain(&config).await;

    let _ = vectors.clear().await;

    match prev_backend {
        Some(v) => std::env::set_var(VECTOR_BACKEND_ENV, v),
        None => std::env::remove_var(VECTOR_BACKEND_ENV),
    }
}

async fn assert_fts_gin_explain(config: &edgequake_storage::PostgresConfig) {
    let pool = postgres_test_config::contract_pg_pool(config).await;
    let table = format!("public.eq_{}_vectors", config.table_prefix());
    let bare = format!("eq_{}_vectors", config.table_prefix());

    // Prove GIN index exists (planner may still Seq Scan tiny warm tables).
    let idx_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint FROM pg_indexes
        WHERE tablename = $1
          AND (indexdef ILIKE '%gin%' OR indexdef ILIKE '%content_tsv%')
        "#,
    )
    .bind(&bare)
    .fetch_one(&pool)
    .await
    .expect("list FTS indexes");
    assert!(idx_count > 0, "content_tsv GIN index must exist on {bare}");

    // Force index path so EXPLAIN asserts GIN/Bitmap (not heap Seq Scan on body).
    let mut tx = pool.begin().await.expect("begin explain tx");
    sqlx::query("SET LOCAL enable_seqscan = off")
        .execute(&mut *tx)
        .await
        .expect("disable seqscan");
    sqlx::query(&format!("ANALYZE {table}"))
        .execute(&mut *tx)
        .await
        .ok();
    let sql = format!(
        r#"EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)
           SELECT v.id
           FROM {table} v
           WHERE v.content_tsv @@ websearch_to_tsquery('english', $1)
           ORDER BY ts_rank_cd(v.content_tsv, websearch_to_tsquery('english', $1)) DESC
           LIMIT 20"#
    );
    let plan_rows: Vec<(String,)> = sqlx::query_as(&sql)
        .bind("uniquephrase060")
        .fetch_all(&mut *tx)
        .await
        .expect("EXPLAIN FTS");
    tx.commit().await.ok();
    let plan = plan_rows
        .into_iter()
        .map(|r| r.0)
        .collect::<Vec<_>>()
        .join("\n");
    let lower = plan.to_lowercase();
    assert!(
        lower.contains("bitmap index scan")
            || lower.contains("index scan")
            || lower.contains("gin"),
        "FTS EXPLAIN must use GIN/Bitmap index path; plan was:\n{plan}"
    );
    assert!(
        !lower.contains("seq scan"),
        "FTS EXPLAIN must not Seq Scan when seqscan disabled; plan was:\n{plan}"
    );
    assert!(
        lower.contains("buffers:") || lower.contains("shared"),
        "EXPLAIN ANALYZE BUFFERS should report buffer stats; plan was:\n{plan}"
    );
    eprintln!("OK EXPLAIN FTS GIN:\n{plan}");
}
