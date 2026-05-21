//! Deletion impact analysis handler.
//!
//! Read-only preview of what a document deletion would affect (entities,
//! relationships, chunks) without performing the actual delete.

use axum::{extract::State, Json};

use crate::error::{ApiError, ApiResult};
use crate::handlers::documents_types::*;
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
    let chunk_ids = state.kv_storage.keys_with_prefix(&chunk_prefix).await?;

    let metadata_key = format!("{}-metadata", document_id);
    let content_key = format!("{}-content", document_id);
    let has_metadata = state.kv_storage.get_by_id(&metadata_key).await?.is_some();
    let has_content = state.kv_storage.get_by_id(&content_key).await?.is_some();

    // Document must have either chunks, metadata, or content
    if chunk_ids.is_empty() && !has_metadata && !has_content {
        return Err(ApiError::NotFound(format!(
            "Document {} not found",
            document_id
        )));
    }

    let chunks_to_delete = chunk_ids.len();
    let mut entities_to_remove = 0usize;
    let mut entities_to_update = 0usize;
    let mut relationships_to_remove = 0usize;
    let mut relationships_to_update = 0usize;

    // Analyze entities (read-only)
    let all_nodes = state.graph_storage.get_all_nodes().await?;
    for node in all_nodes {
        if let Some(source_id) = node.properties.get("source_id").and_then(|v| v.as_str()) {
            let sources: Vec<&str> = source_id.split('|').collect();
            let remaining = sources
                .iter()
                .filter(|s| !s.starts_with(&chunk_prefix) && !s.starts_with(&document_id))
                .count();

            if remaining == 0 {
                entities_to_remove += 1;
            } else if remaining < sources.len() {
                entities_to_update += 1;
            }
        }
    }

    // Analyze edges (read-only)
    let all_edges = state.graph_storage.get_all_edges().await?;
    for edge in all_edges {
        if let Some(source_id) = edge.properties.get("source_id").and_then(|v| v.as_str()) {
            let sources: Vec<&str> = source_id.split('|').collect();
            let remaining = sources
                .iter()
                .filter(|s| !s.starts_with(&chunk_prefix) && !s.starts_with(&document_id))
                .count();

            if remaining == 0 {
                relationships_to_remove += 1;
            } else if remaining < sources.len() {
                relationships_to_update += 1;
            }
        }
    }

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
