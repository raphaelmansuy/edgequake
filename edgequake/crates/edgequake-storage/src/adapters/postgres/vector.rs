//! PostgreSQL vector storage using pgvector extension.

use async_trait::async_trait;
use sqlx::Row;

use super::config::{PostgresConfig, VectorIndexType};
use super::connection::PostgresPool;
use crate::error::{Result, StorageError};
use crate::traits::{VectorSearchResult, VectorStorage};

/// PostgreSQL vector storage using pgvector.
///
/// Supports:
/// - IVFFlat index for approximate nearest neighbor search
/// - HNSW index for faster queries on large datasets  
/// - Cosine, L2, and inner product distance metrics
pub struct PgVectorStorage {
    pool: PostgresPool,
    table_name: String,
    namespace: String,
    dimension: usize,
    index_type: VectorIndexType,
    ivfflat_lists: u32,
    hnsw_m: u32,
    hnsw_ef_construction: u32,
    prefix: String,
}

impl PgVectorStorage {
    /// Create a new pgvector storage.
    pub fn new(config: PostgresConfig) -> Self {
        let prefix = config.table_prefix();
        let table_name = format!("public.eq_{}_vectors", prefix);
        let namespace = config.namespace.clone();
        let dimension = 1536; // Default OpenAI embedding dimension
        let index_type = config.vector_index_type.clone();
        let ivfflat_lists = config.ivfflat_lists;
        let hnsw_m = config.hnsw_m;
        let hnsw_ef_construction = config.hnsw_ef_construction;

        Self {
            pool: PostgresPool::new(config),
            table_name,
            namespace,
            dimension,
            index_type,
            ivfflat_lists,
            hnsw_m,
            hnsw_ef_construction,
            prefix,
        }
    }

    /// Create a new pgvector storage with a specific dimension.
    pub fn with_dimension(config: PostgresConfig, dimension: usize) -> Self {
        let mut storage = Self::new(config);
        storage.dimension = dimension;
        storage
    }

    /// Create the vectors table.
    async fn create_table(&self) -> Result<()> {
        let pool = self.pool.get().await?;

        // Ensure pgvector extension is available
        sqlx::query("CREATE EXTENSION IF NOT EXISTS vector")
            .execute(&pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!("Failed to create vector extension: {}", e))
            })?;

        let sql = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                id TEXT PRIMARY KEY,
                embedding vector({}) NOT NULL,
                metadata JSONB DEFAULT '{{}}',
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
            self.table_name, self.dimension
        );

        sqlx::query(&sql).execute(&pool).await.map_err(|e| {
            StorageError::Database(format!("Failed to create vectors table: {}", e))
        })?;

        // Create vector index
        let index_sql = match self.index_type {
            VectorIndexType::IVFFlat => format!(
                "CREATE INDEX IF NOT EXISTS eq_{}_vectors_embedding_idx ON {} USING ivfflat (embedding vector_cosine_ops) WITH (lists = {})",
                self.prefix, self.table_name, self.ivfflat_lists
            ),
            VectorIndexType::HNSW => format!(
                "CREATE INDEX IF NOT EXISTS eq_{}_vectors_embedding_idx ON {} USING hnsw (embedding vector_cosine_ops) WITH (m = {}, ef_construction = {})",
                self.prefix, self.table_name, self.hnsw_m, self.hnsw_ef_construction
            ),
            VectorIndexType::None => String::new(),
        };

        // Index creation may fail if table is empty, that's OK
        if !index_sql.is_empty() {
            sqlx::query(&index_sql).execute(&pool).await.ok();
        }

        Ok(())
    }

    /// Convert embedding vector to PostgreSQL format.
    fn format_embedding(embedding: &[f32]) -> String {
        let values: Vec<String> = embedding.iter().map(|v| v.to_string()).collect();
        format!("[{}]", values.join(","))
    }

    /// Parse embedding from PostgreSQL text format.
    fn parse_embedding(text: &str) -> Vec<f32> {
        let trimmed = text.trim_start_matches('[').trim_end_matches(']');
        trimmed
            .split(',')
            .filter_map(|s| s.trim().parse::<f32>().ok())
            .collect()
    }
}

#[async_trait]
impl VectorStorage for PgVectorStorage {
    fn namespace(&self) -> &str {
        &self.namespace
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    async fn initialize(&self) -> Result<()> {
        self.pool.initialize().await?;
        self.create_table().await?;
        Ok(())
    }

    async fn finalize(&self) -> Result<()> {
        Ok(())
    }

    async fn query(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        filter_ids: Option<&[String]>,
    ) -> Result<Vec<VectorSearchResult>> {
        let pool = self.pool.get().await?;
        let embedding_str = Self::format_embedding(query_embedding);

        let sql = if let Some(ids) = filter_ids {
            if ids.is_empty() {
                return Ok(Vec::new());
            }
            format!(
                r#"
                SELECT id, metadata, 1 - (embedding <=> $1::vector) as score
                FROM {}
                WHERE id = ANY($2)
                ORDER BY embedding <=> $1::vector
                LIMIT $3
                "#,
                self.table_name
            )
        } else {
            format!(
                r#"
                SELECT id, metadata, 1 - (embedding <=> $1::vector) as score
                FROM {}
                ORDER BY embedding <=> $1::vector
                LIMIT $2
                "#,
                self.table_name
            )
        };

        let rows = if let Some(ids) = filter_ids {
            sqlx::query(&sql)
                .bind(&embedding_str)
                .bind(ids)
                .bind(top_k as i32)
                .fetch_all(&pool)
                .await
        } else {
            sqlx::query(&sql)
                .bind(&embedding_str)
                .bind(top_k as i32)
                .fetch_all(&pool)
                .await
        };

        let rows =
            rows.map_err(|e| StorageError::Database(format!("Vector query failed: {}", e)))?;

        let results = rows
            .iter()
            .map(|row| {
                let id: String = row.get("id");
                let score: f64 = row.get("score");
                let metadata: serde_json::Value = row.get("metadata");
                VectorSearchResult {
                    id,
                    score: score as f32,
                    metadata,
                }
            })
            .collect();

        Ok(results)
    }

    async fn upsert(&self, data: &[(String, Vec<f32>, serde_json::Value)]) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }

        let pool = self.pool.get().await?;

        for (id, embedding, metadata) in data {
            if embedding.len() != self.dimension {
                return Err(StorageError::InvalidQuery(format!(
                    "Embedding dimension mismatch: expected {}, got {}",
                    self.dimension,
                    embedding.len()
                )));
            }

            let embedding_str = Self::format_embedding(embedding);

            let sql = format!(
                r#"
                INSERT INTO {} (id, embedding, metadata)
                VALUES ($1, $2::vector, $3)
                ON CONFLICT (id) DO UPDATE SET
                    embedding = EXCLUDED.embedding,
                    metadata = EXCLUDED.metadata
                "#,
                self.table_name
            );

            sqlx::query(&sql)
                .bind(id)
                .bind(&embedding_str)
                .bind(metadata)
                .execute(&pool)
                .await
                .map_err(|e| StorageError::Database(format!("Upsert failed: {}", e)))?;
        }

        Ok(())
    }

    async fn delete(&self, ids: &[String]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }

        let pool = self.pool.get().await?;

        let sql = format!("DELETE FROM {} WHERE id = ANY($1)", self.table_name);

        sqlx::query(&sql)
            .bind(ids)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("Delete failed: {}", e)))?;

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
            .map_err(|e| StorageError::Database(format!("Delete entity failed: {}", e)))?;

        Ok(())
    }

    async fn delete_entity_relations(&self, entity_name: &str) -> Result<()> {
        let pool = self.pool.get().await?;

        let sql = format!(
            r#"
            DELETE FROM {} 
            WHERE metadata->>'source' = $1 
               OR metadata->>'target' = $1
            "#,
            self.table_name
        );

        sqlx::query(&sql)
            .bind(entity_name)
            .execute(&pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!("Delete entity relations failed: {}", e))
            })?;

        Ok(())
    }

    async fn get_by_id(&self, id: &str) -> Result<Option<Vec<f32>>> {
        let pool = self.pool.get().await?;

        let sql = format!(
            "SELECT embedding::text FROM {} WHERE id = $1",
            self.table_name
        );

        let row: Option<(String,)> = sqlx::query_as(&sql)
            .bind(id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("Get by ID failed: {}", e)))?;

        Ok(row.map(|(embedding_str,)| Self::parse_embedding(&embedding_str)))
    }

    async fn get_by_ids(&self, ids: &[String]) -> Result<Vec<(String, Vec<f32>)>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let pool = self.pool.get().await?;

        let sql = format!(
            "SELECT id, embedding::text FROM {} WHERE id = ANY($1)",
            self.table_name
        );

        let rows: Vec<(String, String)> = sqlx::query_as(&sql)
            .bind(ids)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("Get by IDs failed: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|(id, embedding_str)| (id, Self::parse_embedding(&embedding_str)))
            .collect())
    }

    async fn is_empty(&self) -> Result<bool> {
        let count = self.count().await?;
        Ok(count == 0)
    }

    async fn count(&self) -> Result<usize> {
        let pool = self.pool.get().await?;

        let sql = format!("SELECT COUNT(*) FROM {}", self.table_name);

        let row: (i64,) = sqlx::query_as(&sql)
            .fetch_one(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("Count failed: {}", e)))?;

        Ok(row.0 as usize)
    }

    async fn clear(&self) -> Result<()> {
        let pool = self.pool.get().await?;

        let sql = format!("DELETE FROM {}", self.table_name);

        sqlx::query(&sql)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("Clear failed: {}", e)))?;

        Ok(())
    }
}

impl std::fmt::Debug for PgVectorStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgVectorStorage")
            .field("namespace", &self.namespace)
            .field("dimension", &self.dimension)
            .field("table_name", &self.table_name)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_embedding() {
        let embedding = vec![1.0, 2.0, 3.0];
        let formatted = PgVectorStorage::format_embedding(&embedding);
        assert_eq!(formatted, "[1,2,3]");
    }

    #[test]
    fn test_parse_embedding() {
        let text = "[1,2,3]";
        let parsed = PgVectorStorage::parse_embedding(text);
        assert_eq!(parsed, vec![1.0, 2.0, 3.0]);
    }
}
