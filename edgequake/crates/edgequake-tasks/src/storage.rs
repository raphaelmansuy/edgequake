//! Task storage abstraction and implementations.

use crate::{error::TaskResult, types::Task, types::TaskStatus, types::TaskType};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Trait for task storage backends
#[async_trait]
pub trait TaskStorage: Send + Sync {
    /// Create a new task
    async fn create_task(&self, task: &Task) -> TaskResult<()>;

    /// Get task by track ID
    async fn get_task(&self, track_id: &str) -> TaskResult<Option<Task>>;

    /// Update existing task
    async fn update_task(&self, task: &Task) -> TaskResult<()>;

    /// Delete task by track ID
    async fn delete_task(&self, track_id: &str) -> TaskResult<()>;

    /// List tasks with filters and pagination
    async fn list_tasks(&self, filter: TaskFilter, pagination: Pagination) -> TaskResult<TaskList>;

    /// Get task statistics filtered by tenant/workspace
    ///
    /// WHY: Task statistics must respect tenant isolation to prevent cross-tenant data leakage.
    /// Without filtering, a user in tenant A could see processing counts from tenant B.
    async fn get_statistics(&self, filter: TaskFilter) -> TaskResult<TaskStatistics>;

    /// Get queue metrics for task queue visibility.
    ///
    /// @implements SPEC-001/Objective-B: Workspace-Level Task Queue Visibility
    ///
    /// Returns metrics including:
    /// - Pending/processing counts
    /// - Average and max wait times
    /// - Throughput (docs/minute)
    /// - Worker utilization
    ///
    /// **DEPRECATED**: Use `get_queue_metrics_filtered` for tenant isolation.
    async fn get_queue_metrics(&self) -> TaskResult<QueueMetrics> {
        self.get_queue_metrics_filtered(None, None).await
    }

    /// Get queue metrics filtered by tenant and workspace.
    ///
    /// @implements OODA-04: Multi-tenant isolation for queue metrics
    ///
    /// WHY: Queue metrics MUST respect tenant isolation to prevent cross-tenant
    /// data leakage. Without filtering, a user in workspace A could see the
    /// processing activity of workspace B, violating privacy and causing confusion.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - Optional tenant filter. If None, metrics for all tenants.
    /// * `workspace_id` - Optional workspace filter. If None, metrics for all workspaces.
    ///
    /// # Returns
    ///
    /// Queue metrics filtered to the specified tenant/workspace scope.
    async fn get_queue_metrics_filtered(
        &self,
        tenant_id: Option<uuid::Uuid>,
        workspace_id: Option<uuid::Uuid>,
    ) -> TaskResult<QueueMetrics>;
}

/// Task filter criteria
#[derive(Debug, Clone, Default)]
pub struct TaskFilter {
    pub tenant_id: Option<uuid::Uuid>,
    pub workspace_id: Option<uuid::Uuid>,
    pub status: Option<TaskStatus>,
    pub task_type: Option<TaskType>,
}

/// Pagination parameters
#[derive(Debug, Clone)]
pub struct Pagination {
    pub page: u32,
    pub page_size: u32,
    pub sort_by: SortField,
    pub order: SortOrder,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 20,
            sort_by: SortField::CreatedAt,
            order: SortOrder::Desc,
        }
    }
}

/// Sort field enum
#[derive(Debug, Clone, Copy)]
pub enum SortField {
    CreatedAt,
    UpdatedAt,
}

/// Sort order enum
#[derive(Debug, Clone, Copy)]
pub enum SortOrder {
    Asc,
    Desc,
}

/// Task list response
#[derive(Debug, Clone)]
pub struct TaskList {
    pub tasks: Vec<Task>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

/// Task statistics
#[derive(Debug, Clone)]
pub struct TaskStatistics {
    pub pending: u64,
    pub processing: u64,
    pub indexed: u64,
    pub failed: u64,
    pub cancelled: u64,
    pub total: u64,
}

/// Queue-level metrics for workspace processing visibility.
///
/// @implements SPEC-001/Objective-B: Workspace-Level Task Queue Visibility
///
/// WHY: Users need visibility into the task queue to understand:
/// - How many documents are waiting
/// - How long they'll have to wait
/// - What the system throughput is
///
/// ```text
/// ┌────────────────────────────────────────────────────────────────┐
/// │ WORKSPACE: default-workspace                                   │
/// ├────────────────────────────────────────────────────────────────┤
/// │ Documents:  Pending: 12  Processing: 3  Completed: 156        │
/// │             Failed: 2    Cancelled: 0                          │
/// ├────────────────────────────────────────────────────────────────┤
/// │ Throughput: 2.3 docs/min | Avg wait: 1m 42s                   │
/// └────────────────────────────────────────────────────────────────┘
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueMetrics {
    /// Documents waiting to be processed.
    pub pending_count: u64,

    /// Documents currently being processed.
    pub processing_count: u64,

    /// Active concurrent workers (tasks currently processing).
    pub active_workers: u32,

    /// Maximum concurrent workers allowed.
    pub max_workers: u32,

    /// Worker utilization percentage (0-100).
    pub worker_utilization: u8,

    /// Average wait time in seconds (time from created to started).
    pub avg_wait_time_seconds: f64,

    /// Maximum wait time in queue (oldest pending task).
    pub max_wait_time_seconds: f64,

    /// Documents processed per minute (rolling average).
    pub throughput_per_minute: f64,

    /// Estimated time for new document to start processing.
    pub estimated_queue_time_seconds: f64,

    /// Whether rate limiting is currently active.
    pub rate_limited: bool,

    /// Timestamp of this metrics snapshot.
    pub timestamp: DateTime<Utc>,
}

impl Default for QueueMetrics {
    fn default() -> Self {
        Self {
            pending_count: 0,
            processing_count: 0,
            active_workers: 0,
            max_workers: 4, // Default max workers
            worker_utilization: 0,
            avg_wait_time_seconds: 0.0,
            max_wait_time_seconds: 0.0,
            throughput_per_minute: 0.0,
            estimated_queue_time_seconds: 0.0,
            rate_limited: false,
            timestamp: Utc::now(),
        }
    }
}

/// Type alias for shared storage
pub type SharedTaskStorage = Arc<dyn TaskStorage>;
