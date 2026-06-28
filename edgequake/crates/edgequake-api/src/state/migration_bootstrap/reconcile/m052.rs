//! Migration 052 — PostgreSQL session artifacts SSOT verification.

use std::collections::HashSet;

use sqlx::PgPool;

use super::execute_bootstrap_apply_sql;
use tracing::info;

use super::super::{Migration052Report, MIGRATION_052_VERSION, SQL_052_APPLY};

pub async fn reconcile_migration_052(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
    applied_this_run: &[i64],
) -> Result<Migration052Report, sqlx::Error> {
    let marker_applied = applied_this_run.contains(&MIGRATION_052_VERSION);
    let marker_present = applied_after.contains(&MIGRATION_052_VERSION);

    info!(
        target: "edgequake.migration",
        step = "migration_052_apply_start",
        marker_applied,
        marker_present,
        "Ensuring PG session artifacts SSOT (migration 052)"
    );
    execute_bootstrap_apply_sql(pool, SQL_052_APPLY).await?;

    Ok(Migration052Report {
        marker_present: marker_present || marker_applied,
        apply_executed: true,
    })
}
