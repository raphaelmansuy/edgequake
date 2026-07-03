//! Migration 080 — halfvec embedding conversion (SPEC-042-E E-01).

use std::collections::HashSet;

use sqlx::PgPool;
use tracing::info;

use edgequake_storage::adapters::postgres::VectorStorageMode;

use super::super::{MIGRATION_080_VERSION, SQL_080_APPLY};
use super::execute_bootstrap_apply_sql;

pub async fn reconcile_migration_080(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
    applied_this_run: &[i64],
) -> Result<bool, sqlx::Error> {
    if VectorStorageMode::from_env() != VectorStorageMode::Half {
        return Ok(false);
    }

    let marker_applied = applied_this_run.contains(&MIGRATION_080_VERSION);
    let marker_present = applied_after.contains(&MIGRATION_080_VERSION);

    info!(
        target: "edgequake.migration",
        step = "migration_080_apply_start",
        marker_applied,
        marker_present,
        "Converting vector embeddings to halfvec (migration 080)"
    );
    execute_bootstrap_apply_sql(pool, SQL_080_APPLY).await?;
    Ok(true)
}
