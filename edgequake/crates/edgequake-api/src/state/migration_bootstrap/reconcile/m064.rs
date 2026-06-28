//! Migration 064 — builtin OIDC authorization-code flow (phase 54).

use std::collections::HashSet;

use sqlx::PgPool;

use super::execute_bootstrap_apply_sql;
use tracing::info;

use super::super::{Migration064Report, MIGRATION_064_VERSION, SQL_064_APPLY};

pub async fn reconcile_migration_064(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
    applied_this_run: &[i64],
) -> Result<Migration064Report, sqlx::Error> {
    let marker_applied = applied_this_run.contains(&MIGRATION_064_VERSION);
    let marker_present = applied_after.contains(&MIGRATION_064_VERSION);

    info!(
        target: "edgequake.migration",
        step = "migration_064_apply_start",
        marker_applied,
        marker_present,
        "Ensuring builtin OIDC marker (migration 064)"
    );
    execute_bootstrap_apply_sql(pool, SQL_064_APPLY).await?;

    Ok(Migration064Report {
        marker_present: marker_present || marker_applied,
        apply_executed: true,
    })
}
