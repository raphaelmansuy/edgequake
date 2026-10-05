//! Measure real filtered HNSW, exact recall, ordering, plan, and latency together.
//! Temporary data is connection-local; no persistent user data is touched.
#![cfg(feature = "postgres")]

#[path = "support/ann_fixture.rs"]
mod ann_fixture;
#[path = "support/perf_harness.rs"]
mod perf_harness;
#[path = "support/postgres_access_pool.rs"]
mod postgres_access_pool;

use edgequake_storage::adapters::postgres::{
    build_ann_select_sql, AnnExactReorderPolicy, LocalTimeoutTx, VectorIndexType,
};
use edgequake_storage::PgVectorStorage;
use sqlx::{PgConnection, Postgres, QueryBuilder};
use std::collections::HashSet;
use std::time::Instant;

use ann_fixture::{embedding_text as embedding, DIM};
const ROWS: usize = 6_000;
const TOP_K: usize = 10;
const PROBES: usize = 32;

async fn create_fixture(conn: &mut PgConnection, scope_stride: usize) {
    sqlx::query(
        "CREATE TEMP TABLE access_ann_probe (
        id text PRIMARY KEY, metadata jsonb NOT NULL, embedding vector(32) NOT NULL,
        tenant_id text GENERATED ALWAYS AS (metadata->>'tenant_id') STORED,
        workspace_id text GENERATED ALWAYS AS (metadata->>'workspace_id') STORED)",
    )
    .execute(&mut *conn)
    .await
    .unwrap();
    for start in (0..ROWS).step_by(250) {
        let batch: Vec<_> = (start..(start + 250).min(ROWS))
            .map(|i| {
                let metadata = if i % scope_stride == 0 {
                    serde_json::json!({"workspace_id": "small", "tenant_id": "target"})
                } else {
                    serde_json::json!({"workspace_id": "large", "tenant_id": "other"})
                };
                (i.to_string(), metadata, embedding(i))
            })
            .collect();
        let mut query = QueryBuilder::<Postgres>::new(
            "INSERT INTO access_ann_probe (id, metadata, embedding) ",
        );
        query.push_values(&batch, |mut row, (id, metadata, embedding)| {
            row.push_bind(id)
                .push_bind(metadata)
                .push_bind(embedding)
                .push_unseparated("::vector(32)");
        });
        query.build().execute(&mut *conn).await.unwrap();
    }
    sqlx::query("CREATE INDEX access_ann_probe_hnsw ON access_ann_probe USING hnsw (embedding vector_cosine_ops) WITH (m=16, ef_construction=128)")
        .execute(&mut *conn).await.unwrap();
    sqlx::query("ANALYZE access_ann_probe")
        .execute(&mut *conn)
        .await
        .unwrap();
}

#[tokio::test]
async fn filtered_hnsw_plan_recall_and_latency() {
    let Some(pool) = postgres_access_pool::test_pool().await else {
        return;
    };
    let mut conn = pool.acquire().await.unwrap();
    // A selective workspace owns 5% of rows in the shared ANN index.
    create_fixture(&mut conn, 20).await;

    let policy = AnnExactReorderPolicy {
        enabled: true,
        candidate_k: TOP_K * 4,
    };
    let sql = build_ann_select_sql(
        "access_ann_probe",
        "vector(32)",
        "metadata->>'workspace_id' = 'small' AND metadata->>'tenant_id' = 'target'",
        2,
        TOP_K,
        &policy,
    );
    let version: String =
        sqlx::query_scalar("SELECT extversion FROM pg_extension WHERE extname = 'vector'")
            .fetch_one(&mut *conn)
            .await
            .unwrap();
    let pg_version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    let tuning = PgVectorStorage::search_tuning_statements_with_hnsw_mode(
        VectorIndexType::HNSW,
        TOP_K,
        true,
        true,
        "relaxed_order",
    );
    let mut samples = Vec::with_capacity(PROBES);
    let mut recalls = Vec::with_capacity(PROBES);
    let mut explain = serde_json::Value::Null;

    for probe in 0..PROBES {
        let query_embedding = embedding(100_000 + probe);
        let mut tx = LocalTimeoutTx::begin(&mut conn, 2_000).await.unwrap();
        // Prove this measurement exercises ANN even when a small fixture could
        // make an exact scan cheaper. Normal product planning is unchanged.
        sqlx::query("SET LOCAL enable_seqscan = off")
            .execute(tx.as_mut())
            .await
            .unwrap();
        for statement in &tuning {
            sqlx::query(statement).execute(tx.as_mut()).await.unwrap();
        }
        if probe == 0 {
            explain = sqlx::query_scalar(&format!("EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON) {sql}"))
                .bind(&query_embedding)
                .bind(TOP_K as i64)
                .fetch_one(tx.as_mut())
                .await
                .unwrap();
            let plan = explain.to_string();
            assert!(
                plan.contains("access_ann_probe_hnsw"),
                "must exercise the HNSW index: {plan}"
            );
            assert!(
                plan.contains("Actual Rows"),
                "capture executed plan, not estimates"
            );
            assert!(
                plan.contains("Local Hit Blocks"),
                "capture real buffer counters"
            );
        }
        let start = Instant::now();
        let hits: Vec<(String, serde_json::Value, f64)> = sqlx::query_as(&sql)
            .bind(&query_embedding)
            .bind(TOP_K as i64)
            .fetch_all(tx.as_mut())
            .await
            .unwrap();
        samples.push(start.elapsed());
        assert_eq!(
            hits.len(),
            TOP_K,
            "selective filtering must still fill top-k"
        );
        assert!(
            hits.windows(2).all(|pair| pair[0].2 >= pair[1].2),
            "relaxed ANN must be reordered by score"
        );
        assert!(hits
            .iter()
            .all(|(_, metadata, _)| metadata["tenant_id"] == "target"
                && metadata["workspace_id"] == "small"));

        sqlx::query("SET LOCAL enable_seqscan = on")
            .execute(tx.as_mut())
            .await
            .unwrap();
        sqlx::query("SET LOCAL enable_indexscan = off")
            .execute(tx.as_mut())
            .await
            .unwrap();
        sqlx::query("SET LOCAL enable_bitmapscan = off")
            .execute(tx.as_mut())
            .await
            .unwrap();
        let exact: Vec<(String,)> = sqlx::query_as(
            "SELECT id FROM access_ann_probe WHERE metadata->>'workspace_id' = 'small' AND metadata->>'tenant_id' = 'target' ORDER BY embedding <=> $1::vector(32) LIMIT $2")
            .bind(&query_embedding).bind(TOP_K as i64).fetch_all(tx.as_mut()).await.unwrap();
        let exact: HashSet<_> = exact.into_iter().map(|(id,)| id).collect();
        recalls.push(
            hits.iter().filter(|(id, _, _)| exact.contains(id)).count() as f64 / TOP_K as f64,
        );
        tx.commit().await.unwrap();
    }
    let recall = recalls.iter().sum::<f64>() / PROBES as f64;
    assert!(recall >= 0.95, "mean filtered recall@10 {recall:.3} < 0.95");
    let measured = perf_harness::samples_after_warmup(&samples, 30);
    perf_harness::finish_report("filtered_hnsw_recall", &measured, 500.0, "hnsw_materialized_reorder", true,
        format!("rows={ROWS} dimensions={DIM} selectivity=0.05 recall_at_10={recall:.3} pg={pg_version} pgvector={version}"));
    println!("ANN_EXPLAIN {}", explain);
    drop(conn);
    pool.close().await;
}

async fn measure_natural_plan(conn: &mut PgConnection, scope: &str, op: &str) -> serde_json::Value {
    let predicate = match scope {
        "small" => "tenant_id = 'target' AND workspace_id = 'small'",
        "large" => "tenant_id = 'other' AND workspace_id = 'large'",
        _ => unreachable!("fixture scopes only"),
    };
    let sql = build_ann_select_sql(
        "access_ann_probe",
        "vector(32)",
        predicate,
        2,
        TOP_K,
        &AnnExactReorderPolicy {
            enabled: true,
            candidate_k: TOP_K * 4,
        },
    );
    // Adding zero prevents the distance ORDER BY from matching HNSW. All scan
    // methods stay enabled; the exact reference may use the scope B-tree.
    let exact_sql = format!(
        "SELECT id FROM access_ann_probe WHERE {predicate}
         ORDER BY (embedding <=> $1::vector(32)) + 0 LIMIT $2"
    );
    let tuning = PgVectorStorage::search_tuning_statements_with_hnsw_mode(
        VectorIndexType::HNSW,
        TOP_K * 4,
        true,
        true,
        "relaxed_order",
    );
    let mut samples = Vec::new();
    let mut recalls = Vec::new();
    let mut explain = serde_json::Value::Null;
    for probe in 0..PROBES {
        let vector = embedding(100_000 + probe);
        let mut tx = LocalTimeoutTx::begin(conn, 2_000).await.unwrap();
        for statement in &tuning {
            sqlx::query(statement).execute(tx.as_mut()).await.unwrap();
        }
        if probe == 0 {
            explain = sqlx::query_scalar(&format!("EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON) {sql}"))
                .bind(&vector)
                .bind(TOP_K as i64)
                .fetch_one(tx.as_mut())
                .await
                .unwrap();
        }
        let start = Instant::now();
        let hits: Vec<(String, serde_json::Value, f64)> = sqlx::query_as(&sql)
            .bind(&vector)
            .bind(TOP_K as i64)
            .fetch_all(tx.as_mut())
            .await
            .unwrap();
        samples.push(start.elapsed());
        assert_eq!(hits.len(), TOP_K);
        assert!(hits.windows(2).all(|pair| pair[0].2 >= pair[1].2));
        assert!(hits
            .iter()
            .all(|(_, metadata, _)| metadata["workspace_id"] == scope));
        let exact: HashSet<String> = sqlx::query_scalar(&exact_sql)
            .bind(&vector)
            .bind(TOP_K as i64)
            .fetch_all(tx.as_mut())
            .await
            .unwrap()
            .into_iter()
            .collect();
        recalls.push(
            hits.iter().filter(|(id, _, _)| exact.contains(id)).count() as f64 / TOP_K as f64,
        );
        tx.commit().await.unwrap();
    }
    let recall = recalls.iter().sum::<f64>() / PROBES as f64;
    assert!(recall >= 0.95, "{op}: recall@10 {recall} < 0.95");
    let report = perf_harness::finish_report(
        op, &perf_harness::samples_after_warmup(&samples, 30), 500.0,
        "natural_planner", true,
        format!("rows={ROWS} dimensions={DIM} scope={scope} recall_at_10={recall:.3} scan_switches_forced=false"),
    );
    serde_json::json!({
        "scope": scope, "recall_at_10": recall, "explain": explain,
        "measurement": serde_json::from_str::<serde_json::Value>(&report.to_json_line()).unwrap(),
        "planner_scan_switches_forced": false
    })
}

#[tokio::test]
async fn natural_planner_compares_scope_btree_exact_search_with_shared_hnsw() {
    let Some(pool) = postgres_access_pool::test_pool().await else {
        return;
    };
    let mut conn = pool.acquire().await.unwrap();
    // 1% selective scope and 99% broad scope: same rows, query and tuning before
    // and after adding the composite scope index already used by legacy DDL.
    create_fixture(&mut conn, 100).await;
    let before = measure_natural_plan(&mut conn, "small", "ann_selective_before_scope_index").await;
    sqlx::query("CREATE INDEX access_ann_probe_scope ON access_ann_probe (tenant_id, workspace_id) WHERE tenant_id IS NOT NULL")
        .execute(&mut *conn).await.unwrap();
    sqlx::query("ANALYZE access_ann_probe")
        .execute(&mut *conn)
        .await
        .unwrap();
    let after = measure_natural_plan(&mut conn, "small", "ann_selective_after_scope_index").await;
    let broad = measure_natural_plan(&mut conn, "large", "ann_broad_with_scope_index").await;
    let selective_plan = after["explain"].to_string();
    assert!(
        selective_plan.contains("access_ann_probe_scope"),
        "{selective_plan}"
    );
    assert!(
        !selective_plan.contains("access_ann_probe_hnsw"),
        "small scopes should use exact filtered search: {selective_plan}"
    );
    assert_eq!(after["recall_at_10"], 1.0);
    assert!(
        broad["explain"]
            .to_string()
            .contains("access_ann_probe_hnsw"),
        "broad scope should use shared ANN: {}",
        broad["explain"]
    );
    println!(
        "ANN_PLANNER_REPORT {}",
        serde_json::json!({"before": before, "after": after, "broad": broad})
    );
    drop(conn);
    pool.close().await;
}
