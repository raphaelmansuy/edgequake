//! Composite queue: local channel + external notifier on send.

use async_trait::async_trait;

use crate::delivery::SharedTaskNotifier;
use crate::error::TaskResult;
use crate::queue::{SharedTaskQueue, TaskQueue};
use crate::types::Task;

/// Wraps a local queue and fires notifier on every send (bridged API path).
pub struct BridgedTaskQueue {
    inner: SharedTaskQueue,
    notifier: SharedTaskNotifier,
}

impl BridgedTaskQueue {
    pub fn new(inner: SharedTaskQueue, notifier: SharedTaskNotifier) -> Self {
        Self { inner, notifier }
    }
}

#[async_trait]
impl TaskQueue for BridgedTaskQueue {
    async fn send(&self, task: Task) -> TaskResult<()> {
        self.notifier.notify(&task.track_id).await?;
        self.inner.send(task).await
    }

    async fn receive(&self) -> TaskResult<Task> {
        self.inner.receive().await
    }

    async fn try_receive(&self) -> TaskResult<Option<Task>> {
        self.inner.try_receive().await
    }

    async fn size(&self) -> TaskResult<usize> {
        self.inner.size().await
    }

    fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }
}
