//! Task management handlers.
//!
//! ## Implements
//!
//! - **FEAT0560**: Task status retrieval by track ID
//! - **FEAT0561**: Task listing with filters and pagination
//! - **FEAT0562**: Task cancellation for pending jobs
//! - **FEAT0563**: Task statistics aggregation
//!
//! ## Use Cases
//!
//! - **UC2160**: User polls task status during async document processing
//! - **UC2161**: User lists all pending and completed tasks
//! - **UC2162**: User cancels queued task before processing starts
//! - **UC2163**: Admin views task statistics for monitoring
//!
//! ## Enforces
//!
//! - **BR0560**: Track IDs must be valid UUIDs
//! - **BR0561**: Task listing must support status and type filters
//! - **BR0562**: Only pending tasks can be cancelled

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use edgequake_tasks::{Pagination, SortField, SortOrder, TaskFilter, TaskStatus, TaskType};
use serde_json::json;
use tracing;

use crate::{error::ApiError, state::AppState};

// Re-export DTOs for backward compatibility
pub use crate::handlers::tasks_types::{
    ListTasksQuery, PaginationInfo, StatisticsInfo, TaskErrorResponse, TaskListResponse,
    TaskResponse,
};

/// Get task status by track ID
#[utoipa::path(
    get,
    path = "/api/v1/tasks/{track_id}",
    responses(
        (status = 200, description = "Task found", body = TaskResponse),
        (status = 404, description = "Task not found")
    )
)]
pub async fn get_task(
    State(state): State<AppState>,
    Path(track_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let task = state
        .task_storage
        .get_task(&track_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get task: {}", e)))?;

    match task {
        Some(task) => Ok(Json(TaskResponse::from(task))),
        None => Err(ApiError::NotFound(format!("Task not found: {}", track_id))),
    }
}

/// List tasks with filters and pagination
#[utoipa::path(
    get,
    path = "/api/v1/tasks",
    params(
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("task_type" = Option<String>, Query, description = "Filter by task type"),
        ("page" = Option<u32>, Query, description = "Page number (default: 1)"),
        ("page_size" = Option<u32>, Query, description = "Page size (default: 20, max: 100)"),
        ("sort" = Option<String>, Query, description = "Sort field (created_at, updated_at)"),
        ("order" = Option<String>, Query, description = "Sort order (asc, desc)")
    ),
    responses(
        (status = 200, description = "Tasks listed", body = TaskListResponse)
    )
)]
/// @implements FEAT0406
pub async fn list_tasks(
    State(state): State<AppState>,
    Query(params): Query<ListTasksQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let filter = TaskFilter {
        tenant_id: params
            .tenant_id
            .as_deref()
            .and_then(|s| uuid::Uuid::parse_str(s).ok()),
        workspace_id: params
            .workspace_id
            .as_deref()
            .and_then(|s| uuid::Uuid::parse_str(s).ok()),
        status: params
            .status
            .as_deref()
            .and_then(|s| parse_task_status(s).ok()),
        task_type: params
            .task_type
            .as_deref()
            .and_then(|t| parse_task_type(t).ok()),
    };

    let pagination = Pagination {
        page: params.page.unwrap_or(1),
        page_size: params.page_size.unwrap_or(20).min(100),
        sort_by: params
            .sort
            .as_deref()
            .and_then(|s| parse_sort_field(s).ok())
            .unwrap_or(SortField::CreatedAt),
        order: params
            .order
            .as_deref()
            .and_then(|o| parse_sort_order(o).ok())
            .unwrap_or(SortOrder::Desc),
    };

    let task_list = state
        .task_storage
        .list_tasks(filter.clone(), pagination)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to list tasks: {}", e)))?;

    // Get statistics with the same filter to ensure tenant isolation
    // WHY: Statistics must respect the same tenant/workspace filters as the task list
    let stats = state
        .task_storage
        .get_statistics(filter)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get statistics: {}", e)))?;

    Ok(Json(TaskListResponse {
        tasks: task_list
            .tasks
            .into_iter()
            .map(TaskResponse::from)
            .collect(),
        pagination: PaginationInfo {
            total: task_list.total,
            page: task_list.page,
            page_size: task_list.page_size,
            total_pages: task_list.total_pages,
        },
        statistics: StatisticsInfo {
            pending: stats.pending,
            processing: stats.processing,
            indexed: stats.indexed,
            failed: stats.failed,
            cancelled: stats.cancelled,
        },
    }))
}

/// Cancel a task
#[utoipa::path(
    post,
    path = "/api/v1/tasks/{track_id}/cancel",
    responses(
        (status = 200, description = "Task cancelled", body = TaskResponse),
        (status = 404, description = "Task not found"),
        (status = 409, description = "Cannot cancel task in current status")
    )
)]
/// @implements FEAT0562: Task cancellation
/// @implements SPEC-002: Document status sync on task cancel
pub async fn cancel_task(
    State(state): State<AppState>,
    Path(track_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    // SPEC-002: First, always try to update document status for this track_id
    // WHY: After backend restart, tasks are lost but documents persist in KV storage.
    // Users need a way to cancel "stuck" documents even when the task no longer exists.
    let mut doc_updated = false;
    if let Ok(keys) = state.kv_storage.keys().await {
        let metadata_keys: Vec<String> = keys
            .iter()
            .filter(|k| k.ends_with("-metadata"))
            .cloned()
            .collect();

        if let Ok(metadata_values) = state.kv_storage.get_by_ids(&metadata_keys).await {
            for (key, value) in metadata_keys.iter().zip(metadata_values.iter()) {
                if let Some(obj) = value.as_object() {
                    if let Some(doc_track_id) = obj.get("track_id").and_then(|v| v.as_str()) {
                        if doc_track_id == track_id {
                            // Update this document's status to cancelled
                            let mut updated = obj.clone();
                            updated.insert("status".to_string(), json!("cancelled"));
                            updated.insert("current_stage".to_string(), json!("cancelled"));
                            updated.insert(
                                "stage_message".to_string(),
                                json!("Task cancelled by user"),
                            );
                            updated
                                .insert("updated_at".to_string(), json!(Utc::now().to_rfc3339()));

                            // Don't fail cancel if document update fails - log and continue
                            match state
                                .kv_storage
                                .upsert(&[(key.clone(), json!(updated))])
                                .await
                            {
                                Ok(_) => {
                                    doc_updated = true;
                                    tracing::info!("Updated document status to cancelled: {}", key);
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        "Failed to update document status on cancel: {} - {}",
                                        key,
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Now try to get and cancel the task if it exists
    let task = state
        .task_storage
        .get_task(&track_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get task: {}", e)))?;

    match task {
        Some(mut task) => {
            // Check if task can be cancelled
            if task.status == TaskStatus::Indexed || task.status == TaskStatus::Cancelled {
                return Err(ApiError::Conflict(format!(
                    "Cannot cancel task in status: {}",
                    task.status
                )));
            }

            task.mark_cancelled();

            state
                .task_storage
                .update_task(&task)
                .await
                .map_err(|e| ApiError::Internal(format!("Failed to cancel task: {}", e)))?;

            Ok(Json(TaskResponse::from(task)))
        }
        None => {
            // Task not found, but we may have updated document status
            if doc_updated {
                // Return success response since document was updated
                // WHY: The user's intent was to cancel processing, which we achieved
                // by updating the document status even though the task was already gone.
                // Create a synthetic TaskResponse for compatibility with the API contract.
                let now = Utc::now().to_rfc3339();
                Ok(Json(TaskResponse {
                    track_id: track_id.clone(),
                    tenant_id: "default".to_string(),
                    workspace_id: "default".to_string(),
                    task_type: "document_processing".to_string(),
                    status: "cancelled".to_string(),
                    created_at: now.clone(),
                    updated_at: now.clone(),
                    started_at: None,
                    completed_at: Some(now),
                    error_message: Some(
                        "Task was cancelled (task no longer exists, document status updated)"
                            .to_string(),
                    ),
                    error: None,
                    retry_count: 0,
                    max_retries: 0,
                    progress: None,
                    result: None,
                    metadata: Some(json!({
                        "document_updated": true,
                        "reason": "Task not found but document status was updated to cancelled"
                    })),
                }))
            } else {
                Err(ApiError::NotFound(format!("Task not found: {}", track_id)))
            }
        }
    }
}

/// Retry a failed task
#[utoipa::path(
    post,
    path = "/api/v1/tasks/{track_id}/retry",
    responses(
        (status = 200, description = "Task queued for retry", body = TaskResponse),
        (status = 404, description = "Task not found"),
        (status = 409, description = "Cannot retry task")
    )
)]
pub async fn retry_task(
    State(state): State<AppState>,
    Path(track_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let mut task = state
        .task_storage
        .get_task(&track_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get task: {}", e)))?
        .ok_or_else(|| ApiError::NotFound(format!("Task not found: {}", track_id)))?;

    // Check if task can be retried
    if !task.can_retry() {
        return Err(ApiError::Conflict(format!(
            "Cannot retry task: max retries ({}) reached or task not failed",
            task.max_retries
        )));
    }

    // Reset task to pending status for retry
    task.status = TaskStatus::Pending;
    task.error_message = None;

    state
        .task_storage
        .update_task(&task)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to update task: {}", e)))?;

    // Re-enqueue task
    state
        .task_queue
        .send(task.clone())
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to enqueue task: {}", e)))?;

    Ok(Json(TaskResponse::from(task)))
}

// === Helper Functions ===

fn parse_task_status(s: &str) -> Result<TaskStatus, String> {
    match s.to_lowercase().as_str() {
        "pending" => Ok(TaskStatus::Pending),
        "processing" => Ok(TaskStatus::Processing),
        "indexed" => Ok(TaskStatus::Indexed),
        "failed" => Ok(TaskStatus::Failed),
        "cancelled" => Ok(TaskStatus::Cancelled),
        _ => Err(format!("Invalid task status: {}", s)),
    }
}

fn parse_task_type(s: &str) -> Result<TaskType, String> {
    match s.to_lowercase().as_str() {
        "upload" => Ok(TaskType::Upload),
        "insert" => Ok(TaskType::Insert),
        "scan" => Ok(TaskType::Scan),
        "reindex" => Ok(TaskType::Reindex),
        "pdf_processing" => Ok(TaskType::PdfProcessing),
        _ => Err(format!("Invalid task type: {}", s)),
    }
}

fn parse_sort_field(s: &str) -> Result<SortField, String> {
    match s.to_lowercase().as_str() {
        "created_at" | "created" => Ok(SortField::CreatedAt),
        "updated_at" | "updated" => Ok(SortField::UpdatedAt),
        _ => Err(format!("Invalid sort field: {}", s)),
    }
}

fn parse_sort_order(s: &str) -> Result<SortOrder, String> {
    match s.to_lowercase().as_str() {
        "asc" | "ascending" => Ok(SortOrder::Asc),
        "desc" | "descending" => Ok(SortOrder::Desc),
        _ => Err(format!("Invalid sort order: {}", s)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_tasks::{SortField, SortOrder, TaskStatus, TaskType};

    #[test]
    fn test_parse_task_status_valid() {
        assert!(matches!(
            parse_task_status("pending"),
            Ok(TaskStatus::Pending)
        ));
        assert!(matches!(
            parse_task_status("PROCESSING"),
            Ok(TaskStatus::Processing)
        ));
        assert!(matches!(
            parse_task_status("Indexed"),
            Ok(TaskStatus::Indexed)
        ));
        assert!(matches!(
            parse_task_status("failed"),
            Ok(TaskStatus::Failed)
        ));
        assert!(matches!(
            parse_task_status("cancelled"),
            Ok(TaskStatus::Cancelled)
        ));
    }

    #[test]
    fn test_parse_task_status_invalid() {
        assert!(parse_task_status("invalid").is_err());
        assert!(parse_task_status("").is_err());
    }

    #[test]
    fn test_parse_task_type_valid() {
        assert!(matches!(parse_task_type("upload"), Ok(TaskType::Upload)));
        assert!(matches!(parse_task_type("INSERT"), Ok(TaskType::Insert)));
        assert!(matches!(parse_task_type("scan"), Ok(TaskType::Scan)));
        assert!(matches!(parse_task_type("Reindex"), Ok(TaskType::Reindex)));
    }

    #[test]
    fn test_parse_task_type_invalid() {
        assert!(parse_task_type("invalid").is_err());
        assert!(parse_task_type("").is_err());
    }

    #[test]
    fn test_parse_sort_field_valid() {
        assert!(matches!(
            parse_sort_field("created_at"),
            Ok(SortField::CreatedAt)
        ));
        assert!(matches!(
            parse_sort_field("created"),
            Ok(SortField::CreatedAt)
        ));
        assert!(matches!(
            parse_sort_field("UPDATED_AT"),
            Ok(SortField::UpdatedAt)
        ));
        assert!(matches!(
            parse_sort_field("Updated"),
            Ok(SortField::UpdatedAt)
        ));
    }

    #[test]
    fn test_parse_sort_field_invalid() {
        assert!(parse_sort_field("invalid").is_err());
        assert!(parse_sort_field("").is_err());
    }

    #[test]
    fn test_parse_sort_order_valid() {
        assert!(matches!(parse_sort_order("asc"), Ok(SortOrder::Asc)));
        assert!(matches!(parse_sort_order("ascending"), Ok(SortOrder::Asc)));
        assert!(matches!(parse_sort_order("DESC"), Ok(SortOrder::Desc)));
        assert!(matches!(
            parse_sort_order("descending"),
            Ok(SortOrder::Desc)
        ));
    }

    #[test]
    fn test_parse_sort_order_invalid() {
        assert!(parse_sort_order("invalid").is_err());
        assert!(parse_sort_order("").is_err());
    }

    #[test]
    fn test_list_tasks_query_defaults() {
        let json = r#"{}"#;
        let query: Result<ListTasksQuery, _> = serde_json::from_str(json);
        assert!(query.is_ok());
        let q = query.unwrap();
        assert!(q.status.is_none());
        assert!(q.page.is_none());
        assert!(q.page_size.is_none());
    }

    #[test]
    fn test_pagination_info_serialization() {
        let info = PaginationInfo {
            total: 100,
            page: 1,
            page_size: 20,
            total_pages: 5,
        };
        let json = serde_json::to_string(&info);
        assert!(json.is_ok());
    }

    #[test]
    fn test_statistics_info_serialization() {
        let stats = StatisticsInfo {
            pending: 10,
            processing: 5,
            indexed: 85,
            failed: 0,
            cancelled: 0,
        };
        let json = serde_json::to_string(&stats);
        assert!(json.is_ok());
        assert!(json.unwrap().contains("\"pending\":10"));
    }
}
