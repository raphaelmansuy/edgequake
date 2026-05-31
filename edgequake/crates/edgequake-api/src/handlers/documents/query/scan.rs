//! Directory scan handler — scan filesystem and queue documents.

use axum::{extract::State, Json};
use chrono::Utc;
use tracing::debug;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::state::AppState;

use crate::handlers::documents_types::*;

// ============================================
// GAP-014: Document Scan API
// ============================================

/// Scan a directory and queue documents for processing.
///
/// SECURITY (OODA-248): Path traversal protection.
/// User-provided paths are validated against allowed directories.
#[utoipa::path(
    post,
    path = "/api/v1/documents/scan",
    tag = "Documents",
    request_body = ScanDirectoryRequest,
    responses(
        (status = 200, description = "Directory scanned successfully", body = ScanDirectoryResponse),
        (status = 400, description = "Invalid request"),
        (status = 403, description = "Path not allowed"),
        (status = 404, description = "Directory not found")
    )
)]
pub async fn scan_directory(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Json(request): Json<ScanDirectoryRequest>,
) -> ApiResult<Json<ScanDirectoryResponse>> {
    debug!(
        "scan_directory called with tenant context: tenant_id={:?}, workspace_id={:?}",
        tenant_ctx.tenant_id, tenant_ctx.workspace_id
    );

    // SECURITY (OODA-248): Validate path against allowed directories.
    // WHY: Prevents directory traversal attacks (e.g., ../../../etc/passwd).
    let validated_path =
        crate::path_validation::validate_path(&request.path, &state.path_validation_config)?;

    let base_path = &validated_path.canonical;

    // Path is already validated to exist by validate_path
    if !base_path.is_dir() {
        return Err(ApiError::BadRequest(format!(
            "Path is not a directory: {}",
            request.path
        )));
    }

    // Generate track ID
    let track_id = request.track_id.unwrap_or_else(|| {
        format!(
            "scan_{}_{}",
            Utc::now().format("%Y%m%d_%H%M%S"),
            &Uuid::new_v4().to_string()[..8]
        )
    });

    let mut queued_files = Vec::new();
    let mut skipped_files = Vec::new();
    let mut files_found = 0;

    // Collect files to process
    let entries = collect_files(base_path, request.recursive, request.max_files)?;

    for entry in entries {
        files_found += 1;

        let file_path = entry.path();
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // Check extension filter
        if !request.extensions.is_empty() {
            if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
                if !request
                    .extensions
                    .iter()
                    .any(|e| e.eq_ignore_ascii_case(ext))
                {
                    skipped_files.push(SkippedFile {
                        path: file_path.display().to_string(),
                        reason: format!("Extension .{} not in filter list", ext),
                    });
                    continue;
                }
            } else {
                skipped_files.push(SkippedFile {
                    path: file_path.display().to_string(),
                    reason: "No extension".to_string(),
                });
                continue;
            }
        }

        // Try to read file content
        let content = match std::fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => {
                skipped_files.push(SkippedFile {
                    path: file_path.display().to_string(),
                    reason: format!("Failed to read: {}", e),
                });
                continue;
            }
        };

        if content.trim().is_empty() {
            skipped_files.push(SkippedFile {
                path: file_path.display().to_string(),
                reason: "Empty file".to_string(),
            });
            continue;
        }

        // Check size limit
        if content.len() > state.config.max_document_size {
            skipped_files.push(SkippedFile {
                path: file_path.display().to_string(),
                reason: format!(
                    "Exceeds max size ({} > {})",
                    content.len(),
                    state.config.max_document_size
                ),
            });
            continue;
        }

        // Generate document ID
        let document_id = Uuid::new_v4().to_string();

        // Generate content summary
        let content_summary = crate::validation::generate_content_summary(&content);

        // Store document metadata
        let doc_metadata_key = format!("{}-metadata", document_id);
        let doc_metadata = serde_json::json!({
            "id": document_id,
            "title": file_name,
            "file_path": file_path.display().to_string(),
            "content_summary": content_summary,
            "content_length": content.len(),
            "track_id": track_id,
            "created_at": Utc::now().to_rfc3339(),
            "status": "pending",
        });
        state
            .storage
            .kv_storage
            .upsert(&[(doc_metadata_key, doc_metadata)])
            .await?;

        // Store document content
        let doc_content_key = format!("{}-content", document_id);
        let doc_content = serde_json::json!({
            "content": content,
        });
        state
            .storage
            .kv_storage
            .upsert(&[(doc_content_key, doc_content)])
            .await?;

        if request.async_processing {
            // Create task for background processing
            use edgequake_tasks::{Task, TaskType, TextInsertData};

            // Use tenant context for workspace_id, fallback to "default"
            let workspace_id = tenant_ctx
                .workspace_id
                .clone()
                .unwrap_or_else(|| "default".to_string());
            let tenant_id = tenant_ctx
                .tenant_id
                .clone()
                .unwrap_or_else(|| "default".to_string());

            let task_data = TextInsertData {
                text: content,
                file_source: file_path.display().to_string(),
                workspace_id: workspace_id.clone(),
                metadata: Some(serde_json::json!({
                    "document_id": document_id,
                    "title": file_name,
                    "track_id": track_id,
                    "tenant_id": tenant_id,
                    "workspace_id": workspace_id,
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

            state.enqueue_task(task).await?;
        }

        queued_files.push(file_path.display().to_string());
    }

    Ok(Json(ScanDirectoryResponse {
        track_id,
        files_found,
        files_queued: queued_files.len(),
        files_skipped: skipped_files.len(),
        queued_files,
        skipped_files,
    }))
}

/// Collect files from a directory.
fn collect_files(
    path: &std::path::Path,
    recursive: bool,
    max_files: usize,
) -> Result<Vec<std::fs::DirEntry>, ApiError> {
    let mut files = Vec::new();

    fn visit_dir(
        dir: &std::path::Path,
        recursive: bool,
        max_files: usize,
        files: &mut Vec<std::fs::DirEntry>,
    ) -> Result<(), ApiError> {
        if files.len() >= max_files {
            return Ok(());
        }

        let entries = std::fs::read_dir(dir).map_err(|e| {
            ApiError::Internal(format!("Failed to read directory {}: {}", dir.display(), e))
        })?;

        for entry in entries {
            if files.len() >= max_files {
                break;
            }

            let entry = entry.map_err(|e| {
                ApiError::Internal(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();

            if path.is_file() {
                files.push(entry);
            } else if path.is_dir() && recursive {
                visit_dir(&path, recursive, max_files, files)?;
            }
        }

        Ok(())
    }

    visit_dir(path, recursive, max_files, &mut files)?;
    Ok(files)
}
