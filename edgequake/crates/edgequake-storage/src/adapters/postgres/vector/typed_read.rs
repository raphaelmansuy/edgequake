//! SPEC-091 W3 dual-read: typed `chunk_embeddings` shadow for chunk queries.
//!
//! When `EDGEQUAKE_VECTOR_BACKEND=chunk_embeddings`, chunk-family vector queries
//! (metadata filter carries a `workspace_id`) are served from the typed
//! `chunk_embeddings` table instead of the legacy `eq_*_vectors` rows. On any
//! typed-path error the caller falls back to the legacy path and increments
//! [`vector_backend_fallback_total`] (rollout observability, mirrors
//! `chunk_text_dual_read`). Entity/relationship namespaces never take this path.

use std::sync::atomic::{AtomicU64, Ordering};

use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::StorageError;
use crate::traits::domain::{EmbeddingIndex, ModelId, ScoredChunk, VectorQuery, WorkspaceId};
use crate::traits::VectorSearchResult;

static VECTOR_BACKEND_FALLBACK_TOTAL: AtomicU64 = AtomicU64::new(0);
static VECTOR_BACKEND_TYPED_HIT_TOTAL: AtomicU64 = AtomicU64::new(0);

/// Typed-path errors that fell back to the legacy `eq_*_vectors` query.
pub fn vector_backend_fallback_total() -> u64 {
    VECTOR_BACKEND_FALLBACK_TOTAL.load(Ordering::Relaxed)
}

/// Typed-path queries that served from `chunk_embeddings` successfully.
pub fn vector_backend_typed_hit_total() -> u64 {
    VECTOR_BACKEND_TYPED_HIT_TOTAL.load(Ordering::Relaxed)
}

/// Record one typed→legacy fallback (called by `storage_impl` on typed-path
/// error; public so e2e can exercise the counter contract directly).
pub fn record_fallback() {
    VECTOR_BACKEND_FALLBACK_TOTAL.fetch_add(1, Ordering::Relaxed);
}

fn record_typed_hit() {
    VECTOR_BACKEND_TYPED_HIT_TOTAL.fetch_add(1, Ordering::Relaxed);
}

/// Resolve the `workspaces.workspace_id` UUID for a metadata workspace key.
/// Returns `None` when the key is not a resolvable workspace (typed path is
/// workspace-scoped by construction; absence → caller uses legacy path).
async fn resolve_workspace_uuid(
    pool: &PgPool,
    workspace_key: &str,
) -> Result<Option<Uuid>, StorageError> {
    // Fast path: the key is already a UUID (matches W1/W3 writer contract).
    if let Ok(u) = Uuid::parse_str(workspace_key) {
        return Ok(Some(u));
    }
    let row = sqlx::query_scalar::<_, Uuid>(
        "SELECT workspace_id FROM workspaces WHERE name = $1 OR workspace_id::text = $1 LIMIT 1",
    )
    .bind(workspace_key)
    .fetch_optional(pool)
    .await
    .map_err(StorageError::from)?;
    Ok(row)
}

/// Convert typed scored chunks into legacy-shaped `VectorSearchResult`s.
///
/// Joins `chunks.metadata->>legacy_chunk_key` for the legacy string id so
/// downstream consumers (citation, lineage) observe an unchanged shape; falls
/// back to the chunk UUID string when no legacy key exists (defensive).
async fn scored_to_legacy_results(
    pool: &PgPool,
    scored: Vec<ScoredChunk>,
) -> Result<Vec<VectorSearchResult>, StorageError> {
    if scored.is_empty() {
        return Ok(Vec::new());
    }
    let chunk_ids: Vec<Uuid> = scored.iter().map(|s| s.chunk_id.0).collect();
    let rows = sqlx::query(
        "SELECT id, metadata->>'legacy_chunk_key' AS legacy_key, metadata FROM chunks WHERE id = ANY($1::uuid[])",
    )
    .bind(&chunk_ids)
    .fetch_all(pool)
    .await
    .map_err(StorageError::from)?;

    let mut meta_by_id: std::collections::HashMap<Uuid, (Option<String>, serde_json::Value)> =
        std::collections::HashMap::with_capacity(rows.len());
    for row in rows {
        let id: Uuid = row.try_get("id").map_err(StorageError::from)?;
        let legacy_key: Option<String> = row.try_get("legacy_key").map_err(StorageError::from)?;
        let metadata: serde_json::Value = row.try_get("metadata").map_err(StorageError::from)?;
        meta_by_id.insert(id, (legacy_key, metadata));
    }

    let mut out = Vec::with_capacity(scored.len());
    for s in scored {
        let (id, metadata) = match meta_by_id.get(&s.chunk_id.0) {
            Some((legacy, meta)) => (
                legacy.clone().unwrap_or_else(|| s.chunk_id.0.to_string()),
                meta.clone(),
            ),
            None => (s.chunk_id.0.to_string(), serde_json::json!({})),
        };
        out.push(VectorSearchResult {
            id,
            score: s.score,
            metadata,
        });
    }
    Ok(out)
}

/// Try to serve a chunk query from the typed `chunk_embeddings` table.
///
/// Returns `Ok(Some(results))` when the typed path is authoritative for this
/// query (backend flag on + workspace resolvable), `Ok(None)` when the query
/// is not workspace-scoped (legacy path should run), and `Err` on typed-path
/// failure (caller logs + increments fallback + runs legacy).
pub async fn try_typed_chunk_query(
    pool: &PgPool,
    index: &crate::adapters::postgres::chunk_embedding_index::PgChunkEmbeddingIndex,
    query_embedding: &[f32],
    top_k: usize,
    workspace_key: &str,
    allowed_document_ids: Option<&[String]>,
) -> Result<Option<Vec<VectorSearchResult>>, StorageError> {
    let Some(ws_uuid) = resolve_workspace_uuid(pool, workspace_key).await? else {
        return Ok(None);
    };
    let allowed = crate::adapters::postgres::ann_abac::parse_allow_uuids(allowed_document_ids);
    // SPEC-146: Some([]) must fail-closed (empty hits), never become unscoped.
    if matches!(allowed.as_ref(), Some(v) if v.is_empty())
        && allowed_document_ids.is_some()
    {
        record_typed_hit();
        return Ok(Some(Vec::new()));
    }
    let req = VectorQuery {
        model_id: ModelId(Uuid::nil()),
        workspace_id: Some(WorkspaceId(ws_uuid)),
        embedding: query_embedding.to_vec(),
        limit: top_k as u32,
        allowed_document_ids: allowed,
    };
    let scored = index.search(&req).await?;
    let results = scored_to_legacy_results(pool, scored).await?;
    record_typed_hit();
    Ok(Some(results))
}
