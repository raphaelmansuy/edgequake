//! Migration 055 — auth secure by default verification (AC-4).

use std::collections::HashSet;

use sqlx::PgPool;

use super::execute_bootstrap_apply_sql;
use tracing::info;

use super::super::{Migration055Report, MIGRATION_055_VERSION, SQL_055_APPLY};

pub async fn reconcile_migration_055(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
    applied_this_run: &[i64],
) -> Result<Migration055Report, sqlx::Error> {
    let marker_applied = applied_this_run.contains(&MIGRATION_055_VERSION);
    let marker_present = applied_after.contains(&MIGRATION_055_VERSION);

    info!(
        target: "edgequake.migration",
        step = "migration_055_apply_start",
        marker_applied,
        marker_present,
        "Ensuring auth secure-by-default marker (migration 055)"
    );
    execute_bootstrap_apply_sql(pool, SQL_055_APPLY).await?;

    Ok(Migration055Report {
        marker_present: marker_present || marker_applied,
        apply_executed: true,
    })
}
