//! DDL and schema maintenance for [`super::PgVectorStorage`].

use super::super::config::VectorIndexType;
use super::super::row_count_stats::{self, RowCountStatsConfig};
use super::PgVectorStorage;
use crate::error::{Result, StorageError};

impl PgVectorStorage {
    /// Create the vectors table and indexes.
    pub(crate) async fn create_table(&self) -> Result<()> {
        let pool = self.pool.get().await?;

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

        if !index_sql.is_empty() {
            sqlx::query(&index_sql).execute(&pool).await.ok();
        }

        let gin_sql = format!(
            "CREATE INDEX IF NOT EXISTS eq_{}_vectors_metadata_idx ON {} USING GIN (metadata jsonb_path_ops)",
            self.prefix, self.table_name
        );
        sqlx::query(&gin_sql).execute(&pool).await.ok();

        let add_cols = format!(
            r#"
            ALTER TABLE {} ADD COLUMN IF NOT EXISTS document_id TEXT;
            ALTER TABLE {} ADD COLUMN IF NOT EXISTS tenant_id TEXT;
            ALTER TABLE {} ADD COLUMN IF NOT EXISTS workspace_id TEXT
            "#,
            self.table_name, self.table_name, self.table_name
        );
        for stmt in add_cols.split(';').filter(|s| !s.trim().is_empty()) {
            sqlx::query(stmt.trim()).execute(&pool).await.ok();
        }

        let doc_idx = format!(
            "CREATE INDEX IF NOT EXISTS eq_{}_vectors_doc_id_idx ON {} (document_id) WHERE document_id IS NOT NULL",
            self.prefix, self.table_name
        );
        sqlx::query(&doc_idx).execute(&pool).await.ok();

        let tenant_idx = format!(
            "CREATE INDEX IF NOT EXISTS eq_{}_vectors_tenant_ws_idx ON {} (tenant_id, workspace_id) WHERE tenant_id IS NOT NULL",
            self.prefix, self.table_name
        );
        sqlx::query(&tenant_idx).execute(&pool).await.ok();

        self.ensure_row_count_stats(&pool).await?;

        Ok(())
    }

    pub(crate) async fn ensure_row_count_stats(&self, pool: &sqlx::PgPool) -> Result<()> {
        row_count_stats::ensure_row_count_stats(
            pool,
            &RowCountStatsConfig {
                prefix: &self.prefix,
                table_name: &self.table_name,
                stats_table_name: &self.stats_table_name,
                kind: "vectors",
            },
        )
        .await
    }

    /// Drop the vectors table if it exists.
    pub async fn drop_table(&self) -> Result<()> {
        let pool = self.pool.get().await?;

        let sql = format!("DROP TABLE IF EXISTS {} CASCADE", self.table_name);

        sqlx::query(&sql)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to drop vectors table: {}", e)))?;

        tracing::info!(
            table = %self.table_name,
            "Dropped vector table for dimension migration"
        );

        Ok(())
    }

    /// Check if the table exists in the database.
    pub async fn table_exists(&self) -> Result<bool> {
        let pool = match self.pool.get().await {
            Ok(p) => p,
            Err(_) => return Ok(false),
        };

        let (schema, table) = if self.table_name.contains('.') {
            let parts: Vec<&str> = self.table_name.split('.').collect();
            (parts[0], parts[1])
        } else {
            ("public", self.table_name.as_str())
        };

        let sql = r#"
            SELECT EXISTS (
                SELECT 1 FROM information_schema.tables 
                WHERE table_schema = $1 AND table_name = $2
            )
        "#;

        let exists: (bool,) = sqlx::query_as(sql)
            .bind(schema)
            .bind(table)
            .fetch_one(&pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!("Failed to check table existence: {}", e))
            })?;

        Ok(exists.0)
    }
}
