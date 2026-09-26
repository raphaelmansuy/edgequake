//! SPEC-091 W3 dual-read: typed `chunk_embeddings` shadow for chunk queries.
//!
//! When `EDGEQUAKE_VECTOR_BACKEND=chunk_embeddings`, chunk-family vector queries
//! (metadata filter carries a `workspace_id`) are served from the typed
//! `chunk_embeddings` table instead of the legacy `eq_*_vectors` rows.
//! Typed-path errors propagate to callers; they are never converted to an empty
//! result or a query against retired legacy tables. Entity/relationship
//! namespaces never take this path.

use std::sync::atomic::{AtomicU64, Ordering};

use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::StorageError;
use crate::traits::domain::{
    EmbeddingIndex, ModelId, ScoredChunk, TenantId, VectorQuery, WorkspaceId,
};
use crate::traits::{MetadataFilter, VectorSearchResult};

static VECTOR_BACKEND_FALLBACK_TOTAL: AtomicU64 = AtomicU64::new(0);
static VECTOR_BACKEND_TYPED_HIT_TOTAL: AtomicU64 = AtomicU64::new(0);

/// Historical typed-to-legacy fallback counter retained for rollout telemetry.
pub fn vector_backend_fallback_total() -> u64 {
    VECTOR_BACKEND_FALLBACK_TOTAL.load(Ordering::Relaxed)
}

/// Typed-path queries that served from `chunk_embeddings` successfully.
pub fn vector_backend_typed_hit_total() -> u64 {
    VECTOR_BACKEND_TYPED_HIT_TOTAL.load(Ordering::Relaxed)
}

/// Record one typed→legacy fallback from an explicit rollback path.
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
/// failure (caller must propagate it).
pub async fn try_typed_chunk_query(
    pool: &PgPool,
    index: &crate::adapters::postgres::chunk_embedding_index::PgChunkEmbeddingIndex,
    query_embedding: &[f32],
    top_k: usize,
    workspace_key: &str,
    filter_ids: Option<&[String]>,
    metadata_filter: &MetadataFilter,
) -> Result<Option<Vec<VectorSearchResult>>, StorageError> {
    let Some(ws_uuid) = resolve_workspace_uuid(pool, workspace_key).await? else {
        return Ok(None);
    };
    let Some(req) =
        build_typed_vector_query(ws_uuid, query_embedding, top_k, filter_ids, metadata_filter)
    else {
        return Ok(Some(Vec::new()));
    };
    let scored = index.search(&req).await?;
    let results = scored_to_legacy_results(pool, scored).await?;
    let results = super::super::serving_fence_query::apply_serving_fence(pool, results).await?;
    record_typed_hit();
    Ok(Some(results))
}

/// Build the typed request without weakening any caller-supplied filter.
///
/// Typed relational identifiers are UUIDs. A non-UUID tenant filter, or a
/// non-empty document filter containing no UUIDs, is therefore an
/// authoritative no-match rather than permission to omit the predicate.
fn build_typed_vector_query(
    workspace_id: Uuid,
    query_embedding: &[f32],
    top_k: usize,
    filter_ids: Option<&[String]>,
    metadata_filter: &MetadataFilter,
) -> Option<VectorQuery> {
    let document_ids = metadata_filter.document_ids.as_ref().map(|ids| {
        ids.iter()
            .filter_map(|id| Uuid::parse_str(id).ok())
            .collect::<Vec<_>>()
    });
    if metadata_filter
        .document_ids
        .as_ref()
        .is_some_and(|ids| !ids.is_empty())
        && document_ids.as_ref().is_some_and(Vec::is_empty)
    {
        return None;
    }

    let tenant_id = match metadata_filter.tenant_id.as_deref() {
        Some(id) => Some(TenantId::new(Uuid::parse_str(id).ok()?)),
        None => None,
    };

    Some(VectorQuery {
        model_id: ModelId(Uuid::nil()),
        model_revision: "legacy-current".into(),
        workspace_id: Some(WorkspaceId::new(workspace_id)),
        document_ids,
        tenant_id,
        modalities: metadata_filter.modalities.clone(),
        filter_ids: filter_ids.map(<[String]>::to_vec),
        vector_type: metadata_filter.vector_type.clone(),
        embedding: query_embedding.to_vec(),
        limit: top_k as u32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_query_preserves_document_and_id_filters() {
        let workspace = Uuid::new_v4();
        let document = Uuid::new_v4();
        let tenant = Uuid::new_v4();
        let filter_ids = vec!["legacy-chunk-7".to_string()];
        let metadata = MetadataFilter {
            document_ids: Some(vec![document.to_string()]),
            tenant_id: Some(tenant.to_string()),
            workspace_id: Some(workspace.to_string()),
            vector_type: Some("chunk".into()),
            modalities: Some(vec!["table".into()]),
        };

        let query =
            build_typed_vector_query(workspace, &[0.1, 0.2], 5, Some(&filter_ids), &metadata)
                .expect("valid typed filter");

        assert_eq!(query.document_ids, Some(vec![document]));
        assert_eq!(query.tenant_id, Some(TenantId::new(tenant)));
        assert_eq!(query.modalities, Some(vec!["table".to_string()]));
        assert_eq!(query.filter_ids, Some(filter_ids));
        assert_eq!(query.vector_type.as_deref(), Some("chunk"));
    }

    #[test]
    fn invalid_typed_scope_is_no_match() {
        let metadata = MetadataFilter {
            tenant_id: Some("not-a-uuid".into()),
            ..Default::default()
        };
        assert!(build_typed_vector_query(Uuid::new_v4(), &[0.1], 1, None, &metadata).is_none());
    }
}
