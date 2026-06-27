//! Shared PostgreSQL schema introspection helpers.

use super::config::split_qualified_table_name;
use crate::error::{Result, StorageError};

/// Whether a qualified table name exists in `information_schema.tables`.
pub(crate) async fn relation_exists(pool: &sqlx::PgPool, qualified_name: &str) -> Result<bool> {
    let (schema, table) = split_qualified_table_name(qualified_name);

    let exists: (bool,) = sqlx::query_as(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM information_schema.tables
            WHERE table_schema = $1 AND table_name = $2
        )
        "#,
    )
    .bind(schema)
    .bind(table)
    .fetch_one(pool)
    .await
    .map_err(|e| StorageError::Database(format!("Failed to check relation existence: {}", e)))?;

    Ok(exists.0)
}
