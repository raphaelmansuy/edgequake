//! In-memory task storage implementation for development and testing.

use crate::{
    error::{TaskError, TaskResult},
    storage::*,
    types::Task,
};
use async_trait::async_trait;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

/// In-memory task storage
#[derive(Debug, Clone)]
pub struct MemoryTaskStorage {
    tasks: Arc<RwLock<HashMap<String, Task>>>,
}

impl MemoryTaskStorage {
    /// Create a new memory storage
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for MemoryTaskStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TaskStorage for MemoryTaskStorage {
    async fn create_task(&self, task: &Task) -> TaskResult<()> {
        let mut tasks = self.tasks.write().unwrap();

        if tasks.contains_key(&task.track_id) {
            return Err(TaskError::StorageError(format!(
                "Task already exists: {}",
                task.track_id
            )));
        }

        tasks.insert(task.track_id.clone(), task.clone());
        Ok(())
    }

    async fn get_task(&self, track_id: &str) -> TaskResult<Option<Task>> {
        let tasks = self.tasks.read().unwrap();
        Ok(tasks.get(track_id).cloned())
    }

    async fn update_task(&self, task: &Task) -> TaskResult<()> {
        let mut tasks = self.tasks.write().unwrap();

        if !tasks.contains_key(&task.track_id) {
            return Err(TaskError::TaskNotFound(task.track_id.clone()));
        }

        tasks.insert(task.track_id.clone(), task.clone());
        Ok(())
    }

    async fn delete_task(&self, track_id: &str) -> TaskResult<()> {
        let mut tasks = self.tasks.write().unwrap();

        if tasks.remove(track_id).is_none() {
            return Err(TaskError::TaskNotFound(track_id.to_string()));
        }

        Ok(())
    }

    async fn list_tasks(&self, filter: TaskFilter, pagination: Pagination) -> TaskResult<TaskList> {
        let tasks = self.tasks.read().unwrap();

        // Filter tasks
        let mut filtered: Vec<Task> = tasks
            .values()
            .filter(|task| {
                let status_match = filter.status.is_none_or(|status| task.status == status);
                let type_match = filter
                    .task_type
                    .is_none_or(|task_type| task.task_type == task_type);
                status_match && type_match
            })
            .cloned()
            .collect();

        // Sort tasks
        match pagination.sort_by {
            SortField::CreatedAt => filtered.sort_by(|a, b| match pagination.order {
                SortOrder::Asc => a.created_at.cmp(&b.created_at),
                SortOrder::Desc => b.created_at.cmp(&a.created_at),
            }),
            SortField::UpdatedAt => filtered.sort_by(|a, b| match pagination.order {
                SortOrder::Asc => a.updated_at.cmp(&b.updated_at),
                SortOrder::Desc => b.updated_at.cmp(&a.updated_at),
            }),
        }

        let total = filtered.len() as u64;
        let total_pages = ((total as f64) / (pagination.page_size as f64)).ceil() as u32;

        // Paginate
        let start = ((pagination.page - 1) * pagination.page_size) as usize;
        let end = (start + pagination.page_size as usize).min(filtered.len());
        let page_tasks = filtered[start..end].to_vec();

        Ok(TaskList {
            tasks: page_tasks,
            total,
            page: pagination.page,
            page_size: pagination.page_size,
            total_pages,
        })
    }

    async fn get_statistics(&self, filter: TaskFilter) -> TaskResult<TaskStatistics> {
        use crate::types::TaskStatus;

        let tasks = self.tasks.read().unwrap();

        let mut stats = TaskStatistics {
            pending: 0,
            processing: 0,
            indexed: 0,
            failed: 0,
            cancelled: 0,
            total: 0,
        };

        // WHY: Apply same filtering logic as list_tasks to maintain tenant isolation
        for task in tasks.values() {
            // Skip tasks that don't match filters
            if let Some(tenant_id) = filter.tenant_id {
                if task.tenant_id != tenant_id {
                    continue;
                }
            }

            if let Some(workspace_id) = filter.workspace_id {
                if task.workspace_id != workspace_id {
                    continue;
                }
            }

            if let Some(status) = &filter.status {
                if &task.status != status {
                    continue;
                }
            }

            if let Some(task_type) = &filter.task_type {
                if &task.task_type != task_type {
                    continue;
                }
            }

            // Count this task
            stats.total += 1;
            match task.status {
                TaskStatus::Pending => stats.pending += 1,
                TaskStatus::Processing => stats.processing += 1,
                TaskStatus::Indexed => stats.indexed += 1,
                TaskStatus::Failed => stats.failed += 1,
                TaskStatus::Cancelled => stats.cancelled += 1,
            }
        }

        Ok(stats)
    }

    async fn get_queue_metrics_filtered(
        &self,
        tenant_id: Option<uuid::Uuid>,
        workspace_id: Option<uuid::Uuid>,
    ) -> TaskResult<QueueMetrics> {
        use crate::types::TaskStatus;
        use chrono::Utc;

        let tasks = self.tasks.read().unwrap();
        let now = Utc::now();

        let mut pending_count = 0u64;
        let mut processing_count = 0u64;
        let mut wait_times: Vec<f64> = Vec::new();
        let mut max_wait_time: f64 = 0.0;
        let mut recent_completed = 0u64;

        // 5-minute window for throughput calculation
        let five_minutes_ago = now - chrono::Duration::minutes(5);

        for task in tasks.values() {
            // OODA-04: Filter by tenant_id and workspace_id for multi-tenant isolation
            // WHY: Queue metrics MUST be scoped to the current tenant/workspace.
            if let Some(tid) = tenant_id {
                if task.tenant_id != tid {
                    continue;
                }
            }
            if let Some(wid) = workspace_id {
                if task.workspace_id != wid {
                    continue;
                }
            }

            match task.status {
                TaskStatus::Pending => {
                    pending_count += 1;
                    // Calculate wait time for pending tasks
                    let wait = (now - task.created_at).num_seconds() as f64;
                    if wait > max_wait_time {
                        max_wait_time = wait;
                    }
                }
                TaskStatus::Processing => {
                    processing_count += 1;
                    // Calculate wait time (time before processing started)
                    if let Some(started) = task.started_at {
                        let wait = (started - task.created_at).num_seconds() as f64;
                        wait_times.push(wait);
                    }
                }
                TaskStatus::Indexed => {
                    // Count recently completed for throughput
                    if let Some(completed) = task.completed_at {
                        if completed > five_minutes_ago {
                            recent_completed += 1;
                        }
                        // Include in wait time average
                        if let Some(started) = task.started_at {
                            let wait = (started - task.created_at).num_seconds() as f64;
                            wait_times.push(wait);
                        }
                    }
                }
                _ => {}
            }
        }

        // Calculate averages
        let avg_wait_time_seconds = if wait_times.is_empty() {
            0.0
        } else {
            wait_times.iter().sum::<f64>() / wait_times.len() as f64
        };

        // Throughput: documents per minute over last 5 minutes
        let throughput_per_minute = recent_completed as f64 / 5.0;

        // Estimate queue time based on throughput
        let estimated_queue_time_seconds = if throughput_per_minute > 0.0 {
            (pending_count as f64 / throughput_per_minute) * 60.0
        } else if avg_wait_time_seconds > 0.0 {
            pending_count as f64 * avg_wait_time_seconds
        } else {
            0.0
        };

        // Worker utilization (assuming 4 max workers)
        let max_workers = 4u32;
        let active_workers = processing_count.min(max_workers as u64) as u32;
        let worker_utilization = ((active_workers as f64 / max_workers as f64) * 100.0) as u8;

        Ok(QueueMetrics {
            pending_count,
            processing_count,
            active_workers,
            max_workers,
            worker_utilization,
            avg_wait_time_seconds,
            max_wait_time_seconds: max_wait_time,
            throughput_per_minute,
            estimated_queue_time_seconds,
            rate_limited: false, // TODO: Track rate limiting separately
            timestamp: now,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{TaskStatus, TaskType};

    const TEST_TENANT_ID: &str = "00000000-0000-0000-0000-000000000001";
    const TEST_WORKSPACE_ID: &str = "00000000-0000-0000-0000-000000000002";

    fn test_tenant_id() -> uuid::Uuid {
        uuid::Uuid::parse_str(TEST_TENANT_ID).unwrap()
    }

    fn test_workspace_id() -> uuid::Uuid {
        uuid::Uuid::parse_str(TEST_WORKSPACE_ID).unwrap()
    }

    #[tokio::test]
    async fn test_create_and_get_task() {
        let storage = MemoryTaskStorage::new();
        let task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Upload,
            serde_json::json!({"file_path": "/tmp/test.pdf"}),
        );

        storage.create_task(&task).await.unwrap();

        let retrieved = storage.get_task(&task.track_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().track_id, task.track_id);
    }

    #[tokio::test]
    async fn test_update_task() {
        let storage = MemoryTaskStorage::new();
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            serde_json::json!({"text": "test"}),
        );

        storage.create_task(&task).await.unwrap();

        task.mark_processing();
        storage.update_task(&task).await.unwrap();

        let retrieved = storage.get_task(&task.track_id).await.unwrap().unwrap();
        assert_eq!(retrieved.status, TaskStatus::Processing);
    }

    #[tokio::test]
    async fn test_delete_task() {
        let storage = MemoryTaskStorage::new();
        let task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Scan,
            serde_json::json!({"directory": "/data"}),
        );

        storage.create_task(&task).await.unwrap();
        storage.delete_task(&task.track_id).await.unwrap();

        let retrieved = storage.get_task(&task.track_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_list_tasks_with_filter() {
        let storage = MemoryTaskStorage::new();

        // Create multiple tasks
        for i in 0..5 {
            let mut task = Task::new(
                test_tenant_id(),
                test_workspace_id(),
                TaskType::Upload,
                serde_json::json!({"file": format!("file{}.pdf", i)}),
            );
            if i < 2 {
                task.mark_processing();
            }
            storage.create_task(&task).await.unwrap();
        }

        // Filter by processing status
        let filter = TaskFilter {
            tenant_id: None,
            workspace_id: None,
            status: Some(TaskStatus::Processing),
            task_type: None,
        };

        let result = storage
            .list_tasks(filter, Pagination::default())
            .await
            .unwrap();

        assert_eq!(result.tasks.len(), 2);
        assert_eq!(result.total, 2);
    }

    #[tokio::test]
    async fn test_get_statistics() {
        let storage = MemoryTaskStorage::new();

        // Create tasks with different statuses
        let task1 = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Upload,
            serde_json::json!({}),
        );
        storage.create_task(&task1).await.unwrap();

        let mut task2 = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            serde_json::json!({}),
        );
        task2.mark_processing();
        storage.create_task(&task2).await.unwrap();

        let mut task3 = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Scan,
            serde_json::json!({}),
        );
        task3.mark_success(serde_json::json!({"result": "ok"}));
        storage.create_task(&task3).await.unwrap();

        let stats = storage.get_statistics(TaskFilter::default()).await.unwrap();

        assert_eq!(stats.total, 3);
        assert_eq!(stats.pending, 1);
        assert_eq!(stats.processing, 1);
        assert_eq!(stats.indexed, 1);
    }
}
