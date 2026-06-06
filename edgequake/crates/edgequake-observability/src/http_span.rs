//! Per-request HTTP tracing spans (works with or without OTLP).

use std::future::Future;

use http::HeaderMap;
use tracing::Instrument;

use crate::request_context::{parse_trace_id_from_traceparent, trace_id_from_request_id};

/// Run a future inside a proper HTTP `tracing` span with semantic fields.
///
/// When built with `otel` and inbound W3C headers are present, links this span to the parent trace.
pub async fn with_http_span<Fut, T>(
    request_id: &str,
    method: &str,
    path: &str,
    traceparent: Option<&str>,
    #[allow(unused_variables)] headers: Option<&HeaderMap>,
    fut: Fut,
) -> T
where
    Fut: Future<Output = T>,
{
    let trace_id = traceparent
        .and_then(parse_trace_id_from_traceparent)
        .or_else(|| trace_id_from_request_id(request_id))
        .unwrap_or_else(|| "-".to_string());

    let span = tracing::info_span!(
        "http_request",
        http.method = %method,
        http.target = %path,
        http.status_code = tracing::field::Empty,
        request_id = %request_id,
        trace_id = %trace_id,
        error.code = tracing::field::Empty,
        error.message = tracing::field::Empty,
        error.source = tracing::field::Empty,
        error.retryable = tracing::field::Empty,
        error.phase = tracing::field::Empty,
        otel.name = "HTTP",
        otel.kind = "server",
    );

    #[cfg(feature = "otel")]
    if let Some(hdrs) = headers {
        if hdrs
            .get(crate::request_context::TRACEPARENT_HEADER)
            .is_some()
        {
            use tracing_opentelemetry::OpenTelemetrySpanExt;
            let parent = crate::trace_context::extract_from_headers(hdrs);
            let _ = span.set_parent(parent);
        }
    }

    fut.instrument(span).await
}

/// Record final HTTP status on the current span (call before span closes).
pub fn record_http_status(status: u16) {
    tracing::Span::current().record("http.status_code", status);
}

/// Record HTTP failure with explicit error code (call from API error handler).
pub fn record_http_error(
    status: u16,
    error_code: &str,
    message: &str,
    source: Option<&str>,
    retryable: Option<bool>,
) {
    let span = tracing::Span::current();
    span.record("http.status_code", status);
    span.record("error.code", error_code);
    span.record("error.message", message);
    if let Some(s) = source {
        span.record("error.source", s);
    }
    if let Some(r) = retryable {
        span.record("error.retryable", r);
    }
}
