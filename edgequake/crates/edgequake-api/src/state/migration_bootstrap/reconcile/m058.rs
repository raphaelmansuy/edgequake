//! Migration 058 — KV identity mirror ignored when PG pool (phase 47).

use std::collections::HashSet;

use sqlx::PgPool;

use super::execute_bootstrap_apply_sql;
use tracing::info;

use super::super::{Migration058Report, MIGRATION_058_VERSION, SQL_058_APPLY};

pub async fn reconcile_migration_058(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
    applied_this_run: &[i64],
) -> Result<Migration058Report, sqlx::Error> {
    let marker_applied = applied_this_run.contains(&MIGRATION_058_VERSION);
    let marker_present = applied_after.contains(&MIGRATION_058_VERSION);

    info!(
        target: "edgequake.migration",
        step = "migration_058_apply_start",
        marker_applied,
        marker_present,
        "Ensuring KV mirror ignored-with-pool marker (migration 058)"
    );
    execute_bootstrap_apply_sql(pool, SQL_058_APPLY).await?;

    Ok(Migration058Report {
        marker_present: marker_present || marker_applied,
        apply_executed: true,
    })
}
