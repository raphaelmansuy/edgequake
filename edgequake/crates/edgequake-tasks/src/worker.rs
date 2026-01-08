//! Worker pool for processing tasks from the queue.
//!
//! ## WHY Worker Pool Architecture?
//!
//! Document processing (PDF extraction, embedding generation) is CPU/IO intensive.
//! The worker pool provides:
//! - **Bounded concurrency**: Prevents resource exhaustion during burst uploads
//! - **Task isolation**: One failing task doesn't affect others
//! - **Graceful shutdown**: In-flight tasks complete before termination
//! - **Retry logic**: Transient failures (network, rate limits) auto-recover
//!
//! Default worker count is `num_cpus` because embedding generation is CPU-bound.
//! For IO-bound workloads (e.g., LLM API calls), consider increasing.

use crate::{error::TaskResult, queue::TaskQueue, storage::TaskStorage, types::Task};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

/// Task processor trait - implement this to process different task types
#[async_trait::async_trait]
pub trait TaskProcessor: Send + Sync {
    /// Process a task
    async fn process(&self, task: &mut Task) -> TaskResult<serde_json::Value>;
}

/// Shared task processor
pub type SharedTaskProcessor = Arc<dyn TaskProcessor>;

/// Worker pool configuration
#[derive(Debug, Clone)]
pub struct WorkerPoolConfig {
    /// Number of worker threads
    pub num_workers: usize,

    /// Whether to retry failed tasks automatically
    pub auto_retry: bool,

    /// Delay before retrying failed tasks (seconds)
    pub retry_delay_secs: u64,
}

impl Default for WorkerPoolConfig {
    fn default() -> Self {
        Self {
            // WHY num_cpus: Embedding generation is CPU-bound (SIMD operations).
            // Using all cores maximizes throughput without context-switching overhead.
            // min(2) ensures we can still process tasks on single-core VMs.
            num_workers: num_cpus::get().max(2),
            auto_retry: true,
            // WHY 5 seconds: Balances quick recovery from transient failures
            // (network timeouts, rate limits) without hammering failing services.
            // Exponential backoff should be added for production (future work).
            retry_delay_secs: 5,
        }
    }
}

/// Worker pool for processing tasks
pub struct WorkerPool {
    config: WorkerPoolConfig,
    queue: Arc<dyn TaskQueue>,
    storage: Arc<dyn TaskStorage>,
    processor: SharedTaskProcessor,
    handles: Vec<JoinHandle<()>>,
    shutdown_tx: Option<tokio::sync::broadcast::Sender<()>>,
}

impl WorkerPool {
    /// Create a new worker pool
    pub fn new(
        config: WorkerPoolConfig,
        queue: Arc<dyn TaskQueue>,
        storage: Arc<dyn TaskStorage>,
        processor: SharedTaskProcessor,
    ) -> Self {
        Self {
            config,
            queue,
            storage,
            processor,
            handles: Vec::new(),
            shutdown_tx: None,
        }
    }

    /// Start the worker pool
    pub fn start(&mut self) {
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
        self.shutdown_tx = Some(shutdown_tx.clone());

        info!(
            "Starting worker pool with {} workers",
            self.config.num_workers
        );

        for worker_id in 0..self.config.num_workers {
            let queue = Arc::clone(&self.queue);
            let storage = Arc::clone(&self.storage);
            let processor = Arc::clone(&self.processor);
            let config = self.config.clone();
            let mut shutdown_rx = shutdown_tx.subscribe();

            let handle = tokio::spawn(async move {
                info!("Worker {} started", worker_id);

                loop {
                    tokio::select! {
                        _ = shutdown_rx.recv() => {
                            info!("Worker {} shutting down", worker_id);
                            break;
                        }
                        result = queue.receive() => {
                            match result {
                                Ok(mut task) => {
                                    info!("Worker {} processing task: {}", worker_id, task.track_id);

                                    // Mark as processing
                                    task.mark_processing();
                                    if let Err(e) = storage.update_task(&task).await {
                                        error!("Failed to update task status: {}", e);
                                    }

                                    // Process task
                                    match processor.process(&mut task).await {
                                        Ok(result) => {
                                            task.mark_success(result);
                                            info!("Worker {} completed task: {}", worker_id, task.track_id);
                                        }
                                        Err(e) => {
                                            let error_msg = format!("{}", e);
                                            task.mark_failed(error_msg.clone());
                                            error!(
                                                "Worker {} failed to process task {}: {}",
                                                worker_id, task.track_id, error_msg
                                            );

                                            // Auto-retry if enabled and retries remaining
                                            if config.auto_retry && task.can_retry() {
                                                warn!(
                                                    "Task {} will be retried (attempt {}/{})",
                                                    task.track_id,
                                                    task.retry_count + 1,
                                                    task.max_retries
                                                );

                                                // Schedule retry after delay
                                                let retry_task = task.clone();
                                                let retry_queue = Arc::clone(&queue);
                                                let retry_delay = config.retry_delay_secs;

                                                tokio::spawn(async move {
                                                    tokio::time::sleep(
                                                        tokio::time::Duration::from_secs(retry_delay)
                                                    ).await;

                                                    if let Err(e) = retry_queue.send(retry_task).await {
                                                        error!("Failed to requeue task for retry: {}", e);
                                                    }
                                                });
                                            }
                                        }
                                    }

                                    // Update task in storage
                                    if let Err(e) = storage.update_task(&task).await {
                                        error!("Failed to update task: {}", e);
                                    }
                                }
                                Err(e) => {
                                    if queue.is_closed() {
                                        info!("Worker {} queue closed", worker_id);
                                        break;
                                    }
                                    error!("Worker {} failed to receive task: {}", worker_id, e);
                                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                                }
                            }
                        }
                    }
                }

                info!("Worker {} stopped", worker_id);
            });

            self.handles.push(handle);
        }
    }

    /// Shutdown the worker pool gracefully
    pub async fn shutdown(self) {
        info!("Shutting down worker pool");

        if let Some(shutdown_tx) = self.shutdown_tx {
            let _ = shutdown_tx.send(());
        }

        for handle in self.handles {
            let _ = handle.await;
        }

        info!("Worker pool shut down complete");
    }

    /// Get number of workers
    pub fn num_workers(&self) -> usize {
        self.config.num_workers
    }
}

/// Mock task processor for testing
#[cfg(test)]
pub struct MockTaskProcessor;

#[cfg(test)]
#[async_trait::async_trait]
impl TaskProcessor for MockTaskProcessor {
    async fn process(&self, task: &mut Task) -> TaskResult<serde_json::Value> {
        // Simulate some work
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        Ok(serde_json::json!({
            "status": "success",
            "task_id": task.track_id
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        memory::MemoryTaskStorage,
        queue::ChannelTaskQueue,
        types::{Task, TaskStatus, TaskType},
    };

    #[tokio::test]
    async fn test_worker_pool_processes_tasks() {
        let queue = Arc::new(ChannelTaskQueue::new(10));
        let storage = Arc::new(MemoryTaskStorage::new());
        let processor = Arc::new(MockTaskProcessor);

        let config = WorkerPoolConfig {
            num_workers: 2,
            auto_retry: false,
            retry_delay_secs: 1,
        };

        let mut pool = WorkerPool::new(config, queue.clone(), storage.clone(), processor);
        pool.start();

        // Create and enqueue tasks
        let mut task_ids = Vec::new();
        for i in 0..5 {
            let task = Task::new(TaskType::Insert, serde_json::json!({"index": i}));
            task_ids.push(task.track_id.clone());
            storage.create_task(&task).await.unwrap();
            queue.send(task).await.unwrap();
        }

        // Wait for processing
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Check all tasks completed
        for task_id in task_ids {
            let task = storage.get_task(&task_id).await.unwrap().unwrap();
            assert_eq!(task.status, TaskStatus::Indexed);
        }

        pool.shutdown().await;
    }

    #[tokio::test]
    async fn test_worker_pool_handles_shutdown() {
        let queue = Arc::new(ChannelTaskQueue::new(10));
        let storage = Arc::new(MemoryTaskStorage::new());
        let processor = Arc::new(MockTaskProcessor);

        let config = WorkerPoolConfig {
            num_workers: 2,
            auto_retry: false,
            retry_delay_secs: 1,
        };

        let mut pool = WorkerPool::new(config, queue, storage, processor);
        pool.start();

        // Shutdown immediately
        pool.shutdown().await;
    }
}
