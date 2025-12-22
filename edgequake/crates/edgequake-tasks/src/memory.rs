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

    async fn list_tasks(
        &self,
        filter: TaskFilter,
        pagination: Pagination,
    ) -> TaskResult<TaskList> {
        let tasks = self.tasks.read().unwrap();
        
        // Filter tasks
        let mut filtered: Vec<Task> = tasks
            .values()
            .filter(|task| {
                let status_match = filter
                    .status
                    .map_or(true, |status| task.status == status);
                let type_match = filter
                    .task_type
                    .map_or(true, |task_type| task.task_type == task_type);
                status_match && type_match
            })
            .cloned()
            .collect();

        // Sort tasks
        match pagination.sort_by {
            SortField::CreatedAt => filtered.sort_by(|a, b| {
                match pagination.order {
                    SortOrder::Asc => a.created_at.cmp(&b.created_at),
                    SortOrder::Desc => b.created_at.cmp(&a.created_at),
                }
            }),
            SortField::UpdatedAt => filtered.sort_by(|a, b| {
                match pagination.order {
                    SortOrder::Asc => a.updated_at.cmp(&b.updated_at),
                    SortOrder::Desc => b.updated_at.cmp(&a.updated_at),
                }
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

    async fn get_statistics(&self) -> TaskResult<TaskStatistics> {
        use crate::types::TaskStatus;
        
        let tasks = self.tasks.read().unwrap();
        
        let mut stats = TaskStatistics {
            pending: 0,
            processing: 0,
            indexed: 0,
            failed: 0,
            cancelled: 0,
            total: tasks.len() as u64,
        };

        for task in tasks.values() {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{TaskStatus, TaskType};

    #[tokio::test]
    async fn test_create_and_get_task() {
        let storage = MemoryTaskStorage::new();
        let task = Task::new(
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
        let mut task1 = Task::new(TaskType::Upload, serde_json::json!({}));
        storage.create_task(&task1).await.unwrap();
        
        let mut task2 = Task::new(TaskType::Insert, serde_json::json!({}));
        task2.mark_processing();
        storage.create_task(&task2).await.unwrap();
        
        let mut task3 = Task::new(TaskType::Scan, serde_json::json!({}));
        task3.mark_success(serde_json::json!({"result": "ok"}));
        storage.create_task(&task3).await.unwrap();
        
        let stats = storage.get_statistics().await.unwrap();
        
        assert_eq!(stats.total, 3);
        assert_eq!(stats.pending, 1);
        assert_eq!(stats.processing, 1);
        assert_eq!(stats.indexed, 1);
    }
}
