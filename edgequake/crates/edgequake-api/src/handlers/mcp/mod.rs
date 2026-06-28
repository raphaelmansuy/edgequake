//! MCP Streamable HTTP handler — thin Axum adapter (SPEC-028 SOTA).

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::Value;
use utoipa::ToSchema;

use crate::handlers::auth::ApiOptionalAuth;
use crate::mcp::gateway::body::{parse_mcp_body, ParsedMcpBody};
use crate::mcp::gateway::{handle_mcp_request, McpHandleOutcome};
use crate::middleware::TenantContext;
use crate::state::AppState;

#[derive(Debug, Clone, serde::Deserialize, ToSchema)]
pub struct McpJsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

/// OpenAPI schema anchor (runtime uses raw body parsing in [`mcp_handler`]).
#[utoipa::path(
    post,
    path = "/mcp",
    tag = "MCP",
    request_body = McpJsonRpcRequest,
    responses(
        (status = 200, description = "JSON-RPC 2.0 response"),
        (status = 202, description = "JSON-RPC notification accepted"),
        (status = 413, description = "Payload too large")
    )
)]
#[allow(dead_code)]
pub async fn mcp_openapi(
    _state: State<AppState>,
    _headers: HeaderMap,
    _tenant_ctx: TenantContext,
    Json(_request): Json<McpJsonRpcRequest>,
) -> StatusCode {
    StatusCode::OK
}

pub async fn mcp_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    tenant_ctx: TenantContext,
    auth: ApiOptionalAuth,
    body: Bytes,
) -> Response {
    let role = auth.context().map(|ctx| ctx.role.clone());
    mcp_handler_inner(&state, &headers, &tenant_ctx, role, &body).await
}

/// OpenAPI schema anchor for `/api/v1/mcp` alias.
#[utoipa::path(
    post,
    path = "/api/v1/mcp",
    tag = "MCP",
    request_body = McpJsonRpcRequest,
    responses(
        (status = 200, description = "JSON-RPC 2.0 response"),
        (status = 202, description = "JSON-RPC notification accepted")
    )
)]
#[allow(dead_code)]
pub async fn mcp_openapi_v1(
    _state: State<AppState>,
    _headers: HeaderMap,
    _tenant_ctx: TenantContext,
    Json(_request): Json<McpJsonRpcRequest>,
) -> StatusCode {
    StatusCode::OK
}

/// Alias route: `/api/v1/mcp` (same handler).
pub async fn mcp_handler_v1(
    state: State<AppState>,
    headers: HeaderMap,
    tenant_ctx: TenantContext,
    auth: ApiOptionalAuth,
    body: Bytes,
) -> Response {
    mcp_handler(state, headers, tenant_ctx, auth, body).await
}

#[utoipa::path(
    get,
    path = "/.well-known/oauth-protected-resource",
    tag = "MCP",
    responses((status = 200, description = "OAuth Protected Resource Metadata (RFC 9728)"))
)]
pub async fn mcp_oauth_protected_resource(
    state: State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    crate::mcp::auth::mcp_oauth_protected_resource(state, headers).await
}

/// MCP Registry manifest (official `server.json` format) for discovery and publish SSOT.
#[utoipa::path(
    get,
    path = "/.well-known/mcp/server.json",
    tag = "MCP",
    responses((status = 200, description = "MCP Registry server.json manifest"))
)]
pub async fn mcp_registry_server_json(headers: HeaderMap) -> impl IntoResponse {
    let cfg = crate::mcp::McpPublicConfig::resolve(&headers);
    let manifest = crate::mcp::build_registry_manifest(Some(&cfg.public_base_url()));
    (StatusCode::OK, Json(manifest))
}

async fn mcp_handler_inner(
    state: &AppState,
    headers: &HeaderMap,
    tenant_ctx: &TenantContext,
    auth_role: Option<edgequake_auth::Role>,
    body: &[u8],
) -> Response {
    let parsed = match parse_mcp_body(body) {
        Ok(p) => p,
        Err(err) => {
            return mcp_outcome_to_response(McpHandleOutcome::error(Value::Null, err));
        }
    };

    let outcome = match parsed {
        ParsedMcpBody::Notification { .. } => McpHandleOutcome::notification_accepted(),
        ParsedMcpBody::Request(request) => {
            handle_mcp_request(state, headers, tenant_ctx, request, auth_role).await
        }
    };

    mcp_outcome_to_response(outcome)
}

fn mcp_outcome_to_response(outcome: McpHandleOutcome) -> Response {
    match outcome {
        McpHandleOutcome::Accepted => StatusCode::ACCEPTED.into_response(),
        McpHandleOutcome::Sse { body } => {
            let mut response = crate::mcp::gateway::sse::sse_response(body);
            response.headers_mut().insert(
                crate::mcp::gateway::sse::HEADER_ACCEL_BUFFERING,
                "no".parse().unwrap(),
            );
            response
        }
        McpHandleOutcome::Json {
            status,
            body,
            www_authenticate,
        } => {
            let mut response = (status, Json(body)).into_response();
            if let Some(www) = www_authenticate {
                if let Ok(val) = www.parse() {
                    response.headers_mut().insert("WWW-Authenticate", val);
                }
            }
            response
        }
    }
}
