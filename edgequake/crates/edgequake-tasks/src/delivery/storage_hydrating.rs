//! External worker queue: receive track_id notification → hydrate from Postgres.

use async_trait::async_trait;
use tokio::sync::broadcast;
use tracing::debug;

use crate::delivery::ChannelTaskNotifier;
use crate::error::{TaskError, TaskResult};
use crate::queue::TaskQueue;
use crate::storage::SharedTaskStorage;
use crate::types::Task;

/// Worker-side queue for distributed mode: notifications carry track_id only.
pub struct StorageHydratingTaskQueue {
    storage: SharedTaskStorage,
    notifications: broadcast::Receiver<String>,
}

impl StorageHydratingTaskQueue {
    pub fn new(storage: SharedTaskStorage, notifier: &ChannelTaskNotifier) -> Self {
        Self {
            storage,
            notifications: notifier.subscribe(),
        }
    }

    async fn hydrate_next(&mut self) -> TaskResult<Task> {
        loop {
            let track_id = self
                .notifications
                .recv()
                .await
                .map_err(|_| TaskError::QueueClosed)?;
            debug!(track_id = %track_id, "Hydrating task from Postgres SSOT");
            if let Some(task) = self.storage.get_task(&track_id).await? {
                return Ok(task);
            }
            debug!(track_id = %track_id, "Notification stale; waiting for next");
        }
    }
}

#[async_trait]
impl TaskQueue for StorageHydratingTaskQueue {
    async fn send(&self, _task: Task) -> TaskResult<()> {
        Err(TaskError::UnsupportedOperation(
            "StorageHydratingTaskQueue is receive-only".into(),
        ))
    }

    async fn receive(&self) -> TaskResult<Task> {
        // broadcast::Receiver is not Sync — clone storage and resubscribe per call
        Err(TaskError::UnsupportedOperation(
            "Use StorageHydratingTaskQueue::hydrate_next via dedicated worker adapter".into(),
        ))
    }

    async fn try_receive(&self) -> TaskResult<Option<Task>> {
        Err(TaskError::UnsupportedOperation(
            "StorageHydratingTaskQueue does not support try_receive".into(),
        ))
    }

    async fn size(&self) -> TaskResult<usize> {
        Ok(0)
    }

    fn is_closed(&self) -> bool {
        false
    }
}

impl StorageHydratingTaskQueue {
    /// Blocking receive for external worker loops (mutable receiver state).
    pub async fn receive_hydrated(&mut self) -> TaskResult<Task> {
        self.hydrate_next().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::delivery::TaskNotifier;
    use crate::memory::MemoryTaskStorage;
    use crate::types::TaskType;
    use std::sync::Arc;
    use uuid::Uuid;

    const TEST_TENANT: &str = "00000000-0000-0000-0000-000000000001";
    const TEST_WORKSPACE: &str = "00000000-0000-0000-0000-000000000002";

    #[tokio::test]
    async fn storage_hydrating_loads_task_by_track_id() {
        let storage: SharedTaskStorage = Arc::new(MemoryTaskStorage::new());
        let notifier = ChannelTaskNotifier::new(8);
        let mut hydrating = StorageHydratingTaskQueue::new(Arc::clone(&storage), &notifier);

        let tenant = Uuid::parse_str(TEST_TENANT).unwrap();
        let workspace = Uuid::parse_str(TEST_WORKSPACE).unwrap();
        let task = Task::new(tenant, workspace, TaskType::Insert, serde_json::json!({}));
        let track_id = task.track_id.clone();
        storage.create_task(&task).await.unwrap();

        notifier.notify(&track_id).await.unwrap();
        let loaded = hydrating.receive_hydrated().await.unwrap();
        assert_eq!(loaded.track_id, track_id);
    }
}
