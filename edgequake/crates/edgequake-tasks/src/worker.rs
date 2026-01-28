//! Worker pool for processing tasks from the queue.
//!
//! ## Implements
//!
//! - **FEAT0910**: Worker pool with configurable concurrency
//! - **FEAT0911**: Task processor trait abstraction
//! - **FEAT0912**: Graceful shutdown with task completion
//! - **SPEC-001/Issue-8**: Exponential backoff for retries
//!
//! ## Use Cases
//!
//! - **UC2601**: System spawns workers to process queued tasks
//! - **UC2602**: System retries failed tasks with exponential backoff
//! - **UC2603**: System shuts down gracefully completing in-flight work
//!
//! ## Enforces
//!
//! - **BR0910**: Worker count bounded to prevent resource exhaustion
//! - **BR0911**: In-flight tasks must complete before shutdown
//! - **BR0912**: Retry delays use exponential backoff (2^n * base_delay)
//!
//! ## WHY Worker Pool Architecture?
//!
//! Document processing (PDF extraction, embedding generation) is CPU/IO intensive.
//! The worker pool provides:
//! - **Bounded concurrency**: Prevents resource exhaustion during burst uploads
//! - **Task isolation**: One failing task doesn't affect others
//! - **Graceful shutdown**: In-flight tasks complete before termination
//! - **Retry logic**: Transient failures (network, rate limits) auto-recover
//! - **Exponential backoff**: Prevents hammering failing services
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

    /// Initial delay before retrying failed tasks (milliseconds)
    ///
    /// @implements SPEC-001/Issue-8: Exponential backoff base delay
    pub initial_retry_delay_ms: u64,

    /// Maximum retry delay (milliseconds) to prevent runaway backoff
    ///
    /// @implements SPEC-001/Issue-8: Capped exponential backoff
    pub max_retry_delay_ms: u64,

    /// Backoff multiplier (default: 2.0 for exponential backoff)
    pub backoff_multiplier: f64,
}

impl Default for WorkerPoolConfig {
    fn default() -> Self {
        Self {
            // WHY num_cpus: Embedding generation is CPU-bound (SIMD operations).
            // Using all cores maximizes throughput without context-switching overhead.
            // min(2) ensures we can still process tasks on single-core VMs.
            num_workers: num_cpus::get().max(2),
            auto_retry: true,
            // WHY 1000ms: Starting delay for exponential backoff.
            // With multiplier of 2.0: 1s -> 2s -> 4s -> 8s -> 16s (capped at max)
            initial_retry_delay_ms: 1000,
            // WHY 60s max: Prevents retries from taking forever.
            // 60s is long enough to recover from transient issues but
            // short enough to fail fast on permanent failures.
            max_retry_delay_ms: 60_000,
            // WHY 2.0: Standard exponential backoff multiplier.
            // Each retry waits 2x longer than the previous.
            backoff_multiplier: 2.0,
        }
    }
}

/// Calculate exponential backoff delay for a given retry attempt.
///
/// @implements SPEC-001/Issue-8: Exponential backoff calculation
///
/// Formula: min(initial_delay * multiplier^attempt, max_delay)
///
/// # Arguments
/// * `attempt` - Zero-based retry attempt number (0 = first retry)
/// * `initial_delay_ms` - Base delay in milliseconds
/// * `max_delay_ms` - Maximum delay cap in milliseconds
/// * `multiplier` - Backoff multiplier (typically 2.0)
///
/// # Returns
/// Delay in milliseconds for this retry attempt
fn calculate_backoff_delay(
    attempt: u32,
    initial_delay_ms: u64,
    max_delay_ms: u64,
    multiplier: f64,
) -> u64 {
    let delay = initial_delay_ms as f64 * multiplier.powi(attempt as i32);
    (delay as u64).min(max_delay_ms)
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

                                            // Log circuit breaker status
                                            if task.circuit_breaker_tripped {
                                                error!(
                                                    "Worker {} task {} permanently failed: Circuit breaker tripped after {} consecutive timeouts",
                                                    worker_id,
                                                    task.track_id,
                                                    task.consecutive_timeout_failures
                                                );
                                            } else {
                                                error!(
                                                    "Worker {} failed to process task {}: {} (consecutive timeouts: {})",
                                                    worker_id, task.track_id, error_msg, task.consecutive_timeout_failures
                                                );
                                            }

                                            // Auto-retry if enabled, retries remaining, and circuit breaker not tripped
                                            if config.auto_retry && task.can_retry() && !task.circuit_breaker_tripped {
                                                // Calculate exponential backoff delay
                                                let retry_delay_ms = calculate_backoff_delay(
                                                    task.retry_count as u32,
                                                    config.initial_retry_delay_ms,
                                                    config.max_retry_delay_ms,
                                                    config.backoff_multiplier,
                                                );

                                                warn!(
                                                    "Task {} will be retried (attempt {}/{}) after {}ms",
                                                    task.track_id,
                                                    task.retry_count + 1,
                                                    task.max_retries,
                                                    retry_delay_ms
                                                );

                                                // Schedule retry after exponential backoff delay
                                                let retry_task = task.clone();
                                                let retry_queue = Arc::clone(&queue);

                                                tokio::spawn(async move {
                                                    tokio::time::sleep(
                                                        tokio::time::Duration::from_millis(retry_delay_ms)
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

    const TEST_TENANT_ID: &str = "00000000-0000-0000-0000-000000000001";
    const TEST_WORKSPACE_ID: &str = "00000000-0000-0000-0000-000000000002";

    fn test_tenant_id() -> uuid::Uuid {
        uuid::Uuid::parse_str(TEST_TENANT_ID).unwrap()
    }

    fn test_workspace_id() -> uuid::Uuid {
        uuid::Uuid::parse_str(TEST_WORKSPACE_ID).unwrap()
    }

    #[tokio::test]
    async fn test_worker_pool_processes_tasks() {
        let queue = Arc::new(ChannelTaskQueue::new(10));
        let storage = Arc::new(MemoryTaskStorage::new());
        let processor = Arc::new(MockTaskProcessor);

        let config = WorkerPoolConfig {
            num_workers: 2,
            auto_retry: false,
            initial_retry_delay_ms: 100,
            max_retry_delay_ms: 5000,
            backoff_multiplier: 2.0,
        };

        let mut pool = WorkerPool::new(config, queue.clone(), storage.clone(), processor);
        pool.start();

        // Create and enqueue tasks
        let mut task_ids = Vec::new();
        for i in 0..5 {
            let task = Task::new(
                test_tenant_id(),
                test_workspace_id(),
                TaskType::Insert,
                serde_json::json!({"index": i}),
            );
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
            initial_retry_delay_ms: 100,
            max_retry_delay_ms: 5000,
            backoff_multiplier: 2.0,
        };

        let mut pool = WorkerPool::new(config, queue, storage, processor);
        pool.start();

        // Shutdown immediately
        pool.shutdown().await;
    }
}
