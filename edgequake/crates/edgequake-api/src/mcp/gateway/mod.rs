//! MCP Streamable HTTP gateway entry point.

pub mod body;
pub mod dispatch;
pub mod json_rpc;
pub mod meta;
pub mod sse;
pub mod tool_validation;
pub mod tools;
pub mod validate;
pub mod workspace_policy;

use axum::http::{HeaderMap, StatusCode};

use edgequake_auth::Role;

use crate::middleware::TenantContext;
use crate::state::AppState;

use self::dispatch::dispatch_method;
use self::json_rpc::{error_response, success_response};
use self::meta::{extract_meta, normalize_protocol_version};
use self::sse::{retrieve_sse_stream, wants_sse_response};
use self::validate::{
    reject_session_header, validate_accept, validate_origin, validate_routing_headers,
    workspace_from_param_header,
};

pub use dispatch::DispatchContext;
pub use json_rpc::{GatewayError, JsonRpcRequest, JsonRpcResponse};
pub use tools::tools_list_result;

/// Outcome of an MCP gateway request (JSON-RPC body + optional HTTP status override).
#[allow(clippy::large_enum_variant)]
pub enum McpHandleOutcome {
    Json {
        status: StatusCode,
        body: json_rpc::JsonRpcResponse,
        www_authenticate: Option<String>,
    },
    /// JSON-RPC notification — no response body (EC-MCP-08).
    Accepted,
    /// Streamable HTTP SSE response (MCP-E / EC-MCP-09).
    Sse { body: sse::SseBody },
}

impl McpHandleOutcome {
    pub fn ok(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self::Json {
            status: StatusCode::OK,
            body: success_response(id, result),
            www_authenticate: None,
        }
    }

    pub fn error(id: serde_json::Value, err: json_rpc::GatewayError) -> Self {
        let status = err.json_rpc_http_status();
        Self::Json {
            status,
            body: error_response(id, &err),
            www_authenticate: None,
        }
    }

    pub fn notification_accepted() -> Self {
        Self::Accepted
    }
}

pub async fn handle_mcp_request(
    state: &AppState,
    headers: &HeaderMap,
    tenant_ctx: &TenantContext,
    request: json_rpc::JsonRpcRequest,
    auth_role: Option<Role>,
) -> McpHandleOutcome {
    let id = request.id.clone();

    if let Err(e) = run_transport_validation(state, headers, &request) {
        return McpHandleOutcome::error(id, e);
    }

    let meta = extract_meta(request.params.as_ref());
    let protocol_version = match normalize_protocol_version(
        headers
            .get("mcp-protocol-version")
            .and_then(|v| v.to_str().ok()),
        &meta,
    ) {
        Ok(v) => v,
        Err(e) => return McpHandleOutcome::error(id, e),
    };

    let workspace_header = workspace_from_param_header(headers);
    let ctx = self::dispatch::DispatchContext {
        state,
        tenant_ctx,
        protocol_version: &protocol_version,
        meta: &meta,
        workspace_header,
        auth_role,
    };

    if wants_sse_response(headers, &request.method, request.params.as_ref()) {
        let params = request.params.clone().unwrap_or(serde_json::json!({}));
        return McpHandleOutcome::Sse {
            body: retrieve_sse_stream(&ctx, params, id),
        };
    }

    match dispatch_method(&ctx, &request.method, request.params).await {
        Ok(result) => McpHandleOutcome::ok(id, result),
        Err(e) => McpHandleOutcome::error(id, e),
    }
}

fn run_transport_validation(
    state: &AppState,
    headers: &HeaderMap,
    request: &json_rpc::JsonRpcRequest,
) -> Result<(), json_rpc::GatewayError> {
    let modern = headers.contains_key("mcp-protocol-version")
        || headers.contains_key("mcp-method")
        || request
            .params
            .as_ref()
            .and_then(|p| p.get("_meta"))
            .is_some();

    if modern {
        validate_accept(headers)?;
        reject_session_header(headers)?;
    }

    let allowed = state.security.cors_origins.as_deref();
    validate_origin(headers, allowed)?;

    let meta = extract_meta(request.params.as_ref());
    let protocol_version = normalize_protocol_version(
        headers
            .get("mcp-protocol-version")
            .and_then(|v| v.to_str().ok()),
        &meta,
    )?;

    validate_routing_headers(
        headers,
        &protocol_version,
        &request.method,
        request.params.as_ref(),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use serde_json::json;

    #[tokio::test]
    async fn legacy_tools_list_without_modern_headers() {
        let state = AppState::test_state();
        let headers = HeaderMap::new();
        let tenant = TenantContext::default();
        let req = json_rpc::JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: json!(1),
            method: "tools/list".into(),
            params: None,
        };
        let out = handle_mcp_request(&state, &headers, &tenant, req, None).await;
        match out {
            McpHandleOutcome::Json { status, .. } => assert_eq!(status, StatusCode::OK),
            McpHandleOutcome::Accepted => panic!("unexpected notification"),
            McpHandleOutcome::Sse { .. } => panic!("tools/list must not return SSE"),
        }
    }
}
