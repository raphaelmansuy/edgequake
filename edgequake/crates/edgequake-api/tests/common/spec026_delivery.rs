//! SPEC-026 Phase 4 task delivery E2E helpers (DRY SSOT).

use edgequake_tasks::{
    delivery::StorageHydratingTaskQueue, CancellationRegistry, ChannelTaskNotifier,
    SharedTaskProcessor, SharedTaskStorage,
};
use std::sync::Arc;
use tokio::task::JoinHandle;

/// Background workers for `notify_only` mode: hydrate from Postgres SSOT via notifier.
pub fn spawn_hydrating_workers(
    storage: SharedTaskStorage,
    notifier: Arc<ChannelTaskNotifier>,
    processor: SharedTaskProcessor,
    cancellation_registry: CancellationRegistry,
    num_workers: usize,
) -> Vec<JoinHandle<()>> {
    (0..num_workers.max(1))
        .map(|worker_id| {
            let storage = Arc::clone(&storage);
            let notifier = Arc::clone(&notifier);
            let processor = Arc::clone(&processor);
            let cancel_registry = cancellation_registry.clone();
            tokio::spawn(async move {
                let mut hydrating = StorageHydratingTaskQueue::new(storage.clone(), notifier.as_ref());
                loop {
                    let Ok(mut task) = hydrating.receive_hydrated().await else {
                        break;
                    };
                    task.mark_processing();
                    if storage.update_task(&task).await.is_err() {
                        continue;
                    }
                    let cancel = cancel_registry.register(&task.track_id).await;
                    match processor.process(&mut task, cancel).await {
                        Ok(result) => task.mark_success(result),
                        Err(e) => task.mark_failed(e.to_string()),
                    }
                    cancel_registry.deregister(&task.track_id).await;
                    let _ = storage.update_task(&task).await;
                    tracing::debug!(worker_id, track_id = %task.track_id, "hydrating worker finished task");
                }
            })
        })
        .collect()
}

/// Allow spawned hydrating workers to subscribe before the test enqueues work.
pub async fn wait_for_hydrating_workers_ready() {
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
}
