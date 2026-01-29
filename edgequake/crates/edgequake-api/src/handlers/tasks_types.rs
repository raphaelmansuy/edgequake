//! Task DTO types.
//!
//! This module contains all Data Transfer Objects for the task management API.
//! Extracted from tasks.rs for modularity and single responsibility.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ============================================================================
// Request DTOs
// ============================================================================

/// Query parameters for listing tasks.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ListTasksQuery {
    /// Filter by tenant ID (REQUIRED for multi-tenancy isolation).
    pub tenant_id: Option<String>,

    /// Filter by workspace ID (REQUIRED for workspace-level isolation).
    pub workspace_id: Option<String>,

    /// Filter by task status.
    pub status: Option<String>,

    /// Filter by task type.
    pub task_type: Option<String>,

    /// Page number (1-indexed).
    pub page: Option<u32>,

    /// Number of items per page.
    pub page_size: Option<u32>,

    /// Sort field (created_at, updated_at, status).
    pub sort: Option<String>,

    /// Sort order (asc, desc).
    pub order: Option<String>,
}

// ============================================================================
// Response DTOs
// ============================================================================

/// Task response.
#[derive(Debug, Serialize, ToSchema)]
pub struct TaskResponse {
    /// Task tracking ID.
    pub track_id: String,

    /// Tenant ID for multi-tenancy isolation.
    pub tenant_id: String,

    /// Workspace ID for workspace-level isolation.
    pub workspace_id: String,

    /// Type of task.
    pub task_type: String,

    /// Current status.
    pub status: String,

    /// Creation timestamp (RFC3339).
    pub created_at: String,

    /// Last update timestamp (RFC3339).
    pub updated_at: String,

    /// Start timestamp (RFC3339).
    pub started_at: Option<String>,

    /// Completion timestamp (RFC3339).
    pub completed_at: Option<String>,

    /// Simple error message (backward compatibility).
    pub error_message: Option<String>,

    /// Detailed error information (Phase 1 enhancement).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<TaskErrorResponse>,

    /// Current retry count.
    pub retry_count: i32,

    /// Maximum retry attempts.
    pub max_retries: i32,

    /// Task progress data.
    pub progress: Option<serde_json::Value>,

    /// Task result data.
    pub result: Option<serde_json::Value>,

    /// Task metadata.
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

/// List of tasks with pagination and statistics.
#[derive(Debug, Serialize, ToSchema)]
pub struct TaskListResponse {
    /// Tasks in the current page.
    pub tasks: Vec<TaskResponse>,

    /// Pagination metadata.
    pub pagination: PaginationInfo,

    /// Task statistics.
    pub statistics: StatisticsInfo,
}

/// Pagination information.
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginationInfo {
    /// Total number of tasks.
    pub total: u64,

    /// Current page (1-indexed).
    pub page: u32,

    /// Items per page.
    pub page_size: u32,

    /// Total number of pages.
    pub total_pages: u32,
}

/// Task statistics by status.
#[derive(Debug, Serialize, ToSchema)]
pub struct StatisticsInfo {
    /// Number of pending tasks.
    pub pending: u64,

    /// Number of processing tasks.
    pub processing: u64,

    /// Number of indexed tasks.
    pub indexed: u64,

    /// Number of failed tasks.
    pub failed: u64,

    /// Number of cancelled tasks.
    pub cancelled: u64,
}

// ============================================================================
// From Trait Implementation
// ============================================================================

impl From<edgequake_tasks::Task> for TaskResponse {
    fn from(task: edgequake_tasks::Task) -> Self {
        Self {
            track_id: task.track_id,
            tenant_id: task.tenant_id.to_string(),
            workspace_id: task.workspace_id.to_string(),
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

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_tasks_query_minimal() {
        let json = r#"{}"#;
        let query: ListTasksQuery = serde_json::from_str(json).unwrap();
        assert!(query.status.is_none());
        assert!(query.page.is_none());
    }

    #[test]
    fn test_list_tasks_query_full() {
        let json = r#"{
            "status": "pending",
            "task_type": "indexing",
            "page": 2,
            "page_size": 50,
            "sort": "created_at",
            "order": "desc"
        }"#;
        let query: ListTasksQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.status, Some("pending".to_string()));
        assert_eq!(query.task_type, Some("indexing".to_string()));
        assert_eq!(query.page, Some(2));
        assert_eq!(query.page_size, Some(50));
        assert_eq!(query.sort, Some("created_at".to_string()));
        assert_eq!(query.order, Some("desc".to_string()));
    }

    #[test]
    fn test_task_response_serialization() {
        let response = TaskResponse {
            track_id: "task_123".to_string(),
            tenant_id: "tenant-001".to_string(),
            workspace_id: "workspace-001".to_string(),
            task_type: "indexing".to_string(),
            status: "processing".to_string(),
            created_at: "2026-01-07T12:00:00Z".to_string(),
            updated_at: "2026-01-07T12:05:00Z".to_string(),
            started_at: Some("2026-01-07T12:01:00Z".to_string()),
            completed_at: None,
            error_message: None,
            error: None,
            retry_count: 0,
            max_retries: 3,
            progress: None,
            result: None,
            metadata: None,
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["track_id"], "task_123");
        assert_eq!(json["tenant_id"], "tenant-001");
        assert_eq!(json["workspace_id"], "workspace-001");
        assert_eq!(json["status"], "processing");
        assert!(json.get("error").is_none());
    }

    #[test]
    fn test_task_error_response() {
        let error = TaskErrorResponse {
            message: "Failed to process document".to_string(),
            step: "text_extraction".to_string(),
            reason: "Invalid PDF format".to_string(),
            suggestion: "Verify the PDF file is not corrupted".to_string(),
            retryable: false,
        };
        let json = serde_json::to_value(&error).unwrap();
        assert_eq!(json["message"], "Failed to process document");
        assert_eq!(json["retryable"], false);
    }

    #[test]
    fn test_task_list_response() {
        let response = TaskListResponse {
            tasks: vec![],
            pagination: PaginationInfo {
                total: 100,
                page: 1,
                page_size: 20,
                total_pages: 5,
            },
            statistics: StatisticsInfo {
                pending: 10,
                processing: 5,
                indexed: 80,
                failed: 5,
                cancelled: 0,
            },
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["pagination"]["total"], 100);
        assert_eq!(json["statistics"]["indexed"], 80);
    }

    #[test]
    fn test_pagination_info() {
        let info = PaginationInfo {
            total: 150,
            page: 3,
            page_size: 25,
            total_pages: 6,
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["total"], 150);
        assert_eq!(json["page"], 3);
        assert_eq!(json["total_pages"], 6);
    }

    #[test]
    fn test_statistics_info() {
        let stats = StatisticsInfo {
            pending: 20,
            processing: 10,
            indexed: 200,
            failed: 5,
            cancelled: 2,
        };
        let json = serde_json::to_value(&stats).unwrap();
        assert_eq!(json["pending"], 20);
        assert_eq!(json["processing"], 10);
        assert_eq!(json["indexed"], 200);
        assert_eq!(json["failed"], 5);
        assert_eq!(json["cancelled"], 2);
    }

    #[test]
    fn test_task_response_with_error() {
        let response = TaskResponse {
            track_id: "task_456".to_string(),
            tenant_id: "tenant-002".to_string(),
            workspace_id: "workspace-002".to_string(),
            task_type: "indexing".to_string(),
            status: "failed".to_string(),
            created_at: "2026-01-07T12:00:00Z".to_string(),
            updated_at: "2026-01-07T12:10:00Z".to_string(),
            started_at: Some("2026-01-07T12:01:00Z".to_string()),
            completed_at: Some("2026-01-07T12:10:00Z".to_string()),
            error_message: Some("Processing failed".to_string()),
            error: Some(TaskErrorResponse {
                message: "Failed to extract text".to_string(),
                step: "text_extraction".to_string(),
                reason: "Unsupported format".to_string(),
                suggestion: "Convert to supported format".to_string(),
                retryable: false,
            }),
            retry_count: 3,
            max_retries: 3,
            progress: None,
            result: None,
            metadata: None,
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["status"], "failed");
        assert_eq!(json["retry_count"], 3);
        assert!(json["error"].is_object());
    }

    #[test]
    fn test_list_tasks_query_partial() {
        let json = r#"{"status": "failed", "page": 1}"#;
        let query: ListTasksQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.status, Some("failed".to_string()));
        assert_eq!(query.page, Some(1));
        assert!(query.task_type.is_none());
        assert!(query.sort.is_none());
    }
}
