//! Migration 081 — AGE graph RLS policies (SPEC-042-E E-02).

use std::collections::HashSet;

use sqlx::PgPool;
use tracing::info;

use edgequake_storage::adapters::postgres::{age_rls_requested, age_supports_rls};

use super::super::{MIGRATION_081_VERSION, SQL_081_APPLY};
use super::execute_bootstrap_apply_sql;

pub async fn reconcile_migration_081(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
    applied_this_run: &[i64],
) -> Result<bool, sqlx::Error> {
    if !age_rls_requested() {
        return Ok(false);
    }

    let age_ext: Option<String> =
        sqlx::query_scalar("SELECT extversion FROM pg_extension WHERE extname = 'age'")
            .fetch_optional(pool)
            .await?;

    if !age_supports_rls(age_ext.as_deref()) {
        info!(
            target: "edgequake.migration",
            step = "migration_081_skip",
            age_extversion = ?age_ext,
            "AGE RLS requested but extension < 1.7.0"
        );
        return Ok(false);
    }

    let marker_applied = applied_this_run.contains(&MIGRATION_081_VERSION);
    let marker_present = applied_after.contains(&MIGRATION_081_VERSION);

    info!(
        target: "edgequake.migration",
        step = "migration_081_apply_start",
        marker_applied,
        marker_present,
        "Applying AGE graph RLS policies (migration 081)"
    );
    execute_bootstrap_apply_sql(pool, SQL_081_APPLY).await?;
    Ok(true)
}
