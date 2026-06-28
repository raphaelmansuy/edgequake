//! Track status handler — query documents by track ID.

use axum::{extract::State, Json};

use crate::error::ApiResult;
use crate::middleware::TenantContext;
use crate::services::document_metadata_scan::load_scoped_document_metadata;
use crate::services::tenant_guard::{
    empty_track_status, has_full_tenant_context, warn_missing_tenant_context,
};
use crate::state::StorageRuntime;

use crate::handlers::documents_types::*;

/// Get track status by track ID.
///
/// Returns all documents uploaded with a specific track_id, along with status summary.
#[utoipa::path(
    get,
    path = "/api/v1/documents/track/{track_id}",
    tag = "Documents",
    params(
        ("track_id" = String, Path, description = "Track ID for the batch")
    ),
    responses(
        (status = 200, description = "Track status retrieved", body = TrackStatusResponse),
        (status = 404, description = "Track not found")
    )
)]
pub async fn get_track_status(
    State(storage): State<StorageRuntime>,
    tenant_ctx: TenantContext,
    axum::extract::Path(track_id): axum::extract::Path<String>,
) -> ApiResult<Json<TrackStatusResponse>> {
    if !has_full_tenant_context(&tenant_ctx) {
        warn_missing_tenant_context(&tenant_ctx, "get_track_status");
        return Ok(Json(empty_track_status(track_id)));
    }

    // SPEC-027: scoped metadata scan SSOT (no global keys_like; chunk_count from metadata).
    let metadata_values =
        load_scoped_document_metadata(storage.kv_storage.as_ref(), &tenant_ctx).await?;

    let mut track_docs: Vec<DocumentSummary> = Vec::new();
    let mut created_times: Vec<String> = Vec::new();

    for value in metadata_values {
        if let Some(obj) = value.as_object() {
            let doc_track_id = obj.get("track_id").and_then(|v| v.as_str()).unwrap_or("");

            if doc_track_id == track_id {
                let id = obj
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let chunk_count = obj
                    .get("chunk_count")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize)
                    .unwrap_or(0);

                if let Some(created_at) = obj.get("created_at").and_then(|v| v.as_str()) {
                    created_times.push(created_at.to_string());
                }

                let (error_message, warning_message) =
                    crate::document_metadata::extract_notices_from_metadata(obj);

                track_docs.push(DocumentSummary {
                    id,
                    title: obj.get("title").and_then(|v| v.as_str()).map(String::from),
                    file_name: obj
                        .get("file_name")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    content_summary: obj
                        .get("content_summary")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    content_length: obj
                        .get("content_length")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize),
                    chunk_count,
                    entity_count: obj
                        .get("entity_count")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize),
                    status: obj.get("status").and_then(|v| v.as_str()).map(String::from),
                    error_message,
                    warning_message,
                    track_id: Some(track_id.clone()),
                    created_at: obj
                        .get("created_at")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    updated_at: obj
                        .get("updated_at")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    cost_usd: obj.get("cost_usd").and_then(|v| v.as_f64()),
                    input_tokens: obj
                        .get("input_tokens")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize),
                    output_tokens: obj
                        .get("output_tokens")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize),
                    total_tokens: obj
                        .get("total_tokens")
                        .and_then(|v| v.as_u64())
                        .map(|n| n as usize),
                    llm_model: obj
                        .get("llm_model")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    embedding_model: obj
                        .get("embedding_model")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    source_type: obj
                        .get("source_type")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    current_stage: obj
                        .get("current_stage")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    stage_progress: obj
                        .get("stage_progress")
                        .and_then(|v| v.as_f64())
                        .map(|n| n as f32),
                    stage_message: obj
                        .get("stage_message")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    pdf_id: obj.get("pdf_id").and_then(|v| v.as_str()).map(String::from),
                });
            }
        }
    }

    // Calculate status summary (handle empty track gracefully - documents may still be processing)
    let status_summary = StatusCounts {
        pending: track_docs
            .iter()
            .filter(|d| d.status.as_deref() == Some("pending"))
            .count(),
        processing: track_docs
            .iter()
            .filter(|d| d.status.as_deref() == Some("processing"))
            .count(),
        // SPEC-021 P-B2: only count explicit completed/indexed, NOT NULL.
        completed: track_docs
            .iter()
            .filter(|d| {
                d.status.as_deref() == Some("completed") || d.status.as_deref() == Some("indexed")
            })
            .count(),
        // FIX-5: Track partial_failure status
        partial_failure: track_docs
            .iter()
            .filter(|d| d.status.as_deref() == Some("partial_failure"))
            .count(),
        failed: track_docs
            .iter()
            .filter(|d| d.status.as_deref() == Some("failed"))
            .count(),
        cancelled: track_docs
            .iter()
            .filter(|d| d.status.as_deref() == Some("cancelled"))
            .count(),
        unknown: track_docs
            .iter()
            .filter(|d| {
                d.status.is_none()
                    || !matches!(
                        d.status.as_deref(),
                        Some(
                            "pending"
                                | "processing"
                                | "completed"
                                | "indexed"
                                | "partial_failure"
                                | "failed"
                                | "cancelled"
                        )
                    )
            })
            .count(),
    };

    // Find earliest created_at
    created_times.sort();
    let created_at = created_times.first().cloned();

    // Check if complete (no pending or processing)
    let is_complete = status_summary.pending == 0 && status_summary.processing == 0;

    // Build latest message
    let latest_message = if !is_complete {
        Some(format!(
            "Processing {}/{} documents...",
            status_summary.completed + status_summary.failed,
            track_docs.len()
        ))
    } else if status_summary.failed > 0 {
        Some(format!("Completed with {} errors", status_summary.failed))
    } else {
        Some("All documents processed successfully".to_string())
    };

    Ok(Json(TrackStatusResponse {
        track_id,
        created_at,
        documents: track_docs.clone(),
        total_count: track_docs.len(),
        status_summary,
        is_complete,
        latest_message,
    }))
}
