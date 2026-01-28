//! Task queue implementations for background processing.
//!
//! ## Implements
//!
//! - **FEAT0920**: Task queue trait abstraction
//! - **FEAT0921**: Channel-based queue for in-process tasks
//! - **FEAT0922**: Bounded queue with backpressure
//!
//! ## Use Cases
//!
//! - **UC2610**: System enqueues document for async processing
//! - **UC2611**: Worker receives task from queue
//! - **UC2612**: System applies backpressure when queue full
//!
//! ## Enforces
//!
//! - **BR0920**: Queue capacity bounded to prevent memory exhaustion
//! - **BR0921**: Queue must support concurrent send/receive

use crate::{error::TaskResult, types::Task};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::debug;

/// Trait for task queue implementations
#[async_trait]
pub trait TaskQueue: Send + Sync {
    /// Send a task to the queue
    async fn send(&self, task: Task) -> TaskResult<()>;

    /// Receive a task from the queue (blocking)
    async fn receive(&self) -> TaskResult<Task>;

    /// Try to receive a task (non-blocking)
    async fn try_receive(&self) -> TaskResult<Option<Task>>;

    /// Get queue size (if supported)
    async fn size(&self) -> TaskResult<usize>;

    /// Check if queue is closed
    fn is_closed(&self) -> bool;
}

/// Type alias for shared queue
pub type SharedTaskQueue = Arc<dyn TaskQueue>;

/// Channel-based task queue using tokio::sync::mpsc
pub struct ChannelTaskQueue {
    sender: mpsc::Sender<Task>,
    receiver: Arc<tokio::sync::Mutex<mpsc::Receiver<Task>>>,
    capacity: usize,
}

impl ChannelTaskQueue {
    /// Create a new channel-based task queue
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = mpsc::channel(capacity);

        Self {
            sender,
            receiver: Arc::new(tokio::sync::Mutex::new(receiver)),
            capacity,
        }
    }

    /// Get the queue capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[async_trait]
impl TaskQueue for ChannelTaskQueue {
    async fn send(&self, task: Task) -> TaskResult<()> {
        debug!("Sending task to queue: {}", task.track_id);

        self.sender
            .send(task)
            .await
            .map_err(|_| crate::error::TaskError::QueueClosed)?;

        Ok(())
    }

    async fn receive(&self) -> TaskResult<Task> {
        let mut receiver = self.receiver.lock().await;

        receiver
            .recv()
            .await
            .ok_or(crate::error::TaskError::QueueClosed)
    }

    async fn try_receive(&self) -> TaskResult<Option<Task>> {
        let mut receiver = self.receiver.lock().await;

        match receiver.try_recv() {
            Ok(task) => Ok(Some(task)),
            Err(mpsc::error::TryRecvError::Empty) => Ok(None),
            Err(mpsc::error::TryRecvError::Disconnected) => {
                Err(crate::error::TaskError::QueueClosed)
            }
        }
    }

    async fn size(&self) -> TaskResult<usize> {
        // For mpsc channels, we can't get exact size without draining
        // Return 0 as approximation
        Ok(0)
    }

    fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
}

/// Unbounded channel-based task queue (use with caution in production)
pub struct UnboundedChannelTaskQueue {
    sender: mpsc::UnboundedSender<Task>,
    receiver: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<Task>>>,
}

impl UnboundedChannelTaskQueue {
    /// Create a new unbounded channel-based task queue
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();

        Self {
            sender,
            receiver: Arc::new(tokio::sync::Mutex::new(receiver)),
        }
    }
}

impl Default for UnboundedChannelTaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TaskQueue for UnboundedChannelTaskQueue {
    async fn send(&self, task: Task) -> TaskResult<()> {
        debug!("Sending task to unbounded queue: {}", task.track_id);

        self.sender
            .send(task)
            .map_err(|_| crate::error::TaskError::QueueClosed)?;

        Ok(())
    }

    async fn receive(&self) -> TaskResult<Task> {
        let mut receiver = self.receiver.lock().await;

        receiver
            .recv()
            .await
            .ok_or(crate::error::TaskError::QueueClosed)
    }

    async fn try_receive(&self) -> TaskResult<Option<Task>> {
        let mut receiver = self.receiver.lock().await;

        match receiver.try_recv() {
            Ok(task) => Ok(Some(task)),
            Err(mpsc::error::TryRecvError::Empty) => Ok(None),
            Err(mpsc::error::TryRecvError::Disconnected) => {
                Err(crate::error::TaskError::QueueClosed)
            }
        }
    }

    async fn size(&self) -> TaskResult<usize> {
        Ok(0)
    }

    fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TaskType;

    const TEST_TENANT_ID: &str = "00000000-0000-0000-0000-000000000001";
    const TEST_WORKSPACE_ID: &str = "00000000-0000-0000-0000-000000000002";

    fn test_tenant_id() -> uuid::Uuid {
        uuid::Uuid::parse_str(TEST_TENANT_ID).unwrap()
    }

    fn test_workspace_id() -> uuid::Uuid {
        uuid::Uuid::parse_str(TEST_WORKSPACE_ID).unwrap()
    }

    #[tokio::test]
    async fn test_channel_queue_send_receive() {
        let queue = ChannelTaskQueue::new(10);
        let task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Upload,
            serde_json::json!({"file": "test.pdf"}),
        );
        let track_id = task.track_id.clone();

        queue.send(task).await.unwrap();

        let received = queue.receive().await.unwrap();
        assert_eq!(received.track_id, track_id);
    }

    #[tokio::test]
    async fn test_channel_queue_capacity() {
        let queue = ChannelTaskQueue::new(2);

        let task1 = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            serde_json::json!({}),
        );
        let task2 = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            serde_json::json!({}),
        );

        queue.send(task1).await.unwrap();
        queue.send(task2).await.unwrap();

        // Queue is now full (capacity=2)
        // Third send should block, so we use try_send approach in real code
        assert_eq!(queue.capacity(), 2);
    }

    #[tokio::test]
    async fn test_try_receive_empty() {
        let queue = ChannelTaskQueue::new(10);

        let result = queue.try_receive().await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_unbounded_queue() {
        let queue = UnboundedChannelTaskQueue::new();

        // Send many tasks
        for i in 0..100 {
            let task = Task::new(
                test_tenant_id(),
                test_workspace_id(),
                TaskType::Insert,
                serde_json::json!({"index": i}),
            );
            queue.send(task).await.unwrap();
        }

        // Receive all tasks
        for _ in 0..100 {
            let _task = queue.receive().await.unwrap();
        }

        // Queue should be empty
        let result = queue.try_receive().await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_queue_not_closed() {
        let queue = ChannelTaskQueue::new(10);
        assert!(!queue.is_closed());
    }
}
