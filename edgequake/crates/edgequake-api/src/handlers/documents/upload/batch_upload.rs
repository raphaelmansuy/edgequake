//! Batch file upload handler.

use axum::http::StatusCode;
use axum::{extract::State, Json};

use crate::error::{ApiError, ApiResult};
use crate::handlers::documents::upload::{
    admit_document_for_processing, DocumentAdmissionInput, DocumentAdmissionOutcome,
    GleaningAdmissionOptions, MultipartUploadFields,
};
use crate::handlers::documents_types::*;
use crate::middleware::TenantContext;
use crate::services::{resolve_upload_content, ContentHasher};
use crate::state::AppState;
use axum_extra::extract::Multipart;

/// Upload multiple files via multipart form.
#[utoipa::path(
    post,
    path = "/api/v1/documents/upload/batch",
    tag = "Documents",
    params(
        ("X-Tenant-ID" = Option<String>, Header, description = "Tenant UUID for multi-tenant isolation"),
        ("X-Workspace-ID" = Option<String>, Header, description = "Workspace UUID — scopes uploaded documents"),
    ),
    request_body(content_type = "multipart/form-data", description = "Files to upload"),
    responses(
        (status = 202, description = "Batch accepted for async processing", body = BatchUploadResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn upload_files_batch(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    mut multipart: Multipart,
) -> ApiResult<(StatusCode, Json<BatchUploadResponse>)> {
    let mut results = Vec::new();
    let mut processed = 0usize;
    let mut duplicates = 0usize;
    let mut failed = 0usize;
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut multipart_fields = MultipartUploadFields::default();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to read multipart field: {}", e)))?
    {
        let field_name = field.name().unwrap_or("").to_string();
        match field_name.as_str() {
            "files" | "file" => {
                let filename = field
                    .file_name()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("file_{}.txt", files.len()));
                let content = field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::BadRequest(format!("Failed to read file: {}", e)))?
                    .to_vec();
                files.push((filename, content));
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

    let (batch_chunk_strategy, batch_chunk_options, batch_metadata) =
        multipart_fields.effective_chunk_fields();

    for (filename, content) in files {
        match enqueue_single_file(
            &state,
            &tenant_ctx,
            &filename,
            &content,
            batch_chunk_strategy,
            batch_chunk_options.clone(),
            batch_metadata.clone(),
        )
        .await
        {
            Ok((doc_id, is_duplicate)) => {
                if is_duplicate {
                    duplicates += 1;
                    results.push(BatchFileResult {
                        filename,
                        document_id: Some(doc_id),
                        status: "duplicate".to_string(),
                        error: None,
                    });
                } else {
                    processed += 1;
                    results.push(BatchFileResult {
                        filename,
                        document_id: Some(doc_id),
                        status: "pending".to_string(),
                        error: None,
                    });
                }
            }
            Err(e) => {
                failed += 1;
                results.push(BatchFileResult {
                    filename,
                    document_id: None,
                    status: "failed".to_string(),
                    error: Some(e.to_string()),
                });
            }
        }
    }

    Ok((
        StatusCode::ACCEPTED,
        Json(BatchUploadResponse {
            total_files: results.len(),
            processed,
            duplicates,
            failed,
            results,
        }),
    ))
}

async fn enqueue_single_file(
    state: &AppState,
    tenant_ctx: &TenantContext,
    filename: &str,
    content: &[u8],
    chunk_strategy: Option<edgequake_pipeline::ChunkStrategy>,
    chunk_options: Option<edgequake_pipeline::ChunkOptions>,
    custom_metadata: Option<serde_json::Value>,
) -> Result<(String, bool), ApiError> {
    let resolved =
        resolve_upload_content(state, tenant_ctx.workspace_id_uuid(), filename, content).await?;
    let content_hash = ContentHasher::hash_bytes(content);

    let outcome = admit_document_for_processing(
        state,
        tenant_ctx,
        DocumentAdmissionInput {
            text_content: resolved.text_content,
            title: filename.to_string(),
            source_type: resolved.meta.source_type,
            mime_type: Some(resolved.mime_type),
            raw_byte_size: content.len(),
            content_hash,
            custom_metadata,
            track_id: None,
            gleaning: GleaningAdmissionOptions::default(),
            document_type: None,
            chunk_strategy,
            chunk_options,
            multimodal: resolved.meta.multimodal,
            ingest_mode: resolved.meta.ingest_mode,
            multimodal_manifest: resolved.manifest,
        },
        "batch",
    )
    .await?;

    match outcome {
        DocumentAdmissionOutcome::DuplicateProcessing(dup) => Ok((dup.document_id, true)),
        DocumentAdmissionOutcome::Accepted(accepted) => Ok((accepted.document_id, false)),
    }
}
