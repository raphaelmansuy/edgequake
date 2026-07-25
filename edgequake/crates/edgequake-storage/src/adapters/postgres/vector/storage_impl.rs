use async_trait::async_trait;
use sqlx::Row;

use super::super::ann_exact_reorder_policy::{build_ann_select_sql, AnnExactReorderPolicy};
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

    /**
     * @dataop      DATA-PGVEC-VECTORS-ANN-QUERY-001
     * @engine      pgvector 0.8.x
     * @intent      Top-K approximate nearest-neighbour search (unfiltered), optional id prefilter.
     * @tables      eq_{ns}_vectors(embedding vector|halfvec, id, metadata)
     * @indexes     HNSW/IVFFlat on embedding (vector_cosine_ops); PK(id)
     * @complexity  time: O(ef_search * log N) expected ANN; space: O(K + ef_search); io: ~ef_search pages
     * @limits      - K clamped by caller; ef_search = clamp(4*K, 40, 1000)
     *              - Unfiltered: iterative_scan not set (post-filter N/A)
     *              - Index must fit shared_buffers for stated latency; seq-scan cliff if ANN missing
     * @scaling     See docs/data-layer/benchmarks/001.md
     * @tests       tests/data_layer/data_layer_limits.rs (PG16/17/18)
     * @pgversions  16: ok | 17: ok | 18: ok
     * @docs        specs/088-data-layer/pgvector.md#data-pgvec-vectors-ann-query-001
     */
    async fn query(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        filter_ids: Option<&[String]>,
    ) -> Result<Vec<VectorSearchResult>> {
        let _timer =
            crate::TimedStorageOp::start_dataop(crate::dataop::DATA_PGVEC_VECTORS_ANN_QUERY_001);
        let pool = self.pool.get().await?;
        let embedding_str = Self::format_embedding(query_embedding);
        let emb_type = self.embedding_pg_type();
        let reorder = AnnExactReorderPolicy::from_env();
        // When reorder is on, tune ef_search against the wider candidate pool.
        let tune_k = reorder.effective_candidate_k(top_k);

        let sql = if let Some(ids) = filter_ids {
            if ids.is_empty() {
                return Ok(Vec::new());
            }
            crate::dataop::sql_comment(
                crate::dataop::DATA_PGVEC_VECTORS_ANN_QUERY_001,
                &build_ann_select_sql(
                    &self.table_name,
                    emb_type,
                    "WHERE id = ANY($2)",
                    3,
                    top_k,
                    &reorder,
                ),
            )
        } else {
            crate::dataop::sql_comment(
                crate::dataop::DATA_PGVEC_VECTORS_ANN_QUERY_001,
                &build_ann_select_sql(&self.table_name, emb_type, "", 2, top_k, &reorder),
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

        for stmt in
            Self::search_tuning_statements(self.effective_index_type(), tune_k, false, false)
        {
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
        self.upsert_report_created(data).await.map(|_| ())
    }

    /**
     * @dataop      DATA-PGVEC-VECTORS-UPSERT-BATCH-004
     * @engine      pgvector 0.8.x (secondary: postgres)
     * @intent      Batch upsert embeddings via UNNEST + ON CONFLICT; report newly inserted IDs.
     * @tables      eq_{ns}_vectors
     * @indexes     PK(id); HNSW graph insert cost on new rows
     * @complexity  time: O(B log N) per batch B≤1000; space: O(B * D)
     * @limits      - Batch chunk ≤1000; dim must match table; duplicate IDs last-write-wins
     *              - HNSW insert under concurrent load may bloat; no long txn across network
     * @scaling     Linear in B; index build separate (DDL)
     * @tests       tests/data_layer/data_layer_limits.rs
     * @pgversions  16: ok | 17: ok | 18: ok
     * @docs        specs/088-data-layer/pgvector.md#data-pgvec-vectors-upsert-batch-004
     */
    /// SPEC-059: `RETURNING (xmax = 0) AS inserted` — atomic insert detection.
    async fn upsert_report_created(
        &self,
        data: &[(String, Vec<f32>, serde_json::Value)],
    ) -> Result<Vec<String>> {
        let _timer =
            crate::TimedStorageOp::start_dataop(crate::dataop::DATA_PGVEC_VECTORS_UPSERT_BATCH_004);
        if data.is_empty() {
            return Ok(Vec::new());
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
        // SPEC-047 P3: tunable via EDGEQUAKE_VECTOR_UPSERT_CHUNK (default 1000).
        let chunk_size = crate::vector_upsert_chunk_size();

        let emb_type = self.embedding_pg_type();
        // SPEC-058: populate writable content_tsv from metadata content or KV
        // (content_ref SSOT). Entity/rel rows get empty tsvector (fine — FTS
        // filters by vector_type=chunk).
        let join_kv = self.chunk_kv_table_exists_cached().await.unwrap_or(false);
        let content_tsv_expr = if join_kv {
            format!(
                r#"to_tsvector(
                    'english',
                    coalesce(
                        t.metadata->>'content',
                        (
                            SELECT k.value->>'content'
                            FROM {kv} k
                            WHERE k.key = coalesce(t.metadata->>'content_ref', t.id)
                            LIMIT 1
                        ),
                        ''
                    )
                )"#,
                kv = self.chunk_kv_table_name
            )
        } else {
            "to_tsvector('english', coalesce(t.metadata->>'content', ''))".to_string()
        };
        // SPEC-059: xmax=0 means freshly inserted; non-zero means ON CONFLICT update.
        let sql = format!(
            r#"
            INSERT INTO {table} (id, embedding, metadata, document_id, tenant_id, workspace_id, content_tsv)
            SELECT
                t.id,
                t.embedding::{emb_type},
                t.metadata,
                COALESCE(t.metadata->>'document_id', t.metadata->>'source_document_id'),
                t.metadata->>'tenant_id',
                t.metadata->>'workspace_id',
                {content_tsv_expr}
            FROM UNNEST($1::text[], $2::text[], $3::jsonb[]) AS t(id, embedding, metadata)
            ON CONFLICT (id) DO UPDATE SET
                embedding = EXCLUDED.embedding,
                metadata = EXCLUDED.metadata,
                document_id = EXCLUDED.document_id,
                tenant_id = EXCLUDED.tenant_id,
                workspace_id = EXCLUDED.workspace_id,
                content_tsv = EXCLUDED.content_tsv
            RETURNING id, (xmax = 0) AS inserted
            "#,
            table = self.table_name,
            emb_type = emb_type,
            content_tsv_expr = content_tsv_expr
        );

        let mut tx = pool
            .begin()
            .await
            .map_err(|e| StorageError::Database(format!("Failed to begin upsert tx: {}", e)))?;

        let mut created: Vec<String> = Vec::new();
        for chunk in kept.chunks(chunk_size) {
            let mut ids: Vec<String> = Vec::with_capacity(chunk.len());
            let mut embeddings: Vec<String> = Vec::with_capacity(chunk.len());
            let mut metadatas: Vec<serde_json::Value> = Vec::with_capacity(chunk.len());
            for &i in chunk {
                let (id, embedding, metadata) = &data[i];
                ids.push(id.clone());
                embeddings.push(Self::format_embedding(embedding));
                metadatas.push(metadata.clone());
            }

            let rows = sqlx::query(&sql)
                .bind(&ids)
                .bind(&embeddings)
                .bind(&metadatas)
                .fetch_all(&mut *tx)
                .await
                .map_err(|e| StorageError::Database(format!("Batch upsert failed: {}", e)))?;
            for row in rows {
                let id: String = row.get("id");
                let inserted: bool = row.get("inserted");
                if inserted {
                    created.push(id);
                }
            }
        }

        tx.commit()
            .await
            .map_err(|e| StorageError::Database(format!("Failed to commit upsert tx: {}", e)))?;

        Ok(created)
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

    async fn delete_entities_batch(&self, entity_names: &[String]) -> Result<usize> {
        if entity_names.is_empty() {
            return Ok(0);
        }
        let mut unique = entity_names.to_vec();
        unique.sort();
        unique.dedup();
        let pool = self.pool.get().await?;
        let sql = format!(
            "DELETE FROM {} WHERE metadata->>'entity_name' = ANY($1::text[])",
            self.table_name
        );
        sqlx::query(&sql)
            .bind(&unique)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("Batch delete entities failed: {e}")))?;
        Ok(unique.len())
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

    /// SPEC-047 P1a: wipe all vectors for a document on force_reindex / re-ingest.
    ///
    /// WHY both column and JSONB: dual-write era rows may only have one side set.
    /// WHY id prefix: legacy chunk rows keyed `{doc}-chunk-N` without metadata.
    async fn delete_by_document(&self, document_id: &str) -> Result<usize> {
        if document_id.is_empty() {
            return Ok(0);
        }
        let pool = self.pool.get().await?;
        let chunk_prefix = format!("{document_id}-chunk-%");
        let sql = format!(
            r#"
            DELETE FROM {}
            WHERE document_id = $1
               OR metadata->>'document_id' = $1
               OR metadata->>'source_document_id' = $1
               OR id LIKE $2
            "#,
            self.table_name
        );
        let result = sqlx::query(&sql)
            .bind(document_id)
            .bind(&chunk_prefix)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("Delete by document failed: {}", e)))?;
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
    /**
     * @dataop      DATA-PGVEC-VECTORS-ANN-QUERY-FILTERED-002
     * @engine      pgvector 0.8.x (secondary: postgres)
     * @intent      Tenant/workspace/document-scoped ANN with iterative_scan for post-filter recall.
     * @tables      eq_{ns}_vectors(embedding, tenant_id, workspace_id, document_id, metadata)
     * @indexes     HNSW/IVF + btree tenant/ws/doc; optional partial HNSW by workspace
     * @complexity  time: O(ef_search * log N) + iterative re-scan; space: O(K + max_scan_tuples)
     * @limits      - iterative_scan=relaxed_order; hnsw.max_scan_tuples=20000 default
     *              - Filter selectivity <<1 ⇒ over-fetch / recall risk without iterative_scan
     *              - Hard fail if ANN index missing (fail-closed DDL policy)
     * @scaling     Verified in e2e_spec061 / Q1 filtered paths
     * @tests       tests/data_layer/data_layer_limits.rs
     * @pgversions  16: ok | 17: ok | 18: ok
     * @docs        specs/088-data-layer/pgvector.md#data-pgvec-vectors-ann-query-filtered-002
     */
    async fn query_filtered(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        filter_ids: Option<&[String]>,
        metadata_filter: Option<&MetadataFilter>,
    ) -> Result<Vec<VectorSearchResult>> {
        // SPEC-060: storage op histogram (op label only)
        let _timed = crate::TimedStorageOp::start_dataop(
            crate::dataop::DATA_PGVEC_VECTORS_ANN_QUERY_FILTERED_002,
        );
        // Fast path: if no metadata filter, delegate to standard query
        let mf = match metadata_filter {
            Some(mf) if !mf.is_empty() => mf,
            _ => return self.query(query_embedding, top_k, filter_ids).await,
        };

        // SPEC-065: productize Wave-2 — create hot-workspace partial HNSW when opt-in.
        // Fail-closed: DDL errors propagate (no silent seq-scan).
        let mut wave2_partial_ready = false;
        // SPEC-080: count before begin() — avoids second pool acquire while holding a tx.
        let mut workspace_row_count: Option<u64> = None;
        if let Some(ws) = mf.workspace_id.as_deref() {
            workspace_row_count = Some(self.count_workspace_rows(ws).await?);
            if crate::hnsw_partial_by_workspace_enabled() {
                let _created = self.ensure_hot_workspace_ann(ws).await?;
                wave2_partial_ready = self.partial_ann_index_exists(ws).await?;
            }
        }

        let pool = self.pool.get().await?;
        let embedding_str = Self::format_embedding(query_embedding);
        let emb_type = self.embedding_pg_type();

        let has_id_filter = filter_ids.map(|ids| !ids.is_empty()).unwrap_or(false);
        let filter_sql = mf.build_sql(has_id_filter, 2);
        let where_clause = if filter_sql.conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", filter_sql.conditions.join(" AND "))
        };

        // SPEC-076 A3: opt-in ANN → exact reorder (default OFF).
        let reorder = AnnExactReorderPolicy::from_env();
        let tune_k = reorder.effective_candidate_k(top_k);
        let sql = crate::dataop::sql_comment(
            crate::dataop::DATA_PGVEC_VECTORS_ANN_QUERY_FILTERED_002,
            &build_ann_select_sql(
                &self.table_name,
                emb_type,
                &where_clause,
                filter_sql.next_param,
                top_k,
                &reorder,
            ),
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

        if let Some(modalities) = &mf.modalities {
            let mods: Vec<String> = modalities.clone();
            args.add(&mods)
                .map_err(|e| StorageError::Database(format!("Failed to bind modalities: {}", e)))?;
        }

        args.add(top_k as i32)
            .map_err(|e| StorageError::Database(format!("Failed to bind top_k: {}", e)))?;

        // Resolve capability BEFORE begin(): supports_iterative_scan may acquire a
        // second pool connection (OnceCell init). Doing that while holding a tx
        // deadlocks when pool is saturated (clients ≥ max_connections).
        let iterative_scan = self.supports_iterative_scan().await;

        // QW3: metadata pre-filter present -> raise recall AND enable iterative
        // scan (scoped to this transaction) so the post-filter LIMIT is met.
        let mut tx = pool.begin().await.map_err(|e| {
            StorageError::Database(format!("Failed to begin filtered query tx: {}", e))
        })?;

        for stmt in Self::search_tuning_statements(
            self.effective_index_type(),
            tune_k,
            true,
            iterative_scan,
        ) {
            sqlx::query(&stmt)
                .execute(&mut *tx)
                .await
                .map_err(|e| StorageError::Database(format!("Failed to set search GUC: {}", e)))?;
        }

        // SPEC-067: prefer partial/HNSW over filter+Sort when Wave-2 columns-only applies.
        // SPEC-080 B3: skip bias on tiny workspace slices (let planner choose exact).
        let prefer_columns = crate::filter_column_policy::prefer_denorm_filter_columns();
        for stmt in Self::wave2_planner_bias_statements(
            prefer_columns,
            wave2_partial_ready,
            mf,
            workspace_row_count,
        ) {
            sqlx::query(&stmt).execute(&mut *tx).await.map_err(|e| {
                StorageError::Database(format!("Failed to set Wave-2 planner bias: {e}"))
            })?;
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

    fn supports_native_text_search(&self) -> bool {
        true
    }

    async fn text_search_filtered(
        &self,
        query_text: &str,
        top_k: usize,
        filter_ids: Option<&[String]>,
        metadata_filter: Option<&MetadataFilter>,
    ) -> Result<Vec<VectorSearchResult>> {
        // SPEC-060: storage op histogram (op label only)
        let _timed = crate::TimedStorageOp::start("text_search_filtered");
        self.postgres_text_search_filtered(query_text, top_k, filter_ids, metadata_filter)
            .await
    }

    async fn warmup_workspace_ann(&self, workspace_id: &str) -> Result<bool> {
        // Inherent method on PgVectorStorage (ddl.rs) — avoid trait recursion.
        PgVectorStorage::ensure_hot_workspace_ann(self, workspace_id).await
    }
}
