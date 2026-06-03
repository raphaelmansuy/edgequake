use async_trait::async_trait;
use sqlx::Row;

use super::PgVectorStorage;
use crate::error::{Result, StorageError};
use crate::traits::{MetadataFilter, VectorSearchResult, VectorStorage};

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

        // QW3: run inside a short transaction so we can raise recall via
        // `SET LOCAL` GUCs scoped to just this search (never leaking onto the
        // shared pooled connection). The plain `query()` path applies no
        // metadata post-filter, so iterative_scan is not requested here.
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| StorageError::Database(format!("Failed to begin query tx: {}", e)))?;

        for stmt in Self::search_tuning_statements(self.index_type, top_k, false, false) {
            sqlx::query(&stmt)
                .execute(&mut *tx)
                .await
                .map_err(|e| StorageError::Database(format!("Failed to set search GUC: {}", e)))?;
        }

        let rows = if let Some(ids) = filter_ids {
            sqlx::query(&sql)
                .bind(&embedding_str)
                .bind(ids)
                .bind(top_k as i32)
                .fetch_all(&mut *tx)
                .await
        } else {
            sqlx::query(&sql)
                .bind(&embedding_str)
                .bind(top_k as i32)
                .fetch_all(&mut *tx)
                .await
        };

        let rows =
            rows.map_err(|e| StorageError::Database(format!("Vector query failed: {}", e)))?;

        tx.commit()
            .await
            .map_err(|e| StorageError::Database(format!("Failed to commit query tx: {}", e)))?;

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

        // QW2 edge case #1: validate EVERY embedding dimension up front (fail
        // fast, all-or-nothing). WHY: a single malformed row must not be
        // silently committed alongside good rows, and validating before we
        // build the batch arrays avoids partial writes.
        for (id, embedding, _) in data {
            if embedding.len() != self.dimension {
                return Err(StorageError::InvalidQuery(format!(
                    "Embedding dimension mismatch for id '{}': expected {}, got {}",
                    id,
                    self.dimension,
                    embedding.len()
                )));
            }
        }

        // QW2 edge case #2: de-duplicate IDs WITHIN the batch (last-write-wins).
        // WHY: `INSERT ... SELECT ... ON CONFLICT DO UPDATE` raises
        // "ON CONFLICT DO UPDATE command cannot affect row a second time" if the
        // same conflict target appears twice in one statement. We keep only the
        // last occurrence of each id, matching the previous row-by-row loop's
        // observable behavior (later rows overwrote earlier ones).
        let mut last_index: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::with_capacity(data.len());
        for (i, (id, _, _)) in data.iter().enumerate() {
            last_index.insert(id.as_str(), i);
        }
        let kept: Vec<usize> = (0..data.len())
            .filter(|&i| last_index.get(data[i].0.as_str()) == Some(&i))
            .collect();

        let pool = self.pool.get().await?;

        // QW2: single round trip per chunk via UNNEST instead of one INSERT per
        // row. WHY chunk: bounds per-statement memory/transaction size for very
        // large ingests; UNNEST keeps the bind-parameter count constant (3)
        // regardless of row count, so we are not limited by Postgres' 65535
        // parameter cap. All chunks run in ONE transaction for atomicity.
        const CHUNK: usize = 1_000;

        let sql = format!(
            r#"
            INSERT INTO {} (id, embedding, metadata, document_id, tenant_id, workspace_id)
            SELECT
                t.id,
                t.embedding::vector,
                t.metadata,
                COALESCE(t.metadata->>'document_id', t.metadata->>'source_document_id'),
                t.metadata->>'tenant_id',
                t.metadata->>'workspace_id'
            FROM UNNEST($1::text[], $2::text[], $3::jsonb[]) AS t(id, embedding, metadata)
            ON CONFLICT (id) DO UPDATE SET
                embedding = EXCLUDED.embedding,
                metadata = EXCLUDED.metadata,
                document_id = EXCLUDED.document_id,
                tenant_id = EXCLUDED.tenant_id,
                workspace_id = EXCLUDED.workspace_id
            "#,
            self.table_name
        );

        let mut tx = pool
            .begin()
            .await
            .map_err(|e| StorageError::Database(format!("Failed to begin upsert tx: {}", e)))?;

        for chunk in kept.chunks(CHUNK) {
            let mut ids: Vec<String> = Vec::with_capacity(chunk.len());
            let mut embeddings: Vec<String> = Vec::with_capacity(chunk.len());
            let mut metadatas: Vec<serde_json::Value> = Vec::with_capacity(chunk.len());
            for &i in chunk {
                let (id, embedding, metadata) = &data[i];
                ids.push(id.clone());
                embeddings.push(Self::format_embedding(embedding));
                metadatas.push(metadata.clone());
            }

            sqlx::query(&sql)
                .bind(&ids)
                .bind(&embeddings)
                .bind(&metadatas)
                .execute(&mut *tx)
                .await
                .map_err(|e| StorageError::Database(format!("Batch upsert failed: {}", e)))?;
        }

        tx.commit()
            .await
            .map_err(|e| StorageError::Database(format!("Failed to commit upsert tx: {}", e)))?;

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
        let pool = self.pool.get().await?;

        let sql = format!(
            "SELECT NOT EXISTS (SELECT 1 FROM {} LIMIT 1) AS is_empty",
            self.table_name
        );

        let row: (bool,) = sqlx::query_as(&sql)
            .fetch_one(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("is_empty failed: {}", e)))?;

        Ok(row.0)
    }

    async fn count(&self) -> Result<usize> {
        let pool = self.pool.get().await?;

        // SPEC-011 iter 02 Fix A: O(1) read from maintained counter — never
        // `SELECT COUNT(*) FROM vectors`. Fallback to raw COUNT only if the
        // stats table is somehow absent (defensive, should not happen after init).
        let sql = format!(
            "SELECT row_count FROM {} WHERE id = 1",
            self.stats_table_name
        );

        let row: Option<(i64,)> = sqlx::query_as(&sql)
            .fetch_optional(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("Vector count failed: {}", e)))?;

        if let Some((count,)) = row {
            return Ok(count as usize);
        }

        // SPEC-012 Fix H (self-heal): bootstrap stats on first hit if missing
        // (handles deployments that predate SPEC-011 iter 02).
        tracing::warn!(
            stats_table = %self.stats_table_name,
            "Vector stats row missing — running self-heal"
        );
        if let Err(e) = self.ensure_row_count_stats(&pool).await {
            tracing::warn!(error = %e, "Vector stats self-heal failed; falling back to COUNT(*)");
        }

        let fallback = format!("SELECT COUNT(*) FROM {}", self.table_name);
        let row: (i64,) = sqlx::query_as(&fallback)
            .fetch_one(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("Vector count fallback failed: {}", e)))?;
        Ok(row.0 as usize)
    }

    async fn ping(&self) -> Result<()> {
        let pool = self.pool.get().await?;

        let sql = format!("SELECT 1 FROM {} LIMIT 1", self.table_name);

        sqlx::query(&sql)
            .fetch_optional(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("Vector ping failed: {}", e)))?;

        Ok(())
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

    /// Clear vectors for a specific workspace.
    ///
    /// QW6: match the materialized `workspace_id` column FIRST, then fall back
    /// to the JSONB key.
    ///
    /// # WHY both predicates
    /// Rows written after the SPEC-007 Tier-3 dual-write carry a populated
    /// `workspace_id` column, while rows written before the backfill (or by
    /// older code paths) only carry `metadata->>'workspace_id'`. Matching on
    /// the column alone would silently leave legacy rows behind on delete;
    /// matching on JSONB alone forfeits the column index. The `OR` keeps the
    /// delete correct during and after the migration window.
    async fn clear_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
        let pool = self.pool.get().await?;

        let sql = format!(
            "DELETE FROM {} WHERE workspace_id = $1 OR metadata->>'workspace_id' = $1",
            self.table_name
        );

        let result = sqlx::query(&sql)
            .bind(workspace_id.to_string())
            .execute(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("Clear workspace failed: {}", e)))?;

        Ok(result.rows_affected() as usize)
    }

    /// Query with metadata pre-filter (SPEC-007 Tier 2/3).
    ///
    /// Generates dynamic SQL WHERE clauses from MetadataFilter fields:
    /// - `document_ids` → checks both `document_id` column AND JSONB keys
    /// - `tenant_id` → checks `tenant_id` column (falls back to JSONB)
    /// - `workspace_id` → checks `workspace_id` column (falls back to JSONB)
    ///
    /// Uses Tier 3 (column-based) if materialized columns exist, otherwise
    /// Tier 2 (JSONB extraction) as fallback.
    ///
    /// @implements SPEC-007 R-T2-01, R-T3-01
    async fn query_filtered(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        filter_ids: Option<&[String]>,
        metadata_filter: Option<&MetadataFilter>,
    ) -> Result<Vec<VectorSearchResult>> {
        // Fast path: if no metadata filter, delegate to standard query
        let mf = match metadata_filter {
            Some(mf) if !mf.is_empty() => mf,
            _ => return self.query(query_embedding, top_k, filter_ids).await,
        };

        let pool = self.pool.get().await?;
        let embedding_str = Self::format_embedding(query_embedding);

        let has_id_filter = filter_ids.map(|ids| !ids.is_empty()).unwrap_or(false);
        let filter_sql = mf.build_sql(has_id_filter, 2);
        let where_clause = if filter_sql.conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", filter_sql.conditions.join(" AND "))
        };

        let sql = format!(
            r#"
            SELECT id, metadata, 1 - (embedding <=> $1::vector) as score
            FROM {}
            {}
            ORDER BY embedding <=> $1::vector
            LIMIT ${}
            "#,
            self.table_name, where_clause, filter_sql.next_param
        );

        // Dynamic parameter binding using raw query + manual bind chain
        // sqlx doesn't support truly dynamic args with query(), so we build
        // the query with the right number of bind slots and bind sequentially.
        use sqlx::postgres::PgArguments;
        use sqlx::Arguments;

        let mut args = PgArguments::default();
        args.add(&embedding_str)
            .map_err(|e| StorageError::Database(format!("Failed to bind embedding: {}", e)))?;

        if let Some(ids) = filter_ids {
            if !ids.is_empty() {
                let id_vec: Vec<String> = ids.to_vec();
                args.add(&id_vec).map_err(|e| {
                    StorageError::Database(format!("Failed to bind filter_ids: {}", e))
                })?;
            }
        }

        if let Some(doc_ids) = &mf.document_ids {
            let doc_vec: Vec<String> = doc_ids.clone();
            args.add(&doc_vec).map_err(|e| {
                StorageError::Database(format!("Failed to bind document_ids: {}", e))
            })?;
        }

        if let Some(tid) = &mf.tenant_id {
            args.add(tid)
                .map_err(|e| StorageError::Database(format!("Failed to bind tenant_id: {}", e)))?;
        }

        if let Some(wid) = &mf.workspace_id {
            args.add(wid).map_err(|e| {
                StorageError::Database(format!("Failed to bind workspace_id: {}", e))
            })?;
        }

        if let Some(vtype) = &mf.vector_type {
            args.add(vtype).map_err(|e| {
                StorageError::Database(format!("Failed to bind vector_type: {}", e))
            })?;
        }

        args.add(top_k as i32)
            .map_err(|e| StorageError::Database(format!("Failed to bind top_k: {}", e)))?;

        // QW3: metadata pre-filter present -> raise recall AND enable iterative
        // scan (scoped to this transaction) so the post-filter LIMIT is met.
        let mut tx = pool.begin().await.map_err(|e| {
            StorageError::Database(format!("Failed to begin filtered query tx: {}", e))
        })?;

        let iterative_scan = self.supports_iterative_scan().await;
        for stmt in Self::search_tuning_statements(self.index_type, top_k, true, iterative_scan) {
            sqlx::query(&stmt)
                .execute(&mut *tx)
                .await
                .map_err(|e| StorageError::Database(format!("Failed to set search GUC: {}", e)))?;
        }

        let rows = sqlx::query_with(&sql, args)
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| StorageError::Database(format!("Filtered vector query failed: {}", e)))?;

        tx.commit().await.map_err(|e| {
            StorageError::Database(format!("Failed to commit filtered query tx: {}", e))
        })?;

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
}
