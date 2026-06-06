//! Deletion impact analysis handler.
//!
//! Read-only preview of what a document deletion would affect (entities,
//! relationships, chunks) without performing the actual delete.

use axum::{extract::State, Json};

use crate::error::{ApiError, ApiResult};
use crate::handlers::documents_types::*;
use crate::services::{analyze_deletion_impact_stats, DocumentSourceScope};
use crate::state::AppState;

/// Analyze the impact of deleting a document before actually deleting it.
///
/// This endpoint allows users to preview what would be affected by a document deletion
/// without actually performing the deletion. This is useful for understanding the
/// cascade effects before committing to a destructive operation.
#[utoipa::path(
    get,
    path = "/api/v1/documents/{document_id}/deletion-impact",
    tag = "Documents",
    params(
        ("document_id" = String, Path, description = "Document ID to analyze")
    ),
    responses(
        (status = 200, description = "Deletion impact analysis", body = DeletionImpactResponse),
        (status = 404, description = "Document not found")
    )
)]
pub async fn analyze_deletion_impact(
    State(state): State<AppState>,
    axum::extract::Path(document_id): axum::extract::Path<String>,
) -> ApiResult<Json<DeletionImpactResponse>> {
    let chunk_prefix = format!("{}-chunk-", document_id);
    let chunk_ids = state
        .storage
        .kv_storage
        .keys_with_prefix(&chunk_prefix)
        .await?;

    let metadata_key = format!("{}-metadata", document_id);
    let content_key = format!("{}-content", document_id);
    let has_metadata = state
        .storage
        .kv_storage
        .get_by_id(&metadata_key)
        .await?
        .is_some();
    let has_content = state
        .storage
        .kv_storage
        .get_by_id(&content_key)
        .await?
        .is_some();

    // Document must have either chunks, metadata, or content
    if chunk_ids.is_empty() && !has_metadata && !has_content {
        return Err(ApiError::NotFound(format!(
            "Document {} not found",
            document_id
        )));
    }

    let chunks_to_delete = chunk_ids.len();

    // SPEC-006 P1: bounded impact analysis (document-scoped, no full graph scan)
    let scope = DocumentSourceScope::from_document_id(document_id.clone());
    let impact = analyze_deletion_impact_stats(&state.storage.graph_storage, None, &scope).await?;
    let entities_to_remove = impact.entities_removed;
    let entities_to_update = impact.entities_updated;
    let relationships_to_remove = impact.relationships_removed;
    let relationships_to_update = impact.relationships_updated;

    Ok(Json(DeletionImpactResponse {
        document_id,
        chunks_to_delete,
        entities_to_remove,
        entities_to_update,
        relationships_to_remove,
        relationships_to_update,
        preview_only: true,
    }))
}
