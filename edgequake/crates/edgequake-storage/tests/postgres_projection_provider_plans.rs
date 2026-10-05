//! Production claim SQL against temporary copies of migrated tables/indexes.
#![cfg(feature = "postgres")]
#[path = "support/postgres_access_pool.rs"]
#[allow(dead_code)]
mod postgres_access_pool;

use serde_json::{json, Value};
use sqlx::{Acquire, Row};
use std::time::{Duration, Instant};
use uuid::Uuid;

fn uses_due_index(plan: &Value) -> bool {
    plan.get("Index Cond")
        .and_then(Value::as_str)
        .is_some_and(|s| s.contains("next_attempt_at"))
        || plan
            .get("Plans")
            .and_then(Value::as_array)
            .is_some_and(|children| children.iter().any(uses_due_index))
}

#[tokio::test]
async fn selected_provider_claims_use_migrated_due_index() {
    let Some(pool) =
        postgres_access_pool::test_pool_with_acquire_timeout(Duration::from_secs(10)).await
    else {
        return;
    };
    let mut conn = pool.acquire().await.unwrap();
    for table in [
        "data_bindings",
        "projection_events",
        "projection_deliveries",
        "projection_event_role_proofs",
    ] {
        sqlx::query(&format!(
            "CREATE TEMP TABLE {table} (LIKE public.{table} INCLUDING ALL)"
        ))
        .execute(&mut *conn)
        .await
        .unwrap();
    }
    sqlx::query("INSERT INTO pg_temp.data_bindings (binding_id, tenant_id, workspace_id, role, provider, config_ref, layout, physical_index, generation, state)
        SELECT md5('binding' || i)::uuid, md5('tenant' || i)::uuid, md5('workspace' || i)::uuid,
            CASE WHEN i%2=1 THEN 'graph' ELSE 'vector' END,
            CASE i WHEN 1 THEN 'age' WHEN 2 THEN 'pgvector' WHEN 3 THEN 'selected_graph' ELSE 'selected_vector' END,
            'fixture','fixture','fixture',1,'active' FROM generate_series(1,4) i")
        .execute(&mut *conn).await.unwrap();
    sqlx::query("INSERT INTO pg_temp.projection_events (event_id, tenant_id, workspace_id, object_kind, object_id, object_revision, schema_version, operation, manifest_ref, digest)
        SELECT md5('event' || i)::uuid, md5('tenant' || b)::uuid, md5('workspace' || b)::uuid,
            'document_batch', md5('doc' || i)::uuid, 1, 1, 'upsert', 'fixture', decode(repeat('00',32),'hex')
        FROM (SELECT i, CASE WHEN i%100=0 THEN 3 WHEN i%100=1 THEN 4 ELSE 1+i%2 END AS b FROM generate_series(1,10000) i) f")
        .execute(&mut *conn).await.unwrap();
    sqlx::query("INSERT INTO pg_temp.projection_deliveries (event_id, binding_id, state, next_attempt_at)
        SELECT md5('event' || i)::uuid, md5('binding' || CASE WHEN i%100=0 THEN 3 WHEN i%100=1 THEN 4 ELSE 1+i%2 END)::uuid,
            'pending', now()-interval '1 minute' FROM generate_series(1,10000) i")
        .execute(&mut *conn).await.unwrap();
    for table in [
        "data_bindings",
        "projection_events",
        "projection_deliveries",
        "projection_event_role_proofs",
    ] {
        sqlx::query(&format!("ANALYZE pg_temp.{table}"))
            .execute(&mut *conn)
            .await
            .unwrap();
    }
    // Read the actual production constant so a worker SQL change cannot leave
    // this test measuring an obsolete copy. Only the fixture schema is changed.
    let source = include_str!("../src/projection/ledger.rs");
    let claim = source
        .split("const CLAIM_SQL: &str = r#\"")
        .nth(1)
        .unwrap()
        .split("\"#;")
        .next()
        .unwrap()
        .replace("public.", "pg_temp.");
    let mut reports = Vec::new();
    for mode in ["auto", "force_custom_plan", "force_generic_plan"] {
        let mut elapsed = Vec::new();
        let mut last_plan = Value::Null;
        for probe in 0..24 {
            let mut tx = conn.begin().await.unwrap();
            sqlx::query(&format!("SET LOCAL plan_cache_mode = {mode}"))
                .execute(&mut *tx)
                .await
                .unwrap();
            sqlx::query("SET LOCAL statement_timeout = '2s'")
                .execute(&mut *tx)
                .await
                .unwrap();
            let owner = Uuid::new_v4();
            let start = Instant::now();
            let rows = sqlx::query(&claim)
                .bind(64_i64)
                .bind(owner)
                .bind(30_000_i64)
                .bind("selected_graph")
                .bind("selected_vector")
                .fetch_all(&mut *tx)
                .await
                .unwrap();
            assert_eq!(rows.len(), 64);
            assert!(rows.iter().all(|row| row
                .get::<String, _>("binding_provider")
                .starts_with("selected_")));
            if probe >= 3 {
                elapsed.push(start.elapsed().as_secs_f64() * 1000.0);
            }
            tx.rollback().await.unwrap();
            if probe == 23 {
                let mut tx = conn.begin().await.unwrap();
                sqlx::query(&format!("SET LOCAL plan_cache_mode = {mode}"))
                    .execute(&mut *tx)
                    .await
                    .unwrap();
                last_plan =
                    sqlx::query_scalar(&format!("EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON) {claim}"))
                        .bind(64_i64)
                        .bind(owner)
                        .bind(30_000_i64)
                        .bind("selected_graph")
                        .bind("selected_vector")
                        .fetch_one(&mut *tx)
                        .await
                        .unwrap();
                tx.rollback().await.unwrap();
            }
        }
        assert!(uses_due_index(&last_plan[0]["Plan"]), "{mode}: {last_plan}");
        elapsed.sort_by(f64::total_cmp);
        assert!(
            elapsed[19] < 250.0,
            "claim p95 exceeded fixture budget: {elapsed:?}"
        );
        reports.push(json!({"plan_cache_mode":mode,"samples":21,"p50_ms":elapsed[10],"p95_ms":elapsed[19],"explain":last_plan}));
    }
    let report = json!({"schema":"edgequake.projection.provider-plans.v1","fixture":{"deliveries":10000,"selected_provider_fraction":0.02,"claim_limit":64,"temporary_migrated_indexes":true,"planner_scan_switches_forced":false},"reports":reports});
    println!("PROJECTION_PROVIDER_PLAN_REPORT {report}");
    if let Ok(path) = std::env::var("EQ_PROJECTION_PROVIDER_PLAN_OUTPUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}
