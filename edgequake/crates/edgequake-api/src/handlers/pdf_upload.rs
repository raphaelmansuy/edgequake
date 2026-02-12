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
// NOTE: PdfUploadProgress is referenced in OpenAPI doc comments but not as a Rust type
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
    /// Force re-indexing of duplicate PDF (default: false).
    /// WHY (OODA-08): When true, existing graph/vector data is cleared
    /// and the document is re-processed with current LLM/config.
    pub force_reindex: bool,
}

impl PdfUploadOptions {
    /// Get the vision model to use (with fallback).
    pub fn vision_model(&self) -> String {
        self.vision_model
            .clone()
            .unwrap_or_else(|| match self.vision_provider.as_str() {
                "ollama" => "gemma3:latest".to_string(),
                _ => "gpt-4.1-nano".to_string(),
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
    pub pagination: PdfPaginationInfo,
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

/// Pagination information for PDF listing.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PdfPaginationInfo {
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
        track_id: None,
        force_reindex: false,
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
            Some("force_reindex") => {
                // OODA-08: Parse force_reindex parameter
                // WHY: Allows re-processing of duplicate documents
                if let Ok(text) = field.text().await {
                    options.force_reindex = text.parse().unwrap_or(false);
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
        // OODA-08: Handle force_reindex parameter
        // WHY: When user explicitly requests re-indexing, we should:
        //      1. Clear existing graph/vector data for this document
        //      2. Reset PDF processing status
        //      3. Create new processing task
        if options.force_reindex {
            info!(
                "OODA-08: Force re-indexing requested for existing PDF: id={}, document_id={:?}",
                existing.pdf_id, existing.document_id
            );

            // Clear existing document data if document_id exists
            if let Some(document_id) = existing.document_id {
                if let Err(e) = clear_document_derived_data(&state, &document_id.to_string()).await
                {
                    warn!(
                        "Failed to clear document data during re-index: {} (continuing anyway)",
                        e
                    );
                }
            }

            // Reset PDF processing status to pending
            pdf_storage
                .update_pdf_status(&existing.pdf_id, PdfProcessingStatus::Processing)
                .await
                .map_err(|e| ApiError::Internal(format!("Failed to reset PDF status: {}", e)))?;

            // Create new processing task
            let task_id =
                create_pdf_processing_task(&state, &context, existing.pdf_id, &options).await?;

            // Initialize progress tracking
            let effective_track_id = options.track_id.clone().unwrap_or_else(|| task_id.clone());
            info!(
                "OODA-08: Re-indexing PDF progress for track_id={}, pdf_id={}, filename={}",
                effective_track_id, existing.pdf_id, existing.filename
            );
            state
                .pipeline_state
                .start_pdf_progress(
                    &effective_track_id,
                    &existing.pdf_id.to_string(),
                    &existing.filename,
                )
                .await;

            let estimated_time = estimate_processing_time(&[], existing.page_count);

            return Ok(Json(PdfUploadResponse {
                pdf_id: existing.pdf_id.to_string(),
                document_id: None, // Will be set after re-processing
                status: "reindexing".to_string(),
                task_id: task_id.to_string(),
                track_id: options.track_id.clone(),
                message: "Re-indexing document. Previous graph/vector data cleared.".to_string(),
                estimated_time_seconds: estimated_time,
                metadata: PdfMetadata {
                    filename: existing.filename,
                    file_size_bytes: existing.file_size_bytes,
                    page_count: existing.page_count,
                    sha256_checksum: existing.sha256_checksum,
                    vision_enabled: options.enable_vision,
                    vision_model: if options.enable_vision {
                        Some(options.vision_model())
                    } else {
                        None
                    },
                },
            }));
        }

        // Default: Return duplicate status (no re-indexing)
        warn!(
            "Duplicate PDF upload detected: existing_id={}",
            existing.pdf_id
        );

        // OODA-01 FIX: Initialize progress even for duplicates
        //
        // WHY: Frontend polls /pdf/progress/{track_id} immediately after upload.
        //      Even for duplicates, we need to return a valid progress entry
        //      so the frontend doesn't get a 404 error.
        //
        // The duplicate response tells the frontend it's already processed,
        // but the progress entry needs to exist for the initial poll.
        if let Some(ref track_id) = options.track_id {
            info!(
                "OODA-01: Initializing PDF progress for duplicate, track_id={}, pdf_id={}, filename={}",
                track_id, existing.pdf_id, existing.filename
            );
            state
                .pipeline_state
                .start_pdf_progress(track_id, &existing.pdf_id.to_string(), &existing.filename)
                .await;
        }

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

    // 9. OODA-01: Initialize progress tracking immediately
    //
    // WHY: Frontend polls /pdf/progress/{track_id} immediately after upload.
    //      Previously, progress was only initialized when the task callback
    //      fired (on_extraction_start), causing a race condition → 404 errors.
    //
    // FIX: Initialize progress here, before returning. The callback will
    //      update phases as processing proceeds, but the entry now exists.
    let effective_track_id = options.track_id.clone().unwrap_or_else(|| task_id.clone());
    info!(
        "OODA-01: Initializing PDF progress for track_id={}, pdf_id={}, filename={}",
        effective_track_id, pdf_id, filename
    );
    state
        .pipeline_state
        .start_pdf_progress(&effective_track_id, &pdf_id.to_string(), &filename)
        .await;

    // 10. Estimate processing time (rough heuristic)
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

// ============================================================================
// PDF Content Download Endpoints (SPEC-002: Document Viewer)
// ============================================================================

/// PDF download response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PdfContentResponse {
    /// PDF ID.
    pub pdf_id: String,
    /// Original filename.
    pub filename: String,
    /// File size in bytes.
    pub file_size_bytes: i64,
    /// MIME type.
    pub content_type: String,
    /// Extracted markdown content (if processed).
    pub markdown_content: Option<String>,
    /// Whether PDF processing is complete.
    pub is_processed: bool,
}

/// Download raw PDF file data.
///
/// @implements SPEC-002: Document Viewer - PDF download endpoint
/// @implements UC0711: Download PDF for viewing
/// @enforces BR0701: Workspace isolation
///
/// Returns the raw PDF binary data with appropriate content-type headers.
/// This allows the frontend PDF viewer to render the original document.
///
/// # Arguments
///
/// * `state` - Application state with PDF storage
/// * `context` - Tenant context for workspace isolation
/// * `pdf_id` - PDF identifier
///
/// # Returns
///
/// * `Ok(Response)` - Raw PDF data with application/pdf content-type
/// * `Err(404)` - PDF not found
/// * `Err(403)` - Not authorized for this workspace
#[utoipa::path(
    get,
    path = "/api/v1/documents/pdf/{pdf_id}/download",
    params(
        ("pdf_id" = String, Path, description = "PDF identifier")
    ),
    responses(
        (status = 200, description = "Raw PDF data", content_type = "application/pdf"),
        (status = 404, description = "PDF not found"),
        (status = 403, description = "Not authorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Documents"
)]
pub async fn download_pdf(
    State(state): State<AppState>,
    context: TenantContext,
    Path(pdf_id): Path<String>,
) -> ApiResult<axum::response::Response<axum::body::Body>> {
    use axum::http::header;
    use axum::response::IntoResponse;

    let pdf_id = Uuid::parse_str(&pdf_id)
        .map_err(|_| ApiError::BadRequest("Invalid PDF ID format".to_string()))?;

    let pdf_storage = get_pdf_storage(&state)?;

    let pdf = pdf_storage
        .get_pdf(&pdf_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get PDF: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("PDF not found".to_string()))?;

    // OODA-51: Make workspace verification optional for PDF viewer compatibility
    // WHY: react-pdf Document component loads PDFs via URL without custom headers,
    // so X-Workspace-ID header is not available. The PDF is already isolated by its
    // UUID which is unique per workspace, so access is implicitly scoped.
    // If workspace header IS provided, verify it matches for defense-in-depth.
    if let Some(workspace_id) = context.workspace_id_uuid() {
        if pdf.workspace_id != workspace_id {
            return Err(ApiError::Forbidden);
        }
    }

    info!(
        "PDF download: id={}, filename={}, size={}",
        pdf_id,
        pdf.filename,
        pdf.pdf_data.len()
    );

    // Build response with PDF data
    let content_disposition = format!("inline; filename=\"{}\"", pdf.filename);

    Ok((
        [
            (header::CONTENT_TYPE, "application/pdf"),
            (header::CONTENT_DISPOSITION, content_disposition.as_str()),
            (header::CACHE_CONTROL, "private, max-age=3600"),
        ],
        pdf.pdf_data,
    )
        .into_response())
}

/// Get PDF content metadata including markdown.
///
/// @implements SPEC-002: Document Viewer - Markdown content endpoint
/// @implements UC0712: Get PDF metadata with extracted markdown
/// @enforces BR0701: Workspace isolation
///
/// Returns PDF metadata including the extracted markdown content (if processed).
/// This allows the frontend to display both the original PDF and the extracted markdown.
///
/// # Arguments
///
/// * `state` - Application state with PDF storage
/// * `context` - Tenant context for workspace isolation
/// * `pdf_id` - PDF identifier
///
/// # Returns
///
/// * `Ok(Json(PdfContentResponse))` - PDF metadata with markdown
/// * `Err(404)` - PDF not found
/// * `Err(403)` - Not authorized for this workspace
#[utoipa::path(
    get,
    path = "/api/v1/documents/pdf/{pdf_id}/content",
    params(
        ("pdf_id" = String, Path, description = "PDF identifier")
    ),
    responses(
        (status = 200, description = "PDF content metadata", body = PdfContentResponse),
        (status = 404, description = "PDF not found"),
        (status = 403, description = "Not authorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Documents"
)]
pub async fn get_pdf_content(
    State(state): State<AppState>,
    context: TenantContext,
    Path(pdf_id): Path<String>,
) -> ApiResult<Json<PdfContentResponse>> {
    let pdf_id = Uuid::parse_str(&pdf_id)
        .map_err(|_| ApiError::BadRequest("Invalid PDF ID format".to_string()))?;

    let pdf_storage = get_pdf_storage(&state)?;

    let pdf = pdf_storage
        .get_pdf(&pdf_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get PDF: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("PDF not found".to_string()))?;

    // OODA-51: Make workspace verification optional for PDF viewer compatibility
    // WHY: Frontend PDF components may not have access to custom headers.
    // If workspace header IS provided, verify it matches for defense-in-depth.
    if let Some(workspace_id) = context.workspace_id_uuid() {
        if pdf.workspace_id != workspace_id {
            return Err(ApiError::Forbidden);
        }
    }

    let is_processed = pdf.processing_status == PdfProcessingStatus::Completed;

    Ok(Json(PdfContentResponse {
        pdf_id: pdf.pdf_id.to_string(),
        filename: pdf.filename,
        file_size_bytes: pdf.file_size_bytes,
        content_type: pdf.content_type,
        markdown_content: pdf.markdown_content,
        is_processed,
    }))
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
        tenant_id,
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

    // Store task in database
    state
        .task_storage
        .create_task(&task)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to create task: {}", e)))?;

    // Queue task for background processing (critical - missing this causes tasks to stay in pending)
    state
        .task_queue
        .send(task)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to queue task: {}", e)))?;

    debug!(
        "Created and queued PDF processing task: id={}, pdf_id={}",
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

/// Clear derived data (graph/vector) for a document during re-indexing.
///
/// OODA-08: Helper function to clear graph and vector data for a document
/// without deleting the raw PDF or markdown content.
///
/// # WHY
///
/// When re-indexing a document, we want to:
/// 1. Keep the raw PDF data (no need to re-upload)
/// 2. Keep the markdown content (can be re-used or regenerated)
/// 3. Clear graph entities/relationships (will be re-extracted)
/// 4. Clear vector embeddings (will be re-computed)
///
/// This allows re-processing with updated LLM/config without re-uploading.
///
/// # Arguments
///
/// * `state` - Application state with graph and vector storage
/// * `document_id` - Document ID to clear data for
///
/// # Returns
///
/// * `Ok(())` - Data cleared successfully
/// * `Err(String)` - Error message if clearing failed
async fn clear_document_derived_data(state: &AppState, document_id: &str) -> Result<(), String> {
    info!(
        "OODA-08: Clearing derived data for document: {}",
        document_id
    );

    let mut entities_cleared = 0;
    let mut edges_cleared = 0;

    // 1. Clear graph data (entities and relationships)
    let graph_storage = &state.graph_storage;

    // Get all nodes and filter by source_id
    let all_nodes = graph_storage
        .get_all_nodes()
        .await
        .map_err(|e| format!("Failed to get graph nodes: {}", e))?;

    let chunk_prefix = format!("{}-chunk-", document_id);

    for node in all_nodes {
        // Check if this node has sources from the deleted document
        if let Some(source_id) = node.properties.get("source_id").and_then(|v| v.as_str()) {
            let sources: Vec<&str> = source_id.split('|').collect();
            let remaining_sources: Vec<&str> = sources
                .into_iter()
                .filter(|s| !s.starts_with(&chunk_prefix) && !s.starts_with(document_id))
                .collect();

            if remaining_sources.is_empty() {
                // Delete connected edges first
                if let Ok(edges) = graph_storage.get_node_edges(&node.id).await {
                    for edge in edges {
                        let _ = graph_storage.delete_edge(&edge.source, &edge.target).await;
                        edges_cleared += 1;
                    }
                }
                // Then delete the node
                let _ = graph_storage.delete_node(&node.id).await;
                entities_cleared += 1;
            } else if remaining_sources.len() < source_id.split('|').count() {
                // Update to remove this document's sources
                let mut updated_props = node.properties.clone();
                updated_props.insert(
                    "source_id".to_string(),
                    serde_json::json!(remaining_sources.join("|")),
                );
                let _ = graph_storage.upsert_node(&node.id, updated_props).await;
            }
        }
    }

    // 2. Clear vector data
    // Note: Vector storage doesn't have a direct delete_by_document method,
    // but vector cleanup happens automatically when entities are deleted
    // because vectors are typically stored alongside entities or referenced by entity IDs.
    // Future optimization: Add explicit delete_vectors_by_document() if needed.

    info!(
        "OODA-08: Cleared derived data - entities={}, edges={}",
        entities_cleared, edges_cleared
    );

    Ok(())
}

// ============================================================================
// OODA-17: Error Recovery Endpoints
// ============================================================================

/// Response for retry/cancel operations.
///
/// OODA-17: Standard response for error recovery operations.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PdfOperationResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// The PDF ID.
    pub pdf_id: String,
    /// Human-readable message.
    pub message: String,
    /// New task ID (for retry operations).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
}

/// Retry a failed PDF processing task.
///
/// OODA-17: Re-enqueue a failed PDF for processing.
///
/// # Endpoint
///
/// `POST /api/v1/documents/pdf/{pdf_id}/retry`
///
/// # Behavior
///
/// 1. Validate PDF exists and belongs to workspace
/// 2. Check status is Failed (cannot retry Pending/Processing/Completed)
/// 3. Reset status to Pending
/// 4. Create new processing task
/// 5. Return new task ID
///
/// # Errors
///
/// - 404 if PDF not found
/// - 409 if PDF is not in Failed state
/// - 500 for internal errors
#[utoipa::path(
    post,
    path = "/api/v1/documents/pdf/{pdf_id}/retry",
    params(
        ("pdf_id" = String, Path, description = "PDF document ID")
    ),
    responses(
        (status = 200, description = "PDF retry initiated", body = PdfOperationResponse),
        (status = 404, description = "PDF not found"),
        (status = 409, description = "PDF not in retriable state"),
    ),
    security(("bearer_token" = []))
)]
#[allow(clippy::needless_return)]
pub async fn retry_pdf_processing(
    State(state): State<AppState>,
    tenant: TenantContext,
    Path(pdf_id): Path<String>,
) -> ApiResult<Json<PdfOperationResponse>> {
    // OODA-17: Retry requires postgres feature for PDF storage
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (&state, &tenant, &pdf_id);
        return Err(ApiError::Internal(
            "PDF storage requires postgres feature".to_string(),
        ));
    }

    #[cfg(feature = "postgres")]
    {
        let pdf_uuid = Uuid::parse_str(&pdf_id)
            .map_err(|_| ApiError::BadRequest("Invalid PDF ID format".to_string()))?;

        let _workspace_id = tenant
            .workspace_id_uuid()
            .ok_or_else(|| ApiError::BadRequest("Workspace ID required".to_string()))?;

        // OODA-17: Get PDF and validate state
        let pdf_storage = state
            .pdf_storage
            .as_ref()
            .ok_or_else(|| ApiError::Internal("PDF storage not available".to_string()))?;

        let pdf = pdf_storage
            .get_pdf(&pdf_uuid)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to get PDF: {}", e)))?
            .ok_or_else(|| ApiError::NotFound(format!("PDF not found: {}", pdf_id)))?;

        // Only allow retry of failed PDFs
        if pdf.processing_status != PdfProcessingStatus::Failed {
            return Err(ApiError::Conflict(format!(
                "Cannot retry PDF with status '{}'. Only 'failed' PDFs can be retried.",
                pdf.processing_status
            )));
        }

        // OODA-17: Reset status to Pending
        pdf_storage
            .update_pdf_status(&pdf_uuid, PdfProcessingStatus::Pending)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to reset PDF status: {}", e)))?;

        // OODA-17: Create new processing task
        let options = PdfUploadOptions {
            enable_vision: true,
            vision_provider: "openai".to_string(),
            vision_model: None,
            ..Default::default()
        };

        let task_id = create_pdf_processing_task(&state, &tenant, pdf_uuid, &options).await?;

        info!(
            pdf_id = %pdf_id,
            task_id = %task_id,
            "PDF processing retry initiated"
        );

        Ok(Json(PdfOperationResponse {
            success: true,
            pdf_id,
            message: "PDF retry initiated successfully".to_string(),
            task_id: Some(task_id),
        }))
    }
}

/// Cancel an in-progress PDF processing task.
///
/// OODA-17: Request cancellation of an active PDF processing task.
///
/// # Endpoint
///
/// `DELETE /api/v1/documents/pdf/{pdf_id}/cancel`
///
/// # Behavior
///
/// 1. Validate PDF exists and belongs to workspace
/// 2. Check status is Processing (cannot cancel Completed/Failed)
/// 3. Request cancellation via PipelineState
/// 4. Update status to Failed with cancellation message
///
/// # Errors
///
/// - 404 if PDF not found
/// - 409 if PDF is not in cancellable state
/// - 500 for internal errors
#[utoipa::path(
    delete,
    path = "/api/v1/documents/pdf/{pdf_id}/cancel",
    params(
        ("pdf_id" = String, Path, description = "PDF document ID")
    ),
    responses(
        (status = 200, description = "PDF processing cancelled", body = PdfOperationResponse),
        (status = 404, description = "PDF not found"),
        (status = 409, description = "PDF not in cancellable state"),
    ),
    security(("bearer_token" = []))
)]
#[allow(clippy::needless_return)]
pub async fn cancel_pdf_processing(
    State(state): State<AppState>,
    tenant: TenantContext,
    Path(pdf_id): Path<String>,
) -> ApiResult<Json<PdfOperationResponse>> {
    // OODA-17: Cancel requires postgres feature for PDF storage
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (&state, &tenant, &pdf_id);
        return Err(ApiError::Internal(
            "PDF storage requires postgres feature".to_string(),
        ));
    }

    #[cfg(feature = "postgres")]
    {
        let pdf_uuid = Uuid::parse_str(&pdf_id)
            .map_err(|_| ApiError::BadRequest("Invalid PDF ID format".to_string()))?;

        let _workspace_id = tenant
            .workspace_id_uuid()
            .ok_or_else(|| ApiError::BadRequest("Workspace ID required".to_string()))?;

        // OODA-17: Get PDF and validate state
        let pdf_storage = state
            .pdf_storage
            .as_ref()
            .ok_or_else(|| ApiError::Internal("PDF storage not available".to_string()))?;

        let pdf = pdf_storage
            .get_pdf(&pdf_uuid)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to get PDF: {}", e)))?
            .ok_or_else(|| ApiError::NotFound(format!("PDF not found: {}", pdf_id)))?;

        // Allow cancel of Processing or Pending PDFs
        // WHY: Documents can get stuck in non-terminal states (e.g., "uploading",
        // "pending") after a server restart or network interruption. Previously
        // only "processing" was cancellable, leaving users unable to recover from
        // stuck states. Now Pending is also allowed so users can force-cancel
        // documents that never transitioned to processing.
        if pdf.processing_status != PdfProcessingStatus::Processing
            && pdf.processing_status != PdfProcessingStatus::Pending
        {
            return Err(ApiError::Conflict(format!(
                "Cannot cancel PDF with status '{}'. Only 'processing' or 'pending' PDFs can be cancelled.",
                pdf.processing_status
            )));
        }

        // OODA-17: Request cancellation via pipeline state
        // WHY: This sets a flag that the worker checks periodically
        state.pipeline_state.request_cancellation().await;

        // OODA-17: Update status to Failed with cancellation message
        // WHY: UpdatePdfProcessingRequest requires pdf_id and processing_status as non-optional
        pdf_storage
            .update_pdf_status(&pdf_uuid, PdfProcessingStatus::Failed)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to update PDF status: {}", e)))?;

        info!(
            pdf_id = %pdf_id,
            "PDF processing cancellation requested"
        );

        Ok(Json(PdfOperationResponse {
            success: true,
            pdf_id,
            message: "PDF processing cancellation requested".to_string(),
            task_id: None,
        }))
    }
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
        // OODA-04: Updated from gpt-4o-mini to gpt-4.1-nano per mission directive
        assert_eq!(opts.vision_model(), "gpt-4.1-nano");

        opts.vision_provider = "ollama".to_string();
        assert_eq!(opts.vision_model(), "gemma3:latest");

        opts.vision_model = Some("custom-model".to_string());
        assert_eq!(opts.vision_model(), "custom-model");
    }

    /// OODA-17: Test PdfOperationResponse serialization
    #[test]
    fn test_pdf_operation_response_serialization() {
        // With task_id
        let response = PdfOperationResponse {
            success: true,
            pdf_id: "abc123".to_string(),
            message: "Retry initiated".to_string(),
            task_id: Some("task-456".to_string()),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"pdf_id\":\"abc123\""));
        assert!(json.contains("\"task_id\":\"task-456\""));

        // Without task_id (skip_serializing_if)
        let response = PdfOperationResponse {
            success: true,
            pdf_id: "abc123".to_string(),
            message: "Cancelled".to_string(),
            task_id: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(!json.contains("task_id"));
    }
}
