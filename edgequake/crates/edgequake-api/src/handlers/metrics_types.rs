//! DTOs for metrics handlers.
//!
//! This module contains the types for Prometheus metrics export.

use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};

// ============================================================================
// Response Types
// ============================================================================

/// Metrics response in Prometheus format.
///
/// This type wraps a String containing Prometheus-formatted metrics
/// and implements IntoResponse to serve with the correct content type.
pub struct PrometheusMetrics(pub String);

impl PrometheusMetrics {
    /// Create a new PrometheusMetrics response.
    pub fn new(content: impl Into<String>) -> Self {
        Self(content.into())
    }
}

impl IntoResponse for PrometheusMetrics {
    fn into_response(self) -> Response {
        (
            StatusCode::OK,
            [(
                header::CONTENT_TYPE,
                "text/plain; version=0.0.4; charset=utf-8",
            )],
            self.0,
        )
            .into_response()
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn test_prometheus_metrics_creation() {
        let metrics = PrometheusMetrics::new("edgequake_info{version=\"0.1.0\"} 1");
        assert!(metrics.0.contains("edgequake_info"));
    }

    #[test]
    fn test_prometheus_metrics_into_response() {
        let metrics = PrometheusMetrics::new("# Test metrics");
        let response = metrics.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
