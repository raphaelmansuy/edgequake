//! Request correlation identifiers shared across middleware, logs, and audit.

use std::cell::RefCell;
use std::fmt;

tokio::task_local! {
    /// Current request ID for this async task (set by API middleware).
    static CURRENT_REQUEST_ID: RefCell<Option<String>>;
    /// Active LLM provider label for the current async task (set by query handlers).
    static CURRENT_LLM_PROVIDER: RefCell<Option<String>>;
}

/// Run a future with the given request ID visible to [`current_request_id`].
pub async fn scope_request_id<F, T>(request_id: String, fut: F) -> T
where
    F: std::future::Future<Output = T>,
{
    CURRENT_REQUEST_ID
        .scope(RefCell::new(Some(request_id)), fut)
        .await
}

/// Read the request ID for the current async task, if any.
pub fn current_request_id() -> Option<String> {
    CURRENT_REQUEST_ID
        .try_with(|id| id.borrow().clone())
        .ok()
        .flatten()
}

/// Run a future with the given LLM provider visible to [`current_llm_provider`].
pub async fn scope_llm_provider<F, T>(provider: String, fut: F) -> T
where
    F: std::future::Future<Output = T>,
{
    CURRENT_LLM_PROVIDER
        .scope(RefCell::new(Some(provider)), fut)
        .await
}

/// Read the LLM provider label for the current async task, if any.
pub fn current_llm_provider() -> Option<String> {
    CURRENT_LLM_PROVIDER
        .try_with(|p| p.borrow().clone())
        .ok()
        .flatten()
}

/// Standard request correlation header (lowercase in Axum).
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// Correlation ID alias some clients send.
pub const CORRELATION_ID_HEADER: &str = "x-correlation-id";

/// W3C trace context (forwarded to LLM providers when present).
pub const TRACEPARENT_HEADER: &str = "traceparent";

/// W3C trace state.
pub const TRACESTATE_HEADER: &str = "tracestate";

/// Request-scoped correlation context stored in Axum extensions.
#[derive(Clone, Debug)]
pub struct RequestContext {
    pub request_id: String,
    pub traceparent: Option<String>,
    pub tracestate: Option<String>,
}

impl RequestContext {
    pub fn new(request_id: String) -> Self {
        Self {
            request_id,
            traceparent: None,
            tracestate: None,
        }
    }

    pub fn with_trace(mut self, traceparent: Option<String>, tracestate: Option<String>) -> Self {
        self.traceparent = traceparent;
        self.tracestate = tracestate;
        self
    }
}

impl fmt::Display for RequestContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.request_id)
    }
}

/// Resolve a request ID from inbound headers or generate a new UUID v4.
pub fn resolve_request_id(headers: &http::HeaderMap) -> String {
    if let Some(id) = header_value(headers, REQUEST_ID_HEADER) {
        if is_valid_request_id(&id) {
            return id;
        }
    }
    if let Some(id) = header_value(headers, CORRELATION_ID_HEADER) {
        if is_valid_request_id(&id) {
            return id;
        }
    }
    uuid::Uuid::new_v4().to_string()
}

fn header_value(headers: &http::HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Derive a 32-hex trace id from a UUID-shaped request id (no dashes).
pub fn trace_id_from_request_id(request_id: &str) -> Option<String> {
    let hex: String = request_id
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect();
    if hex.len() == 32 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(hex)
    } else {
        None
    }
}

/// Build W3C `traceparent` for outbound responses when OTEL inject is unavailable.
///
/// Uses the request id as `trace-id` when it is UUID-shaped so browser clients can chain traces.
pub fn synthesize_traceparent_from_request_id(request_id: &str) -> Option<String> {
    let trace_id = trace_id_from_request_id(request_id)?;
    let span_id = uuid::Uuid::new_v4().simple().to_string();
    let span_id = &span_id[..16];
    Some(format!("00-{trace_id}-{span_id}-01"))
}

/// Parse W3C `trace-id` (32 hex chars) from a `traceparent` header value.
pub fn parse_trace_id_from_traceparent(traceparent: &str) -> Option<String> {
    let parts: Vec<&str> = traceparent.trim().split('-').collect();
    if parts.len() >= 4
        && parts[0] == "00"
        && parts[1].len() == 32
        && parts[1].chars().all(|c| c.is_ascii_hexdigit())
    {
        return Some(parts[1].to_string());
    }
    None
}

/// Accept UUIDs and opaque IDs up to 128 chars without control characters.
pub fn is_valid_request_id(id: &str) -> bool {
    if id.is_empty() || id.len() > 128 {
        return false;
    }
    !id.chars().any(|c| c.is_control())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn honors_inbound_request_id() {
        let mut headers = http::HeaderMap::new();
        headers.insert(
            REQUEST_ID_HEADER,
            http::HeaderValue::from_static("550e8400-e29b-41d4-a716-446655440000"),
        );
        let id = resolve_request_id(&headers);
        assert_eq!(id, "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn rejects_control_chars() {
        assert!(!is_valid_request_id("bad\nid"));
    }

    #[test]
    fn parses_trace_id_from_traceparent() {
        let tp = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        assert_eq!(
            parse_trace_id_from_traceparent(tp),
            Some("4bf92f3577b34da6a3ce929d0e0e4736".to_string())
        );
    }

    #[test]
    fn trace_id_from_uuid_request_id() {
        let id = "550e8400-e29b-41d4-a716-446655440000";
        assert_eq!(
            trace_id_from_request_id(id),
            Some("550e8400e29b41d4a716446655440000".to_string())
        );
    }

    #[tokio::test]
    async fn scope_llm_provider_is_visible_in_task() {
        let seen = scope_llm_provider("ollama".to_string(), async { current_llm_provider() }).await;
        assert_eq!(seen, Some("ollama".to_string()));
    }

    #[test]
    fn synthesize_traceparent_is_valid_w3c() {
        let id = "550e8400-e29b-41d4-a716-446655440000";
        let tp = synthesize_traceparent_from_request_id(id).expect("synthesized");
        let parts: Vec<&str> = tp.split('-').collect();
        assert_eq!(parts[0], "00");
        assert_eq!(parts[1].len(), 32);
        assert_eq!(parts[2].len(), 16);
        assert_eq!(parts[3], "01");
        assert_eq!(
            parse_trace_id_from_traceparent(&tp),
            trace_id_from_request_id(id)
        );
    }
}
