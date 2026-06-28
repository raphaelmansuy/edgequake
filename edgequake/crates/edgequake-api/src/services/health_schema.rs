//! Operational schema health queries (SPEC-027 phase 46).
//!
//! ## Why no RLS envelope
//!
//! `_sqlx_migrations` is deployment-global metadata — not tenant-scoped. These reads use a
//! plain pool connection (ops bypass), consistent with migration bootstrap itself.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SqlxMigrationStats {
    pub applied_count: i64,
    pub latest_version: Option<i64>,
    pub last_applied_at: Option<DateTime<Utc>>,
}

/// Aggregate migration stats from sqlx's bookkeeping table.
pub async fn fetch_sqlx_migration_stats(pool: &PgPool) -> Option<SqlxMigrationStats> {
    sqlx::query_as::<_, SqlxMigrationStats>(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE success = true) AS applied_count,
            MAX(version) FILTER (WHERE success = true) AS latest_version,
            MAX(installed_on) FILTER (WHERE success = true) AS last_applied_at
        FROM _sqlx_migrations
        "#,
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}
