//! Structured error logging and span fields (explicit context on failures).

use serde_json::{json, Value};
use tracing::Span;

/// Structured error event for logs + traces (build from API/domain errors).
#[derive(Debug, Clone)]
pub struct ErrorEvent {
    pub request_id: String,
    pub error_code: String,
    pub http_status: u16,
    pub message: String,
    pub source: Option<String>,
    pub retryable: bool,
    pub details: Value,
}

impl ErrorEvent {
    /// Emit at the correct log level with full structured fields.
    pub fn log(&self) {
        let rid = self.request_id.as_str();
        let code = self.error_code.as_str();
        let src = self.source.as_deref().unwrap_or("api");
        let retry = self.retryable;

        if self.http_status >= 500 {
            tracing::error!(
                request_id = %rid,
                error.code = %code,
                http.status = self.http_status,
                error.source = %src,
                error.retryable = retry,
                error.message = %self.message,
                error.details = %self.details,
                "Request failed (server error)"
            );
        } else if self.http_status >= 400 {
            tracing::warn!(
                request_id = %rid,
                error.code = %code,
                http.status = self.http_status,
                error.source = %src,
                error.retryable = retry,
                error.message = %self.message,
                error.details = %self.details,
                "Request failed (client error)"
            );
        } else {
            tracing::info!(
                request_id = %rid,
                error.code = %code,
                http.status = self.http_status,
                error.message = %self.message,
                "Request completed with error status"
            );
        }
    }

    /// Attach error semantics to the active span (HTTP request span).
    pub fn record_on_current_span(&self) {
        let span = Span::current();
        span.record("http.status_code", self.http_status);
        span.record("error.code", self.error_code.as_str());
        span.record("error.message", self.message.as_str());
        if let Some(ref src) = self.source {
            span.record("error.source", src.as_str());
        }
        span.record("error.retryable", self.retryable);

        #[cfg(feature = "otel")]
        {
            use opentelemetry::trace::Status;
            use tracing_opentelemetry::OpenTelemetrySpanExt;
            if self.http_status >= 500 {
                span.set_status(Status::error(format!(
                    "{}: {}",
                    self.error_code, self.message
                )));
            } else if self.http_status >= 400 {
                // Client errors: span records error fields but OTEL status stays OK (expected failures).
                span.set_status(Status::Ok);
            }
        }
    }

    /// Structured log for rate limit rejections (429, not `ApiError`).
    pub fn log_rate_limit_exceeded(
        request_id: &str,
        tenant_id: &str,
        retry_after_seconds: Option<u64>,
    ) {
        let span = Span::current();
        span.record("http.status_code", 429u16);
        span.record("error.code", "RATE_LIMITED");
        span.record("error.source", "rate_limiter");
        span.record("error.retryable", true);
        let rate_limit_message = format!("Too many requests for tenant '{tenant_id}'");
        span.record("error.message", rate_limit_message.as_str());

        tracing::warn!(
            request_id = %request_id,
            tenant_id = %tenant_id,
            error.code = "RATE_LIMITED",
            error.source = "rate_limiter",
            error.retryable = true,
            retry_after_seconds = ?retry_after_seconds,
            http.status = 429u16,
            "Rate limit exceeded"
        );
    }

    /// Structured log for in-stream SSE failures (after HTTP 200).
    ///
    /// `stream_source`: e.g. `query_stream`, `chat_stream`.
    pub fn log_stream_error(
        request_id: &str,
        stream_source: &str,
        error_code: &str,
        message: &str,
        details: Value,
    ) {
        let span = Span::current();
        span.record("error.code", error_code);
        span.record("error.message", message);
        span.record("error.source", stream_source);
        if let Some(phase) = details.get("phase").and_then(|v| v.as_str()) {
            span.record("error.phase", phase);
        }

        tracing::error!(
            request_id = %request_id,
            error.code = %error_code,
            error.message = %message,
            error.source = %stream_source,
            error.details = %details,
            "Streaming request failed"
        );

        #[cfg(feature = "otel")]
        {
            use opentelemetry::trace::Status;
            use tracing_opentelemetry::OpenTelemetrySpanExt;
            span.set_status(Status::error(format!("{error_code}: {message}")));
        }
    }

    /// Client closed SSE connection mid-stream (info — not a server failure).
    pub fn log_stream_disconnect(request_id: &str, stream_source: &str, phase: &str) {
        tracing::info!(
            request_id = %request_id,
            error.source = %stream_source,
            error.action = "client_disconnect",
            stream.phase = %phase,
            "SSE client disconnected"
        );
    }

    /// Log a non-HTTP domain failure (pipeline, tasks) at `warn` with explicit fields.
    pub fn log_domain_warn(source: &str, action: &str, message: &str, details: Value) {
        let request_id = crate::request_context::current_request_id()
            .unwrap_or_else(|| "background".to_string());
        tracing::warn!(
            request_id = %request_id,
            error.source = %source,
            error.action = %action,
            error.message = %message,
            error.details = %details,
            "Domain operation failed"
        );
    }

    /// Log a non-HTTP domain failure at `error` (storage, permanent task failure).
    pub fn log_domain_error(source: &str, action: &str, message: &str, details: Value) {
        let request_id = crate::request_context::current_request_id()
            .unwrap_or_else(|| "background".to_string());
        tracing::error!(
            request_id = %request_id,
            error.source = %source,
            error.action = %action,
            error.message = %message,
            error.details = %details,
            "Domain operation failed (error)"
        );
    }

    /// JSON blob for API `details` field (includes request_id + diagnostics).
    pub fn into_api_details(&self) -> Value {
        json!({
            "request_id": self.request_id,
            "error_code": self.error_code,
            "http_status": self.http_status,
            "retryable": self.retryable,
            "source": self.source,
            "diagnostics": self.details,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_details_include_diagnostics() {
        let event = ErrorEvent {
            request_id: "rid".into(),
            error_code: "INTERNAL_ERROR".into(),
            http_status: 500,
            message: "boom".into(),
            source: Some("api".into()),
            retryable: false,
            details: json!({ "kind": "internal" }),
        };
        let d = event.into_api_details();
        assert_eq!(d["request_id"], "rid");
        assert_eq!(d["diagnostics"]["kind"], "internal");
    }
}
