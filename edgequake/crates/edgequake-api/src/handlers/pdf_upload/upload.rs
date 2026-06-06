use axum::extract::State;
use axum::Json;
use axum_extra::extract::Multipart;
use tracing::{debug, info, warn};

use super::helpers::{
    clear_document_derived_data, create_pdf_processing_task, estimate_processing_time,
    extract_page_count, get_pdf_storage,
};
use super::types::*;
use edgequake_audit::{AuditEventType, AuditResult};

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::services::record_compliance_event;
use crate::state::AppState;
use edgequake_pdf::PdfParserBackend;
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
    params(
        ("X-Tenant-ID" = Option<String>, Header, description = "Tenant UUID for multi-tenant isolation"),
        ("X-Workspace-ID" = Option<String>, Header, description = "Workspace UUID — scopes uploaded PDFs"),
    ),
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
        pdf_parser_backend: None,
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
            Some("pdf_parser_backend") => {
                if let Ok(text) = field.text().await {
                    options.pdf_parser_backend = PdfParserBackend::from_env_str(&text);
                }
            }
            _ => {}
        }
    }

    let file_data = file_data.ok_or_else(|| {
        ApiError::BadRequest("Missing 'file' field in multipart request".to_string())
    })?;
    let response = process_pdf_upload_parts(&state, &context, filename, file_data, options).await?;
    Ok(Json(response))
}

/// Upload multiple PDFs in a single multipart request.
#[utoipa::path(
    post,
    path = "/api/v1/documents/pdf/batch",
    params(
        ("X-Tenant-ID" = Option<String>, Header, description = "Tenant UUID for multi-tenant isolation"),
        ("X-Workspace-ID" = Option<String>, Header, description = "Workspace UUID — scopes uploaded PDFs"),
    ),
    request_body(content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Batch PDF upload accepted", body = PdfBatchUploadResponse),
        (status = 400, description = "Invalid multipart payload"),
    ),
    tag = "Documents"
)]
pub async fn upload_pdf_batch_document(
    State(state): State<AppState>,
    context: TenantContext,
    mut multipart: Multipart,
) -> ApiResult<Json<PdfBatchUploadResponse>> {
    let mut options = PdfUploadOptions {
        enable_vision: true,
        vision_provider: None,
        vision_model: None,
        title: None,
        metadata: None,
        track_id: None,
        force_reindex: false,
        pdf_parser_backend: None,
    };
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to parse multipart: {}", e)))?
    {
        match field.name() {
            Some("file") | Some("files") => {
                let filename = field.file_name().unwrap_or("document.pdf").to_string();
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::BadRequest(format!("Failed to read file: {}", e)))?
                    .to_vec();
                files.push((filename, bytes));
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
                if let Ok(text) = field.text().await {
                    options.force_reindex = text.parse().unwrap_or(false);
                }
            }
            Some("pdf_parser_backend") => {
                if let Ok(text) = field.text().await {
                    options.pdf_parser_backend = PdfParserBackend::from_env_str(&text);
                }
            }
            _ => {}
        }
    }

    if files.is_empty() {
        return Err(ApiError::BadRequest(
            "Missing 'file' or 'files' field in multipart request".to_string(),
        ));
    }

    let mut results = Vec::new();
    let mut accepted = 0usize;
    let mut duplicates = 0usize;
    let mut failed = 0usize;

    for (filename, file_data) in files {
        let result = process_pdf_upload_parts(
            &state,
            &context,
            filename.clone(),
            file_data,
            options.clone(),
        )
        .await;
        match result {
            Ok(resp) => {
                if resp.status == "duplicate" {
                    duplicates += 1;
                } else {
                    accepted += 1;
                }
                results.push(PdfBatchFileResult {
                    filename,
                    status: resp.status,
                    pdf_id: Some(resp.pdf_id),
                    task_id: if resp.task_id.is_empty() {
                        None
                    } else {
                        Some(resp.task_id)
                    },
                    duplicate_of: resp.duplicate_of,
                    error: None,
                });
            }
            Err(err) => {
                failed += 1;
                results.push(PdfBatchFileResult {
                    filename,
                    status: "failed".to_string(),
                    pdf_id: None,
                    task_id: None,
                    duplicate_of: None,
                    error: Some(err.to_string()),
                });
            }
        }
    }

    Ok(Json(PdfBatchUploadResponse {
        total_files: accepted + duplicates + failed,
        accepted,
        duplicates,
        failed,
        results,
    }))
}

async fn process_pdf_upload_parts(
    state: &AppState,
    context: &TenantContext,
    filename: String,
    file_data: Vec<u8>,
    mut options: PdfUploadOptions,
) -> ApiResult<PdfUploadResponse> {
    validate_pdf_data(&file_data)
        .map_err(|e| ApiError::BadRequest(format!("Invalid PDF: {}", e)))?;

    let checksum = calculate_pdf_checksum(&file_data);
    debug!(
        "PDF validation passed: size={}, checksum={}",
        file_data.len(),
        checksum
    );

    let pdf_storage = get_pdf_storage(state)?;
    let workspace_id = context
        .workspace_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("Workspace ID required".to_string()))?;

    let workspace = state
        .workspace_service
        .get_workspace(workspace_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let resolved_backend = options.resolved_backend(workspace.as_ref());

    if resolved_backend == PdfParserBackend::Vision
        && (options.vision_provider.is_none() || options.vision_model.is_none())
    {
        if let Some(ws) = workspace.as_ref() {
            if options.vision_provider.is_none() {
                let effective_vision_provider = ws
                    .vision_llm_provider
                    .as_deref()
                    .filter(|p| !p.is_empty())
                    .unwrap_or(&ws.llm_provider);
                debug!(
                    "SPEC-040: Resolved vision_provider={} (workspace vision={:?}, main={})",
                    effective_vision_provider, ws.vision_llm_provider, ws.llm_provider
                );
                options.vision_provider = Some(effective_vision_provider.to_string());
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

    if let Some(existing) = pdf_storage
        .find_pdf_by_checksum(&workspace_id, &checksum)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to check for duplicates: {}", e)))?
    {
        if options.force_reindex {
            info!(
                "OODA-08: Force re-indexing requested for existing PDF: id={}, document_id={:?}",
                existing.pdf_id, existing.document_id
            );
            if let Some(document_id) = existing.document_id {
                if let Err(e) = clear_document_derived_data(state, &document_id.to_string()).await {
                    warn!(
                        "Failed to clear document data during re-index: {} (continuing anyway)",
                        e
                    );
                }
            }

            pdf_storage
                .update_pdf_status(&existing.pdf_id, PdfProcessingStatus::Processing)
                .await
                .map_err(|e| ApiError::Internal(format!("Failed to reset PDF status: {}", e)))?;

            let task_id = create_pdf_processing_task(
                state,
                context,
                existing.pdf_id,
                &options,
                workspace.as_ref(),
            )
            .await?;

            let effective_track_id = options.track_id.clone().unwrap_or_else(|| task_id.clone());
            state
                .tasks
                .pipeline_state
                .start_pdf_progress(
                    &effective_track_id,
                    &existing.pdf_id.to_string(),
                    &existing.filename,
                )
                .await;
            let estimated_time = estimate_processing_time(&[], existing.page_count);
            return Ok(PdfUploadResponse {
                pdf_id: existing.pdf_id.to_string(),
                document_id: None,
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
                duplicate_of: None,
            });
        }

        if let Some(ref track_id) = options.track_id {
            state
                .tasks
                .pipeline_state
                .start_pdf_progress(track_id, &existing.pdf_id.to_string(), &existing.filename)
                .await;
        }

        let existing_pdf_id = existing.pdf_id.to_string();
        return Ok(PdfUploadResponse {
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
            duplicate_of: Some(existing_pdf_id),
        });
    }

    let page_count = extract_page_count(&file_data);
    let vision_model = if resolved_backend == PdfParserBackend::Vision && options.enable_vision {
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
                    return Ok(PdfUploadResponse {
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
                    });
                }
            }
            return Err(ApiError::Internal(format!("Failed to store PDF: {}", e)));
        }
    };

    let task_id =
        create_pdf_processing_task(state, context, pdf_id, &options, workspace.as_ref()).await?;
    let effective_track_id = options.track_id.clone().unwrap_or_else(|| task_id.clone());
    state
        .tasks
        .pipeline_state
        .start_pdf_progress(&effective_track_id, &pdf_id.to_string(), &filename)
        .await;

    let estimated_time = estimate_processing_time(&file_data, page_count);

    let tenant_for_audit = context
        .tenant_id
        .clone()
        .unwrap_or_else(|| "default".to_string());
    record_compliance_event(
        state,
        tenant_for_audit,
        AuditEventType::DocumentUpload,
        "upload_pdf",
        AuditResult::Success,
        context.workspace_id.clone(),
        context.user_id.clone(),
        Some(("pdf".to_string(), pdf_id.to_string())),
    );

    Ok(PdfUploadResponse {
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
    })
}
