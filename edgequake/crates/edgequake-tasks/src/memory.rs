//! In-memory task storage implementation for development and testing.

use crate::{
    error::{TaskError, TaskResult},
    storage::*,
    types::{Task, TaskStatus},
};
use async_trait::async_trait;
use chrono::Utc;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Duration,
};
use uuid::Uuid;

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

/// Shared filter predicate for list_tasks and get_statistics (tenant/workspace isolation).
fn task_matches_filter(task: &Task, filter: &TaskFilter) -> bool {
    if filter
        .tenant_id
        .is_some_and(|tenant_id| task.tenant_id != tenant_id)
    {
        return false;
    }
    if filter
        .workspace_id
        .is_some_and(|workspace_id| task.workspace_id != workspace_id)
    {
        return false;
    }
    if filter.status.is_some_and(|status| task.status != status) {
        return false;
    }
    if filter
        .task_type
        .is_some_and(|task_type| task.task_type != task_type)
    {
        return false;
    }
    true
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

    async fn find_active_pdf_processing_task(
        &self,
        pdf_id: uuid::Uuid,
        workspace_id: uuid::Uuid,
    ) -> TaskResult<Option<Task>> {
        use crate::types::{TaskStatus, TaskType};

        let tasks = self.tasks.read().unwrap();
        for task in tasks.values() {
            if task.workspace_id != workspace_id {
                continue;
            }
            if !matches!(task.task_type, TaskType::PdfProcessing | TaskType::Insert) {
                continue;
            }
            if !matches!(task.status, TaskStatus::Pending | TaskStatus::Processing) {
                continue;
            }
            if task.pdf_id() == Some(pdf_id) {
                return Ok(Some(task.clone()));
            }
        }
        Ok(None)
    }

    async fn find_active_pdf_ingest_task(
        &self,
        pdf_id: uuid::Uuid,
        workspace_id: uuid::Uuid,
    ) -> TaskResult<Option<Task>> {
        use crate::types::{TaskStatus, TaskType};

        let tasks = self.tasks.read().unwrap();
        for task in tasks.values() {
            if task.workspace_id != workspace_id {
                continue;
            }
            if task.task_type != TaskType::Insert {
                continue;
            }
            if !matches!(task.status, TaskStatus::Pending | TaskStatus::Processing) {
                continue;
            }
            if task.pdf_id() == Some(pdf_id) {
                return Ok(Some(task.clone()));
            }
        }
        Ok(None)
    }

    async fn list_tasks(&self, filter: TaskFilter, pagination: Pagination) -> TaskResult<TaskList> {
        let tasks = self.tasks.read().unwrap();

        // Filter tasks (tenant/workspace isolation matches postgres storage)
        let mut filtered: Vec<Task> = tasks
            .values()
            .filter(|task| task_matches_filter(task, &filter))
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
            if !task_matches_filter(task, &filter) {
                continue;
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

    async fn claim_next(&self, worker_id: &str, lease_ttl: Duration) -> TaskResult<Option<Task>> {
        let mut tasks = self.tasks.write().unwrap();
        let now = Utc::now();

        // SPEC-084 / GH-316: mirror postgres least-loaded workspace-fair claim.
        let eligible: Vec<&Task> = tasks
            .values()
            .filter(|t| match t.status {
                TaskStatus::Pending => true,
                TaskStatus::Processing => t.lease_is_expired(now),
                _ => false,
            })
            .collect();
        let active_count = |ws: uuid::Uuid| -> usize {
            tasks
                .values()
                .filter(|t| {
                    t.workspace_id == ws
                        && t.status == TaskStatus::Processing
                        && !t.lease_is_expired(now)
                })
                .count()
        };
        let fair_workspace = eligible
            .iter()
            .map(|t| t.workspace_id)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .min_by_key(|ws| {
                let oldest = eligible
                    .iter()
                    .filter(|t| t.workspace_id == *ws)
                    .map(|t| t.created_at)
                    .min()
                    .unwrap_or(now);
                (active_count(*ws), oldest)
            });
        let mut candidates: Vec<(chrono::DateTime<Utc>, String)> = eligible
            .into_iter()
            .filter(|t| Some(t.workspace_id) == fair_workspace)
            .map(|t| (t.created_at, t.track_id.clone()))
            .collect();
        candidates.sort_by_key(|(created_at, _)| *created_at);

        let track_id = match candidates.first() {
            Some((_, id)) => id.clone(),
            None => return Ok(None),
        };

        let lease_token = Uuid::new_v4();
        let lease_expires_at = crate::lease_expires_at(now, lease_ttl);

        let task = tasks.get_mut(&track_id).expect("candidate just selected");
        task.status = TaskStatus::Processing;
        if task.started_at.is_none() {
            task.started_at = Some(now);
        }
        task.updated_at = now;
        task.completed_at = None;
        task.lease_owner = Some(worker_id.to_string());
        task.lease_token = Some(lease_token);
        task.lease_expires_at = Some(lease_expires_at);

        Ok(Some(task.clone()))
    }

    async fn refresh_lease(
        &self,
        track_id: &str,
        worker_id: &str,
        lease_token: Uuid,
        lease_ttl: Duration,
    ) -> TaskResult<bool> {
        let mut tasks = self.tasks.write().unwrap();
        let Some(task) = tasks.get_mut(track_id) else {
            return Ok(false);
        };
        if task.status != TaskStatus::Processing
            || task.lease_owner.as_deref() != Some(worker_id)
            || task.lease_token != Some(lease_token)
        {
            return Ok(false);
        }
        let now = Utc::now();
        task.lease_expires_at = Some(crate::lease_expires_at(now, lease_ttl));
        task.updated_at = now;
        Ok(true)
    }

    async fn release_claim(
        &self,
        track_id: &str,
        worker_id: &str,
        lease_token: Uuid,
    ) -> TaskResult<bool> {
        let mut tasks = self.tasks.write().unwrap();
        let Some(task) = tasks.get_mut(track_id) else {
            return Ok(false);
        };
        if task.status != TaskStatus::Processing
            || task.lease_owner.as_deref() != Some(worker_id)
            || task.lease_token != Some(lease_token)
        {
            return Ok(false);
        }
        let now = Utc::now();
        task.status = TaskStatus::Pending;
        task.started_at = None;
        task.updated_at = now;
        task.clear_lease();
        Ok(true)
    }

    async fn get_queue_metrics_filtered(
        &self,
        tenant_id: Option<uuid::Uuid>,
        workspace_id: Option<uuid::Uuid>,
    ) -> TaskResult<QueueMetrics> {
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
    async fn test_list_tasks_scopes_by_workspace() {
        let storage = MemoryTaskStorage::new();
        let tenant_id = test_tenant_id();
        let workspace_a = test_workspace_id();
        let workspace_b = uuid::Uuid::new_v4();

        for workspace_id in [workspace_a, workspace_b] {
            let task = Task::new(
                tenant_id,
                workspace_id,
                TaskType::Insert,
                serde_json::json!({"workspace": workspace_id.to_string()}),
            );
            storage.create_task(&task).await.unwrap();
        }

        let filter = TaskFilter {
            tenant_id: Some(tenant_id),
            workspace_id: Some(workspace_a),
            status: None,
            task_type: None,
        };

        let result = storage
            .list_tasks(filter, Pagination::default())
            .await
            .unwrap();

        assert_eq!(result.tasks.len(), 1);
        assert_eq!(result.total, 1);
        assert_eq!(result.tasks[0].workspace_id, workspace_a);
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

    #[tokio::test]
    async fn claim_next_picks_oldest_pending_and_second_claim_is_none() {
        let storage = MemoryTaskStorage::new();
        let older = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            serde_json::json!({"n": 1}),
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
        let newer = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            serde_json::json!({"n": 2}),
        );
        storage.create_task(&older).await.unwrap();
        storage.create_task(&newer).await.unwrap();

        let claimed = storage
            .claim_next("worker-a", Duration::from_secs(120))
            .await
            .unwrap()
            .expect("should claim");
        assert_eq!(claimed.track_id, older.track_id);
        assert_eq!(claimed.status, TaskStatus::Processing);
        assert_eq!(claimed.lease_owner.as_deref(), Some("worker-a"));
        assert!(claimed.lease_token.is_some());

        let second = storage
            .claim_next("worker-b", Duration::from_secs(120))
            .await
            .unwrap()
            .expect("should claim newer");
        assert_eq!(second.track_id, newer.track_id);

        let none = storage
            .claim_next("worker-c", Duration::from_secs(120))
            .await
            .unwrap();
        assert!(none.is_none());
    }

    #[tokio::test]
    async fn claim_next_skips_cancelled() {
        let storage = MemoryTaskStorage::new();
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            serde_json::json!({}),
        );
        task.mark_cancelled();
        storage.create_task(&task).await.unwrap();

        let claimed = storage
            .claim_next("worker-a", Duration::from_secs(120))
            .await
            .unwrap();
        assert!(claimed.is_none());
    }

    #[tokio::test]
    async fn refresh_lease_cas_and_release_claim() {
        let storage = MemoryTaskStorage::new();
        let task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            serde_json::json!({}),
        );
        storage.create_task(&task).await.unwrap();

        let claimed = storage
            .claim_next("worker-a", Duration::from_secs(120))
            .await
            .unwrap()
            .unwrap();
        let token = claimed.lease_token.unwrap();

        assert!(storage
            .refresh_lease(
                &claimed.track_id,
                "worker-a",
                token,
                Duration::from_secs(120)
            )
            .await
            .unwrap());
        assert!(!storage
            .refresh_lease(
                &claimed.track_id,
                "worker-b",
                token,
                Duration::from_secs(120)
            )
            .await
            .unwrap());
        assert!(!storage
            .refresh_lease(
                &claimed.track_id,
                "worker-a",
                Uuid::new_v4(),
                Duration::from_secs(120)
            )
            .await
            .unwrap());

        assert!(storage
            .release_claim(&claimed.track_id, "worker-a", token)
            .await
            .unwrap());
        let pending = storage.get_task(&claimed.track_id).await.unwrap().unwrap();
        assert_eq!(pending.status, TaskStatus::Pending);
        assert!(pending.lease_owner.is_none());
        assert!(pending.lease_token.is_none());

        // Wrong owner after release
        assert!(!storage
            .release_claim(&claimed.track_id, "worker-a", token)
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn claim_next_reclaims_expired_processing() {
        let storage = MemoryTaskStorage::new();
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            serde_json::json!({}),
        );
        task.mark_processing();
        task.lease_owner = Some("dead-worker".into());
        task.lease_token = Some(Uuid::new_v4());
        task.lease_expires_at = Some(Utc::now() - chrono::Duration::seconds(1));
        storage.create_task(&task).await.unwrap();

        let claimed = storage
            .claim_next("worker-b", Duration::from_secs(120))
            .await
            .unwrap()
            .expect("expired processing should be claimable");
        assert_eq!(claimed.lease_owner.as_deref(), Some("worker-b"));
    }
}
