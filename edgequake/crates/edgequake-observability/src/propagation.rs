//! Harvest W3C / B2B headers for downstream LLM propagation (DRY).

use std::collections::HashMap;

use crate::request_context::{
    CORRELATION_ID_HEADER, REQUEST_ID_HEADER, TRACEPARENT_HEADER, TRACESTATE_HEADER,
};

/// Headers safe to forward to upstream LLM HTTP APIs.
#[derive(Debug, Clone, Default)]
pub struct PropagationHeaders(pub HashMap<String, String>);

impl PropagationHeaders {
    pub fn into_map(self) -> HashMap<String, String> {
        self.0
    }

    pub fn merge_with(
        mut self,
        other: Option<HashMap<String, String>>,
    ) -> Option<HashMap<String, String>> {
        if let Some(extra) = other {
            for (k, v) in extra {
                self.0.entry(k).or_insert(v);
            }
        }
        if self.0.is_empty() {
            None
        } else {
            Some(self.0)
        }
    }
}

/// Extract propagation headers from an incoming HTTP request.
pub fn harvest_propagation_headers(headers: &http::HeaderMap) -> PropagationHeaders {
    let mut map = HashMap::new();
    for (name, key) in [
        (REQUEST_ID_HEADER, REQUEST_ID_HEADER),
        (CORRELATION_ID_HEADER, CORRELATION_ID_HEADER),
        (TRACEPARENT_HEADER, TRACEPARENT_HEADER),
        (TRACESTATE_HEADER, TRACESTATE_HEADER),
        ("x-tenant-id", "x-tenant-id"),
        ("x-workspace-id", "x-workspace-id"),
    ] {
        if let Some(v) = headers.get(name).and_then(|h| h.to_str().ok()) {
            let v = v.trim();
            if !v.is_empty() && !v.chars().any(|c| c.is_control()) {
                map.insert(key.to_string(), v.to_string());
            }
        }
    }
    PropagationHeaders(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harvests_traceparent() {
        let mut h = http::HeaderMap::new();
        h.insert(
            TRACEPARENT_HEADER,
            http::HeaderValue::from_static(
                "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
            ),
        );
        let p = harvest_propagation_headers(&h);
        assert!(p.0.contains_key(TRACEPARENT_HEADER));
    }
}
