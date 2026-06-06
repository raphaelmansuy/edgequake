//! Prometheus metrics registry (DRY — single recorder for `/metrics`).

use std::sync::OnceLock;

use metrics::{
    counter, describe_counter, describe_gauge, describe_histogram, gauge, histogram, Unit,
};

use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

const HTTP_REQUESTS: &str = "edgequake_http_requests_total";
const HTTP_DURATION: &str = "edgequake_http_request_duration_seconds";
const QUERY_REQUESTS: &str = "edgequake_query_requests_total";
const QUERY_DURATION: &str = "edgequake_query_duration_seconds";
const RATE_LIMIT_EXCEEDED: &str = "edgequake_rate_limit_exceeded_total";
const LLM_REQUESTS: &str = "edgequake_llm_requests_total";
const LLM_DURATION: &str = "edgequake_llm_request_duration_seconds";
const DOCUMENT_PROCESSING: &str = "edgequake_document_processing_total";
const DOCUMENT_DURATION: &str = "edgequake_document_processing_duration_seconds";
const STORAGE_ERRORS: &str = "edgequake_storage_errors_total";
const PIPELINE_ERRORS: &str = "edgequake_pipeline_errors_total";
const DB_POOL_CONNECTIONS: &str = "edgequake_db_pool_connections";

/// Pre-register metric metadata so `/metrics` is never an empty body before first request.
fn describe_http_metrics() {
    describe_counter!(
        HTTP_REQUESTS,
        "Total HTTP requests handled by EdgeQuake API"
    );
    describe_histogram!(
        HTTP_DURATION,
        Unit::Seconds,
        "HTTP request duration in seconds"
    );
    describe_counter!(QUERY_REQUESTS, "Total RAG query executions");
    describe_histogram!(
        QUERY_DURATION,
        Unit::Seconds,
        "RAG query end-to-end duration in seconds"
    );
    describe_counter!(
        RATE_LIMIT_EXCEEDED,
        "HTTP requests rejected due to rate limiting"
    );
    describe_counter!(
        LLM_REQUESTS,
        "LLM provider calls (query generation and errors)"
    );
    describe_histogram!(LLM_DURATION, Unit::Seconds, "LLM call duration in seconds");
    describe_counter!(
        DOCUMENT_PROCESSING,
        "Document and PDF processing outcomes by task type and stage"
    );
    describe_histogram!(
        DOCUMENT_DURATION,
        Unit::Seconds,
        "Document processing duration in seconds"
    );
    describe_counter!(
        STORAGE_ERRORS,
        "Storage layer errors surfaced to the API by category"
    );
    describe_counter!(
        PIPELINE_ERRORS,
        "Pipeline errors surfaced to the API by category"
    );
    describe_gauge!(
        DB_POOL_CONNECTIONS,
        "PostgreSQL pool connections (sampled on /metrics scrape)"
    );
}

static PROMETHEUS: OnceLock<PrometheusHandle> = OnceLock::new();

/// Install the global Prometheus recorder. Idempotent.
pub fn init_metrics() {
    let _ = PROMETHEUS.get_or_init(|| {
        let handle = PrometheusBuilder::new()
            .install_recorder()
            .expect("failed to install Prometheus metrics recorder");
        describe_http_metrics();
        // Exporter registers series on first sample; seed so cold `/metrics` is valid Prometheus text.
        counter!(
            HTTP_REQUESTS,
            "method" => "GET",
            "path" => "_bootstrap",
            "status" => "0"
        )
        .increment(0);
        histogram!(HTTP_DURATION, "method" => "GET", "path" => "_bootstrap").record(0.0);
        counter!(
            QUERY_REQUESTS,
            "mode" => "bootstrap",
            "outcome" => "success"
        )
        .increment(0);
        histogram!(QUERY_DURATION, "mode" => "bootstrap").record(0.0);
        counter!(RATE_LIMIT_EXCEEDED, "scope" => "bootstrap").increment(0);
        counter!(
            LLM_REQUESTS,
            "provider" => "bootstrap",
            "operation" => "query",
            "outcome" => "success"
        )
        .increment(0);
        histogram!(LLM_DURATION, "provider" => "bootstrap", "operation" => "query").record(0.0);
        counter!(
            DOCUMENT_PROCESSING,
            "task_type" => "bootstrap",
            "stage" => "pipeline",
            "outcome" => "success"
        )
        .increment(0);
        histogram!(
            DOCUMENT_DURATION,
            "task_type" => "bootstrap",
            "stage" => "pipeline"
        )
        .record(0.0);
        counter!(
            STORAGE_ERRORS,
            "category" => "bootstrap",
            "error_code" => "BOOTSTRAP"
        )
        .increment(0);
        counter!(
            PIPELINE_ERRORS,
            "category" => "bootstrap",
            "error_code" => "BOOTSTRAP"
        )
        .increment(0);
        gauge!(DB_POOL_CONNECTIONS, "state" => "total").set(0.0);
        gauge!(DB_POOL_CONNECTIONS, "state" => "idle").set(0.0);
        gauge!(DB_POOL_CONNECTIONS, "state" => "active").set(0.0);
        handle
    });
}

/// Record a storage error surfaced as an API failure.
pub fn record_storage_error(category: &str, error_code: &str) {
    init_metrics();
    counter!(
        STORAGE_ERRORS,
        "category" => category.to_string(),
        "error_code" => error_code.to_string()
    )
    .increment(1);
}

/// Record a pipeline error surfaced as an API failure.
pub fn record_pipeline_error(category: &str, error_code: &str) {
    init_metrics();
    counter!(
        PIPELINE_ERRORS,
        "category" => category.to_string(),
        "error_code" => error_code.to_string()
    )
    .increment(1);
}

/// Update DB pool gauges (call before Prometheus scrape when pool is available).
pub fn record_db_pool_stats(size: u32, idle: u32) {
    init_metrics();
    let active = size.saturating_sub(idle);
    gauge!(DB_POOL_CONNECTIONS, "state" => "total").set(size as f64);
    gauge!(DB_POOL_CONNECTIONS, "state" => "idle").set(idle as f64);
    gauge!(DB_POOL_CONNECTIONS, "state" => "active").set(active as f64);
}

/// Record document/PDF pipeline processing (task processor layer).
pub fn record_document_processing(task_type: &str, stage: &str, outcome: &str, duration_secs: f64) {
    init_metrics();
    counter!(
        DOCUMENT_PROCESSING,
        "task_type" => task_type.to_string(),
        "stage" => stage.to_string(),
        "outcome" => outcome.to_string()
    )
    .increment(1);
    histogram!(
        DOCUMENT_DURATION,
        "task_type" => task_type.to_string(),
        "stage" => stage.to_string()
    )
    .record(duration_secs);
}

/// Record an LLM provider call.
pub fn record_llm_request(provider: &str, operation: &str, outcome: &str, duration_secs: f64) {
    init_metrics();
    let provider = if provider.is_empty() {
        "unknown".to_string()
    } else {
        provider.to_string()
    };
    counter!(
        LLM_REQUESTS,
        "provider" => provider.clone(),
        "operation" => operation.to_string(),
        "outcome" => outcome.to_string()
    )
    .increment(1);
    histogram!(
        LLM_DURATION,
        "provider" => provider,
        "operation" => operation.to_string()
    )
    .record(duration_secs);
}

/// Record a rate-limited request (429).
pub fn record_rate_limit_exceeded(scope: &str) {
    init_metrics();
    counter!(RATE_LIMIT_EXCEEDED, "scope" => scope.to_string()).increment(1);
}

/// Record a completed RAG query (API handler).
pub fn record_query_completed(mode: &str, outcome: &str, duration_secs: f64) {
    init_metrics();
    counter!(
        QUERY_REQUESTS,
        "mode" => mode.to_string(),
        "outcome" => outcome.to_string()
    )
    .increment(1);
    histogram!(QUERY_DURATION, "mode" => mode.to_string()).record(duration_secs);
}

/// Render metrics in Prometheus text exposition format.
pub fn render_prometheus_metrics() -> String {
    init_metrics();
    PROMETHEUS
        .get()
        .map(|h| h.render())
        .unwrap_or_else(|| "# metrics not initialized\n".to_string())
}

/// Record one HTTP request (called from API middleware).
pub fn record_http_request(method: &str, path: &str, status: u16, duration_secs: f64) {
    init_metrics();
    let status_label = status.to_string();
    let route = normalize_route(path);

    counter!(
        HTTP_REQUESTS,
        "method" => method.to_string(),
        "path" => route.clone(),
        "status" => status_label
    )
    .increment(1);

    histogram!(
        HTTP_DURATION,
        "method" => method.to_string(),
        "path" => route
    )
    .record(duration_secs);
}

/// Normalize paths for metric cardinality (replace UUID-like segments).
pub fn normalize_route(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    let normalized: Vec<String> = parts
        .iter()
        .map(|p| {
            let looks_like_uuid = p.len() == 36 && p.chars().filter(|c| *c == '-').count() == 4;
            let looks_like_hex_id =
                p.len() > 20 && p.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
            if looks_like_uuid || looks_like_hex_id {
                ":id".to_string()
            } else {
                (*p).to_string()
            }
        })
        .collect();
    normalized.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_uuid_in_path() {
        let p = "/api/v1/workspaces/550e8400-e29b-41d4-a716-446655440000/documents";
        let n = normalize_route(p);
        assert!(n.contains(":id"));
    }

    #[test]
    fn scrape_includes_described_metrics_before_traffic() {
        let body = render_prometheus_metrics();
        assert!(
            body.contains(HTTP_REQUESTS),
            "metrics scrape should list HTTP counter before any request: {body:?}"
        );
        assert!(
            body.contains(DOCUMENT_PROCESSING),
            "metrics scrape should list document processing counter: {body:?}"
        );
        assert!(
            body.contains(STORAGE_ERRORS),
            "metrics scrape should list storage error counter: {body:?}"
        );
        assert!(
            body.contains(DB_POOL_CONNECTIONS),
            "metrics scrape should list db pool gauge: {body:?}"
        );
    }
}
