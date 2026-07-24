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
//! - **BR0911**: In-flight tasks drain within shutdown budget (then cancel/abort)
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
    admission::{estimate_task_bytes, AdmissionOutcome, AdmissionPermit, InFlightByteBudget},
    cancellation::CancellationRegistry,
    error::{TaskError, TaskResult},
    queue::TaskQueue,
    storage::TaskStorage,
    task_lease_ttl_from_env,
    tenant_limiter::{TenantConcurrencyLimiter, TryAcquireOutcome},
    types::{FairnessClass, Task, TaskStatus},
};
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::MissedTickBehavior;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn, Instrument};

/// Process-local set of `track_id`s currently waiting in a fairness park.
///
/// WHY: Park releases the claim → Pending. Without this set, other workers
/// reclaim the same Pending row and spawn duplicate park waiters (storm) while
/// newer PdfProcessing stays FIFO-starved behind reclaim loops.
#[derive(Clone, Default)]
struct FairnessParkSet {
    inner: Arc<Mutex<HashSet<String>>>,
}

impl FairnessParkSet {
    fn contains(&self, track_id: &str) -> bool {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains(track_id)
    }

    /// Begin parking `track_id`. Returns `false` if already parked.
    fn try_begin(&self, track_id: &str) -> bool {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(track_id.to_string())
    }

    fn end(&self, track_id: &str) {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(track_id);
    }
}

/// RAII: remove `track_id` from the park set when the park task exits.
struct FairnessParkGuard {
    set: FairnessParkSet,
    track_id: String,
}

impl Drop for FairnessParkGuard {
    fn drop(&mut self) {
        self.set.end(&self.track_id);
    }
}

/// RAII guard that aborts the heartbeat task on drop.
///
/// WHY: If `processor.process()` panics, the stack unwinds and this guard's
/// `Drop` impl fires, aborting the heartbeat. Without this, a panic leaves
/// the heartbeat running forever — the task stays in "processing" with a
/// live heartbeat, and neither the periodic orphan check nor the processing
/// timeout can catch it (timeout is in the same panic scope, orphan check
/// sees a fresh `updated_at`).
struct HeartbeatGuard(JoinHandle<()>);

impl Drop for HeartbeatGuard {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Minimum allowed processing timeout (60 seconds).
///
/// WHY: A timeout of 0 would cause every task to immediately time out,
/// making the system non-functional. Even very fast tasks need a few
/// seconds for LLM API round-trips.
const MIN_PROCESSING_TIMEOUT_SECS: u64 = 60;

/// After releasing an already-parked claim, re-claim immediately (bounded)
/// so newer ingest work is not stuck behind the 2s poll interval.
const MAX_PARK_SKIP_RECLAIMS: usize = 8;

/// Task processor trait - implement this to process different task types.
///
/// Implementors handle both normal processing and cleanup on permanent failure.
///
/// The `CancellationToken` parameter enables cooperative cancellation:
/// processors should periodically check `cancel_token.is_cancelled()` and
/// return early with an appropriate error when cancellation is detected.
#[async_trait::async_trait]
pub trait TaskProcessor: Send + Sync {
    /// Process a task with cooperative cancellation support.
    ///
    /// Implementations MUST check `cancel_token.is_cancelled()` at each
    /// stage boundary (chunking, extraction, embedding, storage) and
    /// return `Err(TaskError::Cancelled)` when cancellation is detected.
    async fn process(
        &self,
        task: &mut Task,
        cancel_token: CancellationToken,
    ) -> TaskResult<serde_json::Value>;

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

    /// Maximum concurrent **ingest** tasks per tenant (Pdf/Insert/… fairness lane).
    ///
    /// WHY: Protects scarce LLM/vision capacity. When a tenant has this many
    /// ingest tasks in flight, excess ingest work parks until a slot frees.
    ///
    /// Default: `max(1, num_workers * 3/4)`. Set to 0 to disable the ingest lane.
    pub max_tasks_per_tenant: usize,

    /// Maximum concurrent **lifecycle** tasks per tenant (Deletion/Wipe lane).
    ///
    /// WHY: Deletion/Wipe are DB-bound and must not share the local LLM clamp
    /// with PdfProcessing — otherwise one delete serializes the tenant and
    /// starves new uploads (stuck Queued).
    ///
    /// Default: same as `max_tasks_per_tenant` (unified). Local Ollama defaults
    /// override to 2 via `resolve_worker_pool_limits`. Set to 0 to disable.
    pub max_lifecycle_tasks_per_tenant: usize,

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
            // Unified default; local extract providers override lifecycle to 2.
            max_lifecycle_tasks_per_tenant: (num_workers * 3 / 4).max(1),
            // WHY 2 hours: Large PDFs (1000+ pages) with vision LLM extraction
            // can take 3+ hours. 2 hours catches most real-world cases while
            // still preventing infinite hangs. Override via
            // TASK_PROCESSING_TIMEOUT_SECS env var.
            processing_timeout_secs: 7200.max(MIN_PROCESSING_TIMEOUT_SECS),
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
    /// SPEC-083 / X-19: global in-flight byte budget (shared across workers).
    admission: Arc<InFlightByteBudget>,
    cancellation_registry: CancellationRegistry,
    /// Rate-limit fairness park DEBUG logs (one line per N parks).
    fairness_park_logs: Arc<AtomicU64>,
    /// Dedupes fairness park waiters per `track_id` (process-local).
    fairness_park_set: FairnessParkSet,
}

impl WorkerPool {
    /// Create a new worker pool
    pub fn new(
        config: WorkerPoolConfig,
        queue: Arc<dyn TaskQueue>,
        storage: Arc<dyn TaskStorage>,
        processor: SharedTaskProcessor,
    ) -> Self {
        // Create dual-lane limiter when either lane is capacity-limited.
        let tenant_limiter =
            if config.max_tasks_per_tenant > 0 || config.max_lifecycle_tasks_per_tenant > 0 {
                Some(TenantConcurrencyLimiter::new(
                    config.max_tasks_per_tenant,
                    config.max_lifecycle_tasks_per_tenant,
                ))
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
            admission: InFlightByteBudget::from_env(),
            cancellation_registry: CancellationRegistry::new(),
            fairness_park_logs: Arc::new(AtomicU64::new(0)),
            fairness_park_set: FairnessParkSet::default(),
        }
    }

    /// Shared in-flight byte admission budget (X-19).
    pub fn admission_budget(&self) -> Arc<InFlightByteBudget> {
        Arc::clone(&self.admission)
    }

    /// Get a reference to the cancellation registry.
    ///
    /// WHY: The cancel API handler needs access to this registry to trigger
    /// cooperative cancellation of in-flight tasks. Store this reference in
    /// your AppState and pass it to the cancel endpoint.
    pub fn cancellation_registry(&self) -> CancellationRegistry {
        self.cancellation_registry.clone()
    }

    /// Optional per-tenant limiter (None when both fairness lanes are unlimited).
    pub fn tenant_limiter(&self) -> Option<TenantConcurrencyLimiter> {
        self.tenant_limiter.clone()
    }

    /// Start the worker pool on the current Tokio runtime.
    pub fn start(&mut self) {
        self.start_on(&tokio::runtime::Handle::current());
    }

    /// Start workers on a dedicated runtime (ingest isolation from Axum serving).
    ///
    /// Nested `tokio::spawn` calls inside worker loops inherit this runtime's
    /// context once the worker task is polled.
    pub fn start_on(&mut self, runtime: &tokio::runtime::Handle) {
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
        self.shutdown_tx = Some(shutdown_tx.clone());

        if let Some(ref limiter) = self.tenant_limiter {
            info!(
                "Starting worker pool: {} workers, ingest={}/tenant lifecycle={}/tenant",
                self.config.num_workers,
                limiter.max_per_tenant(),
                limiter.max_lifecycle_per_tenant()
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
            let park_shutdown_tx = shutdown_tx.clone();
            let tenant_limiter = self.tenant_limiter.clone();
            let admission = Arc::clone(&self.admission);
            let cancel_registry = self.cancellation_registry.clone();
            let fairness_park_logs = Arc::clone(&self.fairness_park_logs);
            let fairness_park_set = self.fairness_park_set.clone();

            let handle = runtime.spawn(async move {
                info!("Worker {} started", worker_id);
                let worker_name = format!("worker-{worker_id}");
                let lease_ttl = task_lease_ttl_from_env();
                let mut poll_interval = tokio::time::interval(Duration::from_secs(2));
                poll_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
                // Skip the immediate first tick so we don't race startup hydrate.
                poll_interval.tick().await;

                loop {
                    // SPEC-057 P1: channel is wake-only; Postgres/memory claim is SSOT.
                    tokio::select! {
                        _ = shutdown_rx.recv() => {
                            info!("Worker {} shutting down", worker_id);
                            break;
                        }
                        result = queue.receive() => {
                            match result {
                                Ok(_wake) => {
                                    // Ignore payload — claim_next authorizes work.
                                }
                                Err(e) => {
                                    if queue.is_closed() {
                                        info!("Worker {} queue closed", worker_id);
                                        break;
                                    }
                                    error!(
                                        error.source = "task_worker",
                                        error.action = "receive_wake",
                                        worker_id = worker_id,
                                        error.message = %e,
                                        "Worker failed to receive wake from queue"
                                    );
                                    tokio::time::sleep(Duration::from_secs(1)).await;
                                    continue;
                                }
                            }
                        }
                        _ = poll_interval.tick() => {
                            // Periodic claim poll for Pending surviving restart / lost wakes.
                        }
                    }

                    // Claim loop: skip already-parked rows and re-claim immediately
                    // (bounded) so FIFO Pending deletions do not starve newer ingest.
                    let mut claimed: Option<(
                        Task,
                        uuid::Uuid,
                        Option<crate::tenant_limiter::FairnessPermit>,
                        AdmissionPermit,
                    )> = None;
                    for _reclaim in 0..=MAX_PARK_SKIP_RECLAIMS {
                        let mut task = match storage.claim_next(&worker_name, lease_ttl).await {
                            Ok(Some(t)) => t,
                            Ok(None) => break,
                            Err(e) => {
                                error!(
                                    error.source = "task_worker",
                                    error.action = "claim_next",
                                    worker_id = worker_id,
                                    error.message = %e,
                                    "Failed to claim next task"
                                );
                                break;
                            }
                        };

                        let lease_token = match task.lease_token {
                            Some(token) => token,
                            None => {
                                warn!(
                                    worker_id = worker_id,
                                    task_id = %task.track_id,
                                    "Claimed task missing lease_token — releasing"
                                );
                                let _ = storage
                                    .release_claim(
                                        &task.track_id,
                                        &worker_name,
                                        uuid::Uuid::nil(),
                                    )
                                    .await;
                                break;
                            }
                        };

                        // FEAT-CANCEL: Drop terminal / cancel-intent after claim.
                        if should_skip_task(&storage, &cancel_registry, &task).await {
                            debug!(
                                worker_id = worker_id,
                                task_id = %task.track_id,
                                "Skipping cancelled or terminal task after claim"
                            );
                            if cancel_registry.has_cancel_intent(&task.track_id).await {
                                task.mark_cancelled();
                                let _ = storage.update_task(&task).await;
                            } else if let Err(e) = storage
                                .release_claim(&task.track_id, &worker_name, lease_token)
                                .await
                            {
                                warn!(
                                    task_id = %task.track_id,
                                    error = %e,
                                    "Failed to release claim for skipped task"
                                );
                            }
                            cancel_registry.deregister(&task.track_id).await;
                            break;
                        }

                        // Already parked: release and re-claim (do not wait for poll).
                        if fairness_park_set.contains(&task.track_id) {
                            debug!(
                                worker_id = worker_id,
                                task_id = %task.track_id,
                                "Claimed task already fairness-parked — releasing without re-park"
                            );
                            if let Err(e) = storage
                                .release_claim(&task.track_id, &worker_name, lease_token)
                                .await
                            {
                                warn!(
                                    task_id = %task.track_id,
                                    error = %e,
                                    "Failed to release claim for already-parked task"
                                );
                                break;
                            }
                            continue;
                        }

                        // FEAT-TENANT-FAIRNESS: release claim before park (no double-process).
                        let fairness_class = task.task_type.fairness_class();
                        let tenant_permit = if let Some(ref limiter) = tenant_limiter {
                            match limiter
                                .try_acquire(task.tenant_id, task.workspace_id, fairness_class)
                                .await
                            {
                                TryAcquireOutcome::Unlimited => None,
                                TryAcquireOutcome::Acquired(permit) => Some(permit),
                                TryAcquireOutcome::AtCapacity => {
                                    let n =
                                        fairness_park_logs.fetch_add(1, Ordering::Relaxed) + 1;
                                    if n == 1 || n.is_multiple_of(32) {
                                        info!(
                                            worker_id = worker_id,
                                            task_id = %task.track_id,
                                            tenant_id = %task.tenant_id,
                                            fairness_class = ?fairness_class,
                                            park_events = n,
                                            park_waiters = limiter.park_waiter_count(),
                                            "Tenant at concurrency limit — releasing claim and parking (aggregated)"
                                        );
                                    }
                                    if !fairness_park_set.try_begin(&task.track_id) {
                                        if let Err(e) = storage
                                            .release_claim(
                                                &task.track_id,
                                                &worker_name,
                                                lease_token,
                                            )
                                            .await
                                        {
                                            warn!(
                                                task_id = %task.track_id,
                                                error = %e,
                                                "Failed to release claim for duplicate park skip"
                                            );
                                            break;
                                        }
                                        continue;
                                    }
                                    if let Err(e) = storage
                                        .release_claim(
                                            &task.track_id,
                                            &worker_name,
                                            lease_token,
                                        )
                                        .await
                                    {
                                        error!(
                                            task_id = %task.track_id,
                                            error = %e,
                                            "Failed to release claim before fairness park"
                                        );
                                        fairness_park_set.end(&task.track_id);
                                    } else if let Ok(Some(pending)) =
                                        storage.get_task(&task.track_id).await
                                    {
                                        spawn_fairness_park(
                                            worker_id,
                                            pending,
                                            fairness_class,
                                            Arc::clone(&queue),
                                            Arc::clone(&storage),
                                            limiter.clone(),
                                            cancel_registry.clone(),
                                            fairness_park_set.clone(),
                                            park_shutdown_tx.subscribe(),
                                        );
                                    } else {
                                        task.status = TaskStatus::Pending;
                                        task.clear_lease();
                                        spawn_fairness_park(
                                            worker_id,
                                            task,
                                            fairness_class,
                                            Arc::clone(&queue),
                                            Arc::clone(&storage),
                                            limiter.clone(),
                                            cancel_registry.clone(),
                                            fairness_park_set.clone(),
                                            park_shutdown_tx.subscribe(),
                                        );
                                    }
                                    break;
                                }
                            }
                        } else {
                            None
                        };

                        // SPEC-083 / X-19: byte-budget admission after fairness slot.
                        let cost = estimate_task_bytes(&task);
                        let admission_permit = match admission.try_admit(cost) {
                            AdmissionOutcome::Admitted(p) => p,
                            AdmissionOutcome::Rejected {
                                requested,
                                in_flight,
                                max_bytes,
                            } => {
                                debug!(
                                    worker_id = worker_id,
                                    task_id = %task.track_id,
                                    requested,
                                    in_flight,
                                    max_bytes,
                                    "Admission over budget — releasing claim for later retry"
                                );
                                // Drop fairness permit before release so the lane frees immediately.
                                drop(tenant_permit);
                                if let Err(e) = storage
                                    .release_claim(&task.track_id, &worker_name, lease_token)
                                    .await
                                {
                                    warn!(
                                        task_id = %task.track_id,
                                        error = %e,
                                        "Failed to release claim after admission reject"
                                    );
                                }
                                // Brief backoff so we do not hot-spin on the same oversized queue.
                                tokio::time::sleep(Duration::from_millis(200)).await;
                                break;
                            }
                        };

                        claimed = Some((task, lease_token, tenant_permit, admission_permit));
                        break;
                    }

                    let (mut task, lease_token, _tenant_permit, _admission_permit) = match claimed {
                        Some(c) => c,
                        None => continue,
                    };

                    info!(
                        "Worker {} processing task: {} (tenant: {})",
                        worker_id, task.track_id, task.tenant_id
                    );

                    let cancel_token = cancel_registry.register(&task.track_id).await;

                    // Lease heartbeat: refresh_lease CAS; lost ownership aborts via cancel.
                    let heartbeat_track_id = task.track_id.clone();
                    let heartbeat_storage = Arc::clone(&storage);
                    let heartbeat_worker = worker_name.clone();
                    let heartbeat_token = lease_token;
                    let heartbeat_ttl = lease_ttl;
                    let heartbeat_cancel = cancel_token.clone();
                    let _heartbeat_guard = HeartbeatGuard(tokio::spawn(async move {
                        let mut interval =
                            tokio::time::interval(tokio::time::Duration::from_secs(60));
                        interval.tick().await;
                        loop {
                            interval.tick().await;
                            match heartbeat_storage
                                .refresh_lease(
                                    &heartbeat_track_id,
                                    &heartbeat_worker,
                                    heartbeat_token,
                                    heartbeat_ttl,
                                )
                                .await
                            {
                                Ok(true) => {}
                                Ok(false) => {
                                    warn!(
                                        task_id = %heartbeat_track_id,
                                        "Lost task lease — aborting processing"
                                    );
                                    heartbeat_cancel.cancel();
                                    break;
                                }
                                Err(e) => {
                                    debug!(
                                        "Lease refresh failed for task {}: {}",
                                        heartbeat_track_id, e
                                    );
                                }
                            }
                        }
                    }));

                    let timeout_duration = tokio::time::Duration::from_secs(
                        task.metadata
                            .as_ref()
                            .and_then(|m| m.get("processing_timeout_secs"))
                            .and_then(|v| v.as_u64())
                            .unwrap_or(config.processing_timeout_secs),
                    );
                    let span_task_id = task.track_id.clone();
                    let span_tenant_id = task.tenant_id;
                    let span_task_type = task.task_type;
                    let process_result = tokio::time::timeout(
                        timeout_duration,
                        processor
                            .process(&mut task, cancel_token.clone())
                            .instrument(tracing::info_span!(
                                "task_process",
                                worker_id = worker_id,
                                task_id = %span_task_id,
                                tenant_id = %span_tenant_id,
                                task_type = ?span_task_type,
                            )),
                    )
                    .await;

                    match process_result {
                        Ok(Ok(result)) => {
                            task.mark_success(result);
                            info!(
                                "Worker {} completed task: {} (tenant: {})",
                                worker_id, task.track_id, task.tenant_id
                            );
                        }
                        Ok(Err(TaskError::Cancelled(msg))) => {
                            task.mark_cancelled();
                            info!(
                                worker_id = worker_id,
                                task_id = %task.track_id,
                                tenant_id = %task.tenant_id,
                                reason = %msg,
                                "Task cancelled — preserving Cancelled status (no retry)"
                            );
                        }
                        Ok(Err(e)) => {
                            let error_msg = format!("{}", e);
                            task.mark_failed_with_details(
                                crate::types::TaskFailureInfo::from_processing_error(
                                    error_msg.clone(),
                                ),
                            );

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

                            // Classify once (SSOT) — reused for the retry gate
                            // and the terminal reason so they never disagree.
                            let is_permanent =
                                crate::is_permanent_ingestion_failure(&error_msg);
                            let will_retry = config.auto_retry
                                && task.can_retry()
                                && !task.circuit_breaker_tripped
                                && !is_permanent;

                            if will_retry {
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

                                // Claim SSOT: retries must be Pending again (wake-only channel).
                                task.status = TaskStatus::Pending;
                                task.completed_at = None;
                                task.clear_lease();

                                let retry_task = task.clone();
                                let retry_queue = Arc::clone(&queue);
                                let retry_cancel = cancel_registry.clone();
                                let retry_storage = Arc::clone(&storage);
                                tokio::spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(
                                        retry_delay_ms,
                                    ))
                                    .await;

                                    let track_id = retry_task.track_id.clone();
                                    if should_skip_task(
                                        &retry_storage,
                                        &retry_cancel,
                                        &retry_task,
                                    )
                                    .await
                                    {
                                        debug!(
                                            task_id = %track_id,
                                            "Skipping retry — task cancelled or terminal"
                                        );
                                        return;
                                    }
                                    if let Err(e) = retry_queue.send(retry_task).await {
                                        error!(
                                            error.source = "task_worker",
                                            error.action = "requeue_retry",
                                            task_id = %track_id,
                                            error.message = %e,
                                            "Failed to wake queue for retry"
                                        );
                                    }
                                });
                            } else {
                                let reason = if task.circuit_breaker_tripped {
                                    format!(
                                        "Circuit breaker tripped after {} consecutive timeouts. \
                                        Last error: {}",
                                        task.consecutive_timeout_failures, error_msg
                                    )
                                } else if is_permanent {
                                    // Deterministic failure (SPEC-045): not retried.
                                    // Surface the actionable cause directly instead of a
                                    // misleading "retries exhausted" count.
                                    format!("Permanent failure (not retryable): {}", error_msg)
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

                            processor.on_permanent_failure(&task, &timeout_msg).await;
                        }
                    }

                    cancel_registry.deregister(&task.track_id).await;

                    if let Err(e) = storage.update_task(&task).await {
                        error!(
                            error.source = "task_worker",
                            error.action = "persist_task_result",
                            worker_id = worker_id,
                            task_id = %task.track_id,
                            task_status = ?task.status,
                            error.message = %e,
                            "Failed to persist task result"
                        );
                    }

                    // _tenant_permit dropped here
                }

                info!("Worker {} stopped", worker_id);
            });

            self.handles.push(handle);
        }

        // Spawn periodic cleanup task for tenant semaphores (same ingest runtime).
        if let Some(limiter) = self.tenant_limiter.clone() {
            runtime.spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
                    limiter.cleanup_idle().await;
                }
            });
        }
    }

    /// Shutdown the worker pool gracefully within the drain budget (SPEC-083 X-31).
    ///
    /// 1. Signal workers to stop claiming new work
    /// 2. Cancel in-flight tasks cooperatively
    /// 3. Await worker joins up to `EDGEQUAKE_SHUTDOWN_DRAIN_SECS` (default 30)
    /// 4. Abort any remaining worker tasks after the budget
    pub async fn shutdown(self) {
        let drain = crate::shutdown_drain_budget();
        info!(
            drain_secs = drain.as_secs(),
            "Shutting down worker pool (SPEC-083 X-31 drain budget)"
        );

        if let Some(shutdown_tx) = self.shutdown_tx {
            let _ = shutdown_tx.send(());
        }

        let cancelled = self.cancellation_registry.cancel_all_active().await;
        if !cancelled.is_empty() {
            info!(
                count = cancelled.len(),
                "Cancelled in-flight tasks for shutdown drain"
            );
        }

        let aborts: Vec<_> = self.handles.iter().map(|h| h.abort_handle()).collect();
        let join_fut = async {
            for handle in self.handles {
                let _ = handle.await;
            }
        };

        match tokio::time::timeout(drain, join_fut).await {
            Ok(()) => info!("Worker pool shut down within drain budget"),
            Err(_) => {
                warn!(
                    drain_secs = drain.as_secs(),
                    "SPEC-083 X-31: shutdown drain budget exceeded — aborting remaining workers"
                );
                for abort in aborts {
                    abort.abort();
                }
            }
        }
    }

    /// Get number of workers
    pub fn num_workers(&self) -> usize {
        self.config.num_workers
    }
}

/// True when the worker must not start (or requeue) this task.
async fn should_skip_task(
    storage: &Arc<dyn TaskStorage>,
    cancel_registry: &CancellationRegistry,
    task: &Task,
) -> bool {
    if cancel_registry.has_cancel_intent(&task.track_id).await {
        return true;
    }
    if task.status == TaskStatus::Cancelled || task.status == TaskStatus::Indexed {
        return true;
    }
    match storage.get_task(&task.track_id).await {
        Ok(Some(stored)) => {
            stored.status == TaskStatus::Cancelled
                || stored.status == TaskStatus::Indexed
                || stored.is_terminal()
        }
        Ok(None) => false,
        Err(e) => {
            warn!(
                task_id = %task.track_id,
                error = %e,
                "Failed to load task status for cancel/terminal guard — proceeding cautiously"
            );
            false
        }
    }
}

/// Park until a tenant permit frees, then re-enqueue once (no polling storm).
#[allow(clippy::too_many_arguments)] // Internal spawn helper; bundling would obscure wake/cancel wiring.
fn spawn_fairness_park(
    worker_id: usize,
    task: Task,
    fairness_class: FairnessClass,
    queue: Arc<dyn TaskQueue>,
    storage: Arc<dyn TaskStorage>,
    limiter: TenantConcurrencyLimiter,
    cancel_registry: CancellationRegistry,
    park_set: FairnessParkSet,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) {
    tokio::spawn(async move {
        let track_id = task.track_id.clone();
        let tenant_id = task.tenant_id;
        let workspace_id = task.workspace_id;
        // Clears park membership on abort/skip. Success path ends park *before*
        // requeue so claim_next is not treated as a duplicate park.
        let mut park_guard = Some(FairnessParkGuard {
            set: park_set,
            track_id: track_id.clone(),
        });

        let permit = tokio::select! {
            result = limiter.acquire(tenant_id, workspace_id, fairness_class) => {
                match result {
                    Ok(permit) => permit,
                    Err(_) => {
                        debug!(
                            worker_id,
                            task_id = %track_id,
                            "Fairness park aborted — semaphore closed"
                        );
                        return;
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                debug!(
                    worker_id,
                    task_id = %track_id,
                    "Fairness park aborted — worker pool shutting down"
                );
                // End park before send so reclaim is not skipped as "already parked".
                drop(park_guard.take());
                let _ = queue.send(task).await;
                return;
            }
        };

        // Release immediately — the worker will try_acquire when it dequeues.
        // Park waiters are FIFO on the semaphore, so churn is O(completions), not O(polls).
        drop(permit);

        if should_skip_task(&storage, &cancel_registry, &task).await {
            debug!(
                worker_id,
                task_id = %track_id,
                "Fairness park complete — dropping cancelled/terminal task"
            );
            cancel_registry.deregister(&track_id).await;
            return;
        }

        // Plan order: remove from park set, then single queue.send.
        drop(park_guard.take());

        if let Err(e) = queue.send(task).await {
            error!(
                error.source = "task_worker",
                error.action = "requeue_after_fairness_park",
                worker_id,
                task_id = %track_id,
                error.message = %e,
                "Failed to requeue task after fairness park"
            );
        }
    });
}

/// Mock task processor for testing
#[cfg(test)]
pub struct MockTaskProcessor;

#[cfg(test)]
#[async_trait::async_trait]
impl TaskProcessor for MockTaskProcessor {
    async fn process(
        &self,
        task: &mut Task,
        _cancel_token: CancellationToken,
    ) -> TaskResult<serde_json::Value> {
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
            max_lifecycle_tasks_per_tenant: 0,
            processing_timeout_secs: 300, // 5 min for tests
        };

        let mut pool = WorkerPool::new(config, queue.clone(), storage.clone(), processor);
        // Smoke: workers can be started on an explicit Handle (ingest runtime split).
        pool.start_on(&tokio::runtime::Handle::current());

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
            max_lifecycle_tasks_per_tenant: 0,
            processing_timeout_secs: 300,
        };

        let mut pool = WorkerPool::new(config, queue, storage, processor);
        pool.start();

        // Shutdown immediately
        pool.shutdown().await;
    }

    /// SPEC-083 X-31: a stuck in-flight task must not block shutdown past the drain budget.
    #[tokio::test]
    async fn e2e_shutdown_drains_or_cancels_within_budget() {
        std::env::set_var(crate::SHUTDOWN_DRAIN_SECS_ENV, "1");

        struct StickyProcessor;
        #[async_trait::async_trait]
        impl TaskProcessor for StickyProcessor {
            async fn process(
                &self,
                _task: &mut Task,
                cancel_token: CancellationToken,
            ) -> TaskResult<serde_json::Value> {
                tokio::select! {
                    _ = cancel_token.cancelled() => {
                        Err(TaskError::Cancelled("shutdown drain".into()))
                    }
                    _ = tokio::time::sleep(Duration::from_secs(120)) => {
                        Ok(serde_json::json!({"status": "unexpected_complete"}))
                    }
                }
            }
        }

        let queue = Arc::new(ChannelTaskQueue::new(10));
        let storage = Arc::new(MemoryTaskStorage::new());
        let processor = Arc::new(StickyProcessor);

        let config = WorkerPoolConfig {
            num_workers: 1,
            auto_retry: false,
            initial_retry_delay_ms: 100,
            max_retry_delay_ms: 5000,
            backoff_multiplier: 2.0,
            max_tasks_per_tenant: 0,
            max_lifecycle_tasks_per_tenant: 0,
            processing_timeout_secs: 300,
        };

        let mut pool = WorkerPool::new(config, queue.clone(), storage.clone(), processor);
        pool.start_on(&tokio::runtime::Handle::current());

        let task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            serde_json::json!({"sticky": true}),
        );
        storage.create_task(&task).await.unwrap();
        queue.send(task).await.unwrap();

        // Let the worker claim and enter the sticky process loop.
        tokio::time::sleep(Duration::from_millis(150)).await;

        let started = std::time::Instant::now();
        pool.shutdown().await;
        let elapsed = started.elapsed();

        assert!(
            elapsed < Duration::from_secs(5),
            "shutdown must finish within drain budget + slack, took {:?}",
            elapsed
        );
    }

    #[tokio::test]
    async fn test_tenant_fairness_limiting() {
        use crate::tenant_limiter::TryAcquireOutcome;
        use crate::types::FairnessClass;

        // Ingest lane=1: only one ingest task per tenant at a time.
        let limiter = crate::tenant_limiter::TenantConcurrencyLimiter::new(1, 2);
        let tenant = test_tenant_id();
        let ws = test_workspace_id();

        let p1 = match limiter.try_acquire(tenant, ws, FairnessClass::Ingest).await {
            TryAcquireOutcome::Acquired(p) => p,
            other => panic!("First ingest acquire should succeed, got {other:?}"),
        };

        assert!(
            matches!(
                limiter.try_acquire(tenant, ws, FairnessClass::Ingest).await,
                TryAcquireOutcome::AtCapacity
            ),
            "Second ingest acquire should be denied"
        );

        // Lifecycle lane stays independent under local-style clamps.
        assert!(
            matches!(
                limiter
                    .try_acquire(tenant, ws, FairnessClass::Lifecycle)
                    .await,
                TryAcquireOutcome::Acquired(_)
            ),
            "Lifecycle lane must remain available while ingest is saturated"
        );

        let other_tenant = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000099").unwrap();
        assert!(matches!(
            limiter
                .try_acquire(other_tenant, ws, FairnessClass::Ingest)
                .await,
            TryAcquireOutcome::Acquired(_)
        ));

        drop(p1);
        assert!(matches!(
            limiter.try_acquire(tenant, ws, FairnessClass::Ingest).await,
            TryAcquireOutcome::Acquired(_)
        ));
    }

    #[test]
    fn test_heartbeat_guard_aborts_on_drop() {
        // Create a tokio runtime for this test
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let handle = tokio::spawn(async {
                // This task should be aborted when the guard is dropped
                tokio::time::sleep(tokio::time::Duration::from_secs(600)).await;
            });

            // Wrap in guard and drop immediately
            let guard = HeartbeatGuard(handle);
            drop(guard);

            // Give tokio a moment to process the abort
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            // If we get here without hanging, the guard correctly aborted the task
        });
    }

    #[test]
    fn test_calculate_backoff_delay_boundaries() {
        // Attempt 0: initial delay
        assert_eq!(calculate_backoff_delay(0, 1000, 60_000, 2.0), 1000);

        // Attempt 1: 1000 * 2 = 2000
        assert_eq!(calculate_backoff_delay(1, 1000, 60_000, 2.0), 2000);

        // Attempt 5: 1000 * 32 = 32000
        assert_eq!(calculate_backoff_delay(5, 1000, 60_000, 2.0), 32000);

        // Attempt 6: 1000 * 64 = 64000, but capped at 60000
        assert_eq!(calculate_backoff_delay(6, 1000, 60_000, 2.0), 60_000);

        // Very large attempt: should be capped, not overflow
        assert_eq!(calculate_backoff_delay(100, 1000, 60_000, 2.0), 60_000);

        // Multiplier of 1.0: delay stays constant
        assert_eq!(calculate_backoff_delay(5, 1000, 60_000, 1.0), 1000);

        // Zero initial delay: always 0
        assert_eq!(calculate_backoff_delay(3, 0, 60_000, 2.0), 0);
    }

    #[test]
    fn test_worker_pool_config_default_values() {
        let config = WorkerPoolConfig::default();

        // Workers should be at least 4
        assert!(config.num_workers >= 4, "Minimum 4 workers");

        // Timeout must be at least MIN_PROCESSING_TIMEOUT_SECS
        assert!(
            config.processing_timeout_secs >= MIN_PROCESSING_TIMEOUT_SECS,
            "Timeout {} < minimum {}",
            config.processing_timeout_secs,
            MIN_PROCESSING_TIMEOUT_SECS
        );

        // Per-tenant limit should be at least 1
        assert!(
            config.max_tasks_per_tenant >= 1,
            "Per-tenant limit must be >= 1"
        );

        // Per-tenant limit should be less than total workers
        assert!(
            config.max_tasks_per_tenant <= config.num_workers,
            "Per-tenant limit {} should be <= total workers {}",
            config.max_tasks_per_tenant,
            config.num_workers
        );

        // Auto-retry should be enabled by default
        assert!(config.auto_retry, "Auto-retry should be on by default");
    }

    #[tokio::test]
    async fn test_worker_pool_timeout_marks_task_failed() {
        // Create a slow processor that exceeds the timeout
        struct SlowProcessor;

        #[async_trait::async_trait]
        impl TaskProcessor for SlowProcessor {
            async fn process(
                &self,
                _task: &mut Task,
                _cancel_token: CancellationToken,
            ) -> TaskResult<serde_json::Value> {
                // Sleep longer than the timeout
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                Ok(serde_json::json!({"status": "should_not_reach"}))
            }

            async fn on_permanent_failure(&self, _task: &Task, _error_msg: &str) {
                // No-op for test
            }
        }

        let queue = Arc::new(ChannelTaskQueue::new(10));
        let storage = Arc::new(MemoryTaskStorage::new());
        let processor: SharedTaskProcessor = Arc::new(SlowProcessor);

        let config = WorkerPoolConfig {
            num_workers: 1,
            auto_retry: false,
            initial_retry_delay_ms: 100,
            max_retry_delay_ms: 5000,
            backoff_multiplier: 2.0,
            max_tasks_per_tenant: 0,
            max_lifecycle_tasks_per_tenant: 0,
            processing_timeout_secs: 1, // 1 second timeout for quick test
        };

        let mut pool = WorkerPool::new(config, queue.clone(), storage.clone(), processor);
        pool.start();

        // Create and enqueue a task
        let task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            serde_json::json!({"test": "timeout"}),
        );
        let track_id = task.track_id.clone();
        storage.create_task(&task).await.unwrap();
        queue.send(task).await.unwrap();

        // Wait for timeout to fire (1s) + some buffer
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        // Task should be marked as failed due to timeout
        let stored = storage.get_task(&track_id).await.unwrap().unwrap();
        assert_eq!(
            stored.status,
            TaskStatus::Failed,
            "Timed-out task should be failed, got {:?}",
            stored.status
        );
        assert!(
            stored
                .error_message
                .as_ref()
                .unwrap_or(&String::new())
                .contains("timed out"),
            "Error message should mention timeout: {:?}",
            stored.error_message
        );

        pool.shutdown().await;
    }

    #[tokio::test]
    async fn test_cancelled_error_preserves_cancelled_status_no_retry() {
        struct CancelProcessor;

        #[async_trait::async_trait]
        impl TaskProcessor for CancelProcessor {
            async fn process(
                &self,
                _task: &mut Task,
                _cancel_token: CancellationToken,
            ) -> TaskResult<serde_json::Value> {
                Err(TaskError::Cancelled("user cancel".into()))
            }
        }

        let queue = Arc::new(ChannelTaskQueue::new(10));
        let storage = Arc::new(MemoryTaskStorage::new());
        let processor: SharedTaskProcessor = Arc::new(CancelProcessor);

        let config = WorkerPoolConfig {
            num_workers: 1,
            auto_retry: true,
            initial_retry_delay_ms: 10,
            max_retry_delay_ms: 50,
            backoff_multiplier: 2.0,
            max_tasks_per_tenant: 0,
            max_lifecycle_tasks_per_tenant: 0,
            processing_timeout_secs: 300,
        };

        let mut pool = WorkerPool::new(config, queue.clone(), storage.clone(), processor);
        pool.start();

        let task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            serde_json::json!({"test": "cancel"}),
        );
        let track_id = task.track_id.clone();
        storage.create_task(&task).await.unwrap();
        queue.send(task).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let stored = storage.get_task(&track_id).await.unwrap().unwrap();
        assert_eq!(stored.status, TaskStatus::Cancelled);
        assert!(!stored.can_retry());

        pool.shutdown().await;
    }

    #[tokio::test]
    async fn test_provider_misconfig_fails_fast_no_retry() {
        // A deterministic provider misconfiguration (missing credential) must
        // fail permanently on the first attempt — no retry budget spent, no
        // misleading "retries exhausted", and an actionable reason recorded.
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct MisconfigProcessor {
            attempts: Arc<AtomicUsize>,
            permanent_reason: Arc<std::sync::Mutex<Option<String>>>,
        }

        #[async_trait::async_trait]
        impl TaskProcessor for MisconfigProcessor {
            async fn process(
                &self,
                _task: &mut Task,
                _cancel_token: CancellationToken,
            ) -> TaskResult<serde_json::Value> {
                self.attempts.fetch_add(1, Ordering::SeqCst);
                Err(TaskError::Processing(
                    "Failed to create vision provider 'mistral': Configuration error: \
                     MISTRAL_API_KEY is not set. To use the Mistral provider, set the \
                     environment variable and restart the server."
                        .to_string(),
                ))
            }

            async fn on_permanent_failure(&self, _task: &Task, error_msg: &str) {
                *self.permanent_reason.lock().unwrap() = Some(error_msg.to_string());
            }
        }

        let queue = Arc::new(ChannelTaskQueue::new(10));
        let storage = Arc::new(MemoryTaskStorage::new());
        let attempts = Arc::new(AtomicUsize::new(0));
        let permanent_reason = Arc::new(std::sync::Mutex::new(None));
        let processor: SharedTaskProcessor = Arc::new(MisconfigProcessor {
            attempts: Arc::clone(&attempts),
            permanent_reason: Arc::clone(&permanent_reason),
        });

        let config = WorkerPoolConfig {
            num_workers: 1,
            auto_retry: true, // retry is ON — misconfig must still not retry
            initial_retry_delay_ms: 10,
            max_retry_delay_ms: 50,
            backoff_multiplier: 2.0,
            max_tasks_per_tenant: 0,
            max_lifecycle_tasks_per_tenant: 0,
            processing_timeout_secs: 300,
        };

        let mut pool = WorkerPool::new(config, queue.clone(), storage.clone(), processor);
        pool.start();

        let task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            serde_json::json!({"test": "misconfig"}),
        );
        let track_id = task.track_id.clone();
        storage.create_task(&task).await.unwrap();
        queue.send(task).await.unwrap();

        // Give ample time for any (incorrect) retries to fire.
        tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;

        let stored = storage.get_task(&track_id).await.unwrap().unwrap();
        assert_eq!(
            stored.status,
            TaskStatus::Failed,
            "misconfig task must be Failed, got {:?}",
            stored.status
        );
        assert_eq!(
            attempts.load(Ordering::SeqCst),
            1,
            "misconfig must not be retried (expected exactly 1 processing attempt)"
        );
        assert!(
            stored.retry_count < stored.max_retries,
            "must fail before exhausting retries: retry_count={} max_retries={}",
            stored.retry_count,
            stored.max_retries
        );
        let reason = permanent_reason.lock().unwrap().clone().unwrap_or_default();
        assert!(
            reason.contains("not retryable"),
            "reason should mark non-retryable, got: {reason}"
        );
        assert!(
            !reason.contains("Retries exhausted"),
            "misleading 'Retries exhausted' must not appear: {reason}"
        );

        pool.shutdown().await;
    }

    #[tokio::test]
    async fn test_cancel_intent_skips_pending_task() {
        let queue = Arc::new(ChannelTaskQueue::new(10));
        let storage = Arc::new(MemoryTaskStorage::new());
        let processor = Arc::new(MockTaskProcessor);

        let config = WorkerPoolConfig {
            num_workers: 1,
            auto_retry: false,
            initial_retry_delay_ms: 100,
            max_retry_delay_ms: 5000,
            backoff_multiplier: 2.0,
            max_tasks_per_tenant: 0,
            max_lifecycle_tasks_per_tenant: 0,
            processing_timeout_secs: 300,
        };

        let mut pool = WorkerPool::new(config, queue.clone(), storage.clone(), processor);
        let registry = pool.cancellation_registry();
        pool.start();

        let task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            serde_json::json!({"test": "pending-cancel"}),
        );
        let track_id = task.track_id.clone();
        storage.create_task(&task).await.unwrap();
        registry.mark_cancel_intent(&track_id).await;
        queue.send(task).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        let stored = storage.get_task(&track_id).await.unwrap().unwrap();
        // SPEC-057 P1: claim then cancel-intent → Cancelled (not left Pending).
        assert_eq!(stored.status, TaskStatus::Cancelled);

        pool.shutdown().await;
    }

    #[tokio::test]
    async fn test_fairness_park_does_not_storm_queue() {
        let queue = Arc::new(ChannelTaskQueue::new(50));
        let storage = Arc::new(MemoryTaskStorage::new());

        struct SlowProcessor;
        #[async_trait::async_trait]
        impl TaskProcessor for SlowProcessor {
            async fn process(
                &self,
                _task: &mut Task,
                _cancel_token: CancellationToken,
            ) -> TaskResult<serde_json::Value> {
                tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                Ok(serde_json::json!({"ok": true}))
            }
        }

        let config = WorkerPoolConfig {
            num_workers: 4,
            auto_retry: false,
            initial_retry_delay_ms: 100,
            max_retry_delay_ms: 5000,
            backoff_multiplier: 2.0,
            max_tasks_per_tenant: 1,
            max_lifecycle_tasks_per_tenant: 1,
            processing_timeout_secs: 300,
        };

        let mut pool = WorkerPool::new(
            config,
            queue.clone(),
            storage.clone(),
            Arc::new(SlowProcessor),
        );
        let limiter = pool.tenant_limiter().expect("limiter enabled");
        pool.start();

        // One slow holder + several Pending that would otherwise reclaim-storm.
        for i in 0..5 {
            let task = Task::new(
                test_tenant_id(),
                test_workspace_id(),
                TaskType::Insert,
                serde_json::json!({"i": i}),
            );
            storage.create_task(&task).await.unwrap();
            queue.send(task).await.unwrap();
        }

        // Let workers reclaim the same Pending repeatedly via poll interval.
        tokio::time::sleep(tokio::time::Duration::from_millis(2500)).await;
        let stats = limiter.stats().await;
        // Deduped park: waiters ≤ backlog tasks; completions stay O(tasks), not O(polls×workers).
        assert!(
            stats.park_waiters <= 5,
            "unexpected park waiter count {}",
            stats.park_waiters
        );
        assert!(
            stats.park_completions <= 8,
            "park reclaim storm: completions={}",
            stats.park_completions
        );
        assert!(
            queue.approximate_depth() <= 8,
            "queue storm detected: depth={}",
            queue.approximate_depth()
        );

        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        pool.shutdown().await;
    }

    #[tokio::test]
    async fn test_delete3_upload1_lifecycle_lane_does_not_starve_ingest() {
        use std::sync::atomic::AtomicUsize;

        struct CountingProcessor {
            deletion_started: Arc<AtomicUsize>,
            pdf_started: Arc<AtomicUsize>,
            pdf_gate: Arc<tokio::sync::Notify>,
        }

        #[async_trait::async_trait]
        impl TaskProcessor for CountingProcessor {
            async fn process(
                &self,
                task: &mut Task,
                _cancel_token: CancellationToken,
            ) -> TaskResult<serde_json::Value> {
                match task.task_type {
                    TaskType::Deletion => {
                        self.deletion_started.fetch_add(1, Ordering::SeqCst);
                        // Hold lifecycle slot while PDF should still acquire ingest.
                        tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;
                    }
                    TaskType::PdfProcessing => {
                        self.pdf_started.fetch_add(1, Ordering::SeqCst);
                        self.pdf_gate.notify_waiters();
                        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                    }
                    _ => {}
                }
                Ok(serde_json::json!({"ok": true}))
            }
        }

        let deletion_started = Arc::new(AtomicUsize::new(0));
        let pdf_started = Arc::new(AtomicUsize::new(0));
        let pdf_gate = Arc::new(tokio::sync::Notify::new());
        let processor: SharedTaskProcessor = Arc::new(CountingProcessor {
            deletion_started: Arc::clone(&deletion_started),
            pdf_started: Arc::clone(&pdf_started),
            pdf_gate: Arc::clone(&pdf_gate),
        });

        let queue = Arc::new(ChannelTaskQueue::new(50));
        let storage = Arc::new(MemoryTaskStorage::new());
        // Local Ollama-style raised defaults: workers=4, ingest=2, lifecycle=4.
        let config = WorkerPoolConfig {
            num_workers: 4,
            auto_retry: false,
            initial_retry_delay_ms: 100,
            max_retry_delay_ms: 5000,
            backoff_multiplier: 2.0,
            max_tasks_per_tenant: 2,
            max_lifecycle_tasks_per_tenant: 4,
            processing_timeout_secs: 300,
        };
        let mut pool = WorkerPool::new(config, queue.clone(), storage.clone(), processor);
        let limiter = pool.tenant_limiter().expect("dual-lane limiter");
        assert_eq!(limiter.max_per_tenant(), 2);
        assert_eq!(limiter.max_lifecycle_per_tenant(), 4);
        pool.start();

        for i in 0..3 {
            let mut task = Task::new(
                test_tenant_id(),
                test_workspace_id(),
                TaskType::Deletion,
                serde_json::json!({
                    "document_id": format!("doc-{i}"),
                }),
            );
            // Align payload id with durable track_id (wipe pattern).
            if let Some(obj) = task.task_data.as_object_mut() {
                obj.insert("deletion_track_id".into(), serde_json::json!(task.track_id));
            }
            storage.create_task(&task).await.unwrap();
            queue.send(task).await.unwrap();
        }
        let pdf = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::PdfProcessing,
            serde_json::json!({"document_id": "pdf-new"}),
        );
        let pdf_id = pdf.track_id.clone();
        storage.create_task(&pdf).await.unwrap();
        queue.send(pdf).await.unwrap();

        // PDF must start while deletions are still running (separate lanes).
        tokio::time::timeout(tokio::time::Duration::from_secs(3), pdf_gate.notified())
            .await
            .expect("PDF should leave Queued and start under lifecycle load");
        assert!(
            pdf_started.load(Ordering::SeqCst) >= 1,
            "PDF ingest lane must run"
        );
        assert!(
            deletion_started.load(Ordering::SeqCst) >= 1,
            "at least one deletion should have started"
        );

        tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
        let stored = storage.get_task(&pdf_id).await.unwrap().unwrap();
        assert_eq!(
            stored.status,
            TaskStatus::Indexed,
            "PDF must complete, got {:?}",
            stored.status
        );
        pool.shutdown().await;
    }
}
