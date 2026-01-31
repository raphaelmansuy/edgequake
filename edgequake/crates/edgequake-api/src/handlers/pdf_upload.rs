//! PDF document upload handlers.
//!
//! @implements SPEC-007: PDF Upload Support with Vision LLM Integration
//! @implements FEAT0701: PDF document upload endpoint
//! @implements FEAT0702: PDF status checking
//! @implements FEAT0703: PDF listing
//!
//! # Implements
//!
//! - **UC0701**: Upload PDF for processing
//! - **UC0702**: Check PDF processing status
//! - **UC0703**: List workspace PDFs
//!
//! # Enforces
//!
//! - **BR0701**: PDFs scoped to workspace
//! - **BR0702**: 100MB file size limit
//! - **BR0703**: Deduplication via SHA-256
//! - **BR0201**: Tenant isolation
//! - **BR0401**: Authentication required
//!
//! # Endpoints
//!
//! | Method | Path | Handler | Description |
//! |--------|------|---------|-------------|
//! | POST | `/api/v1/documents/pdf` | [`upload_pdf_document`] | Upload PDF file |
//! | GET | `/api/v1/documents/pdf/:id` | [`get_pdf_status`] | Get PDF status |
//! | GET | `/api/v1/documents/pdf` | [`list_pdfs`] | List PDFs |
//! | DELETE | `/api/v1/documents/pdf/:id` | [`delete_pdf`] | Delete PDF |

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use axum_extra::extract::Multipart;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::state::AppState;
use chrono::Utc;
use edgequake_storage::{
    calculate_pdf_checksum, validate_pdf_data, CreatePdfRequest, ListPdfFilter, PdfDocumentStorage,
    PdfProcessingStatus,
};
use edgequake_tasks::{PdfProcessingData, Task, TaskStatus, TaskType};

// ============================================================================
// Request/Response Types
// ============================================================================

/// PDF upload options.
#[derive(Debug, Clone, Default)]
pub struct PdfUploadOptions {
    /// Enable vision LLM processing (default: true).
    pub enable_vision: bool,
    /// Vision provider to use (default: "openai").
    pub vision_provider: String,
    /// Vision model override (optional).
    pub vision_model: Option<String>,
    /// Document title (optional).
    pub title: Option<String>,
    /// Custom metadata (optional).
    pub metadata: Option<serde_json::Value>,
    /// Batch tracking ID (optional).
    pub track_id: Option<String>,
}

impl PdfUploadOptions {
    /// Get the vision model to use (with fallback).
    pub fn vision_model(&self) -> String {
        self.vision_model
            .clone()
            .unwrap_or_else(|| match self.vision_provider.as_str() {
                "ollama" => "gemma3:latest".to_string(),
                _ => "gpt-4o-mini".to_string(),
            })
    }
}

/// PDF upload response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PdfUploadResponse {
    /// Generated PDF ID.
    pub pdf_id: String,

    /// Associated document ID (null during processing).
    pub document_id: Option<String>,

    /// Processing status.
    pub status: String,

    /// Background task ID.
    pub task_id: String,

    /// Batch tracking ID (if provided).
    pub track_id: Option<String>,

    /// Human-readable message.
    pub message: String,

    /// Estimated processing time in seconds.
    pub estimated_time_seconds: u64,

    /// PDF metadata.
    pub metadata: PdfMetadata,
}

/// PDF metadata in response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PdfMetadata {
    /// Original filename.
    pub filename: String,

    /// File size in bytes.
    pub file_size_bytes: i64,

    /// Number of pages (if detected).
    pub page_count: Option<i32>,

    /// SHA-256 checksum.
    pub sha256_checksum: String,

    /// Vision enabled flag.
    pub vision_enabled: bool,

    /// Vision model to use.
    pub vision_model: Option<String>,
}

/// PDF status response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PdfStatusResponse {
    /// PDF ID.
    pub pdf_id: String,

    /// Associated document ID (if completed).
    pub document_id: Option<String>,

    /// Processing status.
    pub status: String,

    /// Processing duration in milliseconds (if completed).
    pub processing_duration_ms: Option<i64>,

    /// PDF metadata.
    pub metadata: PdfStatusMetadata,

    /// Extraction errors (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<serde_json::Value>,
}

/// PDF status metadata.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PdfStatusMetadata {
    /// Original filename.
    pub filename: String,

    /// Number of pages.
    pub page_count: Option<i32>,

    /// Extraction method used (if completed).
    pub extraction_method: Option<String>,

    /// Vision model used (if applicable).
    pub vision_model: Option<String>,

    /// When processing completed.
    pub processed_at: Option<String>,
}

/// PDF list query parameters.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ListPdfsQuery {
    /// Filter by status.
    #[serde(default)]
    pub status: Option<String>,

    /// Page number (1-indexed).
    #[serde(default = "default_page")]
    pub page: usize,

    /// Page size.
    #[serde(default = "default_page_size")]
    pub page_size: usize,
}

fn default_page() -> usize {
    1
}

fn default_page_size() -> usize {
    20
}

/// PDF list response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListPdfsResponse {
    /// PDF items.
    pub items: Vec<PdfListItem>,

    /// Pagination info.
    pub pagination: PaginationInfo,
}

/// PDF list item.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PdfListItem {
    /// PDF ID.
    pub pdf_id: String,

    /// Original filename.
    pub filename: String,

    /// Processing status.
    pub status: String,

    /// File size in bytes.
    pub file_size_bytes: i64,

    /// Number of pages.
    pub page_count: Option<i32>,

    /// When uploaded.
    pub created_at: String,

    /// When processed.
    pub processed_at: Option<String>,
}

/// Pagination information.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PaginationInfo {
    /// Current page (1-indexed).
    pub page: usize,

    /// Page size.
    pub page_size: usize,

    /// Total item count.
    pub total_count: i64,

    /// Total pages.
    pub total_pages: usize,
}

// ============================================================================
// Handlers
// ============================================================================

/// Upload a PDF document.
///
/// @implements SPEC-007: PDF Upload Support
/// @implements UC0701: Upload PDF for processing
/// @implements BR0702: 100MB file size limit
/// @implements BR0703: Deduplication via SHA-256
///
/// # Flow
///
/// 1. Parse multipart form data
/// 2. Validate PDF file (size, format, signature)
/// 3. Calculate SHA-256 checksum
/// 4. Check for duplicates
/// 5. Store raw PDF in database
/// 6. Create background processing task
/// 7. Return response with task ID
///
/// # Arguments
///
/// * `state` - Application state with PDF storage
/// * `context` - Tenant context (workspace, tenant IDs)
/// * `multipart` - Multipart form data with PDF file
///
/// # Returns
///
/// * `Ok(Json(PdfUploadResponse))` - Upload successful
/// * `Err(ApiError)` - Validation or storage failure
///
/// # Errors
///
/// - `ApiError::PayloadTooLarge` - File exceeds 100MB
/// - `ApiError::BadRequest` - Invalid PDF format
/// - `ApiError::Conflict` - Duplicate PDF detected
/// - `ApiError::Internal` - Storage failure
#[utoipa::path(
    post,
    path = "/api/v1/documents/pdf",
    request_body(content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "PDF uploaded successfully", body = PdfUploadResponse),
        (status = 400, description = "Invalid PDF or request"),
        (status = 409, description = "Duplicate PDF"),
        (status = 413, description = "File too large"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Documents"
)]
pub async fn upload_pdf_document(
    State(state): State<AppState>,
    context: TenantContext,
    mut multipart: Multipart,
) -> ApiResult<Json<PdfUploadResponse>> {
    info!(
        "PDF upload request: workspace={:?}, tenant={:?}",
        context.workspace_id, context.tenant_id
    );

    // 1. Parse multipart fields
    let mut file_data: Option<Vec<u8>> = None;
    let mut filename = String::from("document.pdf");
    let mut options = PdfUploadOptions {
        enable_vision: true,
        vision_provider: "openai".to_string(),
        vision_model: None,
        title: None,
        metadata: None,
    };

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to parse multipart: {}", e)))?
    {
        match field.name() {
            Some("file") => {
                filename = field.file_name().unwrap_or("document.pdf").to_string();
                file_data = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| ApiError::BadRequest(format!("Failed to read file: {}", e)))?
                        .to_vec(),
                );
            }
            Some("enable_vision") => {
                if let Ok(text) = field.text().await {
                    options.enable_vision = text.parse().unwrap_or(true);
                }
            }
            Some("vision_provider") => {
                if let Ok(text) = field.text().await {
                    options.vision_provider = text;
                }
            }
            Some("vision_model") => {
                if let Ok(text) = field.text().await {
                    options.vision_model = Some(text);
                }
            }
            Some("title") => {
                if let Ok(text) = field.text().await {
                    options.title = Some(text);
                }
            }
            Some("metadata") => {
                if let Ok(text) = field.text().await {
                    if let Ok(json) = serde_json::from_str(&text) {
                        options.metadata = Some(json);
                    }
                }
            }
            Some("track_id") => {
                if let Ok(text) = field.text().await {
                    options.track_id = Some(text);
                }
            }
            _ => {}
        }
    }

    // 2. Validate file data
    let file_data = file_data.ok_or_else(|| {
        ApiError::BadRequest("Missing 'file' field in multipart request".to_string())
    })?;

    validate_pdf_data(&file_data)
        .map_err(|e| ApiError::BadRequest(format!("Invalid PDF: {}", e)))?;

    // 3. Calculate checksum
    let checksum = calculate_pdf_checksum(&file_data);

    debug!(
        "PDF validation passed: size={}, checksum={}",
        file_data.len(),
        checksum
    );

    // 4. Get PDF storage (platform-specific)
    let pdf_storage = get_pdf_storage(&state)?;

    // 5. Extract workspace_id as UUID
    let workspace_id = context
        .workspace_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("Workspace ID required".to_string()))?;

    // 6. Check for duplicates
    if let Some(existing) = pdf_storage
        .find_pdf_by_checksum(&workspace_id, &checksum)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to check for duplicates: {}", e)))?
    {
        warn!(
            "Duplicate PDF upload detected: existing_id={}",
            existing.pdf_id
        );
        return Ok(Json(PdfUploadResponse {
            pdf_id: existing.pdf_id.to_string(),
            document_id: existing.document_id.map(|id| id.to_string()),
            status: "duplicate".to_string(),
            task_id: "".to_string(),
            track_id: options.track_id.clone(),
            message: format!("PDF already uploaded with ID: {}", existing.pdf_id),
            estimated_time_seconds: 0,
            metadata: PdfMetadata {
                filename: existing.filename,
                file_size_bytes: existing.file_size_bytes,
                page_count: existing.page_count,
                sha256_checksum: existing.sha256_checksum,
                vision_enabled: options.enable_vision,
                vision_model: existing.vision_model,
            },
        }));
    }

    // 6. Extract page count (simple PDF parse)
    let page_count = extract_page_count(&file_data);

    // 7. Store raw PDF
    let vision_model = if options.enable_vision {
        Some(options.vision_model())
    } else {
        None
    };

    let pdf_id = pdf_storage
        .create_pdf(CreatePdfRequest {
            workspace_id,
            filename: filename.clone(),
            content_type: "application/pdf".to_string(),
            file_size_bytes: file_data.len() as i64,
            sha256_checksum: checksum.clone(),
            page_count,
            pdf_data: file_data.clone(),
            vision_model: vision_model.clone(),
        })
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to store PDF: {}", e)))?;

    info!(
        "PDF stored: id={}, size={}, pages={:?}",
        pdf_id,
        file_data.len(),
        page_count
    );

    // 8. Create background task
    let task_id = create_pdf_processing_task(&state, &context, pdf_id, &options).await?;

    // 9. Estimate processing time (rough heuristic)
    let estimated_time = estimate_processing_time(&file_data, page_count);

    Ok(Json(PdfUploadResponse {
        pdf_id: pdf_id.to_string(),
        document_id: None,
        status: "processing".to_string(),
        task_id: task_id.to_string(),
        track_id: options.track_id,
        message: "PDF uploaded successfully. Processing in background.".to_string(),
        estimated_time_seconds: estimated_time,
        metadata: PdfMetadata {
            filename,
            file_size_bytes: file_data.len() as i64,
            page_count,
            sha256_checksum: checksum,
            vision_enabled: options.enable_vision,
            vision_model,
        },
    }))
}

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
        .map(|processed| (processed.timestamp_millis() - pdf.created_at.timestamp_millis()));

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

    let status = query
        .status
        .as_ref()
        .and_then(|s| PdfProcessingStatus::from_str(s).ok());

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
        pagination: PaginationInfo {
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

// ============================================================================
// Helper Functions
// ============================================================================

/// Get PDF storage from app state (platform-specific).
/// Get PDF storage from AppState.
///
/// @implements SPEC-007: PDF storage access
/// @enforces BR0701: PostgreSQL-backed PDF storage
#[cfg(feature = "postgres")]
fn get_pdf_storage(state: &AppState) -> ApiResult<Arc<dyn PdfDocumentStorage>> {
    state.pdf_storage.as_ref().map(Arc::clone).ok_or_else(|| {
        ApiError::Internal("PDF storage not initialized (check PostgreSQL setup)".to_string())
    })
}

#[cfg(not(feature = "postgres"))]
fn get_pdf_storage(_state: &AppState) -> ApiResult<Arc<dyn PdfDocumentStorage>> {
    Err(ApiError::Internal(
        "PDF storage not available (postgres feature disabled)".to_string(),
    ))
}

/// Create PDF processing background task.
async fn create_pdf_processing_task(
    state: &AppState,
    context: &TenantContext,
    pdf_id: Uuid,
    options: &PdfUploadOptions,
) -> ApiResult<String> {
    let workspace_id = context
        .workspace_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("Workspace ID required".to_string()))?;

    let tenant_id = context
        .tenant_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("Tenant ID required".to_string()))?;

    let task_data = PdfProcessingData {
        pdf_id,
        workspace_id,
        enable_vision: options.enable_vision,
        vision_provider: options.vision_provider.clone(),
        vision_model: options.vision_model.clone(),
    };

    let track_id = format!("pdf-{}", Uuid::new_v4());

    let task = Task {
        track_id: track_id.clone(),
        tenant_id,
        workspace_id,
        task_type: TaskType::PdfProcessing,
        status: TaskStatus::Pending,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        started_at: None,
        completed_at: None,
        error_message: None,
        error: None,
        retry_count: 0,
        max_retries: 3,
        consecutive_timeout_failures: 0,
        circuit_breaker_tripped: false,
        task_data: serde_json::to_value(&task_data)
            .map_err(|e| ApiError::Internal(format!("Failed to serialize task data: {}", e)))?,
        metadata: None,
        progress: None,
        result: None,
    };

    state
        .task_storage
        .create_task(&task)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to create task: {}", e)))?;

    debug!(
        "Created PDF processing task: id={}, pdf_id={}",
        track_id, pdf_id
    );

    Ok(track_id)
}

/// Extract page count from PDF (simple parse).
fn extract_page_count(pdf_data: &[u8]) -> Option<i32> {
    // Try to parse PDF and get page count
    // This is a simple implementation that looks for /Count in the catalog
    if let Ok(text) = std::str::from_utf8(pdf_data) {
        // Look for /Type /Catalog ... /Pages ... /Count N
        if let Some(count_pos) = text.find("/Count") {
            let after_count = &text[count_pos + 6..];
            if let Some(num_end) = after_count.find(|c: char| !c.is_ascii_digit()) {
                if let Ok(count) = after_count[..num_end].trim().parse::<i32>() {
                    return Some(count);
                }
            }
        }
    }
    None
}

/// Estimate processing time based on file size and page count.
fn estimate_processing_time(pdf_data: &[u8], page_count: Option<i32>) -> u64 {
    let size_mb = (pdf_data.len() as f64) / 1_048_576.0;
    let pages = page_count.unwrap_or(10) as f64;

    // Rough estimate: 2-4 seconds per page with vision, 0.5s without
    // Add overhead for large files
    let base_time = pages * 3.0;
    let size_penalty = size_mb * 0.5;

    (base_time + size_penalty).ceil() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_processing_time() {
        // Small PDF, few pages
        let data = vec![0u8; 100_000]; // 100KB
        let time = estimate_processing_time(&data, Some(5));
        assert!(time >= 15 && time <= 30); // 5 pages * 3s + 0.1MB * 0.5

        // Large PDF, many pages
        let data = vec![0u8; 10_000_000]; // 10MB
        let time = estimate_processing_time(&data, Some(50));
        assert!(time >= 150 && time <= 200); // 50 pages * 3s + 10MB * 0.5
    }

    #[test]
    fn test_pdf_upload_options_vision_model() {
        let mut opts = PdfUploadOptions::default();
        opts.vision_provider = "openai".to_string();
        assert_eq!(opts.vision_model(), "gpt-4o-mini");

        opts.vision_provider = "ollama".to_string();
        assert_eq!(opts.vision_model(), "gemma3:latest");

        opts.vision_model = Some("custom-model".to_string());
        assert_eq!(opts.vision_model(), "custom-model");
    }
}
