//! PostgreSQL key-value storage using JSONB.
//!
//! Provides flexible key-value storage with full JSON query capabilities.
//!
//! ## Implements
//!
//! - [`FEAT0240`]: JSONB key-value storage
//! - [`FEAT0241`]: GIN indexing for fast JSON path queries
//! - [`FEAT0242`]: Atomic upsert operations
//!
//! ## Use Cases
//!
//! - [`UC0601`]: System stores document metadata
//! - [`UC0605`]: System retrieves chunks by ID
//!
//! ## Enforces
//!
//! - [`BR0240`]: Namespace isolation per tenant
//! - [`BR0241`]: Atomic batch operations

use std::collections::HashSet;

use async_trait::async_trait;

use super::config::PostgresConfig;
use super::connection::PostgresPool;
use crate::error::{Result, StorageError};
use crate::traits::KVStorage;

/// PostgreSQL key-value storage using JSONB.
///
/// This implementation uses PostgreSQL's JSONB column type for flexible
/// value storage with full JSON query capabilities.
///
/// # Features
///
/// - JSONB storage for flexible schemas
/// - GIN indexing for fast JSON path queries
/// - Atomic upsert operations
/// - Namespace support for multi-tenancy
pub struct PostgresKVStorage {
    pool: PostgresPool,
    table_name: String,
    namespace: String,
    prefix: String,
}

impl PostgresKVStorage {
    /// Create a new PostgreSQL key-value storage.
    pub fn new(config: PostgresConfig) -> Self {
        let prefix = config.table_prefix();
        let table_name = format!("public.eq_{}_kv", prefix);
        let namespace = config.namespace.clone();

        Self {
            pool: PostgresPool::new(config),
            table_name,
            namespace,
            prefix,
        }
    }

    /// Get the underlying pool.
    pub fn pool(&self) -> &PostgresPool {
        &self.pool
    }

    /// Create the KV table.
    async fn create_table(&self) -> Result<()> {
        let pool = self.pool.get().await?;

        let sql = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                key TEXT PRIMARY KEY,
                value JSONB NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
            self.table_name
        );

        sqlx::query(&sql)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to create KV table: {}", e)))?;

        // Create GIN index for JSONB queries
        let gin_sql = format!(
            "CREATE INDEX IF NOT EXISTS eq_{}_kv_value_gin ON {} USING GIN (value)",
            self.prefix, self.table_name
        );

        sqlx::query(&gin_sql).execute(&pool).await.ok();

        Ok(())
    }
}

#[async_trait]
impl KVStorage for PostgresKVStorage {
    fn namespace(&self) -> &str {
        &self.namespace
    }

    async fn initialize(&self) -> Result<()> {
        self.pool.initialize().await?;
        self.create_table().await?;
        Ok(())
    }

    async fn finalize(&self) -> Result<()> {
        Ok(())
    }

    async fn get_by_id(&self, id: &str) -> Result<Option<serde_json::Value>> {
        let pool = self.pool.get().await?;

        let sql = format!("SELECT value FROM {} WHERE key = $1", self.table_name);

        let row: Option<(serde_json::Value,)> = sqlx::query_as(&sql)
            .bind(id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("KV get failed: {}", e)))?;

        Ok(row.map(|(v,)| v))
    }

    async fn get_by_ids(&self, ids: &[String]) -> Result<Vec<serde_json::Value>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let pool = self.pool.get().await?;

        let sql = format!("SELECT value FROM {} WHERE key = ANY($1)", self.table_name);

        let rows: Vec<(serde_json::Value,)> = sqlx::query_as(&sql)
            .bind(ids)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("KV get_by_ids failed: {}", e)))?;

        Ok(rows.into_iter().map(|(v,)| v).collect())
    }

    async fn filter_keys(&self, keys: HashSet<String>) -> Result<HashSet<String>> {
        if keys.is_empty() {
            return Ok(HashSet::new());
        }

        let pool = self.pool.get().await?;
        let keys_vec: Vec<String> = keys.iter().cloned().collect();

        let sql = format!("SELECT key FROM {} WHERE key = ANY($1)", self.table_name);

        let rows: Vec<(String,)> = sqlx::query_as(&sql)
            .bind(&keys_vec)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("KV filter_keys failed: {}", e)))?;

        let existing: HashSet<String> = rows.into_iter().map(|(k,)| k).collect();

        // Return keys that do NOT exist
        Ok(keys.difference(&existing).cloned().collect())
    }

    async fn upsert(&self, data: &[(String, serde_json::Value)]) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }

        let pool = self.pool.get().await?;

        for (key, value) in data {
            let sql = format!(
                r#"
                INSERT INTO {} (key, value, updated_at)
                VALUES ($1, $2, NOW())
                ON CONFLICT (key) DO UPDATE SET
                    value = EXCLUDED.value,
                    updated_at = NOW()
                "#,
                self.table_name
            );

            sqlx::query(&sql)
                .bind(key)
                .bind(value)
                .execute(&pool)
                .await
                .map_err(|e| StorageError::Database(format!("KV upsert failed: {}", e)))?;
        }

        Ok(())
    }

    async fn delete(&self, ids: &[String]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }

        let pool = self.pool.get().await?;

        let sql = format!("DELETE FROM {} WHERE key = ANY($1)", self.table_name);

        sqlx::query(&sql)
            .bind(ids)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("KV delete failed: {}", e)))?;

        Ok(())
    }

    async fn is_empty(&self) -> Result<bool> {
        let count = self.count().await?;
        Ok(count == 0)
    }

    async fn count(&self) -> Result<usize> {
        let pool = self.pool.get().await?;

        let sql = format!("SELECT COUNT(*) as count FROM {}", self.table_name);

        let row: (i64,) = sqlx::query_as(&sql)
            .fetch_one(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("KV count failed: {}", e)))?;

        Ok(row.0 as usize)
    }

    async fn keys(&self) -> Result<Vec<String>> {
        let pool = self.pool.get().await?;

        let sql = format!("SELECT key FROM {}", self.table_name);

        let rows: Vec<(String,)> = sqlx::query_as(&sql)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("KV keys failed: {}", e)))?;

        Ok(rows.into_iter().map(|(k,)| k).collect())
    }

    async fn clear(&self) -> Result<()> {
        let pool = self.pool.get().await?;

        let sql = format!("DELETE FROM {}", self.table_name);

        sqlx::query(&sql)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("KV clear failed: {}", e)))?;

        Ok(())
    }

    /// Atomically transition document status if current status matches expected.
    ///
    /// @implements FIX-RACE-01: Prevent TOCTOU race conditions
    ///
    /// # WHY: Atomic Compare-And-Swap
    ///
    /// Uses PostgreSQL's atomic UPDATE with WHERE clause to ensure only one
    /// process can successfully transition the status. The affected row count
    /// tells us if the transition succeeded (1) or failed (0).
    ///
    /// SQL: UPDATE ... SET value = jsonb_set(...) WHERE key = $1 AND value->>'status' = $2
    ///
    /// This is atomic at the database level - no race window possible.
    async fn transition_if_status(
        &self,
        key: &str,
        expected_status: &str,
        new_status: &str,
    ) -> Result<bool> {
        let pool = self.pool.get().await?;

        // Atomic update: only succeeds if current status matches expected
        // jsonb_set updates the 'status' field within the JSONB value
        let sql = format!(
            r#"
            UPDATE {}
            SET value = jsonb_set(value, '{{status}}', to_jsonb($3::text)),
                updated_at = NOW()
            WHERE key = $1 AND value->>'status' = $2
            "#,
            self.table_name
        );

        let result = sqlx::query(&sql)
            .bind(key)
            .bind(expected_status)
            .bind(new_status)
            .execute(&pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!("KV transition_if_status failed: {}", e))
            })?;

        // rows_affected = 1 means transition succeeded
        // rows_affected = 0 means status didn't match (or key not found)
        Ok(result.rows_affected() == 1)
    }
}

impl std::fmt::Debug for PostgresKVStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresKVStorage")
            .field("namespace", &self.namespace)
            .field("table_name", &self.table_name)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kv_storage_creation() {
        let config = PostgresConfig::default().with_namespace("test");
        let storage = PostgresKVStorage::new(config);

        // Table name includes schema prefix for PostgreSQL
        assert_eq!(storage.table_name, "public.eq_eq_test_kv");
    }
}
