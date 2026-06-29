//! Streamable HTTP transport validation (MCP 2026-07-28).

use axum::http::{HeaderMap, StatusCode};
use serde_json::Value;

use super::json_rpc::{GatewayError, PROTOCOL_2026_07_28};

const ACCEPT_JSON: &str = "application/json";
const ACCEPT_SSE: &str = "text/event-stream";

/// Validate `Accept` header (EC-MCP-01).
pub fn validate_accept(headers: &HeaderMap) -> Result<(), GatewayError> {
    let accept = headers
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let has_json = accept.contains(ACCEPT_JSON);
    let has_sse = accept.contains(ACCEPT_SSE);

    if !has_json || !has_sse {
        return Err(GatewayError::transport(
            StatusCode::NOT_ACCEPTABLE,
            -32603,
            "Not Acceptable: Client must accept both application/json and text/event-stream",
        ));
    }
    Ok(())
}

/// Validate Origin when present (DNS rebinding protection).
pub fn validate_origin(
    headers: &HeaderMap,
    allowed_origins: Option<&[String]>,
) -> Result<(), GatewayError> {
    let Some(origin) = headers.get("origin").and_then(|v| v.to_str().ok()) else {
        return Ok(());
    };

    if origin.is_empty() {
        return Ok(());
    }

    if let Some(allowed) = allowed_origins {
        if allowed.iter().any(|o| o == origin) {
            return Ok(());
        }
        return Err(GatewayError::transport(
            StatusCode::FORBIDDEN,
            -32603,
            "Forbidden: Origin not allowed",
        ));
    }

    Ok(())
}

/// Reject legacy session header usage (stateless invariant).
pub fn reject_session_header(headers: &HeaderMap) -> Result<(), GatewayError> {
    if headers.contains_key("mcp-session-id") {
        return Err(GatewayError::transport(
            StatusCode::BAD_REQUEST,
            -32602,
            "Mcp-Session-Id is not supported; use retrieval_id handles (2026-07-28 stateless)",
        ));
    }
    Ok(())
}

/// Validate Mcp-Method and Mcp-Name for modern protocol versions.
pub fn validate_routing_headers(
    headers: &HeaderMap,
    protocol_version: &str,
    method: &str,
    params: Option<&Value>,
) -> Result<(), GatewayError> {
    if protocol_version != PROTOCOL_2026_07_28 {
        return Ok(());
    }

    let header_method = headers
        .get("mcp-method")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            GatewayError::transport(
                StatusCode::BAD_REQUEST,
                -32602,
                "Missing required Mcp-Method header",
            )
        })?;

    if !header_method.eq_ignore_ascii_case(method) {
        return Err(GatewayError::transport(
            StatusCode::BAD_REQUEST,
            -32602,
            "HeaderMismatch: Mcp-Method does not match JSON-RPC method",
        ));
    }

    if matches!(method, "tools/call" | "resources/read" | "prompts/get") {
        let expected_name = params
            .and_then(|p| {
                p.get("name")
                    .or_else(|| p.get("uri"))
                    .and_then(|v| v.as_str())
            })
            .ok_or_else(|| {
                GatewayError::transport(
                    StatusCode::BAD_REQUEST,
                    -32602,
                    "Missing tool/resource name in params",
                )
            })?;

        let header_name = headers
            .get("mcp-name")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                GatewayError::transport(
                    StatusCode::BAD_REQUEST,
                    -32602,
                    "Missing required Mcp-Name header",
                )
            })?;

        if header_name != expected_name {
            return Err(GatewayError::transport(
                StatusCode::BAD_REQUEST,
                -32602,
                "HeaderMismatch: Mcp-Name does not match params.name",
            ));
        }
    }

    Ok(())
}

pub fn workspace_from_param_header(headers: &HeaderMap) -> Option<String> {
    headers
        .get("mcp-param-workspace-id")
        .or_else(|| headers.get("x-workspace-id"))
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accept_requires_both_types() {
        let mut h = HeaderMap::new();
        h.insert("accept", "application/json".parse().unwrap());
        assert!(validate_accept(&h).is_err());
        h.insert(
            "accept",
            "application/json, text/event-stream".parse().unwrap(),
        );
        assert!(validate_accept(&h).is_ok());
    }
}
