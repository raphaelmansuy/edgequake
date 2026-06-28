//! Migration 044 — community labels marker.

use std::collections::HashSet;

use sqlx::PgPool;

use super::execute_bootstrap_apply_sql;
use tracing::info;

use super::super::{Migration044Report, MIGRATION_044_VERSION, SQL_044_APPLY};

pub async fn reconcile_migration_044(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
    applied_this_run: &[i64],
) -> Result<Migration044Report, sqlx::Error> {
    let marker_applied = applied_this_run.contains(&MIGRATION_044_VERSION);
    let marker_present = applied_after.contains(&MIGRATION_044_VERSION);
    let needs_apply = marker_applied || marker_present;

    if needs_apply {
        info!(
            target: "edgequake.migration",
            step = "migration_044_apply_start",
            marker_applied,
            "Recording migration 044 community labels marker"
        );
        execute_bootstrap_apply_sql(pool, SQL_044_APPLY).await?;
    }

    Ok(Migration044Report {
        marker_present,
        apply_executed: needs_apply,
    })
}
