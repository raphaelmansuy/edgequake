//! Migration 062 — auth/mod identity SSOT isolation (phase 51).

use std::collections::HashSet;

use sqlx::PgPool;

use super::execute_bootstrap_apply_sql;
use tracing::info;

use super::super::{Migration062Report, MIGRATION_062_VERSION, SQL_062_APPLY};

pub async fn reconcile_migration_062(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
    applied_this_run: &[i64],
) -> Result<Migration062Report, sqlx::Error> {
    let marker_applied = applied_this_run.contains(&MIGRATION_062_VERSION);
    let marker_present = applied_after.contains(&MIGRATION_062_VERSION);

    info!(
        target: "edgequake.migration",
        step = "migration_062_apply_start",
        marker_applied,
        marker_present,
        "Ensuring auth/mod identity SSOT marker (migration 062)"
    );
    execute_bootstrap_apply_sql(pool, SQL_062_APPLY).await?;

    Ok(Migration062Report {
        marker_present: marker_present || marker_applied,
        apply_executed: true,
    })
}
