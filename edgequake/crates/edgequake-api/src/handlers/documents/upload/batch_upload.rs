//! Batch file upload handler.

use axum::http::StatusCode;
use axum::{extract::State, Json};

use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::services::ContentHasher;
use crate::services::{
    build_chunk_kv_records, persist_ingestion_result, resolve_relational_sink,
    PersistIngestionParams,
};
use crate::state::AppState;

use crate::file_validation::validate_file;
use crate::handlers::documents::storage_helpers::get_workspace_vector_storage_strict;
use crate::handlers::documents_types::*;
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
        (status = 201, description = "Batch upload completed", body = BatchUploadResponse),
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

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to read multipart field: {}", e)))?
    {
        let field_name = field.name().unwrap_or("").to_string();

        if field_name == "files" || field_name == "file" {
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
    }

    let workspace_id = tenant_ctx.workspace_id_or_default();
    let tenant_id = tenant_ctx.tenant_id.clone();

    for (filename, content) in files {
        let result = process_single_file(
            &state,
            &tenant_ctx,
            &filename,
            &content,
            &workspace_id,
            tenant_id.clone(),
        )
        .await;

        match result {
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
                        status: "processed".to_string(),
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
        StatusCode::CREATED,
        Json(BatchUploadResponse {
            total_files: results.len(),
            processed,
            duplicates,
            failed,
            results,
        }),
    ))
}

/// Process a single file and return (document_id, is_duplicate).
async fn process_single_file(
    state: &AppState,
    _tenant_ctx: &TenantContext,
    filename: &str,
    content: &[u8],
    workspace_id: &str,
    tenant_id: Option<String>,
) -> Result<(String, bool), ApiError> {
    let (_extension, text_content, _mime_type) =
        validate_file(filename, content, state.config.max_document_size)?;

    let content_hash = ContentHasher::hash_bytes(content);
    let hash_key = ContentHasher::workspace_hash_key(workspace_id, &content_hash);
    if let Some(existing) = state.storage.kv_storage.get_by_id(&hash_key).await? {
        if let Some(doc_id) = existing.as_str() {
            return Ok((doc_id.to_string(), true));
        }
    }

    let document_id = Uuid::new_v4().to_string();

    state
        .storage
        .kv_storage
        .upsert(&[(hash_key, serde_json::json!(document_id))])
        .await?;

    let workspace_pipeline = state.create_workspace_pipeline(workspace_id).await;
    let result = workspace_pipeline
        .process_with_resilience(&document_id, &text_content, None)
        .await?;

    if result.stats.failed_chunks > 0 {
        tracing::warn!(
            document_id = %document_id,
            filename = %filename,
            failed_chunks = result.stats.failed_chunks,
            chunk_count = result.stats.chunk_count,
            "Batch file pipeline completed with partial failures"
        );
    }

    let chunks = build_chunk_kv_records(&document_id, filename, &result);
    state.storage.kv_storage.upsert(&chunks).await?;

    let workspace_vector_storage = get_workspace_vector_storage_strict(state, workspace_id).await?;
    let relational_sink = resolve_relational_sink(state).await;

    persist_ingestion_result(
        state,
        state.storage.graph_storage.clone(),
        workspace_vector_storage,
        relational_sink,
        PersistIngestionParams::for_document(
            &document_id,
            tenant_id,
            workspace_id.to_string(),
            &result,
            edgequake_pipeline::ChunkVectorBuildOptions::STANDARD,
        ),
    )
    .await
    .map_err(|e| ApiError::Internal(format!("Batch upload persist failed: {}", e)))?;

    Ok((document_id, false))
}
