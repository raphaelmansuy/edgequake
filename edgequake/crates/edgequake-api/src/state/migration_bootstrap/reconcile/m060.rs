//! Migration 060 — OAuth2/OIDC honesty + auth_kv_store quarantine (phase 49).

use std::collections::HashSet;

use sqlx::PgPool;

use super::execute_bootstrap_apply_sql;
use tracing::info;

use super::super::{Migration060Report, MIGRATION_060_VERSION, SQL_060_APPLY};

pub async fn reconcile_migration_060(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
    applied_this_run: &[i64],
) -> Result<Migration060Report, sqlx::Error> {
    let marker_applied = applied_this_run.contains(&MIGRATION_060_VERSION);
    let marker_present = applied_after.contains(&MIGRATION_060_VERSION);

    info!(
        target: "edgequake.migration",
        step = "migration_060_apply_start",
        marker_applied,
        marker_present,
        "Ensuring OAuth/OIDC honesty + auth_kv quarantine marker (migration 060)"
    );
    execute_bootstrap_apply_sql(pool, SQL_060_APPLY).await?;

    Ok(Migration060Report {
        marker_present: marker_present || marker_applied,
        apply_executed: true,
    })
}
