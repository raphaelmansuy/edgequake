//! Bounded health probe helpers (SPEC-021 P-G13).
//!
//! First principle: **liveness must never compete with ingestion for DB pool slots**.
//! Deep `/health` checks run storage `ping()` with a hard timeout so a saturated pool
//! returns `degraded` instead of blocking the HTTP handler for `acquire_timeout` (5s).

use std::time::Duration;

use edgequake_storage::traits::{GraphStorage, KVStorage, VectorStorage};
use std::sync::Arc;

/// Per-component ping budget. Sum of three parallel pings stays under 1s wall time.
pub const COMPONENT_PING_TIMEOUT: Duration = Duration::from_millis(750);

/// Run an async probe with a hard timeout; `false` on timeout or error.
pub async fn probe_with_timeout<F, Fut>(timeout: Duration, probe: F) -> bool
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    tokio::time::timeout(timeout, probe())
        .await
        .unwrap_or(false)
}

/// Ping all storage backends in parallel with bounded waits.
pub async fn probe_storage_components(
    kv: Arc<dyn KVStorage>,
    vector: Arc<dyn VectorStorage>,
    graph: Arc<dyn GraphStorage>,
) -> (bool, bool, bool) {
    let timeout = COMPONENT_PING_TIMEOUT;
    let (kv_ok, vector_ok, graph_ok) = tokio::join!(
        probe_with_timeout(timeout, || async { kv.ping().await.is_ok() }),
        probe_with_timeout(timeout, || async { vector.ping().await.is_ok() }),
        probe_with_timeout(timeout, || async { graph.ping().await.is_ok() }),
    );
    (kv_ok, vector_ok, graph_ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn probe_with_timeout_returns_false_on_slow_probe() {
        let ok = probe_with_timeout(Duration::from_millis(10), || async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            true
        })
        .await;
        assert!(!ok);
    }

    #[tokio::test]
    async fn probe_with_timeout_returns_true_when_fast() {
        let ok = probe_with_timeout(Duration::from_millis(50), || async { true }).await;
        assert!(ok);
    }
}
