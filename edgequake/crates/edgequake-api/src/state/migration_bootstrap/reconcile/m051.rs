//! Migration 051 — PostgreSQL identity SSOT primary verification.

use std::collections::HashSet;

use sqlx::PgPool;

use super::execute_bootstrap_apply_sql;
use tracing::info;

use super::super::{Migration051Report, MIGRATION_051_VERSION, SQL_051_APPLY};

pub async fn reconcile_migration_051(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
    applied_this_run: &[i64],
) -> Result<Migration051Report, sqlx::Error> {
    let marker_applied = applied_this_run.contains(&MIGRATION_051_VERSION);
    let marker_present = applied_after.contains(&MIGRATION_051_VERSION);

    info!(
        target: "edgequake.migration",
        step = "migration_051_apply_start",
        marker_applied,
        marker_present,
        "Ensuring PG identity SSOT primary (migration 051)"
    );
    execute_bootstrap_apply_sql(pool, SQL_051_APPLY).await?;

    Ok(Migration051Report {
        marker_present: marker_present || marker_applied,
        apply_executed: true,
    })
}
