//! Worker pool for processing tasks from the queue.
//!
//! ## Implements
//!
//! - **FEAT0910**: Worker pool with configurable concurrency
//! - **FEAT0911**: Task processor trait abstraction
//! - **FEAT0912**: Graceful shutdown with task completion
//! - **SPEC-001/Issue-8**: Exponential backoff for retries
//! - **FEAT-TENANT-FAIRNESS**: Per-tenant concurrency limits
//!
//! ## Use Cases
//!
//! - **UC2601**: System spawns workers to process queued tasks
//! - **UC2602**: System retries failed tasks with exponential backoff
//! - **UC2603**: System shuts down gracefully completing in-flight work
//! - **UC2604**: System prevents one tenant from monopolizing workers
//!
//! ## Enforces
//!
//! - **BR0910**: Worker count bounded to prevent resource exhaustion
//! - **BR0911**: In-flight tasks must complete before shutdown
//! - **BR0912**: Retry delays use exponential backoff (2^n * base_delay)
//! - **BR0913**: Per-tenant concurrency capped at max_tasks_per_tenant
//!
//! ## WHY Worker Pool Architecture?
//!
//! Document processing (PDF extraction, embedding generation) is CPU/IO intensive.
//! The worker pool provides:
//! - **Bounded concurrency**: Prevents resource exhaustion during burst uploads
//! - **Task isolation**: One failing task doesn't affect others
//! - **Tenant fairness**: Per-tenant limits prevent monopolization
//! - **Graceful shutdown**: In-flight tasks complete before termination
//! - **Retry logic**: Transient failures (network, rate limits) auto-recover
//! - **Exponential backoff**: Prevents hammering failing services
//! - **Permanent failure cleanup**: Updates document status on retry exhaustion
//!
//! Default worker count is `num_cpus * 4` because pipeline processing is IO-bound
//! (waiting for LLM API calls, embedding generation). Workers spend most of their
//! time in network I/O, so we need more workers than CPU cores to keep the pipeline
//! saturated. Override via the `WORKER_THREADS` environment variable.

use crate::{
    error::TaskResult, queue::TaskQueue, storage::TaskStorage,
    tenant_limiter::TenantConcurrencyLimiter, types::Task,
};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

/// Task processor trait - implement this to process different task types.
///
/// Implementors handle both normal processing and cleanup on permanent failure.
#[async_trait::async_trait]
pub trait TaskProcessor: Send + Sync {
    /// Process a task.
    async fn process(&self, task: &mut Task) -> TaskResult<serde_json::Value>;

    /// Called when a task has permanently failed (retries exhausted or circuit
    /// breaker tripped). Override to update document status, clean up resources,
    /// or send notifications.
    ///
    /// WHY: Without this callback, documents get stuck in "processing" status
    /// forever when the task fails permanently. The worker knows when retries
    /// are exhausted, but only the processor knows how to update document
    /// metadata and clean up resources.
    ///
    /// Default implementation is a no-op for backward compatibility.
    async fn on_permanent_failure(&self, task: &Task, error_msg: &str) {
        let _ = (task, error_msg); // suppress unused warnings
    }
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

    /// Maximum concurrent tasks per tenant.
    ///
    /// WHY: Prevents one tenant from monopolizing all workers. When a tenant
    /// has `max_tasks_per_tenant` tasks in flight, new tasks from that tenant
    /// are requeued with a short delay so workers can serve other tenants.
    ///
    /// Default: `max(1, num_workers / 2)` — guarantees at least half the
    /// workers remain available for other tenants.
    ///
    /// Set to 0 to disable per-tenant limiting (all workers available to any tenant).
    pub max_tasks_per_tenant: usize,

    /// Maximum time (seconds) a single task can process before being timed out.
    ///
    /// WHY: Without a timeout, processor.process() can hang forever (e.g., stuck
    /// LLM call, unresponsive PDF conversion) while the heartbeat mechanism keeps
    /// the task looking "alive" in the database. This creates phantom "Processing"
    /// banners that never resolve — the orphan recovery can't catch them because
    /// the heartbeat keeps updating `updated_at`.
    ///
    /// Default: 7200s (2 hours) — generous enough for very large PDF processing
    /// (1000+ page documents with vision LLM extraction at ~12s/page ≈ 3.3h) while
    /// still catching truly stuck tasks within a reasonable window.
    /// Override via `TASK_PROCESSING_TIMEOUT_SECS` environment variable.
    pub processing_timeout_secs: u64,
}

impl Default for WorkerPoolConfig {
    fn default() -> Self {
        // WHY num_cpus * 4: Pipeline processing is IO-bound (waiting for LLM API
        // calls and embedding generation). Workers spend most of their time in
        // network I/O, not CPU computation. Higher worker count ensures the
        // pipeline stays saturated with concurrent requests to external services.
        let num_workers = (num_cpus::get() * 4).max(4);
        Self {
            num_workers,
            auto_retry: true,
            initial_retry_delay_ms: 1000,
            max_retry_delay_ms: 60_000,
            backoff_multiplier: 2.0,
            // WHY num_workers * 3/4: For IO-bound workloads, each tenant can
            // use most of the pool while still guaranteeing at least 25% of
            // workers remain available for other tenants.
            max_tasks_per_tenant: (num_workers * 3 / 4).max(1),
            // WHY 2 hours: Large PDFs (1000+ pages) with vision LLM extraction
            // can take 3+ hours. 2 hours catches most real-world cases while
            // still preventing infinite hangs. Override via
            // TASK_PROCESSING_TIMEOUT_SECS env var.
            processing_timeout_secs: 7200,
        }
    }
}

/// Calculate exponential backoff delay for a given retry attempt.
///
/// @implements SPEC-001/Issue-8: Exponential backoff calculation
///
/// Formula: min(initial_delay * multiplier^attempt, max_delay)
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
    tenant_limiter: Option<TenantConcurrencyLimiter>,
}

impl WorkerPool {
    /// Create a new worker pool
    pub fn new(
        config: WorkerPoolConfig,
        queue: Arc<dyn TaskQueue>,
        storage: Arc<dyn TaskStorage>,
        processor: SharedTaskProcessor,
    ) -> Self {
        // Create tenant limiter if max_tasks_per_tenant > 0
        let tenant_limiter = if config.max_tasks_per_tenant > 0 {
            Some(TenantConcurrencyLimiter::new(config.max_tasks_per_tenant))
        } else {
            None
        };

        Self {
            config,
            queue,
            storage,
            processor,
            handles: Vec::new(),
            shutdown_tx: None,
            tenant_limiter,
        }
    }

    /// Start the worker pool
    pub fn start(&mut self) {
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
        self.shutdown_tx = Some(shutdown_tx.clone());

        if let Some(ref limiter) = self.tenant_limiter {
            info!(
                "Starting worker pool: {} workers, max {} tasks/tenant",
                self.config.num_workers,
                limiter.max_per_tenant()
            );
        } else {
            info!(
                "Starting worker pool: {} workers, no tenant limit",
                self.config.num_workers
            );
        }

        for worker_id in 0..self.config.num_workers {
            let queue = Arc::clone(&self.queue);
            let storage = Arc::clone(&self.storage);
            let processor = Arc::clone(&self.processor);
            let config = self.config.clone();
            let mut shutdown_rx = shutdown_tx.subscribe();
            let tenant_limiter = self.tenant_limiter.clone();

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
                                    // FEAT-TENANT-FAIRNESS: Check per-tenant concurrency limit
                                    // before processing. If tenant is at capacity, requeue the
                                    // task with a short delay so this worker can serve other tenants.
                                    let _tenant_permit = if let Some(ref limiter) = tenant_limiter {
                                        match limiter.try_acquire(task.tenant_id).await {
                                            Some(permit) => Some(permit),
                                            None => {
                                                debug!(
                                                    worker_id = worker_id,
                                                    task_id = %task.track_id,
                                                    tenant_id = %task.tenant_id,
                                                    "Tenant at concurrency limit, requeueing task"
                                                );
                                                // Requeue with delay so other tenants' tasks get
                                                // picked up first. The delay is bounded: base 200ms
                                                // to avoid busy-looping when many tasks hit the
                                                // tenant limit simultaneously.
                                                // WHY tokio::spawn: We don't want to block this
                                                // worker — it should immediately pick up the next
                                                // task (which may be for a different tenant).
                                                // WHY bounded: The number of spawned requeue tasks
                                                // is bounded by queue capacity (backpressure from
                                                // the channel's send).
                                                let requeue_task = task;
                                                let requeue_queue = Arc::clone(&queue);
                                                tokio::spawn(async move {
                                                    tokio::time::sleep(
                                                        tokio::time::Duration::from_millis(500)
                                                    ).await;
                                                    if let Err(e) = requeue_queue.send(requeue_task).await {
                                                        error!("Failed to requeue tenant-limited task: {}", e);
                                                    }
                                                });
                                                continue; // Pick next task from queue
                                            }
                                        }
                                    } else {
                                        None
                                    };

                                    info!("Worker {} processing task: {} (tenant: {})", worker_id, task.track_id, task.tenant_id);

                                    // Mark as processing
                                    task.mark_processing();
                                    if let Err(e) = storage.update_task(&task).await {
                                        error!("Failed to update task status: {}", e);
                                    }

                                    // HEARTBEAT: Spawn a background task that periodically
                                    // touches the task's updated_at timestamp. This prevents
                                    // the orphan-recovery logic from marking active tasks as
                                    // orphaned during long-running LLM extraction (>5 min).
                                    // The heartbeat is ONLY useful for periodic runtime orphan
                                    // checks; startup recovery now ignores it (recovers all).
                                    let heartbeat_track_id = task.track_id.clone();
                                    let heartbeat_storage = Arc::clone(&storage);
                                    let heartbeat_handle = tokio::spawn(async move {
                                        let mut interval = tokio::time::interval(
                                            tokio::time::Duration::from_secs(60),
                                        );
                                        interval.tick().await; // Skip first immediate tick
                                        loop {
                                            interval.tick().await;
                                            if let Err(e) = heartbeat_storage
                                                .touch_task(&heartbeat_track_id)
                                                .await
                                            {
                                                debug!(
                                                    "Heartbeat failed for task {}: {}",
                                                    heartbeat_track_id, e
                                                );
                                            }
                                        }
                                    });

                                    // Process task with timeout.
                                    // WHY: Without a timeout, processor.process() can hang
                                    // forever (stuck LLM call, unresponsive PDF conversion)
                                    // while the heartbeat keeps updating updated_at. The orphan
                                    // recovery can never catch these "zombie" tasks. The timeout
                                    // ensures every task eventually completes or fails.
                                    let timeout_duration = tokio::time::Duration::from_secs(
                                        config.processing_timeout_secs,
                                    );
                                    let process_result = tokio::time::timeout(
                                        timeout_duration,
                                        processor.process(&mut task),
                                    )
                                    .await;

                                    match process_result {
                                        Ok(Ok(result)) => {
                                            heartbeat_handle.abort();
                                            task.mark_success(result);
                                            info!("Worker {} completed task: {} (tenant: {})", worker_id, task.track_id, task.tenant_id);
                                        }
                                        Ok(Err(e)) => {
                                            heartbeat_handle.abort();
                                            let error_msg = format!("{}", e);
                                            task.mark_failed(error_msg.clone());

                                            // Log circuit breaker status
                                            if task.circuit_breaker_tripped {
                                                error!(
                                                    worker_id = worker_id,
                                                    task_id = %task.track_id,
                                                    tenant_id = %task.tenant_id,
                                                    consecutive_timeouts = task.consecutive_timeout_failures,
                                                    "Task permanently failed: Circuit breaker tripped"
                                                );
                                            } else {
                                                error!(
                                                    worker_id = worker_id,
                                                    task_id = %task.track_id,
                                                    tenant_id = %task.tenant_id,
                                                    retry_count = task.retry_count,
                                                    max_retries = task.max_retries,
                                                    consecutive_timeouts = task.consecutive_timeout_failures,
                                                    error = %error_msg,
                                                    "Task processing failed"
                                                );
                                            }

                                            // Check if task is permanently failed (no more retries)
                                            let will_retry = config.auto_retry
                                                && task.can_retry()
                                                && !task.circuit_breaker_tripped;

                                            if will_retry {
                                                // Calculate exponential backoff delay
                                                let retry_delay_ms = calculate_backoff_delay(
                                                    task.retry_count as u32,
                                                    config.initial_retry_delay_ms,
                                                    config.max_retry_delay_ms,
                                                    config.backoff_multiplier,
                                                );

                                                warn!(
                                                    task_id = %task.track_id,
                                                    attempt = task.retry_count,
                                                    max_retries = task.max_retries,
                                                    delay_ms = retry_delay_ms,
                                                    "Scheduling retry with exponential backoff"
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
                                            } else {
                                                // PERMANENT FAILURE: No more retries or circuit breaker tripped.
                                                // Notify processor to update document status, clean up resources.
                                                // WHY: Without this, documents remain stuck in "processing"
                                                // status forever after retry exhaustion.
                                                let reason = if task.circuit_breaker_tripped {
                                                    format!(
                                                        "Circuit breaker tripped after {} consecutive timeouts. \
                                                        Last error: {}",
                                                        task.consecutive_timeout_failures, error_msg
                                                    )
                                                } else {
                                                    format!(
                                                        "Retries exhausted ({}/{} attempts). Last error: {}",
                                                        task.retry_count, task.max_retries, error_msg
                                                    )
                                                };
                                                error!(
                                                    task_id = %task.track_id,
                                                    tenant_id = %task.tenant_id,
                                                    "Task permanently failed: {}", reason
                                                );
                                                processor.on_permanent_failure(&task, &reason).await;
                                            }
                                        }
                                        Err(_elapsed) => {
                                            // TIMEOUT: Task processing exceeded the configured
                                            // time limit. This catches stuck LLM calls, hung
                                            // PDF conversions, and other infinite-wait scenarios.
                                            heartbeat_handle.abort();
                                            let timeout_msg = format!(
                                                "Task processing timed out after {} seconds",
                                                config.processing_timeout_secs
                                            );
                                            task.mark_failed(timeout_msg.clone());

                                            error!(
                                                worker_id = worker_id,
                                                task_id = %task.track_id,
                                                tenant_id = %task.tenant_id,
                                                timeout_secs = config.processing_timeout_secs,
                                                "Task timed out — marking as permanently failed"
                                            );

                                            // Timeouts are treated as permanent failures (no retry).
                                            // WHY: If a task timed out once, it's very likely to
                                            // time out again. Retrying would just waste worker time
                                            // and keep the "Processing" banner showing indefinitely.
                                            processor.on_permanent_failure(&task, &timeout_msg).await;
                                        }
                                    }

                                    // Update task in storage
                                    if let Err(e) = storage.update_task(&task).await {
                                        error!("Failed to update task: {}", e);
                                    }

                                    // _tenant_permit is dropped here, releasing the slot
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

        // Spawn periodic cleanup task for tenant semaphores
        if let Some(limiter) = self.tenant_limiter.clone() {
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
                    limiter.cleanup_idle().await;
                }
            });
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
            max_tasks_per_tenant: 0, // disabled for basic test
            processing_timeout_secs: 300, // 5 min for tests
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
            max_tasks_per_tenant: 0,
            processing_timeout_secs: 300,
        };

        let mut pool = WorkerPool::new(config, queue, storage, processor);
        pool.start();

        // Shutdown immediately
        pool.shutdown().await;
    }

    #[tokio::test]
    async fn test_tenant_fairness_limiting() {
        // With max_tasks_per_tenant=1, only 1 task per tenant runs at a time
        let limiter = crate::tenant_limiter::TenantConcurrencyLimiter::new(1);
        let tenant = test_tenant_id();

        let p1 = limiter.try_acquire(tenant).await;
        assert!(p1.is_some(), "First acquire should succeed");

        let p2 = limiter.try_acquire(tenant).await;
        assert!(p2.is_none(), "Second acquire should be denied (at limit)");

        // Different tenant should still get through
        let other_tenant = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000099").unwrap();
        let p3 = limiter.try_acquire(other_tenant).await;
        assert!(p3.is_some(), "Other tenant should not be affected");

        // Release first permit, then acquire should succeed
        drop(p1);
        let p4 = limiter.try_acquire(tenant).await;
        assert!(p4.is_some(), "Should succeed after releasing permit");
    }
}
