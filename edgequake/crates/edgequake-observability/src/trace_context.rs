//! W3C Trace Context extraction for distributed tracing (OTEL feature).

use http::HeaderMap;
use opentelemetry::propagation::{Extractor, Injector, TextMapPropagator};
use opentelemetry::Context;
use opentelemetry_sdk::propagation::TraceContextPropagator;

/// HTTP header carrier for W3C `traceparent` / `tracestate` extraction.
pub struct HttpHeaderExtractor<'a>(pub &'a HeaderMap);

impl Extractor for HttpHeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        // Propagator only needs `get` for W3C traceparent/tracestate.
        Vec::new()
    }
}

/// Extract upstream OTEL context from inbound HTTP headers (W3C Trace Context).
pub fn extract_from_headers(headers: &HeaderMap) -> Context {
    let propagator = TraceContextPropagator::new();
    propagator.extract(&HttpHeaderExtractor(headers))
}

/// HTTP header carrier for injecting W3C trace context into outbound responses.
pub struct HttpHeaderInjector<'a>(pub &'a mut HeaderMap);

impl Injector for HttpHeaderInjector<'_> {
    fn set(&mut self, key: &str, value: String) {
        if let (Ok(name), Ok(v)) = (
            http::HeaderName::from_bytes(key.as_bytes()),
            http::HeaderValue::from_str(&value),
        ) {
            self.0.insert(name, v);
        }
    }
}

/// Inject the current OTEL context as `traceparent` / `tracestate` response headers.
pub fn inject_current_context(headers: &mut HeaderMap) {
    opentelemetry::global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&Context::current(), &mut HttpHeaderInjector(headers));
    });
}

#[cfg(all(test, feature = "otel"))]
mod tests {
    use super::*;
    use http::HeaderMap;
    use opentelemetry::trace::TraceContextExt;

    #[test]
    fn extracts_trace_id_from_traceparent() {
        let mut h = HeaderMap::new();
        h.insert(
            "traceparent",
            http::HeaderValue::from_static(
                "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
            ),
        );
        let cx = extract_from_headers(&h);
        let span = cx.span();
        let sc = span.span_context();
        assert!(sc.is_valid());
        assert_eq!(
            sc.trace_id().to_string(),
            "4bf92f3577b34da6a3ce929d0e0e4736"
        );
    }
}
