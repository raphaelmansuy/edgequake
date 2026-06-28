//! Migration 063 — auth_kv_store service-layer-only SSOT (phase 52).

use std::collections::HashSet;

use sqlx::PgPool;

use super::execute_bootstrap_apply_sql;
use tracing::info;

use super::super::{Migration063Report, MIGRATION_063_VERSION, SQL_063_APPLY};

pub async fn reconcile_migration_063(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
    applied_this_run: &[i64],
) -> Result<Migration063Report, sqlx::Error> {
    let marker_applied = applied_this_run.contains(&MIGRATION_063_VERSION);
    let marker_present = applied_after.contains(&MIGRATION_063_VERSION);

    info!(
        target: "edgequake.migration",
        step = "migration_063_apply_start",
        marker_applied,
        marker_present,
        "Ensuring auth service-layer SSOT marker (migration 063)"
    );
    execute_bootstrap_apply_sql(pool, SQL_063_APPLY).await?;

    Ok(Migration063Report {
        marker_present: marker_present || marker_applied,
        apply_executed: true,
    })
}
