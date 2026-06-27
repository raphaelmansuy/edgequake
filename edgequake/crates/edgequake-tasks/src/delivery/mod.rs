//! External task delivery for horizontal worker scale (SPEC-026 Phase 4 P-12).
//!
//! Postgres `TaskStorage` remains SSOT; delivery notifiers wake workers
//! without duplicating task payloads on the wire.

mod bridge;
mod mode;
mod notifier;
mod storage_hydrating;

pub use bridge::BridgedTaskQueue;
pub use mode::{delivery_mode_from_env, parse_delivery_mode, TaskDeliveryMode};
pub use notifier::{ChannelTaskNotifier, NoopTaskNotifier, SharedTaskNotifier, TaskNotifier};
pub use storage_hydrating::StorageHydratingTaskQueue;

use crate::error::TaskResult;
use crate::queue::SharedTaskQueue;
use crate::storage::SharedTaskStorage;
use crate::types::Task;

/// Enqueue path: persist then deliver per mode.
pub async fn enqueue_with_delivery(
    storage: &SharedTaskStorage,
    queue: &SharedTaskQueue,
    notifier: &dyn TaskNotifier,
    mode: TaskDeliveryMode,
    task: Task,
) -> TaskResult<()> {
    storage.create_task(&task).await?;
    match mode {
        TaskDeliveryMode::Local => queue.send(task).await?,
        TaskDeliveryMode::Bridged => queue.send(task).await?,
        TaskDeliveryMode::NotifyOnly => notifier.notify(&task.track_id).await?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryTaskStorage;
    use crate::queue::ChannelTaskQueue;
    use crate::types::TaskType;
    use std::sync::Arc;
    use uuid::Uuid;

    const TEST_TENANT: &str = "00000000-0000-0000-0000-000000000001";
    const TEST_WORKSPACE: &str = "00000000-0000-0000-0000-000000000002";

    fn test_ids() -> (Uuid, Uuid) {
        (
            Uuid::parse_str(TEST_TENANT).unwrap(),
            Uuid::parse_str(TEST_WORKSPACE).unwrap(),
        )
    }

    #[tokio::test]
    async fn local_delivery_sends_to_channel() {
        let storage: SharedTaskStorage = Arc::new(MemoryTaskStorage::new());
        let queue: SharedTaskQueue = Arc::new(ChannelTaskQueue::new(8));
        let notifier = Arc::new(NoopTaskNotifier);
        let (tenant, workspace) = test_ids();
        let task = Task::new(tenant, workspace, TaskType::Insert, serde_json::json!({}));

        enqueue_with_delivery(
            &storage,
            &queue,
            notifier.as_ref(),
            TaskDeliveryMode::Local,
            task.clone(),
        )
        .await
        .unwrap();

        let received = queue.receive().await.unwrap();
        assert_eq!(received.track_id, task.track_id);
    }

    #[tokio::test]
    async fn bridged_delivery_notifies_and_queues() {
        let storage: SharedTaskStorage = Arc::new(MemoryTaskStorage::new());
        let inner: SharedTaskQueue = Arc::new(ChannelTaskQueue::new(8));
        let notifier = Arc::new(ChannelTaskNotifier::new(8));
        let bridged: SharedTaskQueue = Arc::new(BridgedTaskQueue::new(
            Arc::clone(&inner),
            Arc::clone(&notifier) as SharedTaskNotifier,
        ));
        let (tenant, workspace) = test_ids();
        let task = Task::new(tenant, workspace, TaskType::Insert, serde_json::json!({}));

        enqueue_with_delivery(
            &storage,
            &bridged,
            notifier.as_ref(),
            TaskDeliveryMode::Bridged,
            task.clone(),
        )
        .await
        .unwrap();

        let mut sub = notifier.subscribe();
        let notified = sub.recv().await.unwrap();
        assert_eq!(notified, task.track_id);
        let received = inner.receive().await.unwrap();
        assert_eq!(received.track_id, task.track_id);
    }

    #[tokio::test]
    async fn notify_only_skips_local_send() {
        let storage: SharedTaskStorage = Arc::new(MemoryTaskStorage::new());
        let queue: SharedTaskQueue = Arc::new(ChannelTaskQueue::new(8));
        let notifier = Arc::new(ChannelTaskNotifier::new(8));
        let (tenant, workspace) = test_ids();
        let task = Task::new(tenant, workspace, TaskType::Insert, serde_json::json!({}));
        let track_id = task.track_id.clone();

        enqueue_with_delivery(
            &storage,
            &queue,
            notifier.as_ref(),
            TaskDeliveryMode::NotifyOnly,
            task,
        )
        .await
        .unwrap();

        assert!(queue.try_receive().await.unwrap().is_none());
        let mut hydrating = StorageHydratingTaskQueue::new(Arc::clone(&storage), &notifier);
        let loaded = hydrating.receive_hydrated().await.unwrap();
        assert_eq!(loaded.track_id, track_id);
    }
}
