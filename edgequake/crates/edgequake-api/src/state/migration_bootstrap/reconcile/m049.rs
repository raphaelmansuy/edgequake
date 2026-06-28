//! Migration 049 — membership identity SSOT backfill.

use std::collections::HashSet;

use sqlx::PgPool;

use super::execute_bootstrap_apply_sql;
use tracing::info;

use super::super::{Migration049Report, MIGRATION_049_VERSION, SQL_049_APPLY};

pub async fn reconcile_migration_049(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
    applied_this_run: &[i64],
) -> Result<Migration049Report, sqlx::Error> {
    let marker_applied = applied_this_run.contains(&MIGRATION_049_VERSION);
    let marker_present = applied_after.contains(&MIGRATION_049_VERSION);

    info!(
        target: "edgequake.migration",
        step = "migration_049_apply_start",
        marker_applied,
        marker_present,
        "Ensuring membership SSOT backfill (migration 049)"
    );
    execute_bootstrap_apply_sql(pool, SQL_049_APPLY).await?;

    Ok(Migration049Report {
        marker_present: marker_present || marker_applied,
        apply_executed: true,
    })
}
