//! PostgreSQL vector storage using pgvector extension.

use std::collections::HashMap;

use async_trait::async_trait;
use sqlx::Row;

use crate::error::{Result, StorageError};
use crate::traits::vector::{VectorSearchResult, VectorStorage};
use super::config::{PostgresConfig, VectorIndexType};
use super::connection::PostgresPool;

/// PostgreSQL vector storage using pgvector.
///
/// Supports:
/// - HNSW index for approximate nearest neighbor search
/// - IVFFlat index for faster indexing
/// - Exact brute-force search
///
/// # Example
///
/// ```ignore
/// use edgequake_storage::adapters::postgres::{PostgresConfig, PgVectorStorage};
///
/// let config = PostgresConfig::new("localhost", 5432, "edgequake", "user", "pass")
///     .with_namespace("my-workspace");
///
/// let storage = PgVectorStorage::new(config, 1536).await?;
/// storage.initialize().await?;
/// ```
pub struct PgVectorStorage {
    pool: PostgresPool,
    dimension: usize,
    table_name: String,
    index_name: String,
}

impl PgVectorStorage {
    /// Create a new PgVector storage.
    pub fn new(config: PostgresConfig, dimension: usize) -> Self {
        let prefix = config.table_prefix();
        let table_name = format!("{}_vectors", prefix);
        let index_name = format!("{}_vectors_embedding_idx", prefix);
        
        Self {
            pool: PostgresPool::new(config),
            dimension,
            table_name,
            index_name,
        }
    }
    
    /// Get the underlying pool.
    pub fn pool(&self) -> &PostgresPool {
        &self.pool
    }
    
    /// Create the vectors table.
    async fn create_table(&self) -> Result<()> {
        let pool = self.pool.get().await?;
        
        let create_sql = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                id TEXT PRIMARY KEY,
                embedding vector({}),
                metadata JSONB NOT NULL DEFAULT '{{}}',
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
            self.table_name,
            self.dimension
        );
        
        sqlx::query(&create_sql)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::InitializationError(format!(
                "Failed to create vectors table: {}", e
            )))?;
        
        Ok(())
    }
    
    /// Create the vector index.
    async fn create_index(&self) -> Result<()> {
        let pool = self.pool.get().await?;
        let config = self.pool.config();
        
        let index_sql = match config.vector_index_type {
            VectorIndexType::None => return Ok(()),
            VectorIndexType::HNSW => {
                format!(
                    r#"
                    CREATE INDEX IF NOT EXISTS {} ON {} 
                    USING hnsw (embedding vector_cosine_ops)
                    WITH (m = {}, ef_construction = {})
                    "#,
                    self.index_name,
                    self.table_name,
                    config.hnsw_m,
                    config.hnsw_ef_construction
                )
            }
            VectorIndexType::IVFFlat => {
                format!(
                    r#"
                    CREATE INDEX IF NOT EXISTS {} ON {} 
                    USING ivfflat (embedding vector_cosine_ops)
                    WITH (lists = {})
                    "#,
                    self.index_name,
                    self.table_name,
                    config.ivfflat_lists
                )
            }
        };
        
        sqlx::query(&index_sql)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::InitializationError(format!(
                "Failed to create vector index: {}", e
            )))?;
        
        Ok(())
    }
    
    /// Format a vector for SQL.
    fn format_vector(v: &[f32]) -> String {
        let nums: Vec<String> = v.iter().map(|x| x.to_string()).collect();
        format!("[{}]", nums.join(","))
    }
}

#[async_trait]
impl VectorStorage for PgVectorStorage {
    fn namespace(&self) -> &str {
        &self.pool.config().namespace
    }
    
    fn dimension(&self) -> usize {
        self.dimension
    }
    
    async fn initialize(&self) -> Result<()> {
        self.pool.initialize().await?;
        self.create_table().await?;
        self.create_index().await?;
        Ok(())
    }
    
    async fn finalize(&self) -> Result<()> {
        // PostgreSQL handles durability automatically
        Ok(())
    }
    
    async fn query(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        filter_ids: Option<&[String]>,
    ) -> Result<Vec<VectorSearchResult>> {
        let pool = self.pool.get().await?;
        let query_vec = Self::format_vector(query_embedding);
        
        let (sql, bind_ids): (String, Option<Vec<String>>) = match filter_ids {
            Some(ids) if !ids.is_empty() => {
                let placeholders: Vec<String> = (1..=ids.len())
                    .map(|i| format!("${}", i + 1))
                    .collect();
                (
                    format!(
                        r#"
                        SELECT id, 1 - (embedding <=> $1::vector) as score, metadata
                        FROM {}
                        WHERE id = ANY(ARRAY[{}])
                        ORDER BY embedding <=> $1::vector
                        LIMIT {}
                        "#,
                        self.table_name,
                        placeholders.join(", "),
                        top_k
                    ),
                    Some(ids.to_vec()),
                )
            }
            _ => (
                format!(
                    r#"
                    SELECT id, 1 - (embedding <=> $1::vector) as score, metadata
                    FROM {}
                    ORDER BY embedding <=> $1::vector
                    LIMIT {}
                    "#,
                    self.table_name,
                    top_k
                ),
                None,
            ),
        };
        
        let mut query = sqlx::query(&sql).bind(&query_vec);
        
        if let Some(ids) = &bind_ids {
            for id in ids {
                query = query.bind(id);
            }
        }
        
        let rows = query
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::QueryError(format!("Vector query failed: {}", e)))?;
        
        let results = rows
            .into_iter()
            .map(|row| {
                let id: String = row.get("id");
                let score: f32 = row.get::<f64, _>("score") as f32;
                let metadata: serde_json::Value = row.get("metadata");
                VectorSearchResult { id, score, metadata }
            })
            .collect();
        
        Ok(results)
    }
    
    async fn upsert(
        &self,
        data: &[(String, Vec<f32>, serde_json::Value)],
    ) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        
        let pool = self.pool.get().await?;
        
        for (id, embedding, metadata) in data {
            if embedding.len() != self.dimension {
                return Err(StorageError::InvalidInput(format!(
                    "Expected dimension {}, got {}",
                    self.dimension,
                    embedding.len()
                )));
            }
            
            let vec_str = Self::format_vector(embedding);
            
            let sql = format!(
                r#"
                INSERT INTO {} (id, embedding, metadata, updated_at)
                VALUES ($1, $2::vector, $3, NOW())
                ON CONFLICT (id) DO UPDATE SET
                    embedding = EXCLUDED.embedding,
                    metadata = EXCLUDED.metadata,
                    updated_at = NOW()
                "#,
                self.table_name
            );
            
            sqlx::query(&sql)
                .bind(id)
                .bind(&vec_str)
                .bind(metadata)
                .execute(&pool)
                .await
                .map_err(|e| StorageError::WriteError(format!("Upsert failed: {}", e)))?;
        }
        
        Ok(())
    }
    
    async fn delete(&self, ids: &[String]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        
        let pool = self.pool.get().await?;
        
        let placeholders: Vec<String> = (1..=ids.len())
            .map(|i| format!("${}", i))
            .collect();
        
        let sql = format!(
            "DELETE FROM {} WHERE id = ANY(ARRAY[{}])",
            self.table_name,
            placeholders.join(", ")
        );
        
        let mut query = sqlx::query(&sql);
        for id in ids {
            query = query.bind(id);
        }
        
        query
            .execute(&pool)
            .await
            .map_err(|e| StorageError::WriteError(format!("Delete failed: {}", e)))?;
        
        Ok(())
    }
    
    async fn delete_entity(&self, entity_name: &str) -> Result<()> {
        let pool = self.pool.get().await?;
        
        let sql = format!(
            "DELETE FROM {} WHERE metadata->>'entity_name' = $1",
            self.table_name
        );
        
        sqlx::query(&sql)
            .bind(entity_name)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::WriteError(format!("Delete entity failed: {}", e)))?;
        
        Ok(())
    }
    
    async fn delete_entity_relations(&self, entity_name: &str) -> Result<()> {
        let pool = self.pool.get().await?;
        
        let sql = format!(
            r#"
            DELETE FROM {} WHERE 
                metadata->>'source_entity' = $1 OR 
                metadata->>'target_entity' = $1
            "#,
            self.table_name
        );
        
        sqlx::query(&sql)
            .bind(entity_name)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::WriteError(format!(
                "Delete entity relations failed: {}", e
            )))?;
        
        Ok(())
    }
    
    async fn get_by_id(&self, id: &str) -> Result<Option<Vec<f32>>> {
        let pool = self.pool.get().await?;
        
        let sql = format!(
            "SELECT embedding::text FROM {} WHERE id = $1",
            self.table_name
        );
        
        let row = sqlx::query(&sql)
            .bind(id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| StorageError::QueryError(format!("Get by ID failed: {}", e)))?;
        
        match row {
            Some(row) => {
                let embedding_str: String = row.get("embedding");
                let embedding = parse_vector_string(&embedding_str)?;
                Ok(Some(embedding))
            }
            None => Ok(None),
        }
    }
    
    async fn get_by_ids(&self, ids: &[String]) -> Result<Vec<(String, Vec<f32>)>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        
        let pool = self.pool.get().await?;
        
        let placeholders: Vec<String> = (1..=ids.len())
            .map(|i| format!("${}", i))
            .collect();
        
        let sql = format!(
            "SELECT id, embedding::text FROM {} WHERE id = ANY(ARRAY[{}])",
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
            .map_err(|e| StorageError::QueryError(format!("Get by IDs failed: {}", e)))?;
        
        let mut results = Vec::new();
        for row in rows {
            let id: String = row.get("id");
            let embedding_str: String = row.get("embedding");
            let embedding = parse_vector_string(&embedding_str)?;
            results.push((id, embedding));
        }
        
        Ok(results)
    }
    
    async fn is_empty(&self) -> Result<bool> {
        let count = self.count().await?;
        Ok(count == 0)
    }
    
    async fn count(&self) -> Result<usize> {
        let pool = self.pool.get().await?;
        
        let sql = format!("SELECT COUNT(*) as count FROM {}", self.table_name);
        
        let row = sqlx::query(&sql)
            .fetch_one(&pool)
            .await
            .map_err(|e| StorageError::QueryError(format!("Count failed: {}", e)))?;
        
        let count: i64 = row.get("count");
        Ok(count as usize)
    }
    
    async fn clear(&self) -> Result<()> {
        let pool = self.pool.get().await?;
        
        let sql = format!("TRUNCATE TABLE {}", self.table_name);
        
        sqlx::query(&sql)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::WriteError(format!("Clear failed: {}", e)))?;
        
        Ok(())
    }
}

/// Parse a PostgreSQL vector string like "[1.0,2.0,3.0]" into Vec<f32>.
fn parse_vector_string(s: &str) -> Result<Vec<f32>> {
    let trimmed = s.trim().trim_start_matches('[').trim_end_matches(']');
    
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    
    trimmed
        .split(',')
        .map(|x| {
            x.trim()
                .parse::<f32>()
                .map_err(|e| StorageError::DataCorruption(format!(
                    "Invalid vector element '{}': {}", x, e
                )))
        })
        .collect()
}

impl std::fmt::Debug for PgVectorStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgVectorStorage")
            .field("namespace", &self.pool.config().namespace)
            .field("dimension", &self.dimension)
            .field("table_name", &self.table_name)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_vector() {
        let v = vec![1.0, 2.5, 3.14];
        let formatted = PgVectorStorage::format_vector(&v);
        assert_eq!(formatted, "[1,2.5,3.14]");
    }
    
    #[test]
    fn test_parse_vector_string() {
        let s = "[1.0,2.5,3.14]";
        let v = parse_vector_string(s).unwrap();
        assert_eq!(v.len(), 3);
        assert!((v[0] - 1.0).abs() < f32::EPSILON);
        assert!((v[1] - 2.5).abs() < f32::EPSILON);
    }
    
    #[test]
    fn test_parse_empty_vector() {
        let s = "[]";
        let v = parse_vector_string(s).unwrap();
        assert!(v.is_empty());
    }
}
