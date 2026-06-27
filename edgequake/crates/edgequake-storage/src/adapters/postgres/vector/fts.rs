//! PostgreSQL native FTS for chunk sparse retrieval (SPEC-023 I10).
//!
//! Uses GIN-indexed `content_tsv` + `ts_rank_cd` (BM25-like ranking) instead of
//! re-scoring vector candidates in application memory. Chunk text SSOT is the
//! shared default KV table (SPEC-024 2.5); workspace vector tables only hold embeddings.

use sqlx::Row;

use super::PgVectorStorage;
use crate::adapters::postgres::schema;
use crate::error::{Result, StorageError};
use crate::traits::{MetadataFilter, VectorSearchResult};

const FTS_CONTENT_WITH_KV: &str = "coalesce(v.content_tsv, to_tsvector('english', coalesce(v.metadata->>'content', k.value->>'content', '')))";
const FTS_CONTENT_METADATA_ONLY: &str =
    "coalesce(v.content_tsv, to_tsvector('english', coalesce(v.metadata->>'content', '')))";

impl PgVectorStorage {
    async fn chunk_kv_table_exists_cached(&self) -> Result<bool> {
        if let Some(exists) = self.chunk_kv_table_exists.get() {
            return Ok(*exists);
        }

        let pool = self.pool.get().await?;
        let exists = schema::relation_exists(&pool, &self.chunk_kv_table_name).await?;
        let _ = self.chunk_kv_table_exists.set(exists);
        Ok(exists)
    }

    fn fts_content_expr(join_kv: bool) -> &'static str {
        if join_kv {
            FTS_CONTENT_WITH_KV
        } else {
            FTS_CONTENT_METADATA_ONLY
        }
    }

    /// Full-text search with `ts_rank_cd` over chunk content (native Postgres BM25-like ranking).
    pub(crate) async fn postgres_text_search_filtered(
        &self,
        query_text: &str,
        top_k: usize,
        filter_ids: Option<&[String]>,
        metadata_filter: Option<&MetadataFilter>,
    ) -> Result<Vec<VectorSearchResult>> {
        if query_text.trim().is_empty() || top_k == 0 {
            return Ok(Vec::new());
        }

        let pool = self.pool.get().await?;
        let join_kv = self.chunk_kv_table_exists_cached().await?;
        let content_expr = Self::fts_content_expr(join_kv);
        let mf = metadata_filter.cloned().unwrap_or_default();
        let has_id_filter = filter_ids.map(|ids| !ids.is_empty()).unwrap_or(false);
        let filter_sql = mf.build_sql_with_alias(has_id_filter, 2, Some("v"));

        let mut conditions = vec![format!(
            "{content_expr} @@ websearch_to_tsquery('english', $1)"
        )];
        conditions.extend(filter_sql.conditions);

        let where_clause = format!("WHERE {}", conditions.join(" AND "));

        let kv_join = if join_kv {
            format!("LEFT JOIN {} k ON k.key = v.id", self.chunk_kv_table_name)
        } else {
            String::new()
        };

        let sql = format!(
            r#"
            SELECT v.id, v.metadata,
                   ts_rank_cd(
                       {content_expr},
                       websearch_to_tsquery('english', $1)
                   )::float4 AS score
            FROM {vectors} v
            {kv_join}
            {where_clause}
            ORDER BY score DESC
            LIMIT ${limit_param}
            "#,
            content_expr = content_expr,
            vectors = self.table_name,
            kv_join = kv_join,
            where_clause = where_clause,
            limit_param = filter_sql.next_param
        );

        use sqlx::postgres::PgArguments;
        use sqlx::Arguments;

        let mut args = PgArguments::default();
        args.add(query_text)
            .map_err(|e| StorageError::Database(format!("Failed to bind FTS query text: {}", e)))?;

        if let Some(ids) = filter_ids {
            if !ids.is_empty() {
                let id_vec: Vec<String> = ids.to_vec();
                args.add(&id_vec).map_err(|e| {
                    StorageError::Database(format!("Failed to bind filter_ids: {}", e))
                })?;
            }
        }

        if let Some(doc_ids) = &mf.document_ids {
            args.add(&doc_ids.clone()).map_err(|e| {
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

        let rows = sqlx::query_with(&sql, args)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("Postgres FTS query failed: {}", e)))?;

        Ok(rows
            .iter()
            .map(|row| VectorSearchResult {
                id: row.get("id"),
                score: row.get::<f32, _>("score"),
                metadata: row.get("metadata"),
            })
            .collect())
    }
}
