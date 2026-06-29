//! MCP request body parsing (batch rejection, notifications, size guard).

use axum::http::StatusCode;
use serde_json::Value;

use super::json_rpc::{GatewayError, JsonRpcRequest};

/// Maximum MCP JSON-RPC body size (EC-MCP-10).
pub const MCP_MAX_BODY_BYTES: usize = 1024 * 1024;

/// Parsed MCP POST body — either a call with id or a notification without.
#[derive(Debug, Clone)]
pub enum ParsedMcpBody {
    Request(JsonRpcRequest),
    Notification {
        method: String,
        params: Option<Value>,
    },
}

/// Parse raw POST bytes into a single JSON-RPC object or notification.
pub fn parse_mcp_body(bytes: &[u8]) -> Result<ParsedMcpBody, GatewayError> {
    if bytes.len() > MCP_MAX_BODY_BYTES {
        return Err(GatewayError::transport(
            StatusCode::PAYLOAD_TOO_LARGE,
            -32600,
            format!(
                "Payload Too Large: MCP request body exceeds {} bytes",
                MCP_MAX_BODY_BYTES
            ),
        ));
    }

    let value: Value = serde_json::from_slice(bytes).map_err(|_| {
        GatewayError::transport(StatusCode::BAD_REQUEST, -32700, "Parse error: invalid JSON")
    })?;

    if value.is_array() {
        return Err(GatewayError::transport(
            StatusCode::BAD_REQUEST,
            -32600,
            "Invalid Request: JSON-RPC batch arrays are not supported",
        ));
    }

    if !value.is_object() {
        return Err(GatewayError::transport(
            StatusCode::BAD_REQUEST,
            -32600,
            "Invalid Request: expected JSON object",
        ));
    }

    let method = value
        .get("method")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            GatewayError::transport(
                StatusCode::BAD_REQUEST,
                -32600,
                "Invalid Request: missing method",
            )
        })?
        .to_string();

    if value.get("id").is_none() {
        return Ok(ParsedMcpBody::Notification {
            method,
            params: value.get("params").cloned(),
        });
    }

    let request: JsonRpcRequest = serde_json::from_value(value).map_err(|_| {
        GatewayError::transport(
            StatusCode::BAD_REQUEST,
            -32600,
            "Invalid Request: malformed JSON-RPC object",
        )
    })?;

    Ok(ParsedMcpBody::Request(request))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_batch_array() {
        let body = json!([{"jsonrpc":"2.0","id":1,"method":"ping"}]).to_string();
        let err = parse_mcp_body(body.as_bytes()).unwrap_err();
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn parses_notification_without_id() {
        let body = json!({"jsonrpc":"2.0","method":"ping"}).to_string();
        match parse_mcp_body(body.as_bytes()).unwrap() {
            ParsedMcpBody::Notification { method, .. } => assert_eq!(method, "ping"),
            _ => panic!("expected notification"),
        }
    }
}
