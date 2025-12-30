//! Task management handlers.

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use edgequake_tasks::{Pagination, SortField, SortOrder, TaskFilter, TaskStatus, TaskType};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{error::ApiError, state::AppState};

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

// === Request/Response Types ===

#[derive(Debug, Deserialize, ToSchema)]
pub struct ListTasksQuery {
    pub status: Option<String>,
    pub task_type: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub sort: Option<String>,
    pub order: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TaskResponse {
    pub track_id: String,
    pub task_type: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    /// Simple error message (backward compatibility).
    pub error_message: Option<String>,
    /// Detailed error information (Phase 1 enhancement).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<TaskErrorResponse>,
    pub retry_count: i32,
    pub max_retries: i32,
    pub progress: Option<serde_json::Value>,
    pub result: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

/// Detailed error response for failed tasks.
#[derive(Debug, Serialize, ToSchema)]
pub struct TaskErrorResponse {
    /// High-level error message.
    pub message: String,
    /// Processing step where failure occurred.
    pub step: String,
    /// Specific reason for the failure.
    pub reason: String,
    /// Suggested action to fix the issue.
    pub suggestion: String,
    /// Whether this error is retryable.
    pub retryable: bool,
}

impl From<edgequake_tasks::Task> for TaskResponse {
    fn from(task: edgequake_tasks::Task) -> Self {
        Self {
            track_id: task.track_id,
            task_type: task.task_type.to_string(),
            status: task.status.to_string(),
            created_at: task.created_at.to_rfc3339(),
            updated_at: task.updated_at.to_rfc3339(),
            started_at: task.started_at.map(|t| t.to_rfc3339()),
            completed_at: task.completed_at.map(|t| t.to_rfc3339()),
            error_message: task.error_message,
            error: task.error.map(|e| TaskErrorResponse {
                message: e.message,
                step: e.step,
                reason: e.reason,
                suggestion: e.suggestion,
                retryable: e.retryable,
            }),
            retry_count: task.retry_count,
            max_retries: task.max_retries,
            progress: task.progress.and_then(|p| serde_json::to_value(p).ok()),
            result: task.result,
            metadata: task.metadata,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TaskListResponse {
    pub tasks: Vec<TaskResponse>,
    pub pagination: PaginationInfo,
    pub statistics: StatisticsInfo,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginationInfo {
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StatisticsInfo {
    pub pending: u64,
    pub processing: u64,
    pub indexed: u64,
    pub failed: u64,
    pub cancelled: u64,
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
