//! SPEC-026 Phase 4 — external task delivery contract tests.

use edgequake_tasks::storage::SharedTaskStorage;
use edgequake_tasks::{
    delivery_mode_from_env, enqueue_with_delivery, parse_delivery_mode, BridgedTaskQueue,
    ChannelTaskNotifier, NoopTaskNotifier, SharedTaskNotifier, StorageHydratingTaskQueue,
    TaskDeliveryMode, TaskNotifier,
};
use edgequake_tasks::{memory::MemoryTaskStorage, queue::ChannelTaskQueue, types::TaskType, Task};
use std::sync::Arc;
use uuid::Uuid;

const TEST_TENANT: &str = "00000000-0000-0000-0000-000000000001";
const TEST_WORKSPACE: &str = "00000000-0000-0000-0000-000000000002";

fn ids() -> (Uuid, Uuid) {
    (
        Uuid::parse_str(TEST_TENANT).unwrap(),
        Uuid::parse_str(TEST_WORKSPACE).unwrap(),
    )
}

#[tokio::test]
async fn local_delivery_sends_to_channel() {
    let storage: SharedTaskStorage = Arc::new(MemoryTaskStorage::new());
    let queue: Arc<dyn edgequake_tasks::TaskQueue> = Arc::new(ChannelTaskQueue::new(8));
    let notifier = Arc::new(NoopTaskNotifier);
    let (t, w) = ids();
    let task = Task::new(t, w, TaskType::Insert, serde_json::json!({}));
    enqueue_with_delivery(
        &storage,
        &queue,
        notifier.as_ref(),
        TaskDeliveryMode::Local,
        task.clone(),
    )
    .await
    .unwrap();
    assert_eq!(queue.receive().await.unwrap().track_id, task.track_id);
}

#[tokio::test]
async fn storage_hydrating_loads_task_by_track_id() {
    let storage: SharedTaskStorage = Arc::new(MemoryTaskStorage::new());
    let notifier = ChannelTaskNotifier::new(8);
    let mut hydrating = StorageHydratingTaskQueue::new(Arc::clone(&storage), &notifier);
    let (t, w) = ids();
    let task = Task::new(t, w, TaskType::Insert, serde_json::json!({}));
    let track_id = task.track_id.clone();
    storage.create_task(&task).await.unwrap();
    notifier.notify(&track_id).await.unwrap();
    assert_eq!(
        hydrating.receive_hydrated().await.unwrap().track_id,
        track_id
    );
}

#[test]
fn delivery_mode_from_env_parses() {
    assert_eq!(parse_delivery_mode("local"), TaskDeliveryMode::Local);
    assert_eq!(parse_delivery_mode("bridged"), TaskDeliveryMode::Bridged);
    assert_eq!(
        parse_delivery_mode("notify_only"),
        TaskDeliveryMode::NotifyOnly
    );
    std::env::set_var("EDGEQUAKE_TASK_DELIVERY", "bridged");
    assert_eq!(delivery_mode_from_env(), TaskDeliveryMode::Bridged);
    std::env::remove_var("EDGEQUAKE_TASK_DELIVERY");
}

#[tokio::test]
async fn bridged_queue_notifies_on_send() {
    let inner: Arc<dyn edgequake_tasks::TaskQueue> = Arc::new(ChannelTaskQueue::new(4));
    let notifier = Arc::new(ChannelTaskNotifier::new(4));
    let bridged: Arc<dyn edgequake_tasks::TaskQueue> = Arc::new(BridgedTaskQueue::new(
        Arc::clone(&inner),
        Arc::clone(&notifier) as SharedTaskNotifier,
    ));
    let (t, w) = ids();
    let task = Task::new(t, w, TaskType::Insert, serde_json::json!({}));
    let track_id = task.track_id.clone();
    let mut sub = notifier.subscribe();
    bridged.send(task).await.unwrap();
    assert_eq!(sub.recv().await.unwrap(), track_id);
}
