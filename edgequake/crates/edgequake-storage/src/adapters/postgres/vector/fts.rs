//! PostgreSQL native FTS for chunk sparse retrieval (SPEC-023 I10).
//!
//! Uses GIN-indexed `content_tsv` + `ts_rank_cd` (BM25-like ranking) instead of
//! re-scoring vector candidates in application memory.

use sqlx::Row;

use super::PgVectorStorage;
use crate::error::{Result, StorageError};
use crate::traits::{MetadataFilter, VectorSearchResult};

impl PgVectorStorage {
    /// Full-text search with `ts_rank_cd` over `content_tsv` (native Postgres BM25-like ranking).
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
        let mf = metadata_filter.cloned().unwrap_or_default();
        let has_id_filter = filter_ids.map(|ids| !ids.is_empty()).unwrap_or(false);
        let filter_sql = mf.build_sql_with_alias(has_id_filter, 2, Some("v"));

        let mut conditions = vec![
            "to_tsvector('english', coalesce(v.metadata->>'content', k.value->>'content', '')) \
             @@ websearch_to_tsquery('english', $1)"
                .to_string(),
        ];
        conditions.extend(filter_sql.conditions);

        let where_clause = format!("WHERE {}", conditions.join(" AND "));

        let sql = format!(
            r#"
            SELECT v.id, v.metadata,
                   ts_rank_cd(
                       to_tsvector('english', coalesce(v.metadata->>'content', k.value->>'content', '')),
                       websearch_to_tsquery('english', $1)
                   )::float4 AS score
            FROM {vectors} v
            LEFT JOIN {kv} k ON k.key = v.id
            {where_clause}
            ORDER BY score DESC
            LIMIT ${limit_param}
            "#,
            vectors = self.table_name,
            kv = self.kv_table_name,
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
