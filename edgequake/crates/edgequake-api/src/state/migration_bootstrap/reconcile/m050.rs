//! Migration 050 — PostgreSQL RLS context function verification.

use std::collections::HashSet;

use sqlx::PgPool;

use super::execute_bootstrap_apply_sql;
use tracing::info;

use super::super::{Migration050Report, MIGRATION_050_VERSION, SQL_050_APPLY};

pub async fn reconcile_migration_050(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
    applied_this_run: &[i64],
) -> Result<Migration050Report, sqlx::Error> {
    let marker_applied = applied_this_run.contains(&MIGRATION_050_VERSION);
    let marker_present = applied_after.contains(&MIGRATION_050_VERSION);

    info!(
        target: "edgequake.migration",
        step = "migration_050_apply_start",
        marker_applied,
        marker_present,
        "Verifying PostgreSQL RLS context functions (migration 050)"
    );
    execute_bootstrap_apply_sql(pool, SQL_050_APPLY).await?;

    Ok(Migration050Report {
        marker_present: marker_present || marker_applied,
        apply_executed: true,
    })
}
