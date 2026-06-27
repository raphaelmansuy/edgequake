//! Multipart file upload handler.

use axum::http::StatusCode;
use axum::{extract::State, Json};
use tracing::debug;

use edgequake_audit::{AuditEventType, AuditResult};

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::services::{record_compliance_event, ContentHasher};
use crate::state::AppState;

use crate::handlers::documents::upload::{
    admit_document_for_processing, DocumentAdmissionInput, DocumentAdmissionOutcome,
    GleaningAdmissionOptions, MultipartUploadFields, ADMISSION_ACCEPTED_STATUS,
};
use crate::handlers::documents_types::*;
use crate::services::resolve_upload_content;
use axum_extra::extract::Multipart;

/// Upload a file via multipart form.
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
    let mut multipart_fields = MultipartUploadFields::default();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to read multipart field: {}", e)))?
    {
        let field_name = field.name().unwrap_or("").to_string();
        match field_name.as_str() {
            "file" => {
                filename = field
                    .file_name()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unnamed.txt".to_string());
                content = field
                    .bytes()
                    .await
                    .map_err(|e| {
                        ApiError::BadRequest(format!("Failed to read file content: {}", e))
                    })?
                    .to_vec();
            }
            "metadata" | "chunk_strategy" | "chunk_options" => {
                let text = field.text().await.map_err(|e| {
                    ApiError::BadRequest(format!("Failed to read {field_name}: {e}"))
                })?;
                multipart_fields.ingest_text_field(&field_name, &text);
            }
            _ => {}
        }
    }

    if content.is_empty() {
        return Err(ApiError::BadRequest("No file provided".to_string()));
    }

    let resolved =
        resolve_upload_content(&state, tenant_ctx.workspace_id_uuid(), &filename, &content).await?;
    let text_content = resolved.text_content;
    let mime_type = resolved.mime_type;
    let upload_meta = resolved.meta;

    let content_hash = ContentHasher::hash_bytes(&content);
    let (chunk_strategy, chunk_options, metadata) = multipart_fields.effective_chunk_fields();

    let outcome = admit_document_for_processing(
        &state,
        &tenant_ctx,
        DocumentAdmissionInput {
            text_content,
            title: filename.clone(),
            source_type: upload_meta.source_type,
            mime_type: Some(mime_type),
            raw_byte_size: content.len(),
            content_hash: content_hash.clone(),
            custom_metadata: metadata,
            track_id: None,
            gleaning: GleaningAdmissionOptions::default(),
            document_type: None,
            chunk_strategy,
            chunk_options,
            multimodal: upload_meta.multimodal,
            ingest_mode: upload_meta.ingest_mode,
            multimodal_manifest: resolved.manifest,
        },
        "upload",
    )
    .await?;

    match outcome {
        DocumentAdmissionOutcome::DuplicateProcessing(dup) => Ok((
            StatusCode::OK,
            Json(FileUploadResponse {
                document_id: dup.document_id,
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
        )),
        DocumentAdmissionOutcome::Accepted(accepted) => {
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
                Some(("document".to_string(), accepted.document_id.clone())),
            );

            Ok((
                ADMISSION_ACCEPTED_STATUS,
                Json(FileUploadResponse {
                    document_id: accepted.document_id,
                    filename,
                    size: content.len(),
                    content_hash: accepted.content_hash,
                    status: "pending".to_string(),
                    task_id: Some(accepted.task_id),
                    track_id: Some(accepted.track_id),
                    chunk_count: 0,
                    entity_count: 0,
                    relationship_count: 0,
                    is_duplicate: false,
                }),
            ))
        }
    }
}
