//! Migration 056 — KV auth consolidated to auth_kv_store (IMP-026).

use std::collections::HashSet;

use sqlx::PgPool;

use super::execute_bootstrap_apply_sql;
use tracing::info;

use super::super::{Migration056Report, MIGRATION_056_VERSION, SQL_056_APPLY};

pub async fn reconcile_migration_056(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
    applied_this_run: &[i64],
) -> Result<Migration056Report, sqlx::Error> {
    let marker_applied = applied_this_run.contains(&MIGRATION_056_VERSION);
    let marker_present = applied_after.contains(&MIGRATION_056_VERSION);

    info!(
        target: "edgequake.migration",
        step = "migration_056_apply_start",
        marker_applied,
        marker_present,
        "Ensuring KV auth consolidation marker (migration 056)"
    );
    execute_bootstrap_apply_sql(pool, SQL_056_APPLY).await?;

    Ok(Migration056Report {
        marker_present: marker_present || marker_applied,
        apply_executed: true,
    })
}
