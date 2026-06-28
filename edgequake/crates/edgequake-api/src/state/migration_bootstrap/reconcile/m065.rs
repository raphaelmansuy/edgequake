//! Migration 065 — auth KV eliminated; AuthMemoryStore only (phase 55).

use std::collections::HashSet;

use sqlx::PgPool;

use super::execute_bootstrap_apply_sql;
use tracing::info;

use super::super::{Migration065Report, MIGRATION_065_VERSION, SQL_065_APPLY};

pub async fn reconcile_migration_065(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
    applied_this_run: &[i64],
) -> Result<Migration065Report, sqlx::Error> {
    let marker_applied = applied_this_run.contains(&MIGRATION_065_VERSION);
    let marker_present = applied_after.contains(&MIGRATION_065_VERSION);

    info!(
        target: "edgequake.migration",
        step = "migration_065_apply_start",
        marker_applied,
        marker_present,
        "Ensuring auth KV eliminated marker (migration 065)"
    );
    execute_bootstrap_apply_sql(pool, SQL_065_APPLY).await?;

    Ok(Migration065Report {
        marker_present: marker_present || marker_applied,
        apply_executed: true,
    })
}
