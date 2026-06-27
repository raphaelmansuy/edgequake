//! Task queue and pipeline progress runtime (SPEC-017 P1-04).

use edgequake_tasks::{
    delivery_mode_from_env, enqueue_with_delivery, BridgedTaskQueue, CancellationRegistry,
    ChannelTaskNotifier, NoopTaskNotifier, PipelineState, SharedTaskNotifier, SharedTaskQueue,
    SharedTaskStorage, Task, TaskDeliveryMode,
};

use std::sync::Arc;

use crate::error::{ApiError, ApiResult};
use crate::handlers::ProgressBroadcaster;
use crate::services::PdfAdmissionRegistry;

/// Background task processing and real-time progress broadcasting.
#[derive(Clone)]
pub struct TaskRuntime {
    pub storage: SharedTaskStorage,
    pub queue: SharedTaskQueue,
    pub pipeline_state: PipelineState,
    pub progress_broadcaster: ProgressBroadcaster,
    pub cancellation_registry: CancellationRegistry,
    /// P-G15: closes TOCTOU between single-flight check and task row creation.
    pub pdf_admission: Arc<crate::services::PdfAdmissionRegistry>,
    delivery_mode: TaskDeliveryMode,
    notifier: SharedTaskNotifier,
    /// Present when delivery uses [`ChannelTaskNotifier`] (bridged / notify_only).
    channel_notifier: Option<Arc<ChannelTaskNotifier>>,
}

impl TaskRuntime {
    /// Build a runtime bundle with fresh pipeline progress and cancellation state.
    pub fn new(storage: SharedTaskStorage, queue: SharedTaskQueue) -> Self {
        Self::with_delivery(storage, queue, delivery_mode_from_env())
    }

    /// Build runtime with explicit delivery mode (tests / bridged workers).
    pub fn with_delivery(
        storage: SharedTaskStorage,
        queue: SharedTaskQueue,
        delivery_mode: TaskDeliveryMode,
    ) -> Self {
        let (notifier, channel_notifier): (SharedTaskNotifier, Option<Arc<ChannelTaskNotifier>>) =
            match delivery_mode {
                TaskDeliveryMode::Local => (Arc::new(NoopTaskNotifier), None),
                TaskDeliveryMode::Bridged | TaskDeliveryMode::NotifyOnly => {
                    let channel = Arc::new(ChannelTaskNotifier::new(256));
                    (Arc::clone(&channel) as SharedTaskNotifier, Some(channel))
                }
            };
        let queue = match delivery_mode {
            TaskDeliveryMode::Bridged => {
                Arc::new(BridgedTaskQueue::new(queue, Arc::clone(&notifier))) as SharedTaskQueue
            }
            _ => queue,
        };
        Self {
            storage,
            queue,
            pipeline_state: PipelineState::new(),
            progress_broadcaster: ProgressBroadcaster::default(),
            cancellation_registry: CancellationRegistry::new(),
            pdf_admission: Arc::new(PdfAdmissionRegistry::default()),
            delivery_mode,
            notifier,
            channel_notifier,
        }
    }

    pub fn delivery_mode(&self) -> TaskDeliveryMode {
        self.delivery_mode
    }

    pub fn task_notifier(&self) -> SharedTaskNotifier {
        Arc::clone(&self.notifier)
    }

    /// Channel notifier for hydrating external workers (bridged / notify_only).
    pub fn channel_notifier(&self) -> Option<Arc<ChannelTaskNotifier>> {
        self.channel_notifier.as_ref().map(Arc::clone)
    }

    /// Persist a task and enqueue it for background processing (SPEC-017 P2-01).
    pub async fn enqueue(&self, task: Task) -> ApiResult<()> {
        enqueue_with_delivery(
            &self.storage,
            &self.queue,
            self.notifier.as_ref(),
            self.delivery_mode,
            task,
        )
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to enqueue task: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_tasks::TaskType;
    use std::sync::Arc;
    use uuid::Uuid;

    #[tokio::test]
    async fn enqueue_persists_and_queues() {
        let runtime = TaskRuntime::new(
            Arc::new(edgequake_tasks::memory::MemoryTaskStorage::new()),
            Arc::new(edgequake_tasks::queue::ChannelTaskQueue::new(8)),
        );

        let task = Task::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            TaskType::Insert,
            serde_json::json!({"content": "enqueue test"}),
        );
        let track_id = task.track_id.clone();
        runtime.enqueue(task).await.expect("enqueue");

        let stored = runtime
            .storage
            .get_task(&track_id)
            .await
            .expect("lookup")
            .expect("task exists");
        assert_eq!(stored.track_id, track_id);
    }
}
