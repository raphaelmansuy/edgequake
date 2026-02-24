use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use tracing::info;
use uuid::Uuid;

use super::helpers::get_pdf_storage;
use super::types::*;
use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::state::AppState;
use edgequake_storage::ListPdfFilter;

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
        return Err(ApiError::Forbidden);
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
        ("page_size" = Option<usize>, Query, description = "Page size")
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

    let list = pdf_storage
        .list_pdfs(ListPdfFilter {
            workspace_id,
            processing_status: status,
            page: Some(query.page),
            page_size: Some(query.page_size),
        })
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to list PDFs: {}", e)))?;

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
        return Err(ApiError::Forbidden);
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
/// * `track_id` - Upload tracking ID (returned from upload response)
///
/// # Returns
///
/// * `Ok(Json)` - Progress data as JSON
/// * `Err(404)` - Progress not found (upload completed or not started)
#[utoipa::path(
    get,
    path = "/api/v1/documents/pdf/progress/{track_id}",
    params(
        ("track_id" = String, Path, description = "Upload tracking ID from upload response")
    ),
    responses(
        (status = 200, description = "Progress data (PdfUploadProgress)"),
        (status = 404, description = "Progress not found (completed or not started)"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Documents"
)]
pub async fn get_pdf_progress(
    State(state): State<AppState>,
    Path(track_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let progress = state
        .pipeline_state
        .get_pdf_progress(&track_id)
        .await
        .ok_or_else(|| {
            ApiError::NotFound(
                "Progress not found. Upload may have completed or not yet started.".to_string(),
            )
        })?;

    // Serialize to JSON value to avoid utoipa schema requirements
    let json_value = serde_json::to_value(&progress)
        .map_err(|e| ApiError::Internal(format!("Failed to serialize progress: {}", e)))?;

    Ok(Json(json_value))
}
