//! PostgreSQL key-value storage using JSONB.

use async_trait::async_trait;
use sqlx::Row;

use crate::error::{Result, StorageError};
use crate::traits::KVStorage;
use super::config::PostgresConfig;
use super::connection::PostgresPool;

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
///
/// # Example
///
/// ```ignore
/// use edgequake_storage::adapters::postgres::{PostgresConfig, PostgresKVStorage};
///
/// let config = PostgresConfig::new("localhost", 5432, "edgequake", "user", "pass")
///     .with_namespace("my-workspace");
///
/// let storage = PostgresKVStorage::new(config).await?;
/// storage.initialize().await?;
/// ```
pub struct PostgresKVStorage {
    pool: PostgresPool,
    table_name: String,
}

impl PostgresKVStorage {
    /// Create a new PostgreSQL key-value storage.
    pub fn new(config: PostgresConfig) -> Self {
        let prefix = config.table_prefix();
        let table_name = format!("{}_kv", prefix);
        
        Self {
            pool: PostgresPool::new(config),
            table_name,
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
            .map_err(|e| StorageError::InitializationError(format!(
                "Failed to create KV table: {}", e
            )))?;
        
        // Create GIN index for JSONB queries
        let gin_sql = format!(
            "CREATE INDEX IF NOT EXISTS {}_value_gin ON {} USING GIN (value)",
            self.table_name, self.table_name
        );
        
        sqlx::query(&gin_sql).execute(&pool).await.ok();
        
        Ok(())
    }
}

#[async_trait]
impl KVStorage for PostgresKVStorage {
    fn namespace(&self) -> &str {
        &self.pool.config().namespace
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
        
        let sql = format!(
            "SELECT value FROM {} WHERE key = $1",
            self.table_name
        );
        
        let row = sqlx::query(&sql)
            .bind(id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| StorageError::QueryError(format!("KV get failed: {}", e)))?;
        
        match row {
            Some(row) => {
                let value: serde_json::Value = row.get("value");
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }
    
    async fn get_by_ids(
        &self,
        ids: &[String],
    ) -> Result<Vec<serde_json::Value>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        
        let pool = self.pool.get().await?;
        
        let placeholders: Vec<String> = (1..=ids.len())
            .map(|i| format!("${}", i))
            .collect();
        
        let sql = format!(
            "SELECT value FROM {} WHERE key = ANY(ARRAY[{}])",
            self.table_name,
            placeholders.join(", ")
        );
        
        let mut query = sqlx::query(&sql);
        for id in ids {
            query = query.bind(id);
        }
        
        let rows = query
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::QueryError(format!("KV get_by_ids failed: {}", e)))?;
        
        let mut results = Vec::new();
        for row in rows {
            let value: serde_json::Value = row.get("value");
            results.push(value);
        }
        
        Ok(results)
    }
    
    async fn filter_keys(&self, keys: Vec<String>) -> Result<Vec<String>> {
        if keys.is_empty() {
            return Ok(Vec::new());
        }
        
        let pool = self.pool.get().await?;
        
        let placeholders: Vec<String> = (1..=keys.len())
            .map(|i| format!("${}", i))
            .collect();
        
        let sql = format!(
            "SELECT key FROM {} WHERE key = ANY(ARRAY[{}])",
            self.table_name,
            placeholders.join(", ")
        );
        
        let mut query = sqlx::query(&sql);
        for key in &keys {
            query = query.bind(key);
        }
        
        let rows = query
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::QueryError(format!("KV filter_keys failed: {}", e)))?;
        
        let existing: Vec<String> = rows.into_iter().map(|row| row.get("key")).collect();
        
        Ok(existing)
    }
    
    async fn index_done_callback(&self) -> Result<()> {
        // No special handling needed
        Ok(())
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
                .map_err(|e| StorageError::WriteError(format!("KV upsert failed: {}", e)))?;
        }
        
        Ok(())
    }
    
    async fn delete(&self, ids: Vec<String>) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        
        let pool = self.pool.get().await?;
        
        let placeholders: Vec<String> = (1..=ids.len())
            .map(|i| format!("${}", i))
            .collect();
        
        let sql = format!(
            "DELETE FROM {} WHERE key = ANY(ARRAY[{}])",
            self.table_name,
            placeholders.join(", ")
        );
        
        let mut query = sqlx::query(&sql);
        for id in &ids {
            query = query.bind(id);
        }
        
        query
            .execute(&pool)
            .await
            .map_err(|e| StorageError::WriteError(format!("KV delete failed: {}", e)))?;
        
        Ok(())
    }
    
    async fn drop(&self) -> Result<()> {
        let pool = self.pool.get().await?;
        
        let sql = format!("DROP TABLE IF EXISTS {} CASCADE", self.table_name);
        
        sqlx::query(&sql)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::WriteError(format!("KV drop failed: {}", e)))?;
        
        Ok(())
    }
    
    async fn count(&self) -> Result<usize> {
        let pool = self.pool.get().await?;
        
        let sql = format!("SELECT COUNT(*) as count FROM {}", self.table_name);
        
        let row = sqlx::query(&sql)
            .fetch_one(&pool)
            .await
            .map_err(|e| StorageError::QueryError(format!("KV count failed: {}", e)))?;
        
        let count: i64 = row.get("count");
        Ok(count as usize)
    }
    
    async fn all_keys(&self) -> Result<Vec<String>> {
        let pool = self.pool.get().await?;
        
        let sql = format!("SELECT key FROM {}", self.table_name);
        
        let rows = sqlx::query(&sql)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::QueryError(format!("KV all_keys failed: {}", e)))?;
        
        let keys = rows.into_iter().map(|row| row.get("key")).collect();
        
        Ok(keys)
    }
}

impl std::fmt::Debug for PostgresKVStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresKVStorage")
            .field("namespace", &self.pool.config().namespace)
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
        
        assert_eq!(storage.table_name, "eq_test_kv");
    }
}
