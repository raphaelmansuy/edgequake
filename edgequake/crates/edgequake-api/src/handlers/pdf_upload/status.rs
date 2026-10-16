use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::Event;
use axum::response::Response;
use axum::Json;
use std::time::Duration;
use tracing::info;
use uuid::Uuid;

use super::helpers::get_pdf_storage;
use super::types::*;
use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::state::AppState;
use crate::streaming::live_sse;
use edgequake_storage::ListPdfFilter;
use edgequake_tasks::PdfUploadProgress;

/// Get PDF processing status.
///
/// @implements UC0702: Check PDF processing status
///
/// # Arguments
///
/// * `state` - Application state
/// * `context` - Tenant context
/// * `pdf_id` - PDF identifier
///
/// # Returns
///
/// * `Ok(Json(PdfStatusResponse))` - Status retrieved
/// * `Err(ApiError::NotFound)` - PDF not found
#[utoipa::path(
    get,
    path = "/api/v1/documents/pdf/{pdf_id}",
    params(
        ("pdf_id" = String, Path, description = "PDF identifier")
    ),
    responses(
        (status = 200, description = "PDF status", body = PdfStatusResponse),
        (status = 404, description = "PDF not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Documents"
)]
pub async fn get_pdf_status(
    State(state): State<AppState>,
    context: TenantContext,
    Path(pdf_id): Path<String>,
) -> ApiResult<Json<PdfStatusResponse>> {
    let pdf_id = Uuid::parse_str(&pdf_id)
        .map_err(|_| ApiError::BadRequest("Invalid PDF ID format".to_string()))?;

    let pdf_storage = get_pdf_storage(&state)?;

    let pdf = pdf_storage
        .get_pdf(&pdf_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get PDF: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("PDF not found".to_string()))?;

    // Verify workspace access
    let workspace_id = context
        .workspace_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("Workspace ID required".to_string()))?;

    if pdf.workspace_id != workspace_id {
        return Err(ApiError::forbidden());
    }

    let processing_duration_ms = pdf
        .processed_at
        .map(|processed| processed.timestamp_millis() - pdf.created_at.timestamp_millis());

    Ok(Json(PdfStatusResponse {
        pdf_id: pdf.pdf_id.to_string(),
        document_id: pdf.document_id.map(|id| id.to_string()),
        status: pdf.processing_status.as_str().to_string(),
        processing_duration_ms,
        metadata: PdfStatusMetadata {
            filename: pdf.filename,
            page_count: pdf.page_count,
            extraction_method: pdf.extraction_method.map(|m| m.as_str().to_string()),
            vision_model: pdf.vision_model,
            processed_at: pdf.processed_at.map(|t| t.to_rfc3339()),
        },
        errors: pdf.extraction_errors,
    }))
}

/// List PDFs in workspace.
///
/// @implements UC0703: List workspace PDFs
///
/// # Arguments
///
/// * `state` - Application state
/// * `context` - Tenant context
/// * `query` - Query parameters (status, pagination)
///
/// # Returns
///
/// * `Ok(Json(ListPdfsResponse))` - PDF list with pagination
#[utoipa::path(
    get,
    path = "/api/v1/documents/pdf",
    params(
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("page" = Option<usize>, Query, description = "Page number (1-indexed)"),
        ("page_size" = Option<usize>, Query, description = "Page size"),
        ("cursor" = Option<String>, Query, description = "Keyset cursor from next_cursor. Replaces the page offset.")
    ),
    responses(
        (status = 200, description = "PDF list", body = ListPdfsResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = "Documents"
)]
pub async fn list_pdfs(
    State(state): State<AppState>,
    context: TenantContext,
    Query(query): Query<ListPdfsQuery>,
) -> ApiResult<Json<ListPdfsResponse>> {
    let pdf_storage = get_pdf_storage(&state)?;

    let workspace_id = context.workspace_id_uuid();

    let status = query.status.as_ref().and_then(|s| s.parse().ok());
    let keyset = super::cursor::parse_pdf_cursor(query.cursor.as_deref());

    let list = pdf_storage
        .list_pdfs(ListPdfFilter {
            workspace_id,
            processing_status: status,
            page: Some(if keyset.is_some() { 1 } else { query.page }),
            page_size: Some(query.page_size),
            after_created_at: keyset.map(|(at, _)| at),
            after_pdf_id: keyset.map(|(_, id)| id),
        })
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to list PDFs: {}", e)))?;

    let next_cursor = if list.has_more {
        list.items
            .last()
            .map(|pdf| super::cursor::encode_pdf_cursor(pdf.created_at, pdf.pdf_id))
    } else {
        None
    };

    let items: Vec<PdfListItem> = list
        .items
        .into_iter()
        .map(|pdf| PdfListItem {
            pdf_id: pdf.pdf_id.to_string(),
            filename: pdf.filename,
            status: pdf.processing_status.as_str().to_string(),
            file_size_bytes: pdf.file_size_bytes,
            page_count: pdf.page_count,
            created_at: pdf.created_at.to_rfc3339(),
            processed_at: pdf.processed_at.map(|t| t.to_rfc3339()),
        })
        .collect();

    let total_pages = ((list.total_count as f64) / (list.page_size as f64)).ceil() as usize;

    Ok(Json(ListPdfsResponse {
        items,
        pagination: PdfPaginationInfo {
            page: list.page,
            page_size: list.page_size,
            total_count: list.total_count,
            total_pages,
            next_cursor,
        },
    }))
}

/// Delete a PDF document.
///
/// @implements BR0701: Workspace isolation
///
/// # Arguments
///
/// * `state` - Application state
/// * `context` - Tenant context
/// * `pdf_id` - PDF identifier
///
/// # Returns
///
/// * `Ok(StatusCode::NO_CONTENT)` - PDF deleted
/// * `Err(ApiError::NotFound)` - PDF not found
#[utoipa::path(
    delete,
    path = "/api/v1/documents/pdf/{pdf_id}",
    params(
        ("pdf_id" = String, Path, description = "PDF identifier")
    ),
    responses(
        (status = 204, description = "PDF deleted"),
        (status = 404, description = "PDF not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Documents"
)]
pub async fn delete_pdf(
    State(state): State<AppState>,
    context: TenantContext,
    Path(pdf_id): Path<String>,
) -> ApiResult<StatusCode> {
    let pdf_id = Uuid::parse_str(&pdf_id)
        .map_err(|_| ApiError::BadRequest("Invalid PDF ID format".to_string()))?;

    let pdf_storage = get_pdf_storage(&state)?;

    // Verify existence and workspace access
    let pdf = pdf_storage
        .get_pdf(&pdf_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get PDF: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("PDF not found".to_string()))?;

    let workspace_id = context
        .workspace_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("Workspace ID required".to_string()))?;

    if pdf.workspace_id != workspace_id {
        return Err(ApiError::forbidden());
    }

    pdf_storage
        .delete_pdf(&pdf_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to delete PDF: {}", e)))?;

    info!("PDF deleted: id={}", pdf_id);

    Ok(StatusCode::NO_CONTENT)
}

/// Get PDF upload progress by track ID.
///
/// @implements SPEC-001-upload-pdf: Progress query endpoint
/// @implements OODA-14: GET progress endpoint
///
/// Returns real-time progress data for a PDF upload, including:
/// - Progress for all 6 pipeline phases
/// - Overall completion percentage
/// - Estimated time remaining
/// - Error details if any phase failed
///
/// # Arguments
///
/// * `state` - Application state with PipelineState
/// * `track_id` - Server job id (`PdfUploadResponse.task_id`). Not the optional
///   client batch `track_id` (SPEC-054 / GitHub #300).
///
/// # Returns
///
/// * `Ok(Json)` - Progress data as JSON
/// * `Err(404)` - Progress not found (upload completed or not started)
#[utoipa::path(
    get,
    path = "/api/v1/documents/pdf/progress/{track_id}",
    params(
        ("track_id" = String, Path, description = "Server task_id from PdfUploadResponse (not client batch track_id)")
    ),
    responses(
        (status = 200, description = "Progress data", body = PdfUploadProgress),
        (status = 404, description = "Progress not found (completed or not started)"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Documents"
)]
pub async fn get_pdf_progress(
    State(state): State<AppState>,
    context: TenantContext,
    Path(track_id): Path<String>,
) -> ApiResult<Json<PdfUploadProgress>> {
    // SPEC-083 S-02: ownership check — foreign track → 404.
    let task =
        crate::services::task_scope::get_task_for_context(&state, &track_id, &context).await?;

    // After restart the in-memory map is empty while the durable task may still
    // be live — rehydrate a skeleton so the UI does not hard-404 (SPEC-054).
    let progress =
        super::progress_identity::get_or_rehydrate_pdf_progress(&state, &track_id, &task)
            .await
            .ok_or_else(|| {
                ApiError::NotFound(
                    "Progress not found. Upload may have completed or not yet started.".to_string(),
                )
            })?;

    Ok(Json(progress))
}

/// SSE (Server-Sent Events) endpoint for real-time PDF progress streaming.
///
/// @implements FEAT-PROGRESS-SSE: Server-Sent Events for PDF progress
///
/// Provides a continuous stream of progress events for a PDF upload.
/// This is more efficient than polling for large documents (1000+ pages)
/// because the server pushes updates as they happen.
///
/// The stream automatically closes when:
/// - Processing completes (is_complete = true)
/// - Processing fails (is_failed = true)
/// - Client disconnects
/// - No progress found for 60 seconds (upload may have completed before connection)
///
/// # Event Format
///
/// Each SSE event is a JSON `PdfUploadProgress` object with:
/// - `track_id`, `pdf_id`, `filename`
/// - `phases[]`: Array of phase progress data
/// - `overall_percentage`: 0-100
/// - `is_complete`, `is_failed`
///
/// # Example Client Usage
///
/// ```javascript
/// const eventSource = new EventSource('/api/v1/documents/pdf/progress/stream/track-123');
/// eventSource.onmessage = (event) => {
///     const progress = JSON.parse(event.data);
///     console.log(`${progress.overall_percentage}% — ${progress.phases[1].message}`);
/// };
/// eventSource.addEventListener('complete', () => eventSource.close());
/// eventSource.addEventListener('error', () => eventSource.close());
/// ```
#[utoipa::path(
    get,
    path = "/api/v1/documents/pdf/progress/stream/{track_id}",
    params(
        ("track_id" = String, Path, description = "Server task_id from PdfUploadResponse (not client batch track_id)")
    ),
    responses(
        (status = 200, description = "SSE stream of PdfUploadProgress events",
            content(
                (PdfUploadProgress = "text/event-stream")
            )
        ),
        (status = 404, description = "Progress not found"),
    ),
    tag = "Documents"
)]
pub async fn get_pdf_progress_stream(
    State(state): State<AppState>,
    context: TenantContext,
    Path(track_id): Path<String>,
) -> Result<Response, ApiError> {
    // SPEC-083 S-02: ownership check before opening the SSE stream.
    crate::services::task_scope::get_task_for_context(&state, &track_id, &context).await?;

    let state_for_stream = state.clone();
    let tid = track_id.clone();

    // Adaptive poll interval based on whether we've seen progress yet
    // WHY: For large docs, we don't need to poll faster than the page processing rate.
    // The progress callback already debounces metadata updates.
    let stream = async_stream::stream! {
        let mut miss_count = 0u32;
        let mut last_percentage = -1.0_f32;
        let poll_interval = Duration::from_millis(500);

        loop {
            // Prefer live in-memory progress; on miss, reload the durable task so
            // a terminal Indexed/Failed row rehydrates correctly (not a stale
            // Processing snapshot from stream-open time).
            let progress = if let Some(live) = state_for_stream
                .tasks
                .pipeline_state
                .get_pdf_progress(&tid)
                .await
            {
                Some(live)
            } else if let Ok(Some(fresh_task)) =
                state_for_stream.tasks.storage.get_task(&tid).await
            {
                super::progress_identity::get_or_rehydrate_pdf_progress(
                    &state_for_stream,
                    &tid,
                    &fresh_task,
                )
                .await
            } else {
                None
            };

            if let Some(progress) = progress {
                miss_count = 0;
                let current_pct = progress.overall_percentage;

                // Only send event if percentage changed or it's a terminal state
                if (current_pct - last_percentage).abs() > 0.01
                    || progress.is_complete
                    || progress.is_failed
                {
                    last_percentage = current_pct;
                    if let Ok(json) = serde_json::to_string(&progress) {
                        let event_type = if progress.is_complete {
                            "complete"
                        } else if progress.is_failed {
                            "error"
                        } else {
                            "progress"
                        };

                        yield Ok(Event::default()
                            .event(event_type)
                            .data(json));

                        // Terminal state → close stream
                        if progress.is_complete || progress.is_failed {
                            break;
                        }
                    }
                }
            } else {
                miss_count += 1;
                // After 120 misses (60 seconds at 500ms intervals), give up
                if miss_count > 120 {
                    yield Ok(Event::default()
                        .event("timeout")
                        .data(r#"{"error":"Progress not found or already completed"}"#));
                    break;
                }
            }

            tokio::time::sleep(poll_interval).await;
        }
    };

    Ok(live_sse(stream))
}
