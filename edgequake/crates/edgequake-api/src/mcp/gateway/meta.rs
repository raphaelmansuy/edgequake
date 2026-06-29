//! MCP `_meta` envelope parsing (2026-07-28).

use serde_json::Value;

use edgequake_observability::{PropagationHeaders, TRACEPARENT_HEADER, TRACESTATE_HEADER};

use super::json_rpc::{GatewayError, PROTOCOL_2025_11_25, PROTOCOL_2026_07_28};

#[derive(Debug, Clone, Default)]
pub struct RequestMeta {
    pub protocol_version: Option<String>,
    pub client_name: Option<String>,
    pub client_version: Option<String>,
    pub traceparent: Option<String>,
    pub tracestate: Option<String>,
}

pub fn extract_meta(params: Option<&Value>) -> RequestMeta {
    let mut meta = RequestMeta::default();
    let Some(params) = params else {
        return meta;
    };

    let meta_obj = params
        .get("_meta")
        .or_else(|| params.get("arguments").and_then(|a| a.get("_meta")));

    let Some(obj) = meta_obj.and_then(|v| v.as_object()) else {
        return meta;
    };

    meta.protocol_version = obj
        .get("io.modelcontextprotocol/protocolVersion")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    if let Some(info) = obj
        .get("io.modelcontextprotocol/clientInfo")
        .and_then(|v| v.as_object())
    {
        meta.client_name = info
            .get("name")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        meta.client_version = info
            .get("version")
            .and_then(|v| v.as_str())
            .map(str::to_string);
    }
    meta.traceparent = obj
        .get("traceparent")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    meta.tracestate = obj
        .get("tracestate")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    meta
}

pub fn normalize_protocol_version(
    header_version: Option<&str>,
    meta: &RequestMeta,
) -> Result<String, GatewayError> {
    let version = header_version
        .or(meta.protocol_version.as_deref())
        .unwrap_or(PROTOCOL_2025_11_25);

    if version == PROTOCOL_2026_07_28 || version == PROTOCOL_2025_11_25 {
        if let (Some(h), Some(m)) = (header_version, meta.protocol_version.as_deref()) {
            if h != m {
                return Err(GatewayError::transport(
                    axum::http::StatusCode::BAD_REQUEST,
                    -32602,
                    "HeaderMismatch: MCP-Protocol-Version header does not match _meta protocolVersion",
                ));
            }
        }
        return Ok(version.to_string());
    }

    Err(GatewayError::transport(
        axum::http::StatusCode::BAD_REQUEST,
        -32602,
        format!(
            "UnsupportedProtocolVersionError: supported [{}]",
            [PROTOCOL_2026_07_28, PROTOCOL_2025_11_25].join(", ")
        ),
    ))
}

/// Build propagation headers from MCP `_meta` trace context (W3C traceparent).
pub fn propagation_from_meta(meta: &RequestMeta) -> PropagationHeaders {
    let mut map = std::collections::HashMap::new();
    if let Some(tp) = &meta.traceparent {
        map.insert(TRACEPARENT_HEADER.to_string(), tp.clone());
    }
    if let Some(ts) = &meta.tracestate {
        map.insert(TRACESTATE_HEADER.to_string(), ts.clone());
    }
    PropagationHeaders(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn propagation_from_meta_includes_traceparent() {
        let meta = RequestMeta {
            traceparent: Some("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01".into()),
            ..Default::default()
        };
        let p = propagation_from_meta(&meta);
        assert!(p.0.contains_key(TRACEPARENT_HEADER));
    }
}
