//! Task storage abstraction and implementations.

use crate::{error::TaskResult, types::Task, types::TaskStatus, types::TaskType};
use async_trait::async_trait;
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

    /// Get task statistics
    async fn get_statistics(&self) -> TaskResult<TaskStatistics>;
}

/// Task filter criteria
#[derive(Debug, Clone, Default)]
pub struct TaskFilter {
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

/// Type alias for shared storage
pub type SharedTaskStorage = Arc<dyn TaskStorage>;
