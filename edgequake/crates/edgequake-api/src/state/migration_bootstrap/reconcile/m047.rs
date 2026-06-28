//! Migration 047 — workspace document KV index backfill.

use std::collections::HashSet;

use sqlx::PgPool;

use super::execute_bootstrap_apply_sql;
use tracing::info;

use super::super::{Migration047Report, MIGRATION_047_VERSION, SQL_047_APPLY};

pub async fn reconcile_migration_047(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
    applied_this_run: &[i64],
) -> Result<Migration047Report, sqlx::Error> {
    let marker_applied = applied_this_run.contains(&MIGRATION_047_VERSION);
    let marker_present = applied_after.contains(&MIGRATION_047_VERSION);

    // Idempotent — safe on every bootstrap; picks up new metadata rows written
    // before write-path hooks sync the wsdoc index, and new workspace KV tables.
    info!(
        target: "edgequake.migration",
        step = "migration_047_apply_start",
        marker_applied,
        marker_present,
        "Ensuring workspace document KV index (migration 047)"
    );
    execute_bootstrap_apply_sql(pool, SQL_047_APPLY).await?;

    Ok(Migration047Report {
        marker_present: marker_present || marker_applied,
        apply_executed: true,
    })
}
