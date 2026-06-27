//! Text-based document upload handler.

use axum::http::StatusCode;
use axum::{extract::State, Json};
use tracing::debug;

use crate::error::ApiResult;
use crate::middleware::TenantContext;
use crate::services::ContentHasher;
use crate::state::AppState;

use crate::handlers::documents::upload::{
    admit_document_for_processing, parse_upload_chunk_fields, DocumentAdmissionInput,
    DocumentAdmissionOutcome, GleaningAdmissionOptions, ADMISSION_ACCEPTED_STATUS,
};
use crate::handlers::documents_types::*;

/// Upload a document for processing.
#[utoipa::path(
    post,
    path = "/api/v1/documents",
    tag = "Documents",
    params(
        ("X-Tenant-ID" = Option<String>, Header, description = "Tenant UUID for multi-tenant isolation"),
        ("X-Workspace-ID" = Option<String>, Header, description = "Workspace UUID — scopes uploaded documents"),
    ),
    request_body = UploadDocumentRequest,
    responses(
        (status = 202, description = "Document accepted for async processing", body = UploadDocumentResponse),
        (status = 400, description = "Invalid request"),
        (status = 413, description = "Document too large")
    )
)]
pub async fn upload_document(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Json(request): Json<UploadDocumentRequest>,
) -> ApiResult<(StatusCode, Json<UploadDocumentResponse>)> {
    debug!(
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        "Uploading document with tenant context"
    );

    crate::validation::validate_content(&request.content, state.config.max_document_size)?;

    if !request.async_processing {
        tracing::info!(
            "Sync upload requested but async is mandatory; enqueuing background task (P-G2b)"
        );
    }

    let content_hash = ContentHasher::hash_str(&request.content);
    let content_length = request.content.len();
    let title = request
        .title
        .clone()
        .unwrap_or_else(|| "Untitled".to_string());

    let (chunk_strategy, chunk_options) =
        parse_upload_chunk_fields(request.chunk_strategy.as_deref(), request.chunk_options);

    let outcome = admit_document_for_processing(
        &state,
        &tenant_ctx,
        DocumentAdmissionInput {
            text_content: request.content,
            title: title.clone(),
            source_type: "markdown",
            mime_type: None,
            raw_byte_size: content_length,
            content_hash,
            custom_metadata: request.metadata,
            track_id: request.track_id,
            gleaning: GleaningAdmissionOptions {
                enable_gleaning: request.enable_gleaning,
                max_gleaning: request.max_gleaning,
            },
            document_type: Some("markdown"),
            chunk_strategy,
            chunk_options,
            multimodal: false,
            ingest_mode: None,
            multimodal_manifest: None,
        },
        "upload",
    )
    .await?;

    match outcome {
        DocumentAdmissionOutcome::DuplicateProcessing(dup) => Ok((
            StatusCode::OK,
            Json(UploadDocumentResponse {
                document_id: dup.document_id.clone(),
                status: "duplicate_processing".to_string(),
                task_id: None,
                track_id: String::new(),
                duplicate_of: Some(dup.document_id),
                chunk_count: None,
                entity_count: None,
                relationship_count: None,
                cost: None,
            }),
        )),
        DocumentAdmissionOutcome::Accepted(accepted) => Ok((
            ADMISSION_ACCEPTED_STATUS,
            Json(UploadDocumentResponse {
                document_id: accepted.document_id,
                status: "pending".to_string(),
                task_id: Some(accepted.task_id),
                track_id: accepted.track_id,
                duplicate_of: None,
                chunk_count: None,
                entity_count: None,
                relationship_count: None,
                cost: None,
            }),
        )),
    }
}
