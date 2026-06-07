//! Metrics handlers for observability.
//!
//! ## Implements
//!
//! - **FEAT0590**: Prometheus metrics endpoint for scraping
//! - **FEAT0591**: HTTP request counter metrics
//! - **FEAT0592**: Request duration histogram
//!
//! ## Enforces
//!
//! - **BR0590**: Metrics must be in Prometheus text format
//! - **BR0591**: Metrics endpoint must not require authentication

use axum::extract::State;

use crate::state::AppState;

pub use crate::handlers::metrics_types::PrometheusMetrics;

/// Sample runtime gauges (DB pool) immediately before scrape.
fn refresh_runtime_gauges(state: &AppState) {
    #[cfg(feature = "postgres")]
    if let Some(ref pool) = state.pg_pool {
        edgequake_observability::record_db_pool_stats(
            pool.size(),
            pool.num_idle().min(u32::MAX as usize) as u32,
        );
    }
    #[cfg(not(feature = "postgres"))]
    let _ = state;
}

/// Get Prometheus metrics (live recorder — no static placeholders).
///
/// GET /metrics
#[utoipa::path(
    get,
    path = "/metrics",
    tag = "Observability",
    responses(
        (status = 200, description = "Prometheus metrics in text format", content_type = "text/plain"),
        (status = 500, description = "Failed to gather metrics")
    )
)]
pub async fn get_metrics(State(state): State<AppState>) -> PrometheusMetrics {
    edgequake_observability::init_metrics();
    refresh_runtime_gauges(&state);
    PrometheusMetrics(edgequake_observability::render_prometheus_metrics())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    #[tokio::test]
    async fn test_get_metrics() {
        let state = AppState::test_state();
        let response = get_metrics(State(state)).await;
        let response = response.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_metrics_format() {
        let state = AppState::test_state();
        let response = get_metrics(State(state)).await;
        let metrics = response.0;
        assert!(
            metrics.contains("edgequake_http_requests_total"),
            "expected pre-registered HTTP counter in scrape: {metrics:?}"
        );
    }
}
