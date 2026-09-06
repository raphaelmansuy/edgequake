use axum::extract::State;
use axum::Json;
use axum_extra::extract::Multipart;
use tracing::{debug, info, warn};

use super::helpers::{
    clear_document_derived_data_in_workspace, create_pdf_processing_task, estimate_processing_time,
    extract_page_count, get_pdf_storage,
};
use super::progress_identity::seed_pdf_job_progress;
use super::types::*;
use edgequake_audit::{AuditEventType, AuditResult};

use crate::error::{ApiError, ApiResult};
use crate::handlers::auth::ApiOptionalAuth;
use crate::middleware::TenantContext;
use crate::multipart_upload::{
    ensure_batch_file_cap, stream_field_to_tempfile, StreamedUploadFile,
};
use crate::services::{
    record_compliance_event, recycle_orphan_workspace_pdf, workspace_has_visible_document_for_pdf,
};
use crate::services::spec146_authz::{
    apply_security_labels_to_metadata, finish_security_admit, parse_security_labels,
    SecurityFormOverrides,
};
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
/// @implements BR0702: 50 MiB file size limit (SPEC-083 D-44 / MAX_UPLOAD_BYTES)
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
/// - `ApiError::PayloadTooLarge` - File exceeds 50 MiB
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
    auth: ApiOptionalAuth,
    mut multipart: Multipart,
) -> ApiResult<Json<PdfUploadResponse>> {
    crate::services::spec146_authz::require_ingest_when_abac(&state, &context, &auth)?;
    info!(
        "PDF upload request: workspace={:?}, tenant={:?}",
        context.workspace_id, context.tenant_id
    );

    // 1. Parse multipart fields (SPEC-083 D-51: stream file to temp, not field.bytes())
    let mut streamed_file: Option<StreamedUploadFile> = None;
    let mut options = PdfUploadOptions {
        enable_vision: true,
        vision_provider: None, // None = apply workspace config then server default
        vision_model: None,
        title: None,
        metadata: None,
        track_id: None,
        force_reindex: false,
        pdf_parser_backend: None,
        process_options: None,
        vision_reasoning_effort: None,
        vision_extract: Default::default(),
    };
    let mut security_form = SecurityFormOverrides::default();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to parse multipart: {}", e)))?
    {
        let field_name = field.name().map(|s| s.to_string());
        match field_name.as_deref() {
            Some("file") => {
                // SPEC-083 S-12: never persist raw multipart path components.
                let filename = crate::file_validation::sanitize_filename(
                    field.file_name().unwrap_or("document.pdf"),
                );
                streamed_file = Some(stream_field_to_tempfile(field, filename).await?);
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
            Some("vision_reasoning_effort") => {
                if let Ok(text) = field.text().await {
                    let t = text.trim();
                    if !t.is_empty() {
                        options.vision_reasoning_effort = Some(t.to_string());
                    }
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
            Some("process_options") => {
                if let Ok(text) = field.text().await {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        options.process_options = Some(trimmed.to_string());
                    }
                }
            }

            Some("vision_extract_images") => {
                if let Ok(text) = field.text().await {
                    if let Ok(v) = text.trim().parse::<bool>() {
                        options.vision_extract.extract_images = Some(v);
                    }
                }
            }
            Some("vision_extract_charts") => {
                if let Ok(text) = field.text().await {
                    if let Ok(v) = text.trim().parse::<bool>() {
                        options.vision_extract.extract_charts = Some(v);
                    }
                }
            }
            Some("vision_extract_figures") => {
                if let Ok(text) = field.text().await {
                    if let Ok(v) = text.trim().parse::<bool>() {
                        options.vision_extract.extract_figures = Some(v);
                    }
                }
            }
            Some("vision_page_system_prompt") => {
                if let Ok(text) = field.text().await {
                    options.vision_extract.page_system_prompt = Some(text);
                }
            }
            Some("vision_image_system_prompt") => {
                if let Ok(text) = field.text().await {
                    options.vision_extract.image_system_prompt = Some(text);
                }
            }
            Some("vision_chart_system_prompt") => {
                if let Ok(text) = field.text().await {
                    options.vision_extract.chart_system_prompt = Some(text);
                }
            }
            Some("vision_figure_system_prompt") => {
                if let Ok(text) = field.text().await {
                    options.vision_extract.figure_system_prompt = Some(text);
                }
            }
            Some(name @ ("classification" | "share_mode" | "security_status" | "export_control"
            | "pii" | "project_id")) => {
                if let Ok(text) = field.text().await {
                    security_form.ingest_text_field(name, &text);
                }
            }
            _ => {}
        }
    }

    let streamed = streamed_file.ok_or_else(|| {
        ApiError::BadRequest("Missing 'file' field in multipart request".to_string())
    })?;
    let (filename, file_data) = streamed.into_bytes()?;
    let response =
        process_pdf_upload_parts(&state, &context, filename, file_data, options, security_form)
            .await?;
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
    auth: ApiOptionalAuth,
    mut multipart: Multipart,
) -> ApiResult<Json<PdfBatchUploadResponse>> {
    crate::services::spec146_authz::require_ingest_when_abac(&state, &context, &auth)?;
    let mut options = PdfUploadOptions {
        enable_vision: true,
        vision_provider: None,
        vision_model: None,
        title: None,
        metadata: None,
        track_id: None,
        force_reindex: false,
        pdf_parser_backend: None,
        process_options: None,
        vision_reasoning_effort: None,
        vision_extract: Default::default(),
    };
    // SPEC-083 D-51: stream each file to temp; cap batch count; process sequentially.
    let mut files: Vec<StreamedUploadFile> = Vec::new();
    let mut security_form = SecurityFormOverrides::default();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to parse multipart: {}", e)))?
    {
        let field_name = field.name().map(|s| s.to_string());
        match field_name.as_deref() {
            Some("file") | Some("files") => {
                ensure_batch_file_cap(files.len())?;
                let filename = crate::file_validation::sanitize_filename(
                    field.file_name().unwrap_or("document.pdf"),
                );
                files.push(stream_field_to_tempfile(field, filename).await?);
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
            Some("vision_reasoning_effort") => {
                if let Ok(text) = field.text().await {
                    let t = text.trim();
                    if !t.is_empty() {
                        options.vision_reasoning_effort = Some(t.to_string());
                    }
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
            Some("process_options") => {
                if let Ok(text) = field.text().await {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        options.process_options = Some(trimmed.to_string());
                    }
                }
            }

            Some("vision_extract_images") => {
                if let Ok(text) = field.text().await {
                    if let Ok(v) = text.trim().parse::<bool>() {
                        options.vision_extract.extract_images = Some(v);
                    }
                }
            }
            Some("vision_extract_charts") => {
                if let Ok(text) = field.text().await {
                    if let Ok(v) = text.trim().parse::<bool>() {
                        options.vision_extract.extract_charts = Some(v);
                    }
                }
            }
            Some("vision_extract_figures") => {
                if let Ok(text) = field.text().await {
                    if let Ok(v) = text.trim().parse::<bool>() {
                        options.vision_extract.extract_figures = Some(v);
                    }
                }
            }
            Some("vision_page_system_prompt") => {
                if let Ok(text) = field.text().await {
                    options.vision_extract.page_system_prompt = Some(text);
                }
            }
            Some("vision_image_system_prompt") => {
                if let Ok(text) = field.text().await {
                    options.vision_extract.image_system_prompt = Some(text);
                }
            }
            Some("vision_chart_system_prompt") => {
                if let Ok(text) = field.text().await {
                    options.vision_extract.chart_system_prompt = Some(text);
                }
            }
            Some("vision_figure_system_prompt") => {
                if let Ok(text) = field.text().await {
                    options.vision_extract.figure_system_prompt = Some(text);
                }
            }
            Some(name @ ("classification" | "share_mode" | "security_status" | "export_control"
            | "pii" | "project_id")) => {
                if let Ok(text) = field.text().await {
                    security_form.ingest_text_field(name, &text);
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

    for streamed in files {
        let (filename, file_data) = streamed.into_bytes()?;
        let result = process_pdf_upload_parts(
            &state,
            &context,
            filename.clone(),
            file_data,
            options.clone(),
            security_form.clone(),
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
    security_form: SecurityFormOverrides,
) -> ApiResult<PdfUploadResponse> {
    // SPEC-083 S-12: magic-byte MIME must match .pdf (rejects exe-as-pdf, etc.).
    crate::file_validation::validate_magic_matches_extension("pdf", &file_data)?;
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
    if let Some(ref ws) = workspace {
        options.apply_workspace(ws);
    }
    let tenant = match context.tenant_id_uuid() {
        Some(tid) => state
            .workspace_service
            .get_tenant(tid)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?,
        None => None,
    };
    let resolved_backend = options.resolved_backend(workspace.as_ref(), tenant.as_ref());

    // Legacy debug breadcrumb when Vision still lacks a model after workspace apply.
    if resolved_backend == PdfParserBackend::Vision
        && options.enable_vision
        && options.vision_model.as_ref().is_none_or(|s| s.is_empty())
    {
        debug!(
            "Vision backend selected but no vision_model after workspace apply; \
             will use provider default via vision_model()"
        );
    }

    let mut existing_pdf = pdf_storage
        .find_pdf_by_checksum(&workspace_id, &checksum)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to check for duplicates: {}", e)))?;

    if let Some(ref existing) = existing_pdf {
        let is_visible_duplicate =
            workspace_has_visible_document_for_pdf(state, context, existing).await?;
        if !is_visible_duplicate {
            info!(
                pdf_id = %existing.pdf_id,
                workspace_id = %workspace_id,
                "Orphan PDF checksum collision — recycling row for fresh upload"
            );
            recycle_orphan_workspace_pdf(pdf_storage.clone(), existing).await?;
            existing_pdf = None;
        }
    }

    if let Some(existing) = existing_pdf {
        if options.force_reindex {
            info!(
                "OODA-08: Force re-indexing requested for existing PDF: id={}, document_id={:?}",
                existing.pdf_id, existing.document_id
            );
            // Replace = TRUE re-conversion: reuse the existing document id so the
            // old document is updated in-place (no orphan), and force
            // restart_from_scratch so the PDF -> markdown conversion re-runs.
            let existing_document_id = existing.document_id.map(|id| id.to_string());
            if let Some(ref document_id) = existing_document_id {
                let workspace_id = context.workspace_id.as_deref();
                if let Err(e) =
                    clear_document_derived_data_in_workspace(state, document_id, workspace_id).await
                {
                    warn!(
                        "Failed to clear document data during re-index: {} (continuing anyway)",
                        e
                    );
                }
                // Clear cached markdown + KV content/chunks so the resume shortcut
                // cannot reuse the stale conversion. This is the authoritative
                // re-conversion signal (DRY with reprocess mode=full).
                if let Err(e) =
                    crate::handlers::documents::storage_helpers::clear_document_markdown_and_content(
                        state,
                        document_id,
                        &existing.pdf_id,
                    )
                    .await
                {
                    warn!(
                        "Failed to clear cached markdown during re-index: {} (continuing anyway)",
                        e
                    );
                }

                // Reset the existing document's KV metadata so the UI shows it
                // returning to processing on the SAME document id (no new UUID).
                // Must clear stale completion message/counts — otherwise Active Runs
                // paints "Processed N chunks…" under Prepare and nests a dead PDF meter.
                let metadata_key =
                    crate::services::document_metadata_scan::metadata_key_for_document(document_id);
                if let Ok(Some(mut metadata)) =
                    state.storage.kv_storage.get_by_id(&metadata_key).await
                {
                    if let Some(obj) = metadata.as_object_mut() {
                        let page_hint = existing
                            .page_count
                            .filter(|&n| n > 0)
                            .map(|n| format!("Converting PDF to Markdown (0/{n} pages)"));
                        crate::services::reprocess_stage_reset::apply_pdf_convert_restart_admit(
                            obj,
                            page_hint.as_deref(),
                        );
                        // SPEC-054: do not write client batch track_id into metadata.track_id.
                        // Progress/list SSOT is the server task id, set after enqueue below.
                        if let Some(client_batch) = options
                            .track_id
                            .as_ref()
                            .map(|s| s.trim())
                            .filter(|s| !s.is_empty())
                        {
                            obj.insert(
                                "batch_track_id".to_string(),
                                serde_json::json!(client_batch),
                            );
                        }
                    }
                    let _ = crate::services::upsert_metadata_kv_with_index(
                        state.storage.kv_storage.as_ref(),
                        &metadata_key,
                        metadata,
                    )
                    .await;
                }
                // Keep documents.track_id from pointing at a purged insert-* while
                // KV already shows converting (dual-SSOT drift → Task not found).
                crate::services::task_document_sync::touch_relational_document_track_status_best_effort(
                    document_id,
                    None,
                    "processing",
                )
                .await;

                // Cancel in-flight work for this document before requeueing.
                // Finished Failed/Indexed/Cancelled rows stay in `tasks` (issue #386).
                let ws_for_tasks = context
                    .workspace_id
                    .clone()
                    .unwrap_or_else(|| "default".to_string());
                let purged =
                    crate::handlers::documents::storage_helpers::purge_persisted_tasks_for_document(
                        state,
                        document_id,
                        None,
                        Some(&ws_for_tasks),
                    )
                    .await;
                if purged > 0 {
                    info!(
                        document_id = %document_id,
                        inflight_tasks_cancelled = purged,
                        "Cancelled in-flight tasks before force re-index"
                    );
                }
            }

            pdf_storage
                .update_pdf_status(&existing.pdf_id, PdfProcessingStatus::Processing)
                .await
                .map_err(|e| ApiError::Internal(format!("Failed to reset PDF status: {}", e)))?;

            let enqueue = create_pdf_processing_task(
                state,
                context,
                existing.pdf_id,
                &options,
                workspace.as_ref(),
                super::helpers::PdfReprocessIntent {
                    existing_document_id: existing_document_id.clone(),
                    restart_from_scratch: true, // re-run PDF -> markdown conversion
                    reprocess_mode: Some(edgequake_tasks::ReprocessMode::Full),
                },
                existing.page_count,
                existing.file_size_bytes.max(0) as u64,
            )
            .await?;

            // SPEC-054 / #300: metadata.track_id and progress key are always server task_id.
            if let Some(ref document_id) = existing_document_id {
                let metadata_key =
                    crate::services::document_metadata_scan::metadata_key_for_document(document_id);
                if let Ok(Some(mut metadata)) =
                    state.storage.kv_storage.get_by_id(&metadata_key).await
                {
                    if let Some(obj) = metadata.as_object_mut() {
                        obj.insert("track_id".to_string(), serde_json::json!(enqueue.track_id));
                    }
                    let _ = crate::services::upsert_metadata_kv_with_index(
                        state.storage.kv_storage.as_ref(),
                        &metadata_key,
                        metadata,
                    )
                    .await;
                }
                crate::services::task_document_sync::touch_relational_document_track_status_best_effort(
                    document_id,
                    Some(enqueue.track_id.as_str()),
                    "processing",
                )
                .await;
            }

            seed_pdf_job_progress(
                state,
                &enqueue.track_id,
                &existing.pdf_id.to_string(),
                &existing.filename,
                options.track_id.as_deref(),
            )
            .await;
            let estimated_time = estimate_processing_time(
                existing.file_size_bytes.max(0) as u64,
                existing.page_count,
                options.resolved_backend(workspace.as_ref(), tenant.as_ref()),
                &options.resolved_vision_provider(workspace.as_ref(), tenant.as_ref()),
            );
            return Ok(PdfUploadResponse {
                pdf_id: existing.pdf_id.to_string(),
                document_id: existing_document_id,
                status: "reindexing".to_string(),
                task_id: enqueue.track_id.to_string(),
                track_id: options.track_id.clone(),
                message: "Re-indexing document. PDF will be re-converted to markdown and graph/vector data cleared.".to_string(),
                estimated_time_seconds: estimated_time,
                ingestion_estimate: None,
                metadata: PdfMetadata {
                    filename: existing.filename,
                    file_size_bytes: existing.file_size_bytes,
                    page_count: existing.page_count,
                    sha256_checksum: existing.sha256_checksum,
                    vision_enabled: options.enable_vision,
                    vision_model: if options.enable_vision {
                        Some(options.vision_model(workspace.as_ref(), tenant.as_ref()))
                    } else {
                        None
                    },
                },
                duplicate_of: None,
                queue_position: enqueue.queue.as_ref().map(|q| q.position),
                eta_seconds: enqueue.queue.as_ref().map(|q| q.eta_seconds),
                eta_basis: enqueue
                    .queue
                    .as_ref()
                    .map(|q| q.basis.as_str().to_string()),
            });
        }

        // Duplicate without reindex: no new task → do not seed progress.
        let existing_pdf_id = existing.pdf_id.to_string();
        return Ok(PdfUploadResponse {
            pdf_id: existing_pdf_id.clone(),
            document_id: existing.document_id.map(|id| id.to_string()),
            status: "duplicate".to_string(),
            task_id: "".to_string(),
            track_id: options.track_id.clone(),
            message: format!("PDF already uploaded with ID: {}", existing_pdf_id),
            estimated_time_seconds: 0,
            ingestion_estimate: None,
            metadata: PdfMetadata {
                filename: existing.filename,
                file_size_bytes: existing.file_size_bytes,
                page_count: existing.page_count,
                sha256_checksum: existing.sha256_checksum,
                vision_enabled: options.enable_vision,
                vision_model: existing.vision_model,
            },
            duplicate_of: Some(existing_pdf_id),
            queue_position: None,
            eta_seconds: None,
            eta_basis: None,
        });
    }

    let file_size_bytes = file_data.len() as u64;
    let page_count = extract_page_count(&file_data).await;
    let vision_model = if resolved_backend == PdfParserBackend::Vision && options.enable_vision {
        Some(options.vision_model(workspace.as_ref(), tenant.as_ref()))
    } else {
        None
    };
    // SPEC-038 REQ-038-11: move bytes into storage — avoid clone() doubling RAM on admit.
    let pdf_id = match pdf_storage
        .create_pdf(CreatePdfRequest {
            workspace_id,
            filename: filename.clone(),
            content_type: "application/pdf".to_string(),
            file_size_bytes: file_size_bytes as i64,
            sha256_checksum: checksum.clone(),
            page_count,
            pdf_data: file_data,
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
                        ingestion_estimate: None,
                        metadata: PdfMetadata {
                            filename: existing.filename,
                            file_size_bytes: existing.file_size_bytes,
                            page_count: existing.page_count,
                            sha256_checksum: existing.sha256_checksum,
                            vision_enabled: options.enable_vision,
                            vision_model: existing.vision_model,
                        },
                        duplicate_of: Some(existing_pdf_id),
                        queue_position: None,
                        eta_seconds: None,
                        eta_basis: None,
                    });
                }
            }
            return Err(ApiError::Internal(format!("Failed to store PDF: {}", e)));
        }
    };

    let enqueue = create_pdf_processing_task(
        state,
        context,
        pdf_id,
        &options,
        workspace.as_ref(),
        super::helpers::PdfReprocessIntent::fresh(), // fresh upload — mint a new document id
        page_count,
        file_size_bytes,
    )
    .await?;

    let workspace_id = context
        .workspace_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("Workspace ID required".to_string()))?;
    let tenant_id = context
        .tenant_id_uuid()
        .ok_or_else(|| ApiError::BadRequest("Tenant ID required".to_string()))?;

    crate::services::provision_queued_pdf_document_shell(
        &state.storage.kv_storage,
        &enqueue.document_id,
        &crate::services::QueuedPdfDocumentShell {
            pdf_id,
            filename: filename.clone(),
            tenant_id,
            workspace_id,
            track_id: enqueue.track_id.clone(),
            file_size_bytes: file_size_bytes as i64,
            sha256_checksum: checksum.clone(),
            page_count,
        },
    )
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to provision queued document: {e}")))?;

    // SPEC-146: dual-write security labels into KV + documents columns.
    let security_labels =
        parse_security_labels(options.metadata.as_ref(), &security_form);
    let metadata_key =
        crate::services::document_metadata_scan::metadata_key_for_document(&enqueue.document_id);
    if let Ok(Some(mut metadata)) = state.storage.kv_storage.get_by_id(&metadata_key).await {
        apply_security_labels_to_metadata(&mut metadata, &security_labels);
        let _ = crate::services::upsert_metadata_kv_with_index(
            state.storage.kv_storage.as_ref(),
            &metadata_key,
            metadata,
        )
        .await;
    }
    finish_security_admit(
        state,
        context,
        &enqueue.document_id,
        &security_labels,
        context.user_id.as_deref(),
    )
    .await;

    if let Err(error) = crate::services::provision_relational_document_shell(
        state,
        &enqueue.document_id,
        &crate::services::RelationalDocumentShell {
            title: filename.clone(),
            tenant_id,
            workspace_id,
            track_id: enqueue.track_id.clone(),
            source_type: "pdf".to_string(),
            file_size_bytes: file_size_bytes.min(i64::MAX as u64) as i64,
            content_hash: Some(checksum.clone()),
            content_type: Some("application/pdf".to_string()),
        },
    )
    .await
    {
        tracing::warn!(
            document_id = %enqueue.document_id,
            track_id = %enqueue.track_id,
            error = %error,
            "PDF admitted but relational queue shell could not be projected"
        );
    }

    // SPEC-054 / #300: seed progress under server task_id only.
    seed_pdf_job_progress(
        state,
        &enqueue.track_id,
        &pdf_id.to_string(),
        &filename,
        options.track_id.as_deref(),
    )
    .await;

    let estimated_time = estimate_processing_time(
        file_size_bytes,
        page_count,
        resolved_backend,
        &options.resolved_vision_provider(workspace.as_ref(), tenant.as_ref()),
    );
    let ingestion_estimate = {
        let pages = page_count.unwrap_or(1).max(1) as usize;
        let profile = crate::services::LargeDocumentProfile::new(pages, file_size_bytes);
        Some(profile.ingestion_estimate(
            resolved_backend,
            &options.resolved_vision_provider(workspace.as_ref(), tenant.as_ref()),
        ))
    };

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
        document_id: Some(enqueue.document_id),
        status: "queued".to_string(),
        task_id: enqueue.track_id.to_string(),
        track_id: options.track_id,
        message: "PDF uploaded successfully. Processing in background.".to_string(),
        estimated_time_seconds: estimated_time,
        ingestion_estimate,
        metadata: PdfMetadata {
            filename,
            file_size_bytes: file_size_bytes as i64,
            page_count,
            sha256_checksum: checksum,
            vision_enabled: options.enable_vision,
            vision_model,
        },
        duplicate_of: None,
        queue_position: enqueue.queue.as_ref().map(|q| q.position),
        eta_seconds: enqueue.queue.as_ref().map(|q| q.eta_seconds),
        eta_basis: enqueue.queue.as_ref().map(|q| q.basis.as_str().to_string()),
    })
}
