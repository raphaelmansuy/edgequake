//! Migration 054 — identity/session PG RLS envelope verification.

use std::collections::HashSet;

use sqlx::PgPool;

use super::execute_bootstrap_apply_sql;
use tracing::info;

use super::super::{Migration054Report, MIGRATION_054_VERSION, SQL_054_APPLY};

pub async fn reconcile_migration_054(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
    applied_this_run: &[i64],
) -> Result<Migration054Report, sqlx::Error> {
    let marker_applied = applied_this_run.contains(&MIGRATION_054_VERSION);
    let marker_present = applied_after.contains(&MIGRATION_054_VERSION);

    info!(
        target: "edgequake.migration",
        step = "migration_054_apply_start",
        marker_applied,
        marker_present,
        "Ensuring identity PG RLS envelope (migration 054)"
    );
    execute_bootstrap_apply_sql(pool, SQL_054_APPLY).await?;

    Ok(Migration054Report {
        marker_present: marker_present || marker_applied,
        apply_executed: true,
    })
}
