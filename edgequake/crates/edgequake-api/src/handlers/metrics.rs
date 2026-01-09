//! Metrics handlers for observability.
//!
//! ## Implements
//!
//! - **FEAT0590**: Prometheus metrics endpoint for scraping
//! - **FEAT0591**: HTTP request counter metrics
//! - **FEAT0592**: Request duration histogram
//! - **FEAT0593**: In-flight request gauge
//!
//! ## Use Cases
//!
//! - **UC2190**: DevOps scrapes metrics for Prometheus/Grafana dashboards
//! - **UC2191**: Operator monitors request latency distribution
//! - **UC2192**: Alert system triggers on high error rates
//!
//! ## Enforces
//!
//! - **BR0590**: Metrics must be in Prometheus text format
//! - **BR0591**: Metrics endpoint must not require authentication

// Re-export DTOs from metrics_types for backwards compatibility
pub use crate::handlers::metrics_types::PrometheusMetrics;

// ============================================================================
// Handlers
// ============================================================================

/// Get Prometheus metrics.
///
/// GET /metrics
///
/// Returns metrics in Prometheus text format for scraping.
#[utoipa::path(
    get,
    path = "/metrics",
    tag = "Observability",
    responses(
        (status = 200, description = "Prometheus metrics in text format", content_type = "text/plain"),
        (status = 500, description = "Failed to gather metrics")
    )
)]
pub async fn get_metrics() -> PrometheusMetrics {
    // TODO: In production, use prometheus crate to gather actual metrics
    // For now, return placeholder metrics
    let metrics = r#"# HELP edgequake_info EdgeQuake version information
# TYPE edgequake_info gauge
edgequake_info{version="0.1.0"} 1

# HELP edgequake_http_requests_total Total HTTP requests
# TYPE edgequake_http_requests_total counter
edgequake_http_requests_total{method="GET",path="/health",status="200"} 0
edgequake_http_requests_total{method="GET",path="/ready",status="200"} 0
edgequake_http_requests_total{method="POST",path="/api/v1/documents",status="200"} 0
edgequake_http_requests_total{method="POST",path="/api/v1/query",status="200"} 0

# HELP edgequake_http_request_duration_seconds HTTP request duration in seconds
# TYPE edgequake_http_request_duration_seconds histogram
edgequake_http_request_duration_seconds_bucket{le="0.01"} 0
edgequake_http_request_duration_seconds_bucket{le="0.05"} 0
edgequake_http_request_duration_seconds_bucket{le="0.1"} 0
edgequake_http_request_duration_seconds_bucket{le="0.5"} 0
edgequake_http_request_duration_seconds_bucket{le="1.0"} 0
edgequake_http_request_duration_seconds_bucket{le="5.0"} 0
edgequake_http_request_duration_seconds_bucket{le="+Inf"} 0
edgequake_http_request_duration_seconds_sum 0
edgequake_http_request_duration_seconds_count 0

# HELP edgequake_http_requests_in_flight Current number of HTTP requests being processed
# TYPE edgequake_http_requests_in_flight gauge
edgequake_http_requests_in_flight 0

# HELP edgequake_documents_total Total documents by status
# TYPE edgequake_documents_total counter
edgequake_documents_total{status="uploaded"} 0
edgequake_documents_total{status="indexed"} 0
edgequake_documents_total{status="failed"} 0

# HELP edgequake_documents_processing_duration_seconds Document processing duration
# TYPE edgequake_documents_processing_duration_seconds histogram
edgequake_documents_processing_duration_seconds_bucket{le="1.0"} 0
edgequake_documents_processing_duration_seconds_bucket{le="5.0"} 0
edgequake_documents_processing_duration_seconds_bucket{le="10.0"} 0
edgequake_documents_processing_duration_seconds_bucket{le="30.0"} 0
edgequake_documents_processing_duration_seconds_bucket{le="60.0"} 0
edgequake_documents_processing_duration_seconds_bucket{le="+Inf"} 0
edgequake_documents_processing_duration_seconds_sum 0
edgequake_documents_processing_duration_seconds_count 0

# HELP edgequake_queries_total Total queries executed
# TYPE edgequake_queries_total counter
edgequake_queries_total{mode="local"} 0
edgequake_queries_total{mode="global"} 0
edgequake_queries_total{mode="hybrid"} 0
edgequake_queries_total{mode="naive"} 0
edgequake_queries_total{mode="mix"} 0

# HELP edgequake_query_duration_seconds Query execution duration
# TYPE edgequake_query_duration_seconds histogram
edgequake_query_duration_seconds_bucket{le="0.1"} 0
edgequake_query_duration_seconds_bucket{le="0.5"} 0
edgequake_query_duration_seconds_bucket{le="1.0"} 0
edgequake_query_duration_seconds_bucket{le="5.0"} 0
edgequake_query_duration_seconds_bucket{le="10.0"} 0
edgequake_query_duration_seconds_bucket{le="+Inf"} 0
edgequake_query_duration_seconds_sum 0
edgequake_query_duration_seconds_count 0

# HELP edgequake_tasks_total Total background tasks by status
# TYPE edgequake_tasks_total counter
edgequake_tasks_total{status="pending"} 0
edgequake_tasks_total{status="running"} 0
edgequake_tasks_total{status="completed"} 0
edgequake_tasks_total{status="failed"} 0
edgequake_tasks_total{status="cancelled"} 0

# HELP edgequake_task_queue_size Current number of tasks in queue
# TYPE edgequake_task_queue_size gauge
edgequake_task_queue_size 0

# HELP edgequake_llm_requests_total Total LLM requests
# TYPE edgequake_llm_requests_total counter
edgequake_llm_requests_total{provider="openai"} 0
edgequake_llm_requests_total{provider="mock"} 0

# HELP edgequake_llm_tokens_total Total tokens used
# TYPE edgequake_llm_tokens_total counter
edgequake_llm_tokens_total{type="prompt"} 0
edgequake_llm_tokens_total{type="completion"} 0

# HELP edgequake_storage_operations_total Total storage operations
# TYPE edgequake_storage_operations_total counter
edgequake_storage_operations_total{type="kv",operation="read"} 0
edgequake_storage_operations_total{type="kv",operation="write"} 0
edgequake_storage_operations_total{type="vector",operation="search"} 0
edgequake_storage_operations_total{type="vector",operation="upsert"} 0
edgequake_storage_operations_total{type="graph",operation="query"} 0
edgequake_storage_operations_total{type="graph",operation="write"} 0

# HELP edgequake_graph_entities_total Total entities in knowledge graph
# TYPE edgequake_graph_entities_total gauge
edgequake_graph_entities_total 0

# HELP edgequake_graph_relationships_total Total relationships in knowledge graph
# TYPE edgequake_graph_relationships_total gauge
edgequake_graph_relationships_total 0

"#;

    PrometheusMetrics(metrics.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    #[tokio::test]
    async fn test_get_metrics() {
        let response = get_metrics().await;
        let response = response.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_metrics_format() {
        let response = get_metrics().await;
        let metrics = response.0;

        // Check for required metrics
        assert!(metrics.contains("edgequake_info"));
        assert!(metrics.contains("edgequake_http_requests_total"));
        assert!(metrics.contains("edgequake_queries_total"));
        assert!(metrics.contains("edgequake_documents_total"));
    }
}
