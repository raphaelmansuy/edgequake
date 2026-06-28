//! Migration 057 — KV identity mirror deprecated (phase 46).

use std::collections::HashSet;

use sqlx::PgPool;

use super::execute_bootstrap_apply_sql;
use tracing::info;

use super::super::{Migration057Report, MIGRATION_057_VERSION, SQL_057_APPLY};

pub async fn reconcile_migration_057(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
    applied_this_run: &[i64],
) -> Result<Migration057Report, sqlx::Error> {
    let marker_applied = applied_this_run.contains(&MIGRATION_057_VERSION);
    let marker_present = applied_after.contains(&MIGRATION_057_VERSION);

    info!(
        target: "edgequake.migration",
        step = "migration_057_apply_start",
        marker_applied,
        marker_present,
        "Ensuring KV identity mirror deprecated marker (migration 057)"
    );
    execute_bootstrap_apply_sql(pool, SQL_057_APPLY).await?;

    Ok(Migration057Report {
        marker_present: marker_present || marker_applied,
        apply_executed: true,
    })
}
