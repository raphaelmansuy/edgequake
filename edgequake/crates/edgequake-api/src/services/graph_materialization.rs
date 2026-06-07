//! Graph materialization admission — SPEC-006 (SRP/DRY).
//!
//! Single entry for semaphore + query timeout on graph materialization endpoints.

use std::future::Future;
use std::time::Duration;

use tokio::sync::OwnedSemaphorePermit;
use tracing::warn;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// Holds a graph materialization slot until dropped.
pub struct GraphMaterializationGuard {
    _permit: OwnedSemaphorePermit,
}

/// Acquire global materialization permit (503 immediately when at capacity — never queues).
pub fn admit_graph_materialization(state: &AppState) -> ApiResult<GraphMaterializationGuard> {
    let permit = state
        .graph_materialize
        .try_acquire_owned()
        .ok_or_else(ApiError::graph_materialization_busy)?;
    Ok(GraphMaterializationGuard { _permit: permit })
}

/// Query timeout from AppState resource SSOT.
pub fn graph_query_timeout(state: &AppState) -> Duration {
    Duration::from_secs(state.resource_budget().graph_query_timeout_secs)
}

fn is_db_statement_timeout(message: &str) -> bool {
    message.contains("statement timeout") || message.contains("canceling statement")
}

/// Run a graph storage query with timeout budget; never falls back to full-graph scan.
pub async fn run_timed_graph_query<T, E>(
    state: &AppState,
    label: &'static str,
    fut: impl Future<Output = Result<T, E>>,
) -> ApiResult<T>
where
    E: std::fmt::Display + Into<ApiError>,
{
    let timeout = graph_query_timeout(state);
    match tokio::time::timeout(timeout, fut).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(e)) => {
            let message = e.to_string();
            if is_db_statement_timeout(&message) {
                warn!(
                    label,
                    error = %message,
                    "Database graph query timed out — SPEC-006: no full-graph fallback"
                );
                Err(ApiError::graph_query_timeout())
            } else {
                Err(e.into())
            }
        }
        Err(_) => {
            warn!(
                label,
                timeout_secs = timeout.as_secs(),
                "Graph query timed out (tokio) — SPEC-006: no full-graph fallback"
            );
            Err(ApiError::graph_query_timeout())
        }
    }
}

#[cfg(test)]
mod tests {
    use edgequake_core::GraphMaterializationSemaphore;
    use std::sync::Arc;

    #[tokio::test]
    async fn admit_returns_busy_when_semaphore_exhausted() {
        let sem = Arc::new(GraphMaterializationSemaphore::new(1));
        let _held = sem.acquire_owned().await.expect("first slot");
        assert!(sem.try_acquire_owned().is_none());
    }
}
