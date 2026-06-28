//! Migration 048 — auth user lockout columns (identity SSOT).

use std::collections::HashSet;

use sqlx::PgPool;

use super::execute_bootstrap_apply_sql;
use tracing::info;

use super::super::{Migration048Report, MIGRATION_048_VERSION, SQL_048_APPLY};

pub async fn reconcile_migration_048(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
    applied_this_run: &[i64],
) -> Result<Migration048Report, sqlx::Error> {
    let marker_applied = applied_this_run.contains(&MIGRATION_048_VERSION);
    let marker_present = applied_after.contains(&MIGRATION_048_VERSION);

    info!(
        target: "edgequake.migration",
        step = "migration_048_apply_start",
        marker_applied,
        marker_present,
        "Ensuring auth user lockout columns (migration 048)"
    );
    execute_bootstrap_apply_sql(pool, SQL_048_APPLY).await?;

    Ok(Migration048Report {
        marker_present: marker_present || marker_applied,
        apply_executed: true,
    })
}
