//! Migration 059 — PG-only vs KV test-harness branch SSOT (phase 48).

use std::collections::HashSet;

use sqlx::PgPool;

use super::execute_bootstrap_apply_sql;
use tracing::info;

use super::super::{Migration059Report, MIGRATION_059_VERSION, SQL_059_APPLY};

pub async fn reconcile_migration_059(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
    applied_this_run: &[i64],
) -> Result<Migration059Report, sqlx::Error> {
    let marker_applied = applied_this_run.contains(&MIGRATION_059_VERSION);
    let marker_present = applied_after.contains(&MIGRATION_059_VERSION);

    info!(
        target: "edgequake.migration",
        step = "migration_059_apply_start",
        marker_applied,
        marker_present,
        "Ensuring PG-only auth branch SSOT marker (migration 059)"
    );
    execute_bootstrap_apply_sql(pool, SQL_059_APPLY).await?;

    Ok(Migration059Report {
        marker_present: marker_present || marker_applied,
        apply_executed: true,
    })
}
