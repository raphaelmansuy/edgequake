//! Chunk-level retry and listing handlers (FEAT0408, FEAT0409).
//!
//! Provides endpoints for retrying specific failed chunks and listing
//! which chunks failed during extraction. Currently placeholder
//! implementations pending chunk-level storage.

use axum::{extract::State, Json};
use tracing::debug;

use crate::error::{ApiError, ApiResult};
use crate::handlers::documents_types::*;
use crate::state::AppState;

/// Retry failed chunks for a specific document.
///
/// @implements FEAT0408 (Chunk retry handler)
///
/// # OODA-03: Chunk-Level Retry Queue
///
/// This endpoint allows retrying specific failed chunks without reprocessing the entire document.
/// Currently returns a placeholder response; full implementation pending chunk-level storage.
///
/// ## Architecture Note
///
/// Full implementation requires:
/// 1. Storing individual chunk content in failed_chunks table
/// 2. Re-running extraction on specific chunks
/// 3. Merging results into existing graph data
///
/// This is a scaffolding endpoint to enable frontend integration while backend is developed.
#[utoipa::path(
    post,
    path = "/api/v1/documents/{document_id}/retry-chunks",
    tag = "Documents",
    params(
        ("document_id" = String, Path, description = "Document ID to retry chunks for")
    ),
    request_body = RetryChunksRequest,
    responses(
        (status = 200, description = "Chunks queued for retry", body = RetryChunksResponse),
        (status = 404, description = "Document not found"),
        (status = 501, description = "Chunk-level retry not yet implemented")
    )
)]
pub async fn retry_failed_chunks(
    State(state): State<AppState>,
    axum::extract::Path(document_id): axum::extract::Path<String>,
    Json(request): Json<RetryChunksRequest>,
) -> ApiResult<Json<RetryChunksResponse>> {
    debug!(
        "retry_failed_chunks called for document: {}, chunks: {:?}, force: {}",
        document_id, request.chunk_indices, request.force
    );

    // Verify document exists
    let metadata_key = format!("{}-metadata", document_id);
    let doc_exists = state
        .storage
        .kv_storage
        .get_by_id(&metadata_key)
        .await?
        .is_some();

    if !doc_exists {
        return Err(ApiError::NotFound(format!(
            "Document {} not found",
            document_id
        )));
    }

    // OODA-03: Placeholder implementation
    // Full implementation requires:
    // 1. Query failed_chunks table for document
    // 2. Retrieve chunk content from storage
    // 3. Re-run extraction pipeline on specific chunks
    // 4. Merge extracted entities/relationships into graph
    // 5. Update failed_chunks status

    let chunks_to_retry = if request.chunk_indices.is_empty() {
        // Would query failed_chunks table here
        vec![]
    } else {
        request.chunk_indices.clone()
    };

    tracing::info!(
        document_id = %document_id,
        chunks = ?chunks_to_retry,
        "Chunk retry requested (placeholder - full implementation pending)"
    );

    Ok(Json(RetryChunksResponse {
        document_id: document_id.clone(),
        chunks_queued: chunks_to_retry.len(),
        chunk_indices: chunks_to_retry,
        message: "Chunk-level retry is pending implementation. Use /documents/reprocess to retry the entire document.".to_string(),
        implemented: false,
    }))
}

/// List failed chunks for a document.
///
/// @implements FEAT0409
///
/// Returns information about chunks that failed during extraction,
/// allowing the user to decide which to retry.
#[utoipa::path(
    get,
    path = "/api/v1/documents/{document_id}/failed-chunks",
    tag = "Documents",
    params(
        ("document_id" = String, Path, description = "Document ID to list failed chunks for")
    ),
    responses(
        (status = 200, description = "List of failed chunks", body = ListFailedChunksResponse),
        (status = 404, description = "Document not found")
    )
)]
pub async fn list_failed_chunks(
    State(state): State<AppState>,
    axum::extract::Path(document_id): axum::extract::Path<String>,
) -> ApiResult<Json<ListFailedChunksResponse>> {
    debug!("list_failed_chunks called for document: {}", document_id);

    // Verify document exists
    let metadata_key = format!("{}-metadata", document_id);
    let metadata = state.storage.kv_storage.get_by_id(&metadata_key).await?;

    if metadata.is_none() {
        return Err(ApiError::NotFound(format!(
            "Document {} not found",
            document_id
        )));
    }

    // Get chunk count from metadata
    let chunk_count = metadata
        .as_ref()
        .and_then(|m| m.get("chunk_count"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    // OODA-03: Placeholder - would query failed_chunks table
    // For now, return empty list since we don't persist failed chunks yet
    let failed_chunks: Vec<FailedChunkInfo> = vec![];

    Ok(Json(ListFailedChunksResponse {
        document_id: document_id.clone(),
        failed_chunks,
        total_chunks: chunk_count,
        successful_chunks: chunk_count, // Placeholder - all successful if no failures recorded
    }))
}
