//! SPEC-091 W3 — typed chunk embeddings Postgres adapter (`EmbeddingIndex` port).
//!
//! Cosine ANN over `chunk_embeddings` (migration 108). Batch-first (LAW-D7):
//! single `unnest` round trips for model upsert + embedding upsert. Idempotent
//! via `ON CONFLICT (model_id, chunk_id) DO NOTHING` (LD-05).

use async_trait::async_trait;
use sqlx::{PgPool, Row};
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
    /// Logical model name (e.g. `text-embedding-3-small`) mapped into
    /// `embedding_models` at upsert/search time.
    model_name: String,
}

impl PgChunkEmbeddingIndex {
    pub fn new(pool: PgPool, model_name: impl Into<String>) -> Self {
        Self {
            pool,
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
        name: &str,
        dimensions: i32,
    ) -> Result<Option<ModelId>, StorageError> {
        let id: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM embedding_models WHERE name = $1 AND dimensions = $2",
        )
        .bind(name)
        .bind(dimensions)
        .fetch_optional(&self.pool)
        .await
        .map_err(StorageError::from)?;
        Ok(id.map(ModelId))
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
        let workspace_ids: Vec<Uuid> = rows.iter().map(|r| r.workspace_id.0).collect();
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
        let dim = validate_ann_dimensions(req.embedding.len() as i32)?;
        let model_id = match self.find_model_id(&self.model_name, dim).await? {
            Some(id) => id,
            // Registry miss for the requested dimension → authoritative empty set
            // (mirrors legacy "no table" behavior), not an error.
            None => return Ok(Vec::new()),
        };

        // SPEC-146: empty allow-set is fail-closed (never all-pass).
        if let Some(allow) = req.allowed_document_ids.as_ref() {
            if allow.is_empty() {
                return Ok(Vec::new());
            }
        }

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

        let want = req.limit as usize;
        let fetch_limit = if req.allowed_document_ids.is_some() {
            super::ann_abac::abac_overfetch_limit(req.limit) as i64
        } else {
            req.limit as i64
        };

        // Filtered ANN: SET LOCAL iterative_scan inside a short TX (LAW-146-21).
        let use_tx = req.allowed_document_ids.is_some();
        let mut tx = if use_tx {
            let mut t = self.pool.begin().await.map_err(StorageError::from)?;
            let _ = sqlx::query("SET LOCAL hnsw.iterative_scan = 'relaxed_order'")
                .execute(&mut *t)
                .await;
            Some(t)
        } else {
            None
        };

        let rows = if let Some(ws) = req.workspace_id {
            if let Some(allow) = req.allowed_document_ids.as_ref() {
                let allow_pred = super::ann_abac::document_id_allow_sql(
                    "c.document_id",
                    4,
                    allow.len(),
                );
                let q = format!(
                    r#"
                    SELECT ce.chunk_id,
                           1.0 - ((ce.embedding::{cast}) <=> $1::{cast}) AS score
                    FROM chunk_embeddings ce
                    JOIN chunks c ON c.id = ce.chunk_id
                    WHERE ce.model_id = $2
                      AND ce.dimensions = {dim}
                      AND ce.workspace_id = $3
                      AND {allow_pred}
                    ORDER BY (ce.embedding::{cast}) <=> $1::{cast}
                    LIMIT $5
                    "#
                );
                let exec = sqlx::query(&q)
                    .bind(&vector)
                    .bind(model_id.0)
                    .bind(ws.0)
                    .bind(allow.as_slice())
                    .bind(fetch_limit);
                match tx.as_mut() {
                    Some(t) => exec.fetch_all(&mut **t).await.map_err(StorageError::from)?,
                    None => exec.fetch_all(&self.pool).await.map_err(StorageError::from)?,
                }
            } else {
                let q = format!(
                    r#"
                    SELECT chunk_id, 1.0 - ((embedding::{cast}) <=> $1::{cast}) AS score
                    FROM chunk_embeddings
                    WHERE model_id = $2 AND dimensions = {dim} AND workspace_id = $3
                    ORDER BY (embedding::{cast}) <=> $1::{cast} LIMIT $4
                    "#
                );
                sqlx::query(&q)
                    .bind(&vector)
                    .bind(model_id.0)
                    .bind(ws.0)
                    .bind(fetch_limit)
                    .fetch_all(&self.pool)
                    .await
                    .map_err(StorageError::from)?
            }
        } else if let Some(allow) = req.allowed_document_ids.as_ref() {
            let allow_pred = super::ann_abac::document_id_allow_sql(
                "c.document_id",
                3,
                allow.len(),
            );
            let q = format!(
                r#"
                SELECT ce.chunk_id,
                       1.0 - ((ce.embedding::{cast}) <=> $1::{cast}) AS score
                FROM chunk_embeddings ce
                JOIN chunks c ON c.id = ce.chunk_id
                WHERE ce.model_id = $2
                  AND ce.dimensions = {dim}
                  AND {allow_pred}
                ORDER BY (ce.embedding::{cast}) <=> $1::{cast}
                LIMIT $4
                "#
            );
            let exec = sqlx::query(&q)
                .bind(&vector)
                .bind(model_id.0)
                .bind(allow.as_slice())
                .bind(fetch_limit);
            match tx.as_mut() {
                Some(t) => exec.fetch_all(&mut **t).await.map_err(StorageError::from)?,
                None => exec.fetch_all(&self.pool).await.map_err(StorageError::from)?,
            }
        } else {
            let q = format!(
                r#"
                SELECT chunk_id, 1.0 - ((embedding::{cast}) <=> $1::{cast}) AS score
                FROM chunk_embeddings
                WHERE model_id = $2 AND dimensions = {dim}
                ORDER BY (embedding::{cast}) <=> $1::{cast} LIMIT $3
                "#
            );
            sqlx::query(&q)
                .bind(&vector)
                .bind(model_id.0)
                .bind(fetch_limit)
                .fetch_all(&self.pool)
                .await
                .map_err(StorageError::from)?
        };

        if let Some(t) = tx {
            t.commit().await.map_err(StorageError::from)?;
        }

        let mut out = Vec::with_capacity(rows.len().min(want));
        for row in rows.into_iter().take(want) {
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
            .bind(workspace.0)
            .execute(&self.pool)
            .await
            .map_err(StorageError::from)?
            .rows_affected();
        Ok(deleted)
    }
}
