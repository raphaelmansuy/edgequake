//! Migration 045 — vector content FTS indexes.

use std::collections::HashSet;

use sqlx::PgPool;
use tracing::info;

use super::super::{Migration045Report, MIGRATION_045_VERSION, SQL_045_APPLY};

pub async fn reconcile_migration_045(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
    applied_this_run: &[i64],
) -> Result<Migration045Report, sqlx::Error> {
    let marker_applied = applied_this_run.contains(&MIGRATION_045_VERSION);
    let marker_present = applied_after.contains(&MIGRATION_045_VERSION);
    let needs_apply = marker_applied || marker_present;

    if needs_apply {
        info!(
            target: "edgequake.migration",
            step = "migration_045_apply_start",
            marker_applied,
            "Adding vector content_tsv GIN indexes (migration 045)"
        );
        sqlx::query(SQL_045_APPLY).execute(pool).await?;
    }

    Ok(Migration045Report {
        marker_present,
        apply_executed: needs_apply,
    })
}
