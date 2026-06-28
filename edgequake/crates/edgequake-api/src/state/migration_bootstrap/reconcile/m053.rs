//! Migration 053 — PG-only auth reads verification (KV not SSOT when pool available).

use std::collections::HashSet;

use sqlx::PgPool;

use super::execute_bootstrap_apply_sql;
use tracing::info;

use super::super::{Migration053Report, MIGRATION_053_VERSION, SQL_053_APPLY};

pub async fn reconcile_migration_053(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
    applied_this_run: &[i64],
) -> Result<Migration053Report, sqlx::Error> {
    let marker_applied = applied_this_run.contains(&MIGRATION_053_VERSION);
    let marker_present = applied_after.contains(&MIGRATION_053_VERSION);

    info!(
        target: "edgequake.migration",
        step = "migration_053_apply_start",
        marker_applied,
        marker_present,
        "Ensuring PG-only auth reads (migration 053)"
    );
    execute_bootstrap_apply_sql(pool, SQL_053_APPLY).await?;

    Ok(Migration053Report {
        marker_present: marker_present || marker_applied,
        apply_executed: true,
    })
}
