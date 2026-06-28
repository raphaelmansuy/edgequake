//! Migration 041 — document stats / cost columns (SPEC-021 P-A1).
//!
//! Idempotent reconcile for dev DBs that predated migration 041 or missed the
//! sqlx apply (e.g. partial manual schema). Checks `documents.cost_usd` and
//! applies the embedded DDL when absent.

use sqlx::PgPool;

use super::execute_bootstrap_apply_sql;
use tracing::info;

use super::super::SQL_041_APPLY;

pub async fn reconcile_migration_041(pool: &PgPool) -> Result<bool, sqlx::Error> {
    let column_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = 'documents'
              AND column_name = 'cost_usd'
        )",
    )
    .fetch_one(pool)
    .await?;

    if column_exists {
        return Ok(false);
    }

    info!(
        target: "edgequake.migration",
        step = "migration_041_reconcile",
        "documents.cost_usd missing — applying migration 041 document stats columns"
    );
    execute_bootstrap_apply_sql(pool, SQL_041_APPLY).await?;
    Ok(true)
}
