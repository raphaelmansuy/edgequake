//! Batch file upload handler.

use axum::http::StatusCode;
use axum::{extract::State, Json};
use chrono::Utc;

use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::services::ContentHasher;
use crate::state::AppState;

use crate::file_validation::validate_file;
use crate::handlers::documents::storage_helpers::{
    resolve_workspace_duplicate_for_reingestion, DuplicateReingestAction,
};
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
        (status = 202, description = "Batch accepted for async processing", body = BatchUploadResponse),
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
        let result = enqueue_single_file(
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

/// Enqueue a single file for async worker processing (SPEC-024 Phase 1.1).
async fn enqueue_single_file(
    state: &AppState,
    tenant_ctx: &TenantContext,
    filename: &str,
    content: &[u8],
    workspace_id: &str,
    tenant_id: Option<String>,
) -> Result<(String, bool), ApiError> {
    let (_extension, text_content, mime_type) =
        validate_file(filename, content, state.config.max_document_size)?;

    let content_hash = ContentHasher::hash_bytes(content);
    let hash_key = ContentHasher::workspace_hash_key(workspace_id, &content_hash);
    match resolve_workspace_duplicate_for_reingestion(state, &hash_key, workspace_id).await? {
        DuplicateReingestAction::NoDuplicate => {}
        DuplicateReingestAction::ClearedForReingestion { old_document_id } => {
            tracing::info!(
                old_doc_id = %old_document_id,
                workspace_id = %workspace_id,
                filename = %filename,
                "Batch duplicate — cleared for re-ingestion (SPEC-024 pass 12)"
            );
        }
        DuplicateReingestAction::StillProcessing {
            existing_document_id,
        } => {
            return Ok((existing_document_id, true));
        }
    }

    let document_id = Uuid::new_v4().to_string();
    let track_id = format!(
        "batch_{}_{}",
        Utc::now().format("%Y%m%d%H%M%S"),
        &Uuid::new_v4().to_string()[..8]
    );

    state
        .storage
        .kv_storage
        .upsert(&[(hash_key, serde_json::json!(document_id))])
        .await?;

    let doc_metadata_key = format!("{}-metadata", document_id);
    state
        .storage
        .kv_storage
        .upsert(&[(
            doc_metadata_key,
            serde_json::json!({
                "id": document_id,
                "title": filename,
                "file_name": filename,
                "source_type": "file",
                "content_hash": content_hash,
                "track_id": track_id,
                "status": "pending",
                "tenant_id": tenant_id,
                "workspace_id": workspace_id,
            }),
        )])
        .await?;

    state
        .storage
        .kv_storage
        .upsert(&[(
            format!("{}-content", document_id),
            serde_json::json!({ "content": text_content }),
        )])
        .await?;

    use edgequake_tasks::{Task, TaskType, TextInsertData};

    let tenant = tenant_ctx.tenant_id_or_default();
    let task_data = TextInsertData {
        text: text_content,
        file_source: filename.to_string(),
        workspace_id: workspace_id.to_string(),
        metadata: Some(serde_json::json!({
            "document_id": document_id,
            "title": filename,
            "tenant_id": tenant,
            "workspace_id": workspace_id,
            "source_type": "file",
            "mime_type": mime_type,
            "content_hash": content_hash,
        })),
    };

    let task = Task::new(
        uuid::Uuid::parse_str(&tenant)
            .map_err(|_| ApiError::ValidationError("Invalid tenant ID".to_string()))?,
        uuid::Uuid::parse_str(workspace_id)
            .map_err(|_| ApiError::ValidationError("Invalid workspace ID".to_string()))?,
        TaskType::Insert,
        serde_json::to_value(task_data).unwrap(),
    );

    state.enqueue_task(task).await?;

    Ok((document_id, false))
}
