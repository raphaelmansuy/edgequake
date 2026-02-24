//! Batch file upload handler.

use axum::http::StatusCode;
use axum::{extract::State, Json};

use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::services::ContentHasher;
use crate::state::AppState;

use crate::file_validation::validate_file;
#[allow(unused_imports)]
use crate::handlers::documents::storage_helpers::get_workspace_vector_storage_with_fallback;
use crate::handlers::documents_types::*;
use axum_extra::extract::Multipart;

/// Upload multiple files via multipart form.
#[utoipa::path(
    post,
    path = "/api/v1/documents/upload/batch",
    tag = "Documents",
    request_body(content_type = "multipart/form-data", description = "Files to upload"),
    responses(
        (status = 201, description = "Batch upload completed", body = BatchUploadResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn upload_files_batch(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> ApiResult<(StatusCode, Json<BatchUploadResponse>)> {
    let mut results = Vec::new();
    let mut processed = 0usize;
    let mut duplicates = 0usize;
    let mut failed = 0usize;

    // Collect all files first
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

    // Process each file (uses default workspace for batch uploads)
    // WHY-OODA81: Batch upload uses "default" workspace for dedup scoping
    // For proper workspace isolation, use the single file upload endpoint with tenant context
    let workspace_id = "default".to_string();
    for (filename, content) in files {
        let result = process_single_file(&state, &filename, &content, &workspace_id).await;

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
///
/// WHY-OODA81: workspace_id parameter enables workspace-scoped duplicate detection.
/// Same document in different workspaces is allowed (multi-tenancy support).
///
/// Note: Uses default vector storage for batch uploads without tenant context.
/// For workspace-specific storage, use the main upload_file endpoint with tenant context.
async fn process_single_file(
    state: &AppState,
    filename: &str,
    content: &[u8],
    workspace_id: &str,
) -> Result<(String, bool), ApiError> {
    // Validate file (size, extension, UTF-8, non-empty)
    let (_extension, text_content, _mime_type) =
        validate_file(filename, content, state.config.max_document_size)?;

    // WHY-OODA83: Use ContentHasher service for consistent hash computation (DRY)
    let content_hash = ContentHasher::hash_bytes(content);

    // WHY-OODA81+83: Use ContentHasher for workspace-scoped hash key
    let hash_key = ContentHasher::workspace_hash_key(workspace_id, &content_hash);
    if let Some(existing) = state.kv_storage.get_by_id(&hash_key).await? {
        if let Some(doc_id) = existing.as_str() {
            return Ok((doc_id.to_string(), true));
        }
    }

    // Generate document ID
    let document_id = Uuid::new_v4().to_string();

    // Store hash mapping
    state
        .kv_storage
        .upsert(&[(hash_key, serde_json::json!(document_id))])
        .await?;

    // Process through pipeline (resilient - tolerates partial chunk failures)
    let result = state
        .pipeline
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

    // Store chunks
    let chunks: Vec<(String, serde_json::Value)> = result
        .chunks
        .iter()
        .map(|c| {
            (
                c.id.clone(),
                serde_json::json!({
                    "content": c.content,
                    "document_id": document_id,
                    "index": c.index,
                    "source_file": filename,
                }),
            )
        })
        .collect();

    state.kv_storage.upsert(&chunks).await?;

    // Store chunk embeddings in vector storage for semantic search
    // Note: Batch upload uses default vector storage since there's no workspace context.
    // For workspace-specific storage, use the main upload_file endpoint with tenant context.
    for chunk in &result.chunks {
        if let Some(embedding) = &chunk.embedding {
            let metadata = serde_json::json!({
                "type": "chunk",
                "document_id": document_id,
                "index": chunk.index,
                "content": chunk.content,
                "source_file": filename,
            });

            let _ = state
                .vector_storage
                .upsert(&[(chunk.id.clone(), embedding.clone(), metadata)])
                .await;
        }
    }

    Ok((document_id, false))
}
