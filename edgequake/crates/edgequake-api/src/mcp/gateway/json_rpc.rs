//! JSON-RPC envelopes and error mapping (MCP / JSON-RPC 2.0).

use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::ApiError;

pub const PROTOCOL_2026_07_28: &str = "2026-07-28";
pub const PROTOCOL_2025_11_25: &str = "2025-11-25";

pub const DEPRECATED_METHODS: &[&str] = &[
    "sampling/createMessage",
    "roots/list",
    "logging/setLevel",
    "notifications/cancelled",
];

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcErrorBody>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcErrorBody {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug)]
pub enum GatewayError {
    Transport {
        status: StatusCode,
        code: i32,
        message: String,
    },
    Api(ApiError),
}

impl GatewayError {
    pub fn transport(status: StatusCode, code: i32, message: impl Into<String>) -> Self {
        Self::Transport {
            status,
            code,
            message: message.into(),
        }
    }

    pub fn status(&self) -> StatusCode {
        match self {
            Self::Transport { status, .. } => *status,
            Self::Api(e) => e.status_code(),
        }
    }

    /// HTTP status for JSON-RPC responses (application errors use 200 + error body).
    pub fn json_rpc_http_status(&self) -> StatusCode {
        match self {
            Self::Transport { status, .. } => *status,
            Self::Api(e) => match e.status_code() {
                StatusCode::UNAUTHORIZED
                | StatusCode::FORBIDDEN
                | StatusCode::TOO_MANY_REQUESTS => e.status_code(),
                _ => StatusCode::OK,
            },
        }
    }

    pub fn json_rpc_error(&self) -> JsonRpcErrorBody {
        match self {
            Self::Transport { code, message, .. } => JsonRpcErrorBody {
                code: *code,
                message: message.clone(),
                data: None,
            },
            Self::Api(e) => api_error_to_json_rpc(e),
        }
    }
}

pub fn api_error_to_json_rpc(err: &ApiError) -> JsonRpcErrorBody {
    let code = match err.status_code().as_u16() {
        400 => -32602,
        401 => -32001,
        403 => -32003,
        404 | 410 => -32004,
        _ => -32603,
    };
    JsonRpcErrorBody {
        code,
        message: err.to_string(),
        data: None,
    }
}

pub fn success_response(id: Value, result: Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: Some(result),
        error: None,
    }
}

pub fn error_response(id: Value, err: &GatewayError) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: None,
        error: Some(err.json_rpc_error()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ApiError;

    #[test]
    fn api_not_found_maps_to_32004() {
        let err = GatewayError::Api(ApiError::NotFound("missing".into()));
        assert_eq!(err.json_rpc_http_status(), StatusCode::OK);
        assert_eq!(err.json_rpc_error().code, -32004);
    }
}
