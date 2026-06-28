//! Migration 045 — vector content FTS indexes.

use std::collections::HashSet;

use sqlx::PgPool;

use super::execute_bootstrap_apply_sql;
use tracing::info;

use super::super::{Migration045Report, MIGRATION_045_VERSION, SQL_045_APPLY};

pub async fn reconcile_migration_045(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
    applied_this_run: &[i64],
) -> Result<Migration045Report, sqlx::Error> {
    let marker_applied = applied_this_run.contains(&MIGRATION_045_VERSION);
    let marker_present = applied_after.contains(&MIGRATION_045_VERSION);

    // Idempotent — safe on every bootstrap; picks up workspace vector tables created
    // after the first 045 marker (content_tsv is optional for FTS when KV join works).
    info!(
        target: "edgequake.migration",
        step = "migration_045_apply_start",
        marker_applied,
        marker_present,
        "Ensuring vector content_tsv GIN indexes (migration 045)"
    );
    execute_bootstrap_apply_sql(pool, SQL_045_APPLY).await?;

    Ok(Migration045Report {
        marker_present: marker_present || marker_applied,
        apply_executed: true,
    })
}
