//! Migration 043 — Apache AGE extension upgrade.

use std::collections::HashSet;

use sqlx::PgPool;

use super::execute_bootstrap_apply_sql;
use tracing::info;

use super::super::helpers::extension_version_at_least;
use super::super::{Migration043Report, MIGRATION_043_VERSION, SQL_043_APPLY};

pub async fn reconcile_migration_043(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
    applied_this_run: &[i64],
) -> Result<Migration043Report, sqlx::Error> {
    let age_available: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = 'age')")
            .fetch_one(pool)
            .await?;

    if !age_available {
        return Ok(Migration043Report {
            age_available: false,
            extversion_before: None,
            extversion_after: None,
            shipped_extversion: None,
            extension_updated: false,
        });
    }

    let extversion_before: Option<String> =
        sqlx::query_scalar("SELECT extversion FROM pg_extension WHERE extname = 'age'")
            .fetch_optional(pool)
            .await?;

    let shipped_extversion: Option<String> = sqlx::query_scalar(
        "SELECT default_version FROM pg_available_extensions WHERE name = 'age'",
    )
    .fetch_optional(pool)
    .await?;

    let marker_applied = applied_this_run.contains(&MIGRATION_043_VERSION);
    let marker_present = applied_after.contains(&MIGRATION_043_VERSION);
    let catalog_behind_shipped = match (extversion_before.as_deref(), shipped_extversion.as_deref())
    {
        (Some(current), Some(shipped)) => !extension_version_at_least(current, shipped),
        _ => false,
    };
    let needs_apply = marker_applied || marker_present || catalog_behind_shipped;

    if needs_apply {
        info!(
            target: "edgequake.migration",
            step = "migration_043_apply_start",
            marker_applied,
            catalog_behind_shipped,
            extversion = ?extversion_before,
            shipped = ?shipped_extversion,
            "Running Apache AGE extension upgrade (migration 043)"
        );
        execute_bootstrap_apply_sql(pool, SQL_043_APPLY).await?;
    }

    let extversion_after: Option<String> =
        sqlx::query_scalar("SELECT extversion FROM pg_extension WHERE extname = 'age'")
            .fetch_optional(pool)
            .await?;

    Ok(Migration043Report {
        age_available: true,
        extversion_before,
        extversion_after,
        shipped_extversion,
        extension_updated: needs_apply,
    })
}
