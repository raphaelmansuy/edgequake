//! Interactive document/tenant read-path admission (local-ingest hardening).
//!
//! Control plane (`/live`, `/health`) must not share fate with long-running
//! ingest. Document list/detail and tenant/workspace list acquire a
//! [`ReadPathDbPermit`] and run under a hard deadline; on pressure they return
//! 503 `read_path_busy` instead of hanging until the pool acquire timeout.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::{timeout_at, Instant};
use tracing::{info, warn};

use crate::error::{ApiError, ApiResult};

/// Default document read deadline (ms) when env is unset.
///
/// SPEC-089 Phase 4 / F-336-13 / LAW-H2: any SQL under this envelope must use
/// a Postgres kill **strictly under** this budget (storage
/// `interactive_statement_timeout_ms` = budget − 250ms). Worker task wall-clock
/// (≤7200s) is LLM-bound — per-statement kills stay on `LocalTimeoutTx` / AGE
/// session GUCs, not this interactive budget.
pub const DEFAULT_DOCUMENTS_READ_TIMEOUT_MS: u64 = 2_500;

/// Cap on workspace KV metadata entries scanned before pagination.
pub const MAX_LIST_METADATA_ENTRIES: usize = 2_000;

/// Process-wide permit for interactive HTTP DB reads (list/get docs, tenants, workspaces).
///
/// Sized `max(2, DATABASE_POOL_SIZE / 8)`. Ingest workers never take this permit.
#[derive(Debug, Clone)]
pub struct ReadPathDbPermit {
    semaphore: Arc<Semaphore>,
    max_concurrent: usize,
}

impl ReadPathDbPermit {
    pub fn new(max_concurrent: usize) -> Self {
        let max_concurrent = max_concurrent.max(1);
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent,
        }
    }

    /// Size from pool: `max(2, pool_size / 8)`.
    pub fn from_pool_size(pool_size: usize) -> Self {
        Self::new((pool_size / 8).max(2))
    }

    pub fn from_env() -> Self {
        let pool_size: usize = std::env::var("DATABASE_POOL_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(32);
        let permit = Self::from_pool_size(pool_size);
        info!(
            read_path_permits = permit.max_concurrent,
            database_pool_size = pool_size,
            "Read-path DB bulkhead configured"
        );
        permit
    }

    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }

    /// Wait up to `timeout` for a permit; on deadline return `read_path_busy`.
    pub async fn acquire(&self, timeout: Duration) -> ApiResult<OwnedSemaphorePermit> {
        let wait_started = Instant::now();
        match tokio::time::timeout(timeout, self.semaphore.clone().acquire_owned()).await {
            Ok(Ok(permit)) => {
                let waited_ms = wait_started.elapsed().as_millis() as u64;
                if waited_ms >= 50 {
                    info!(
                        waited_ms,
                        max_concurrent = self.max_concurrent,
                        available = self.semaphore.available_permits(),
                        "Read-path permit acquired after wait"
                    );
                }
                Ok(permit)
            }
            Ok(Err(_)) => {
                warn!(
                    timeout_ms = timeout.as_millis() as u64,
                    max_concurrent = self.max_concurrent,
                    "Read-path permit closed — returning read_path_busy"
                );
                Err(ApiError::read_path_busy(timeout.as_millis() as u64))
            }
            Err(_) => {
                warn!(
                    timeout_ms = timeout.as_millis() as u64,
                    max_concurrent = self.max_concurrent,
                    available = self.semaphore.available_permits(),
                    "Read-path permit wait exceeded — returning read_path_busy"
                );
                Err(ApiError::read_path_busy(timeout.as_millis() as u64))
            }
        }
    }
}

/// Resolve `EDGEQUAKE_DOCUMENTS_READ_TIMEOUT_MS` (default 2500ms).
pub fn documents_read_timeout() -> Duration {
    let ms = std::env::var("EDGEQUAKE_DOCUMENTS_READ_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_DOCUMENTS_READ_TIMEOUT_MS)
        .clamp(500, 30_000);
    Duration::from_millis(ms)
}

/// Postgres kill budget for SQL under [`run_with_read_path_guard`] (LAW-H2).
///
/// Strictly less than [`documents_read_timeout`] so abandoned futures cannot
/// leave zombie pool holders (SPEC-089 Phase 4 / F-336-13).
#[cfg(feature = "postgres")]
pub fn documents_read_pg_timeout_ms() -> u32 {
    edgequake_storage::interactive_statement_timeout_ms()
}

#[cfg(not(feature = "postgres"))]
pub fn documents_read_pg_timeout_ms() -> u32 {
    documents_read_timeout()
        .as_millis()
        .saturating_sub(250)
        .max(1) as u32
}

/// Remaining budget until `deadline` (zero if already elapsed).
fn remaining_until(deadline: Instant) -> Duration {
    deadline.saturating_duration_since(Instant::now())
}

/// Run an interactive read under the bulkhead + a **single** wall-clock deadline.
///
/// Permit wait and handler work share one envelope (`documents_read_timeout()`),
/// so worst-case latency is one timeout — not 2× stacked budgets.
pub async fn run_with_read_path_guard<T, F, Fut>(
    permits: &ReadPathDbPermit,
    work: F,
) -> ApiResult<T>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ApiResult<T>>,
{
    let timeout = documents_read_timeout();
    let deadline = Instant::now() + timeout;
    let retry_after_ms = timeout.as_millis() as u64;

    let remaining = remaining_until(deadline);
    if remaining.is_zero() {
        return Err(ApiError::read_path_busy(retry_after_ms));
    }
    let _permit = permits.acquire(remaining).await?;

    let remaining = remaining_until(deadline);
    if remaining.is_zero() {
        return Err(ApiError::read_path_busy(retry_after_ms));
    }
    match timeout_at(deadline, work()).await {
        Ok(result) => result,
        Err(_) => {
            warn!(
                timeout_ms = retry_after_ms,
                reason = "work_deadline",
                "Interactive read path exceeded deadline"
            );
            Err(ApiError::read_path_busy(retry_after_ms))
        }
    }
}

/// App wait for task stats before list reconcile.
///
/// LAW-H2: must be **greater** than tasks `STATS_STATEMENT_TIMEOUT_MS` (500)
/// so Postgres cancels the COUNT before tokio abandons the future (zombie pool).
const ENTITY_RECONCILE_STATS_APP_TIMEOUT_MS: u64 = 550;

/// Whether list should skip AGE entity-count reconcile (queue critical / stats hang).
pub async fn should_skip_entity_reconcile(
    task_storage: &edgequake_tasks::SharedTaskStorage,
) -> bool {
    use crate::task_queue_pressure::health_degraded_by_queue;

    match tokio::time::timeout(
        Duration::from_millis(ENTITY_RECONCILE_STATS_APP_TIMEOUT_MS),
        task_storage.get_statistics(edgequake_tasks::storage::TaskFilter::default()),
    )
    .await
    {
        Ok(Ok(stats)) => {
            let skip = health_degraded_by_queue(stats.pending);
            if skip {
                info!(
                    pending = stats.pending,
                    "Skipping AGE entity-count reconcile under queue pressure"
                );
            }
            skip
        }
        Ok(Err(e)) => {
            warn!(error = %e, "Task stats failed — skipping AGE entity-count reconcile");
            true
        }
        Err(_) => {
            warn!("Task stats timed out — skipping AGE entity-count reconcile");
            true
        }
    }
}

/// Warn when local LLM + oversized pool (do not silently shrink operator overrides).
pub fn warn_if_local_pool_oversized(pool_size: usize, llm_provider_name: &str) {
    let allow_high = std::env::var("EDGEQUAKE_ALLOW_LOCAL_HIGH_CONCURRENCY")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if allow_high {
        return;
    }
    if edgequake_pipeline::is_local_extraction_provider(llm_provider_name) && pool_size > 16 {
        warn!(
            pool_size,
            provider = llm_provider_name,
            "DATABASE_POOL_SIZE > 16 with local LLM — consider DATABASE_POOL_SIZE=16 for Ollama \
             (set EDGEQUAKE_ALLOW_LOCAL_HIGH_CONCURRENCY=1 to silence)"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permit_size_is_at_least_two() {
        assert_eq!(ReadPathDbPermit::from_pool_size(8).max_concurrent(), 2);
        assert_eq!(ReadPathDbPermit::from_pool_size(32).max_concurrent(), 4);
        assert_eq!(ReadPathDbPermit::from_pool_size(64).max_concurrent(), 8);
    }

    #[test]
    fn documents_read_pg_timeout_under_app_budget() {
        let prev = std::env::var("EDGEQUAKE_DOCUMENTS_READ_TIMEOUT_MS").ok();
        std::env::remove_var("EDGEQUAKE_DOCUMENTS_READ_TIMEOUT_MS");
        let app_ms = documents_read_timeout().as_millis() as u32;
        let pg_ms = documents_read_pg_timeout_ms();
        assert!(pg_ms < app_ms, "LAW-H2: pg={pg_ms} must be < app={app_ms}");
        if let Some(v) = prev {
            std::env::set_var("EDGEQUAKE_DOCUMENTS_READ_TIMEOUT_MS", v);
        }
    }

    #[tokio::test]
    async fn acquire_times_out_when_saturated() {
        let permits = ReadPathDbPermit::new(1);
        let _held = permits
            .acquire(Duration::from_millis(50))
            .await
            .expect("first");
        let err = permits
            .acquire(Duration::from_millis(30))
            .await
            .expect_err("should busy");
        assert_eq!(err.code(), "read_path_busy");
    }

    #[tokio::test]
    async fn single_envelope_includes_permit_wait() {
        // Floor clamp is 500ms — hold the only permit; guard must fail within
        // ~one envelope (~500ms), not 2× stacked (~1000ms).
        std::env::set_var("EDGEQUAKE_DOCUMENTS_READ_TIMEOUT_MS", "500");
        let permits = ReadPathDbPermit::new(1);
        let _held = permits
            .acquire(Duration::from_millis(50))
            .await
            .expect("hold");

        let started = Instant::now();
        let err = run_with_read_path_guard(&permits, || async { Ok::<_, ApiError>(()) })
            .await
            .expect_err("should busy under saturation");
        assert_eq!(err.code(), "read_path_busy");
        let elapsed = started.elapsed();
        assert!(
            elapsed < Duration::from_millis(750),
            "elapsed {:?} exceeded single-envelope budget (would be ~1s if stacked)",
            elapsed
        );
        std::env::remove_var("EDGEQUAKE_DOCUMENTS_READ_TIMEOUT_MS");
    }

    #[test]
    fn documents_read_timeout_clamps_bounds() {
        std::env::set_var("EDGEQUAKE_DOCUMENTS_READ_TIMEOUT_MS", "1");
        assert_eq!(documents_read_timeout(), Duration::from_millis(500));
        std::env::set_var("EDGEQUAKE_DOCUMENTS_READ_TIMEOUT_MS", "999999");
        assert_eq!(documents_read_timeout(), Duration::from_millis(30_000));
        std::env::set_var("EDGEQUAKE_DOCUMENTS_READ_TIMEOUT_MS", "2500");
        assert_eq!(documents_read_timeout(), Duration::from_millis(2500));
        std::env::remove_var("EDGEQUAKE_DOCUMENTS_READ_TIMEOUT_MS");
    }

    #[tokio::test]
    async fn work_deadline_uses_remaining_budget_after_acquire() {
        std::env::set_var("EDGEQUAKE_DOCUMENTS_READ_TIMEOUT_MS", "500");
        let permits = ReadPathDbPermit::new(1);
        let started = Instant::now();
        let err = run_with_read_path_guard(&permits, || async {
            tokio::time::sleep(Duration::from_millis(800)).await;
            Ok::<_, ApiError>(())
        })
        .await
        .expect_err("slow work must hit single envelope");
        assert_eq!(err.code(), "read_path_busy");
        let elapsed = started.elapsed();
        assert!(
            elapsed < Duration::from_millis(750),
            "elapsed {:?} should be ~500ms envelope, not full sleep",
            elapsed
        );
        std::env::remove_var("EDGEQUAKE_DOCUMENTS_READ_TIMEOUT_MS");
    }

    #[tokio::test]
    async fn permit_released_after_work_timeout() {
        std::env::set_var("EDGEQUAKE_DOCUMENTS_READ_TIMEOUT_MS", "500");
        let permits = ReadPathDbPermit::new(1);
        let _ = run_with_read_path_guard(&permits, || async {
            tokio::time::sleep(Duration::from_millis(800)).await;
            Ok::<_, ApiError>(())
        })
        .await;
        // After timed-out work, the held permit must be free for the next caller.
        let acquired = permits
            .acquire(Duration::from_millis(50))
            .await
            .expect("permit should be released after timeout");
        drop(acquired);
        std::env::remove_var("EDGEQUAKE_DOCUMENTS_READ_TIMEOUT_MS");
    }
}
