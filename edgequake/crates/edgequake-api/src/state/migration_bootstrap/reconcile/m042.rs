use std::collections::HashSet;

use sqlx::PgPool;

use super::execute_bootstrap_apply_sql;
use tracing::info;

use super::super::helpers::pgvector_supports_iterative_scan;
use super::super::{Migration042Report, MIGRATION_042_VERSION, SQL_042_APPLY};

pub async fn reconcile_migration_042(
    pool: &PgPool,
    applied_after: &HashSet<i64>,
    applied_this_run: &[i64],
) -> Result<Migration042Report, sqlx::Error> {
    let pgvector_available: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = 'vector')")
            .fetch_one(pool)
            .await?;

    if !pgvector_available {
        return Ok(Migration042Report {
            pgvector_available: false,
            extversion_before: None,
            extversion_after: None,
            shipped_extversion: None,
            iterative_scan_capable: false,
            indexes_rebuilt: false,
            vector_tables_checked: 0,
        });
    }

    let extversion_before: Option<String> =
        sqlx::query_scalar("SELECT extversion FROM pg_extension WHERE extname = 'vector'")
            .fetch_optional(pool)
            .await?;

    let shipped_extversion: Option<String> = sqlx::query_scalar(
        "SELECT default_version FROM pg_available_extensions WHERE name = 'vector'",
    )
    .fetch_optional(pool)
    .await?;

    let marker_applied = applied_this_run.contains(&MIGRATION_042_VERSION);
    let marker_present = applied_after.contains(&MIGRATION_042_VERSION);
    let needs_apply = marker_applied
        || (marker_present
            && extversion_before
                .as_deref()
                .map(pgvector_supports_iterative_scan)
                == Some(false));

    if needs_apply {
        info!(
            target: "edgequake.migration",
            step = "migration_042_apply_start",
            marker_applied,
            extversion = ?extversion_before,
            "Running pgvector upgrade + ANN index rebuild (migration 042)"
        );
        execute_bootstrap_apply_sql(pool, SQL_042_APPLY).await?;
    }

    let extversion_after: Option<String> =
        sqlx::query_scalar("SELECT extversion FROM pg_extension WHERE extname = 'vector'")
            .fetch_optional(pool)
            .await?;

    let vector_tables_checked: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM pg_tables WHERE schemaname = 'public' AND tablename LIKE 'eq\\_%\\_vectors' ESCAPE '\\'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let iterative_scan_capable = extversion_after
        .as_deref()
        .map(pgvector_supports_iterative_scan)
        .unwrap_or(false);

    Ok(Migration042Report {
        pgvector_available: true,
        extversion_before,
        extversion_after,
        shipped_extversion,
        iterative_scan_capable,
        indexes_rebuilt: needs_apply,
        vector_tables_checked: vector_tables_checked as usize,
    })
}
