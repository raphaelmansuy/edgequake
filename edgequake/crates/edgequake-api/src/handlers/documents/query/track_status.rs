//! Track status handler — query documents by track ID.

use axum::{extract::State, Json};

use crate::error::ApiResult;
use crate::middleware::TenantContext;
use crate::services::document_metadata_scan::load_scoped_document_metadata_for_progress;
use crate::services::tenant_guard::{
    empty_track_status, has_full_tenant_context, warn_missing_tenant_context,
};
use crate::state::{StorageRuntime, TaskRuntime};

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
    State(tasks): State<TaskRuntime>,
    tenant_ctx: TenantContext,
    axum::extract::Path(track_id): axum::extract::Path<String>,
) -> ApiResult<Json<TrackStatusResponse>> {
    if !has_full_tenant_context(&tenant_ctx) {
        warn_missing_tenant_context(&tenant_ctx, "get_track_status");
        return Ok(Json(empty_track_status(track_id)));
    }

    // SPEC-027 + SPEC-086: include staging in-flight docs (same SSOT as progress).
    let metadata_values =
        load_scoped_document_metadata_for_progress(storage.kv_storage.as_ref(), &tenant_ctx)
            .await?;

    let mut track_docs: Vec<DocumentSummary> = Vec::new();
    let mut created_times: Vec<String> = Vec::new();

    for value in metadata_values {
        if let Some(obj) = value.as_object() {
            let doc_track_id = obj.get("track_id").and_then(|v| v.as_str()).unwrap_or("");
            let client_track_id = obj
                .get("client_track_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // SPEC-084 / GH-318: match insert task id OR client batch correlation id.
            if doc_track_id == track_id || client_track_id == track_id {
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
                    display_status: None,
                    ui_phase: None,
                });
            }
        }
    }

    // SPEC-057 P4: project display_status / ui_phase SSOT for track payloads.
    crate::services::ingestion_status_mapper::enrich_document_summaries_with_cancel(
        &mut track_docs,
        &tasks.cancellation_registry,
    )
    .await;

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

    let registered_count = track_docs.len();
    // SPEC-084 / GH-318: expected batch size from KV meta (client-declared).
    let expected_count = storage
        .kv_storage
        .get_by_id(&format!("track_expected:{track_id}"))
        .await
        .ok()
        .flatten()
        .and_then(|v| v.get("expected_count").and_then(|n| n.as_u64()))
        .map(|n| n as usize);

    let no_active = status_summary.pending == 0 && status_summary.processing == 0;
    let registered_enough = expected_count
        .map(|exp| registered_count >= exp)
        .unwrap_or(true);
    let is_complete = no_active && registered_enough;

    // Build latest message
    let denom = expected_count.unwrap_or(registered_count).max(1);
    let latest_message = if !is_complete {
        Some(format!(
            "Processing {}/{} documents...",
            status_summary.completed + status_summary.failed + status_summary.partial_failure,
            denom
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
        expected_count,
        registered_count,
        latest_message,
    }))
}
