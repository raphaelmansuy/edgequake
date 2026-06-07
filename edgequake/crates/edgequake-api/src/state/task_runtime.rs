//! Task queue and pipeline progress runtime (SPEC-017 P1-04).

use edgequake_tasks::{
    CancellationRegistry, PipelineState, SharedTaskQueue, SharedTaskStorage, Task,
};

use crate::error::{ApiError, ApiResult};
use crate::handlers::ProgressBroadcaster;

/// Background task processing and real-time progress broadcasting.
#[derive(Clone)]
pub struct TaskRuntime {
    pub storage: SharedTaskStorage,
    pub queue: SharedTaskQueue,
    pub pipeline_state: PipelineState,
    pub progress_broadcaster: ProgressBroadcaster,
    pub cancellation_registry: CancellationRegistry,
}

impl TaskRuntime {
    /// Build a runtime bundle with fresh pipeline progress and cancellation state.
    pub fn new(storage: SharedTaskStorage, queue: SharedTaskQueue) -> Self {
        Self {
            storage,
            queue,
            pipeline_state: PipelineState::new(),
            progress_broadcaster: ProgressBroadcaster::default(),
            cancellation_registry: CancellationRegistry::new(),
        }
    }

    /// Persist a task and enqueue it for background processing (SPEC-017 P2-01).
    pub async fn enqueue(&self, task: Task) -> ApiResult<()> {
        self.storage
            .create_task(&task)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to create task: {}", e)))?;
        self.queue
            .send(task)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to queue task: {}", e)))?;
        Ok(())
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
        let runtime = TaskRuntime {
            storage: Arc::new(edgequake_tasks::memory::MemoryTaskStorage::new()),
            queue: Arc::new(edgequake_tasks::queue::ChannelTaskQueue::new(8)),
            pipeline_state: PipelineState::new(),
            progress_broadcaster: ProgressBroadcaster::default(),
            cancellation_registry: CancellationRegistry::new(),
        };

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
