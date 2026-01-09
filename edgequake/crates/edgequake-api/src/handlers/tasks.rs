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
use edgequake_tasks::{Pagination, SortField, SortOrder, TaskFilter, TaskStatus, TaskType};

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
pub async fn list_tasks(
    State(state): State<AppState>,
    Query(params): Query<ListTasksQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let filter = TaskFilter {
        status: params
            .status
            .as_deref()
/// @implements FEAT0406
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
        .list_tasks(filter, pagination)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to list tasks: {}", e)))?;

    // Get statistics
    let stats = state
        .task_storage
        .get_statistics()
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
pub async fn cancel_task(
    State(state): State<AppState>,
    Path(track_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let mut task = state
        .task_storage
        .get_task(&track_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get task: {}", e)))?
        .ok_or_else(|| ApiError::NotFound(format!("Task not found: {}", track_id)))?;

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
