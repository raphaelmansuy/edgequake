use axum::extract::State;
use axum::Json;
use axum_extra::extract::Multipart;
use tracing::{debug, info, warn};

use super::helpers::{
    clear_document_derived_data, create_pdf_processing_task, estimate_processing_time,
    extract_page_count, get_pdf_storage,
};
use super::types::*;
use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::state::AppState;
use edgequake_storage::{
    calculate_pdf_checksum, validate_pdf_data, CreatePdfRequest, PdfProcessingStatus,
};

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
        vision_provider: None, // None = apply workspace config then server default
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
                    options.vision_provider = Some(text);
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

    // 5b. SPEC-040: Apply workspace-level vision LLM config as defaults.
    // Priority: form explicit > workspace config > server default.
    // WHY: Workspace can pin a specific vision provider/model for all PDF uploads,
    // avoiding the need for callers to pass vision_provider/vision_model every time.
    if options.vision_provider.is_none() || options.vision_model.is_none() {
        if let Ok(Some(ws)) = state.workspace_service.get_workspace(workspace_id).await {
            if options.vision_provider.is_none() {
                if let Some(ref wp) = ws.vision_llm_provider {
                    debug!(
                        "SPEC-040: Applying workspace vision_provider={} from workspace config",
                        wp
                    );
                    options.vision_provider = Some(wp.clone());
                }
            }
            if options.vision_model.is_none() {
                if let Some(ref wm) = ws.vision_llm_model {
                    debug!(
                        "SPEC-040: Applying workspace vision_model={} from workspace config",
                        wm
                    );
                    options.vision_model = Some(wm.clone());
                }
            }
        }
    }

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
                duplicate_of: None, // Re-indexing = already decided to replace
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

        let existing_pdf_id = existing.pdf_id.to_string();
        return Ok(Json(PdfUploadResponse {
            pdf_id: existing_pdf_id.clone(),
            document_id: existing.document_id.map(|id| id.to_string()),
            status: "duplicate".to_string(),
            task_id: "".to_string(),
            track_id: options.track_id.clone(),
            message: format!("PDF already uploaded with ID: {}", existing_pdf_id),
            estimated_time_seconds: 0,
            metadata: PdfMetadata {
                filename: existing.filename,
                file_size_bytes: existing.file_size_bytes,
                page_count: existing.page_count,
                sha256_checksum: existing.sha256_checksum,
                vision_enabled: options.enable_vision,
                vision_model: existing.vision_model,
            },
            // WHY: This field is what the frontend checks to trigger the
            // DuplicateUploadDialog, enabling the user to reprocess or skip.
            duplicate_of: Some(existing_pdf_id),
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

    let pdf_id = match pdf_storage
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
    {
        Ok(id) => id,
        Err(e) => {
            // FIX-DUPLICATE-BUG: Handle concurrent upload race condition gracefully.
            // WHY: If the unique constraint fires (two uploads of the same PDF arrived
            // simultaneously), re-fetch the existing PDF and return a duplicate response
            // instead of a 500 error.
            let err_msg = format!("{}", e);
            if err_msg.contains("already exists") || err_msg.contains("concurrent upload") {
                warn!(
                    "Concurrent duplicate PDF detected via DB constraint: checksum={}",
                    checksum
                );
                if let Ok(Some(existing)) = pdf_storage
                    .find_pdf_by_checksum(&workspace_id, &checksum)
                    .await
                {
                    let existing_pdf_id = existing.pdf_id.to_string();
                    return Ok(Json(PdfUploadResponse {
                        pdf_id: existing_pdf_id.clone(),
                        document_id: existing.document_id.map(|id| id.to_string()),
                        status: "duplicate".to_string(),
                        task_id: "".to_string(),
                        track_id: options.track_id.clone(),
                        message: format!(
                            "PDF already uploaded with ID: {} (concurrent upload detected)",
                            existing_pdf_id
                        ),
                        estimated_time_seconds: 0,
                        metadata: PdfMetadata {
                            filename: existing.filename,
                            file_size_bytes: existing.file_size_bytes,
                            page_count: existing.page_count,
                            sha256_checksum: existing.sha256_checksum,
                            vision_enabled: options.enable_vision,
                            vision_model: existing.vision_model,
                        },
                        duplicate_of: Some(existing_pdf_id),
                    }));
                }
            }
            return Err(ApiError::Internal(format!("Failed to store PDF: {}", e)));
        }
    };

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
        duplicate_of: None,
    }))
}
