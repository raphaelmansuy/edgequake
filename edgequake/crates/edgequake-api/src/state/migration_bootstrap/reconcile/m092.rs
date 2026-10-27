//! Migration 092 — eq_* denorm reconcile (every bootstrap, SPEC-069 / SPEC-083).
//!
//! First principle: M092 sqlx file is a marker only. Runtime SSOT lives in
//! `migrations/support/092/apply.sql` so graphs created after migrate still get
//! columns/indexes/triggers at boot — never mid-delete under query timeout.

use sqlx::PgPool;
use tracing::{info, warn};

use super::super::reconcile_state::{record_reconcile_state, sha384_hex};
use super::super::{Migration092Report, SQL_092_APPLY};
use super::execute_bootstrap_apply_sql;

/// Ensure eq_* columns, indexes, and sync triggers exist on every AGE graph.
///
/// Returns a readiness report: `graphs_ready` / `graphs_degraded` after apply.
/// SPEC-083: no longer silent `Ok(true)` when columns are still missing.
pub async fn reconcile_migration_092(pool: &PgPool) -> Result<Migration092Report, sqlx::Error> {
    if edgequake_storage::graph_cutover_is_ready(pool).await {
        info!(
            target: "edgequake.migration",
            step = "migration_092_skip",
            reason = "cutover_ready",
            "Skipping eq_* denorm on AGE graphs — plain graph ledger is ready"
        );
        return Ok(Migration092Report {
            age_available: false,
            apply_executed: false,
            graphs_checked: 0,
            graphs_ready: 0,
            graphs_degraded: Vec::new(),
            fallback_env_enabled: eq_id_fallback_env(),
        });
    }
    let age_available: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age')")
            .fetch_one(pool)
            .await
            .unwrap_or(false);

    if !age_available {
        return Ok(Migration092Report {
            age_available: false,
            apply_executed: false,
            graphs_checked: 0,
            graphs_ready: 0,
            graphs_degraded: Vec::new(),
            fallback_env_enabled: eq_id_fallback_env(),
        });
    }

    let maintenance = eq_maintenance_env();
    // Prepend GUC on the same raw_sql connection so apply.sql sees maintenance mode
    // (pool-scoped SET on a prior checkout would not stick).
    let apply_sql = if maintenance {
        info!(
            target: "edgequake.migration",
            step = "migration_092_maintenance",
            "EDGEQUAKE_EQ_MAINTENANCE=1 — lock_timeout=120s + batched NULL-only backfill"
        );
        format!("SELECT set_config('edgequake.eq_maintenance', '1', false);\n{SQL_092_APPLY}")
    } else {
        SQL_092_APPLY.to_string()
    };

    info!(
        target: "edgequake.migration",
        step = "migration_092_apply_start",
        maintenance,
        "Reconciling eq_* denorm schema on all AGE graphs (M092 / SPEC-069)"
    );
    let apply_started = std::time::Instant::now();
    execute_bootstrap_apply_sql(pool, &apply_sql).await?;
    let apply_ms = apply_started.elapsed().as_millis() as i64;

    // Only score graphs that already have AGE child Node+EDGE tables. Incomplete
    // stubs (e.g. leftover bind_probe*) match support/092 CONTINUE — not degraded.
    let graphs: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT g.name
        FROM ag_catalog.ag_graph g
        WHERE EXISTS (
          SELECT 1 FROM pg_tables n
          WHERE n.schemaname = g.name AND n.tablename = 'Node'
        )
        AND EXISTS (
          SELECT 1 FROM pg_tables e
          WHERE e.schemaname = g.name AND e.tablename = 'EDGE'
        )
        ORDER BY g.name
        "#,
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut graphs_ready = 0usize;
    let mut graphs_degraded = Vec::new();
    for g in &graphs {
        if graph_eq_columns_ready(pool, g).await? {
            graphs_ready += 1;
        } else {
            graphs_degraded.push(g.clone());
        }
    }

    if !graphs_degraded.is_empty() {
        warn!(
            target: "edgequake.migration",
            step = "migration_092_degraded",
            degraded = ?graphs_degraded,
            ready = graphs_ready,
            "eq_* columns still missing after reconcile — /ready will fail unless EDGEQUAKE_EQ_ID_FALLBACK=1"
        );
    }

    let outcome = if graphs_degraded.is_empty() {
        "ok"
    } else {
        "degraded"
    };
    if let Err(e) = record_reconcile_state(
        pool,
        "092",
        &sha384_hex(SQL_092_APPLY.as_bytes()),
        Some(apply_ms),
        outcome,
    )
    .await
    {
        warn!(
            target: "edgequake.migration",
            step = "migration_092_reconcile_state",
            error = %e,
            "Failed to record reconcile state (non-fatal)"
        );
    }

    Ok(Migration092Report {
        age_available: true,
        apply_executed: true,
        graphs_checked: graphs.len(),
        graphs_ready,
        graphs_degraded,
        fallback_env_enabled: eq_id_fallback_env(),
    })
}

fn eq_id_fallback_env() -> bool {
    matches!(
        std::env::var("EDGEQUAKE_EQ_ID_FALLBACK").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    )
}

/// Planned maintenance window for large-graph eq_* DDL (SPEC-083 / P0).
fn eq_maintenance_env() -> bool {
    matches!(
        std::env::var("EDGEQUAKE_EQ_MAINTENANCE").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    )
}

/// D-30: require `eq_rel_type` + 3-col unique so mid-upgrade graphs are not
/// skipped by M092 reconcile (LAW-2 schema readiness).
async fn graph_eq_columns_ready(pool: &PgPool, graph: &str) -> Result<bool, sqlx::Error> {
    let ready: bool = sqlx::query_scalar(
        r#"
        SELECT
          EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_schema = $1 AND table_name = 'Node' AND column_name = 'eq_node_id'
          )
          AND EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_schema = $1 AND table_name = 'EDGE' AND column_name = 'eq_source_id'
          )
          AND EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_schema = $1 AND table_name = 'EDGE' AND column_name = 'eq_target_id'
          )
          AND EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_schema = $1 AND table_name = 'EDGE' AND column_name = 'eq_rel_type'
          )
          AND EXISTS (
            SELECT 1 FROM pg_indexes
            WHERE schemaname = $1 AND indexname = 'idx_edge_eq_source_target_rel'
          )
        "#,
    )
    .bind(graph)
    .fetch_one(pool)
    .await
    .unwrap_or(false);
    Ok(ready)
}
