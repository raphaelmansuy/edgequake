//! Migration 061 — auth_kv_store handler isolation (phase 50).

use std::collections::HashSet;

use sqlx::PgPool;

use super::execute_bootstrap_apply_sql;
use tracing::info;

use super::super::{Migration061Report, MIGRATION_061_VERSION, SQL_061_APPLY};

pub async fn reconcile_migration_061(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
    applied_this_run: &[i64],
) -> Result<Migration061Report, sqlx::Error> {
    let marker_applied = applied_this_run.contains(&MIGRATION_061_VERSION);
    let marker_present = applied_after.contains(&MIGRATION_061_VERSION);

    info!(
        target: "edgequake.migration",
        step = "migration_061_apply_start",
        marker_applied,
        marker_present,
        "Ensuring auth_kv handler isolation marker (migration 061)"
    );
    execute_bootstrap_apply_sql(pool, SQL_061_APPLY).await?;

    Ok(Migration061Report {
        marker_present: marker_present || marker_applied,
        apply_executed: true,
    })
}
