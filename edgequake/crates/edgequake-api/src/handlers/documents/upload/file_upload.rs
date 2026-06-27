//! Multipart file upload handler.

use axum::http::StatusCode;
use axum::{extract::State, Json};
use chrono::Utc;
use tracing::debug;
use uuid::Uuid;

use edgequake_audit::{AuditEventType, AuditResult};

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::services::{record_compliance_event, ContentHasher};
use crate::state::AppState;

use crate::file_validation::{image_mime_type, is_image_extension, validate_file};
#[allow(unused_imports)]
use crate::handlers::documents::storage_helpers::get_workspace_vector_storage_with_fallback;
use crate::handlers::documents::storage_helpers::{
    resolve_workspace_duplicate_for_reingestion, DuplicateReingestAction,
};
use crate::handlers::documents::upload::image_extract::extract_text_from_image;
use crate::handlers::documents_types::*;
use axum_extra::extract::Multipart;

/// Upload a file via multipart form.
///
/// Supports text-based files: .txt, .md, .json, .csv, .html
#[utoipa::path(
    post,
    path = "/api/v1/documents/upload",
    tag = "Documents",
    params(
        ("X-Tenant-ID" = Option<String>, Header, description = "Tenant UUID for multi-tenant isolation"),
        ("X-Workspace-ID" = Option<String>, Header, description = "Workspace UUID — scopes uploaded documents"),
    ),
    request_body(content_type = "multipart/form-data", description = "File to upload"),
    responses(
        (status = 202, description = "File accepted for async processing", body = FileUploadResponse),
        (status = 201, description = "File uploaded successfully (deprecated sync)", body = FileUploadResponse),
        (status = 400, description = "Invalid file or request"),
        (status = 409, description = "Duplicate file (already processed)"),
        (status = 413, description = "File too large")
    )
)]
pub async fn upload_file(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    mut multipart: Multipart,
) -> ApiResult<(StatusCode, Json<FileUploadResponse>)> {
    debug!(
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        "Uploading file with tenant context"
    );

    let mut filename = String::new();
    let mut content = Vec::new();
    let mut metadata: Option<serde_json::Value> = None;

    // Process multipart fields
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to read multipart field: {}", e)))?
    {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "file" => {
                // Get filename
                filename = field
                    .file_name()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unnamed.txt".to_string());

                // Read file content
                content = field
                    .bytes()
                    .await
                    .map_err(|e| {
                        ApiError::BadRequest(format!("Failed to read file content: {}", e))
                    })?
                    .to_vec();
            }
            "metadata" => {
                // Optional metadata field
                let text = field
                    .text()
                    .await
                    .map_err(|e| ApiError::BadRequest(format!("Failed to read metadata: {}", e)))?;

                if !text.is_empty() {
                    metadata = serde_json::from_str(&text).ok();
                }
            }
            _ => {
                // Ignore unknown fields
            }
        }
    }

    // Validate we got a file
    if content.is_empty() {
        return Err(ApiError::BadRequest("No file provided".to_string()));
    }

    // Determine if this is an image or a text-based document.
    let raw_ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    let (text_content, mime_type) = if is_image_extension(&raw_ext) {
        // ── Image path: extract text via the workspace vision LLM ────────────
        // WHY: Images are binary; the standard UTF-8 validation path would
        // reject them.  Instead we call the vision LLM once to extract all
        // readable text/structure, then treat the result as the document body.
        let mime = image_mime_type(&raw_ext).unwrap_or("image/png");
        // WHY: If the configured LLM doesn't support vision (e.g. Mistral text-only),
        // we still ingest the image as a document with a descriptive placeholder rather
        // than returning a hard error to the user.
        let extracted = match extract_text_from_image(
            &content,
            mime,
            &filename,
            state.query.llm_provider.as_ref(),
        )
        .await
        {
            Ok(text) => text,
            Err(e) => {
                tracing::warn!(
                    filename = %filename,
                    error = %e,
                    "Vision extraction failed; storing image with placeholder text"
                );
                format!(
                    "# Image Document: {filename}\n\n\
                     *Automatic text extraction failed: {e}*\n\n\
                     Configure a vision-capable LLM (e.g., gpt-4o, gemma3:12b, llava) \
                     to enable OCR/text extraction from image uploads."
                )
            }
        };
        (extracted, mime)
    } else {
        // ── Text path: validate size, extension, and UTF-8 ───────────────────
        let (_, text, mt) = validate_file(&filename, &content, state.config.max_document_size)?;
        (text, mt)
    };

    // WHY-OODA83: Use ContentHasher service for consistent hash computation (DRY)
    let content_hash = ContentHasher::hash_bytes(&content);
    debug!(content_hash = %content_hash, "Computed content hash");

    // Extract tenant context for workspace-scoped uniqueness
    // WHY-OODA81: Uniqueness must be scoped to workspace, not global
    // Same document in different workspaces is allowed (multi-tenancy)
    let workspace_id_for_storage = tenant_ctx.workspace_id_or_default();

    // WHY-OODA81+83: Use ContentHasher for workspace-scoped hash key
    // FIX-4: Duplicates now trigger re-ingestion instead of rejection
    let hash_key = ContentHasher::workspace_hash_key(&workspace_id_for_storage, &content_hash);
    debug!(hash_key = %hash_key, workspace_id = %workspace_id_for_storage, "Checking for workspace-scoped duplicate hash");
    match resolve_workspace_duplicate_for_reingestion(&state, &hash_key, &workspace_id_for_storage)
        .await?
    {
        DuplicateReingestAction::NoDuplicate => {}
        DuplicateReingestAction::ClearedForReingestion { old_document_id } => {
            tracing::info!(
                old_doc_id = %old_document_id,
                workspace_id = %workspace_id_for_storage,
                filename = %filename,
                "Duplicate file found - old data deleted, proceeding with re-ingestion"
            );
        }
        DuplicateReingestAction::StillProcessing {
            existing_document_id,
        } => {
            tracing::warn!(
                old_doc_id = %existing_document_id,
                filename = %filename,
                "Duplicate file is still being processed - cannot re-ingest"
            );
            return Ok((
                StatusCode::OK,
                Json(FileUploadResponse {
                    document_id: existing_document_id,
                    filename,
                    size: content.len(),
                    content_hash,
                    status: "duplicate_processing".to_string(),
                    task_id: None,
                    track_id: None,
                    chunk_count: 0,
                    entity_count: 0,
                    relationship_count: 0,
                    is_duplicate: true,
                }),
            ));
        }
    }

    // Generate document ID
    let document_id = Uuid::new_v4().to_string();

    // Store hash mapping for deduplication (workspace-scoped)
    state
        .storage
        .kv_storage
        .upsert(&[(hash_key, serde_json::json!(document_id))])
        .await?;

    // Generate content summary
    let content_summary = crate::validation::generate_content_summary(&text_content);

    // Generate track ID
    let track_id = format!(
        "upload_{}_{}",
        Utc::now().format("%Y%m%d%H%M%S"),
        &Uuid::new_v4().to_string()[..8]
    );

    // Store comprehensive document metadata
    let doc_metadata_key = format!("{}-metadata", document_id);
    let workspace_id = tenant_ctx.workspace_id_or_default();
    let tenant_id = tenant_ctx.tenant_id_or_default();
    let doc_metadata = serde_json::json!({
        "id": document_id,
        "title": filename,
        "file_name": filename,
        "file_size": content.len(),
        "mime_type": mime_type,
        "source_type": "file",
        "content_summary": content_summary,
        "content_length": text_content.len(),
        "content_hash": content_hash,
        "track_id": track_id,
        "created_at": Utc::now().to_rfc3339(),
        "status": "pending",
        "tenant_id": tenant_id,
        "workspace_id": workspace_id,
        "custom_metadata": metadata,
    });
    state
        .storage
        .kv_storage
        .upsert(&[(doc_metadata_key.clone(), doc_metadata)])
        .await?;

    // Store document content
    let doc_content_key = format!("{}-content", document_id);
    let doc_content = serde_json::json!({
        "content": text_content,
    });
    state
        .storage
        .kv_storage
        .upsert(&[(doc_content_key, doc_content)])
        .await?;

    // SPEC-024 Phase 1.1: async worker path (same as text_upload / P-G2b).
    use edgequake_tasks::{Task, TaskType, TextInsertData};

    let task_data = TextInsertData {
        text: text_content.clone(),
        file_source: filename.clone(),
        workspace_id: workspace_id.clone(),
        metadata: Some(serde_json::json!({
            "document_id": document_id,
            "title": filename,
            "tenant_id": tenant_id,
            "workspace_id": workspace_id,
            "source_type": "file",
            "mime_type": mime_type,
            "content_hash": content_hash,
        })),
    };

    let task = Task::new(
        uuid::Uuid::parse_str(&tenant_id)
            .map_err(|_| ApiError::ValidationError("Invalid tenant ID".to_string()))?,
        uuid::Uuid::parse_str(&workspace_id)
            .map_err(|_| ApiError::ValidationError("Invalid workspace ID".to_string()))?,
        TaskType::Insert,
        serde_json::to_value(task_data).unwrap(),
    );
    let task_id = task.track_id.clone();

    state.enqueue_task(task).await?;

    let tenant_for_audit = tenant_ctx
        .tenant_id
        .clone()
        .unwrap_or_else(|| "default".to_string());
    record_compliance_event(
        &state,
        tenant_for_audit,
        AuditEventType::DocumentUpload,
        "upload_file",
        AuditResult::Success,
        tenant_ctx.workspace_id.clone(),
        tenant_ctx.user_id.clone(),
        Some(("document".to_string(), document_id.clone())),
    );

    Ok((
        StatusCode::ACCEPTED,
        Json(FileUploadResponse {
            document_id,
            filename,
            size: content.len(),
            content_hash,
            status: "pending".to_string(),
            task_id: Some(task_id),
            track_id: Some(track_id),
            chunk_count: 0,
            entity_count: 0,
            relationship_count: 0,
            is_duplicate: false,
        }),
    ))
}
