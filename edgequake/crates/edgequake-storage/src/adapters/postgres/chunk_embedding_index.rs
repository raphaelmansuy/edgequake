//! SPEC-091 W3 — typed chunk embeddings Postgres adapter (`EmbeddingIndex` port).
//!
//! Cosine ANN over `chunk_embeddings` (migration 108). Batch-first (LAW-D7):
//! single `unnest` round trips for model upsert + embedding upsert. Idempotent
//! via `ON CONFLICT (model_id, chunk_id) DO NOTHING` (LD-05).

use async_trait::async_trait;
use sqlx::{PgConnection, PgPool, Row};
use uuid::Uuid;

use crate::adapters::postgres::typed_embedding_dims::{
    validate_ann_dimensions, validate_typed_embedding_batch_dims,
};
use crate::error::StorageError;
use crate::traits::domain::{
    EmbeddingCapabilities, EmbeddingIndex, EmbeddingRow, ModelId, ScoredChunk, UpsertReport,
    VectorQuery, WorkspaceId,
};

/// Postgres adapter serving typed chunk embeddings from `chunk_embeddings`.
///
/// Model registry is deduped by `(name, dimensions)`; every upsert resolves the
/// `embedding_models.id` for the supplied logical model first.
pub struct PgChunkEmbeddingIndex {
    pool: PgPool,
    search_session: super::typed_ann_search::TypedAnnSearch,
    /// Logical model name (e.g. `text-embedding-3-small`) mapped into
    /// `embedding_models` at upsert/search time.
    model_name: String,
}

impl PgChunkEmbeddingIndex {
    pub fn new(pool: PgPool, model_name: impl Into<String>) -> Self {
        Self {
            pool,
            search_session: Default::default(),
            model_name: model_name.into(),
        }
    }

    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    /// Resolve (and create if missing) the `embedding_models.id` for this
    /// adapter's `(name, dimensions)`. `dimensions` is taken from the first
    /// batch row; search uses the model already registered under this name.
    async fn resolve_model_id(&self, dimensions: i32) -> Result<ModelId, StorageError> {
        let id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO embedding_models (name, dimensions)
            VALUES ($1, $2)
            ON CONFLICT (name, dimensions) DO UPDATE SET name = EXCLUDED.name
            RETURNING id
            "#,
        )
        .bind(&self.model_name)
        .bind(dimensions)
        .fetch_one(&self.pool)
        .await
        .map_err(StorageError::from)?;
        Ok(ModelId(id))
    }

    /// Look up the `embedding_models.id` for a `(name, dimensions)` pair
    /// without creating it (search must not mutate the registry).
    async fn find_model_id(
        &self,
        conn: &mut PgConnection,
        name: &str,
        dimensions: i32,
    ) -> Result<Option<ModelId>, StorageError> {
        let id: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM embedding_models WHERE name = $1 AND dimensions = $2",
        )
        .bind(name)
        .bind(dimensions)
        .fetch_optional(conn)
        .await
        .map_err(StorageError::from)?;
        Ok(id.map(ModelId))
    }

    /// `$4 IS NULL OR document_id = ANY($4)` hides the document from a generic
    /// plan, so the scan reads every chunk and then keeps one document.
    /// When the caller passes documents, the statement requires the predicate
    /// and `idx_chunks_document` starts at those rows.
    const DOCUMENT_PREDICATE_OPTIONAL: &'static str =
        "AND ($4::uuid[] IS NULL OR c.document_id = ANY($4))";
    const DOCUMENT_PREDICATE_REQUIRED: &'static str = "AND c.document_id = ANY($4)";

    /// `$6 IS NULL OR modality = ANY($6)` hides the modality from a generic
    /// plan. When the caller passes modalities, the statement requires the
    /// predicate and `idx_chunks_modality` starts at those rows.
    const MODALITY_PREDICATE_OPTIONAL: &'static str =
        "AND ($6::text[] IS NULL OR c.modality = ANY($6))";
    const MODALITY_PREDICATE_REQUIRED: &'static str = "AND c.modality = ANY($6)";

    fn apply_selective_predicates(sql: String, documents: bool, modality: bool) -> String {
        let sql = if documents {
            sql.replace(
                Self::DOCUMENT_PREDICATE_OPTIONAL,
                Self::DOCUMENT_PREDICATE_REQUIRED,
            )
        } else {
            sql
        };
        if modality {
            sql.replace(
                Self::MODALITY_PREDICATE_OPTIONAL,
                Self::MODALITY_PREDICATE_REQUIRED,
            )
        } else {
            sql
        }
    }

    /// Unscoped chunk ANN. Workspace and tenant stay optional: requiring
    /// either one makes a generic plan read every embedding in a large scope
    /// instead of stopping on the HNSW index. Search still replans with the
    /// actual binds. Document, modality, and id each have their own statement
    /// when the caller passes them.
    fn search_sql(dim: i32, cast: &str, documents: bool, modality: bool) -> String {
        let sql = format!(
            r#"
            SELECT ce.chunk_id,
                   1.0 - ((ce.embedding::{cast}) <=> $1::{cast}) AS score
            FROM chunk_embeddings ce
            JOIN chunks c ON c.id = ce.chunk_id
            WHERE ce.model_id = $2
              AND ce.dimensions = {dim}
              AND ($3::uuid IS NULL OR ce.workspace_id = $3)
              AND ($4::uuid[] IS NULL OR c.document_id = ANY($4))
              AND ($5::uuid IS NULL OR c.tenant_id = $5)
              AND ($6::text[] IS NULL OR c.modality = ANY($6))
              AND ($7::text IS NULL OR lower($7) = 'chunk')
            ORDER BY (ce.embedding::{cast}) <=> $1::{cast}
            LIMIT $8
            "#
        );
        Self::apply_selective_predicates(sql, documents, modality)
    }

    /// Id-filtered chunk ANN. `$7 IS NULL OR id::text = ANY($7)` cannot use
    /// `chunks_pkey`, so the plan seq-scans every chunk and keeps the
    /// requested id. This statement looks each id up first. `OFFSET 0` keeps
    /// that lookup from being pulled into the distance index. Legacy keys
    /// compare the stored `legacy_chunk_key` column (migration 203). Equality
    /// on that text column is leakproof, so forced row security can use
    /// `idx_chunks_tenant_legacy_key`. `metadata->>'legacy_chunk_key'` is not
    /// leakproof and makes each key re-read the tenant's chunks. Values that
    /// parse as UUIDs use `chunks_pkey`.
    fn search_sql_in_ids(dim: i32, cast: &str, documents: bool, modality: bool) -> String {
        let sql = format!(
            r#"
            SELECT ce.chunk_id,
                   1.0 - ((ce.embedding::{cast}) <=> $1::{cast}) AS score
            FROM (
                SELECT c.id
                FROM unnest($7::text[]) AS wanted(key)
                JOIN LATERAL (
                    SELECT c.id
                    FROM chunks c
                    WHERE c.legacy_chunk_key = wanted.key
                    OFFSET 0
                ) c ON true
                UNION
                SELECT c.id
                FROM unnest($8::uuid[]) AS wanted(id)
                JOIN LATERAL (
                    SELECT c.id
                    FROM chunks c
                    WHERE c.id = wanted.id
                    OFFSET 0
                ) c ON true
            ) hit
            JOIN chunk_embeddings ce
              ON ce.chunk_id = hit.id
             AND ce.model_id = $2
             AND ce.dimensions = {dim}
            JOIN chunks c ON c.id = hit.id
            WHERE ($3::uuid IS NULL OR ce.workspace_id = $3)
              AND ($4::uuid[] IS NULL OR c.document_id = ANY($4))
              AND ($5::uuid IS NULL OR c.tenant_id = $5)
              AND ($6::text[] IS NULL OR c.modality = ANY($6))
              AND ($9::text IS NULL OR lower($9) = 'chunk')
            ORDER BY (ce.embedding::{cast}) <=> $1::{cast}
            LIMIT $10
            "#
        );
        Self::apply_selective_predicates(sql, documents, modality)
    }
}

#[async_trait]
impl EmbeddingIndex for PgChunkEmbeddingIndex {
    fn capabilities(&self) -> EmbeddingCapabilities {
        EmbeddingCapabilities {
            metric: "cosine",
            supports_filters: true,
            supports_rerank: false,
        }
    }

    async fn upsert_batch(
        &self,
        _model: ModelId,
        rows: &[EmbeddingRow],
    ) -> Result<UpsertReport, StorageError> {
        if rows.is_empty() {
            return Ok(UpsertReport::default());
        }
        let dimensions = validate_typed_embedding_batch_dims(
            rows.iter().map(|r| r.dimensions),
            rows.iter().map(|r| r.embedding.len()),
        )?;
        let model_id = self.resolve_model_id(dimensions).await?;

        let chunk_ids: Vec<Uuid> = rows.iter().map(|r| r.chunk_id.0).collect();
        let workspace_ids: Vec<Uuid> = rows.iter().map(|r| r.workspace_id.into_uuid()).collect();
        let dims: Vec<i32> = rows.iter().map(|r| r.dimensions).collect();
        // Unconstrained halfvec (mig 132); text cast keeps sqlx simple.
        let vectors: Vec<String> = rows
            .iter()
            .map(|r| {
                let mut s = String::with_capacity(r.embedding.len() * 8 + 2);
                s.push('[');
                for (i, v) in r.embedding.iter().enumerate() {
                    if i > 0 {
                        s.push(',');
                    }
                    s.push_str(&v.to_string());
                }
                s.push(']');
                s
            })
            .collect();

        let upserted: i64 = sqlx::query(
            r#"
            INSERT INTO chunk_embeddings (model_id, chunk_id, workspace_id, embedding, dimensions)
            SELECT $1, c, w, v::halfvec, d
            FROM unnest($2::uuid[], $3::uuid[], $4::text[], $5::int[]) AS t(c, w, v, d)
            ON CONFLICT (model_id, chunk_id) DO NOTHING
            "#,
        )
        .bind(model_id.0)
        .bind(&chunk_ids)
        .bind(&workspace_ids)
        .bind(&vectors)
        .bind(&dims)
        .execute(&self.pool)
        .await
        .map_err(StorageError::from)?
        .rows_affected() as i64;

        Ok(UpsertReport {
            upserted: upserted as u64,
            ..Default::default()
        })
    }

    async fn search(&self, req: &VectorQuery) -> Result<Vec<ScoredChunk>, StorageError> {
        if req.limit == 0
            || req.document_ids.as_ref().is_some_and(Vec::is_empty)
            || req.modalities.as_ref().is_some_and(Vec::is_empty)
            || req.filter_ids.as_ref().is_some_and(Vec::is_empty)
            || req
                .vector_type
                .as_deref()
                .is_some_and(|kind| !kind.eq_ignore_ascii_case("chunk"))
        {
            return Ok(Vec::new());
        }

        let dim = validate_ann_dimensions(req.embedding.len() as i32)?;
        let candidate_limit =
            super::typed_ann_search::TypedAnnSearch::candidate_limit(req.limit as usize);
        let mut conn = self.pool.acquire().await.map_err(StorageError::from)?;
        let mut tx = self
            .search_session
            .begin(
                &mut conn,
                candidate_limit,
                req.tenant_id.map(|id| id.into_uuid()),
                req.workspace_id.map(|id| id.into_uuid()),
            )
            .await?;
        let model_id = match self
            .find_model_id(tx.as_mut(), &self.model_name, dim)
            .await?
        {
            Some(id) => id,
            // Registry miss for the requested dimension → authoritative empty set
            // (mirrors legacy "no table" behavior), not an error.
            None => {
                tx.commit().await?;
                return Ok(Vec::new());
            }
        };

        // Dim-scoped expression HNSW (mig 132) requires matching cast + dimensions filter.
        let cast = format!("halfvec({dim})");
        let vector = {
            let mut s = String::with_capacity(req.embedding.len() * 8 + 2);
            s.push('[');
            for (i, v) in req.embedding.iter().enumerate() {
                if i > 0 {
                    s.push(',');
                }
                s.push_str(&v.to_string());
            }
            s.push(']');
            s
        };

        let documents = req.document_ids.as_ref().is_some();
        let modality = req.modalities.as_ref().is_some();
        let in_ids = req.filter_ids.as_ref().is_some();
        let uuid_ids: Vec<Uuid> = req
            .filter_ids
            .iter()
            .flatten()
            .filter_map(|id| Uuid::parse_str(id).ok())
            .collect();
        let inner = if in_ids {
            Self::search_sql_in_ids(dim, &cast, documents, modality)
        } else {
            Self::search_sql(dim, &cast, documents, modality)
        };
        let q = super::typed_ann_search::TypedAnnSearch::ordered_sql(&inner, req.limit);
        let query = sqlx::query(&q)
            .bind(&vector)
            .bind(model_id.0)
            .bind(req.workspace_id.map(|id| id.into_uuid()))
            .bind(req.document_ids.as_deref())
            .bind(req.tenant_id.map(|id| id.into_uuid()))
            .bind(req.modalities.as_deref());
        let query = if in_ids {
            query
                .bind(req.filter_ids.as_deref())
                .bind(&uuid_ids)
                .bind(req.vector_type.as_deref())
                .bind(candidate_limit as i64)
        } else {
            query
                .bind(req.vector_type.as_deref())
                .bind(candidate_limit as i64)
        };
        let rows = query
            .fetch_all(tx.as_mut())
            .await
            .map_err(StorageError::from)?;
        tx.commit().await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let chunk_id: Uuid = row.try_get("chunk_id").map_err(StorageError::from)?;
            let score: f64 = row.try_get("score").map_err(StorageError::from)?;
            out.push(ScoredChunk {
                chunk_id: chunk_id.into(),
                score: score as f32,
            });
        }
        Ok(out)
    }

    async fn delete_for_workspace(&self, workspace: WorkspaceId) -> Result<u64, StorageError> {
        let deleted = sqlx::query("DELETE FROM chunk_embeddings WHERE workspace_id = $1")
            .bind(workspace.into_uuid())
            .execute(&self.pool)
            .await
            .map_err(StorageError::from)?
            .rows_affected();
        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::PgChunkEmbeddingIndex;

    #[test]
    fn typed_search_requires_a_passed_document_or_modality() {
        let open = PgChunkEmbeddingIndex::search_sql(1536, "halfvec(1536)", false, false);
        assert!(open.contains("$4::uuid[] IS NULL OR c.document_id = ANY($4)"));
        assert!(open.contains("$6::text[] IS NULL OR c.modality = ANY($6)"));
        assert!(open.contains("$3::uuid IS NULL OR ce.workspace_id = $3"));
        assert!(open.contains("$5::uuid IS NULL OR c.tenant_id = $5"));
        assert!(!open.contains("id::text = ANY"));

        let documents = PgChunkEmbeddingIndex::search_sql(1536, "halfvec(1536)", true, false);
        assert!(documents.contains("AND c.document_id = ANY($4)"));
        assert!(!documents.contains("$4::uuid[] IS NULL"));
        assert!(documents.contains("$5::uuid IS NULL OR c.tenant_id = $5"));

        let modality = PgChunkEmbeddingIndex::search_sql(1536, "halfvec(1536)", false, true);
        assert!(modality.contains("AND c.modality = ANY($6)"));
        assert!(!modality.contains("$6::text[] IS NULL"));
    }

    #[test]
    fn typed_search_by_id_looks_up_each_id() {
        let sql = PgChunkEmbeddingIndex::search_sql_in_ids(1536, "halfvec(1536)", false, false);
        assert!(sql.contains("OFFSET 0"));
        assert!(sql.contains("c.legacy_chunk_key = wanted.key"));
        assert!(!sql.contains("metadata->>'legacy_chunk_key'"));
        assert!(sql.contains("c.id = wanted.id"));
        assert!(!sql.contains("$7::text[] IS NULL"));
        assert!(!sql.contains("id::text = ANY"));
        assert!(sql.contains("LIMIT $10"));

        let scoped = PgChunkEmbeddingIndex::search_sql_in_ids(1536, "halfvec(1536)", true, true);
        assert!(scoped.contains("AND c.document_id = ANY($4)"));
        assert!(!scoped.contains("$4::uuid[] IS NULL"));
        assert!(scoped.contains("AND c.modality = ANY($6)"));
        assert!(!scoped.contains("$6::text[] IS NULL"));
        assert!(scoped.contains("OFFSET 0"));
    }
}
