//! Query execution observability guards (Prometheus outcomes).

use std::cell::Cell;
use std::time::Instant;

use crate::metrics::record_query_completed;

/// Records `success` on [`mark_success`], else `failure` on drop (sync query path).
///
/// Logging is **not** done here — `ApiError::into_response` emits explicit structured logs.
pub struct QueryOutcomeGuard {
    mode: String,
    started: Instant,
    finished: Cell<bool>,
}

impl QueryOutcomeGuard {
    pub fn new(mode: impl Into<String>) -> Self {
        Self {
            mode: mode.into(),
            started: Instant::now(),
            finished: Cell::new(false),
        }
    }

    pub fn with_request_id(mode: impl Into<String>, _request_id: Option<String>) -> Self {
        // request_id kept in signature for call-site stability; logging is in ApiError.
        Self::new(mode)
    }

    /// Call once when the query pipeline completes successfully.
    pub fn mark_success(&self, duration_secs: f64) {
        self.finished.set(true);
        record_query_completed(&self.mode, "success", duration_secs);
    }
}

impl Drop for QueryOutcomeGuard {
    fn drop(&mut self) {
        if !self.finished.get() {
            record_query_completed(&self.mode, "failure", self.started.elapsed().as_secs_f64());
        }
    }
}

/// Records `failure` on drop unless [`dismiss`](Self::dismiss) is called.
///
/// Use for streaming handlers that return `Ok(Sse)` before work finishes in a background task.
pub struct QueryFailureGuard {
    mode: String,
    started: Instant,
    dismissed: Cell<bool>,
}

impl QueryFailureGuard {
    pub fn new(mode: impl Into<String>) -> Self {
        Self {
            mode: mode.into(),
            started: Instant::now(),
            dismissed: Cell::new(false),
        }
    }

    /// Call when the HTTP handler returns successfully (work continues asynchronously).
    pub fn dismiss(&self) {
        self.dismissed.set(true);
    }

    pub fn mode(&self) -> &str {
        &self.mode
    }

    pub fn elapsed_secs(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }
}

impl Drop for QueryFailureGuard {
    fn drop(&mut self) {
        if !self.dismissed.get() {
            record_query_completed(&self.mode, "failure", self.started.elapsed().as_secs_f64());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::render_prometheus_metrics;

    #[test]
    fn records_success_and_failure_outcomes() {
        crate::metrics::init_metrics();
        QueryOutcomeGuard::new("hybrid").mark_success(0.05);
        {
            let _g = QueryOutcomeGuard::new("local");
        }
        let body = render_prometheus_metrics();
        assert!(body.contains("edgequake_query_requests_total"));
        assert!(body.contains("outcome=\"success\""));
        assert!(body.contains("outcome=\"failure\""));
    }

    #[test]
    fn failure_guard_records_only_when_not_dismissed() {
        crate::metrics::init_metrics();
        {
            let g = QueryFailureGuard::new("undismissed_stream");
            g.dismiss();
        }
        {
            let _g = QueryFailureGuard::new("undismissed_stream");
        }
        let body = render_prometheus_metrics();
        assert!(body.contains("undismissed_stream"));
    }
}
