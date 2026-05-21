//! Track status handler — query documents by track ID.

use axum::{extract::State, Json};

use crate::error::ApiResult;
use crate::state::AppState;

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
    State(state): State<AppState>,
    axum::extract::Path(track_id): axum::extract::Path<String>,
) -> ApiResult<Json<TrackStatusResponse>> {
    let metadata_keys = state.kv_storage.keys_like("%-metadata").await?;
    let chunk_keys = state.kv_storage.keys_like("%-chunk-%").await?;

    // Fetch all metadata
    let metadata_values = state.kv_storage.get_by_ids(&metadata_keys).await?;

    // Group chunks by document
    let mut doc_chunks: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for key in &chunk_keys {
        if let Some(doc_id) = key.split("-chunk-").next() {
            if !doc_id.ends_with("-metadata") && !doc_id.ends_with("-content") {
                *doc_chunks.entry(doc_id.to_string()).or_default() += 1;
            }
        }
    }

    // Filter documents by track_id
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

                let chunk_count = doc_chunks.get(&id).copied().unwrap_or(0);

                if let Some(created_at) = obj.get("created_at").and_then(|v| v.as_str()) {
                    created_times.push(created_at.to_string());
                }

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
                    error_message: obj
                        .get("error_message")
                        .and_then(|v| v.as_str())
                        .map(String::from),
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
                    // SPEC-002: Unified Ingestion Pipeline fields
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
        completed: track_docs
            .iter()
            .filter(|d| {
                d.status.is_none()
                    || d.status.as_deref() == Some("completed")
                    || d.status.as_deref() == Some("indexed")
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
