//! Paired measurements of production entity lookup SQL on migrated indexes.
//! No planner scan switches: exercise auto, custom, and generic prepared plans.
#![cfg(feature = "postgres")]

#[path = "../src/postgres_entity_sink/queries.rs"]
mod entity_queries;
#[path = "../../edgequake-storage/tests/support/perf_harness.rs"]
mod perf_harness;
#[path = "../../edgequake-storage/tests/support/postgres_access_pool.rs"]
#[allow(dead_code)]
mod postgres_access_pool;

use serde_json::{json, Value};
use sqlx::{Acquire, PgConnection, Row};
use std::time::{Duration, Instant};
use uuid::Uuid;

const ROWS: usize = 60_000;
const PROBES: usize = 32;

// Historical production SQL, deliberately retained only as the paired baseline.
const BEFORE_SINGLE: &str = "SELECT id FROM entities
    WHERE workspace_id IS NOT DISTINCT FROM $2
      AND tenant_id IS NOT DISTINCT FROM $3
      AND (name = $1 OR name = ($2::text || '::' || $1))
    ORDER BY CASE WHEN name = $1 THEN 0 ELSE 1 END LIMIT 1";
const BEFORE_BATCH: &str = "SELECT id, name FROM entities
    WHERE workspace_id IS NOT DISTINCT FROM $2
      AND tenant_id IS NOT DISTINCT FROM $3 AND name = ANY($1)";

fn has_scoped_index(plan: &Value) -> bool {
    if let Some(condition) = plan.get("Index Cond").and_then(Value::as_str) {
        if ["tenant_id", "workspace_id"]
            .iter()
            .all(|column| condition.contains(column))
        {
            return true;
        }
    }
    plan.get("Plans")
        .and_then(Value::as_array)
        .is_some_and(|children| children.iter().any(has_scoped_index))
}

fn buffers(plan: &Value) -> u64 {
    [
        "Shared Hit Blocks",
        "Shared Read Blocks",
        "Local Hit Blocks",
        "Local Read Blocks",
    ]
    .iter()
    .map(|key| plan[*key].as_u64().unwrap_or(0))
    .sum()
}

async fn explain(conn: &mut PgConnection, execute: &str) -> Value {
    sqlx::query_scalar(&format!(
        "EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON) {execute}"
    ))
    .persistent(false)
    .fetch_one(conn)
    .await
    .unwrap()
}

async fn ids(conn: &mut PgConnection, sql: &str) -> Vec<Uuid> {
    let mut ids: Vec<Uuid> = sqlx::query(sql)
        .persistent(false)
        .fetch_all(conn)
        .await
        .unwrap()
        .iter()
        .map(|row| row.try_get("id").unwrap())
        .collect();
    ids.sort();
    ids
}

async fn measure_pair(
    conn: &mut PgConnection,
    kind: &str,
    mode: &str,
    workspace: Uuid,
    tenant: Uuid,
) {
    let batch = kind == "batch";
    let before = if batch { BEFORE_BATCH } else { BEFORE_SINGLE };
    let after = if batch {
        entity_queries::entity_batch_lookup_sql(true, true)
    } else {
        entity_queries::entity_lookup_sql(true, true)
    };
    let first_type = if batch { "text[]" } else { "text" };
    let first_arg = if batch {
        "ARRAY['entity_42','entity_43']"
    } else {
        "'entity_42'"
    };
    let mut executions = Vec::new();
    for (label, sql) in [("before", before), ("after", after.as_str())] {
        let name = format!("entity_{kind}_{mode}_{label}");
        sqlx::query(&format!("PREPARE {name}({first_type},uuid,uuid) AS {sql}"))
            .execute(&mut *conn)
            .await
            .unwrap();
        executions.push(format!(
            "EXECUTE {name}({first_arg},'{workspace}'::uuid,'{tenant}'::uuid)"
        ));
    }
    let expected = ids(conn, &executions[0]).await;
    assert_eq!(expected.len(), if batch { 2 } else { 1 });
    let mut samples = [Vec::new(), Vec::new()];
    for probe in 0..PROBES {
        // Alternate order so the candidate does not always get the warmed pages.
        for index in if probe % 2 == 0 { [0, 1] } else { [1, 0] } {
            let start = Instant::now();
            assert_eq!(ids(conn, &executions[index]).await, expected);
            samples[index].push(start.elapsed());
        }
    }
    // Explain after repeated use: auto mode has passed the five custom executions.
    let before_plan = explain(conn, &executions[0]).await;
    let after_plan = explain(conn, &executions[1]).await;
    assert!(
        has_scoped_index(&after_plan[0]["Plan"]),
        "{mode}/{kind}: both scope keys must be index conditions: {after_plan}"
    );
    let before_buffers = buffers(&before_plan[0]["Plan"]);
    let after_buffers = buffers(&after_plan[0]["Plan"]);
    assert!(
        after_buffers * 10 < before_buffers,
        "{mode}/{kind}: expected at least 10x less buffer work, {before_buffers} -> {after_buffers}"
    );
    let reports: Vec<_> = ["before", "after"]
        .iter()
        .enumerate()
        .map(|(index, label)| {
            perf_harness::finish_report(
                &format!("entity_lookup_{kind}_{mode}_{label}"),
                &perf_harness::samples_after_warmup(&samples[index], 30),
                500.0,
                if index == 0 {
                    "name_postfilter"
                } else {
                    "scoped_btree"
                },
                true,
                format!("rows={ROWS} tenants=600 common_names=100 plan_cache_mode={mode}"),
            )
        })
        .collect();
    println!(
        "INDEX_PLAN_REPORT {}",
        json!({
            "kind": kind, "mode": mode, "rows": ROWS,
            "planner_scan_switches_forced": false,
            "before": {"measurement": serde_json::from_str::<Value>(&reports[0].to_json_line()).unwrap(), "explain": before_plan},
            "after": {"measurement": serde_json::from_str::<Value>(&reports[1].to_json_line()).unwrap(), "explain": after_plan},
            "latency_ratio": reports[1].p95_ms / reports[0].p95_ms,
            "buffer_ratio": after_buffers as f64 / before_buffers as f64
        })
    );
}

async fn verify_nullable_scopes(conn: &mut PgConnection, workspace: Uuid, tenant: Uuid) {
    for (workspace, tenant) in [
        (Some(workspace), Some(tenant)),
        (Some(workspace), None),
        (None, Some(tenant)),
        (None, None),
    ] {
        let expected: Uuid = sqlx::query_scalar(
            "SELECT id FROM entities WHERE workspace_id IS NOT DISTINCT FROM $1
             AND tenant_id IS NOT DISTINCT FROM $2 AND name='entity_42'",
        )
        .bind(workspace)
        .bind(tenant)
        .fetch_one(&mut *conn)
        .await
        .unwrap();
        let single = entity_queries::entity_lookup_sql(workspace.is_some(), tenant.is_some());
        let actual: Uuid = sqlx::query_scalar(&single)
            .bind("entity_42")
            .bind(workspace)
            .bind(tenant)
            .fetch_one(&mut *conn)
            .await
            .unwrap();
        assert_eq!(
            actual, expected,
            "bare name must win within this exact scope"
        );
        let batch = entity_queries::entity_batch_lookup_sql(workspace.is_some(), tenant.is_some());
        let actual: Vec<Uuid> = sqlx::query_scalar(&batch)
            .bind(vec!["entity_42", "entity_43"])
            .bind(workspace)
            .bind(tenant)
            .fetch_all(&mut *conn)
            .await
            .unwrap();
        assert_eq!(actual.len(), 2, "never widen a NULL scope");
        assert!(actual.contains(&expected));
        if workspace.is_some() {
            // Bare-first resolution must also fall back to a legacy prefixed name.
            sqlx::query("DELETE FROM entities WHERE id=$1")
                .bind(expected)
                .execute(&mut *conn)
                .await
                .unwrap();
            let legacy: Option<Uuid> = sqlx::query_scalar(&single)
                .bind("entity_42")
                .bind(workspace)
                .bind(tenant)
                .fetch_optional(&mut *conn)
                .await
                .unwrap();
            assert!(legacy.is_some_and(|id| id != expected));
        }
    }
}

#[tokio::test]
async fn entity_lookup_uses_scope_index_in_auto_custom_and_generic_plans() {
    let Some(pool) =
        postgres_access_pool::test_pool_with_acquire_timeout(Duration::from_secs(10)).await
    else {
        return;
    };
    let mut conn = pool.acquire().await.unwrap();
    // Copy the migrated indexes, generated columns and defaults, but no data or
    // FK references. TEMP tables disappear when this connection closes.
    sqlx::query("CREATE TEMP TABLE entities (LIKE public.entities INCLUDING ALL)")
        .execute(&mut *conn)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO entities(name,entity_type,tenant_id,workspace_id,sync_status)
        SELECT 'entity_' || (i%100)::text, 'PERSON', md5((i/100)::text)::uuid,
               md5(('ws_' || (i/100)::text))::uuid, 'synced' FROM generate_series(0,59999) i",
    )
    .execute(&mut *conn)
    .await
    .unwrap();
    let (tenant, workspace): (Uuid, Uuid) =
        sqlx::query_as("SELECT md5('400')::uuid, md5('ws_400')::uuid")
            .fetch_one(&mut *conn)
            .await
            .unwrap();
    for (workspace, tenant) in [(Some(workspace), None), (None, Some(tenant)), (None, None)] {
        for name in ["entity_42", "entity_43"] {
            sqlx::query("INSERT INTO entities(name,entity_type,tenant_id,workspace_id,sync_status) VALUES($1,'PERSON',$2,$3,'synced')")
                .bind(name).bind(tenant).bind(workspace).execute(&mut *conn).await.unwrap();
        }
    }
    for tenant in [Some(tenant), None] {
        sqlx::query("INSERT INTO entities(name,entity_type,tenant_id,workspace_id,sync_status) VALUES($1,'PERSON',$2,$3,'synced')")
            .bind(format!("{workspace}::entity_42")).bind(tenant).bind(workspace)
            .execute(&mut *conn).await.unwrap();
    }
    sqlx::query("ANALYZE entities")
        .execute(&mut *conn)
        .await
        .unwrap();
    for (mode, setting) in [
        ("auto", "auto"),
        ("custom", "force_custom_plan"),
        ("generic", "force_generic_plan"),
    ] {
        let mut tx = conn.begin().await.unwrap();
        sqlx::query(&format!("SET LOCAL plan_cache_mode = {setting}"))
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("SET LOCAL statement_timeout = '2s'")
            .execute(&mut *tx)
            .await
            .unwrap();
        for kind in ["single", "batch"] {
            measure_pair(&mut tx, kind, mode, workspace, tenant).await;
        }
        tx.commit().await.unwrap();
    }
    for setting in ["auto", "force_custom_plan", "force_generic_plan"] {
        let mut tx = conn.begin().await.unwrap();
        sqlx::query(&format!("SET LOCAL plan_cache_mode = {setting}"))
            .execute(&mut *tx)
            .await
            .unwrap();
        verify_nullable_scopes(&mut tx, workspace, tenant).await;
        tx.rollback().await.unwrap();
    }
    drop(conn);
    pool.close().await;
}
