//! SPEC-028 MCP E2E harness (DRY — shared by transport, tool, and OAuth tests).

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use edgequake_auth::Role;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

pub const MCP_ACCEPT: &str = "application/json, text/event-stream";
pub const MCP_PROTOCOL: &str = "2026-07-28";

pub fn mcp_server_config() -> ServerConfig {
    ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: false,
    }
}

pub fn build_mcp_app(state: AppState) -> axum::Router {
    Server::new(mcp_server_config(), state).build_router()
}

pub fn default_mcp_app() -> axum::Router {
    build_mcp_app(AppState::test_state())
}

pub fn auth_enabled_mcp_state() -> AppState {
    let mut state = AppState::test_state();
    state.auth.config.auth_enabled = true;
    state.auth.config.dev_mode = false;
    state.auth.config.api_keys = vec!["master-mcp-test-key".to_string()];
    state
}

/// Issue an EdgeQuake JWT for MCP Bearer auth e2e (same verifier as OIDC login).
pub fn issue_test_jwt(state: &AppState, role: Role) -> String {
    state
        .auth
        .jwt
        .generate_token(Uuid::new_v4(), role)
        .expect("sign test jwt")
}

pub fn mcp_post_bearer(uri: &str, token: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("Content-Type", "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(body.to_string()))
        .unwrap()
}

pub fn mcp_post_bearer_and_api_key(
    uri: &str,
    token: &str,
    api_key: &str,
    body: Value,
) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("Content-Type", "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-API-Key", api_key)
        .body(Body::from(body.to_string()))
        .unwrap()
}

pub async fn mcp_tools_call_bearer(
    app: &axum::Router,
    uri: &str,
    token: &str,
    name: &str,
    arguments: Value,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(mcp_post_bearer(
            uri,
            token,
            tools_call_body(name, arguments),
        ))
        .await
        .unwrap();
    let status = response.status();
    (status, parse_json(response).await)
}

pub async fn parse_json(response: axum::response::Response) -> Value {
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("read body");
    serde_json::from_slice(&body).expect("parse json")
}

pub fn mcp_post_legacy(uri: &str, body: Value) -> Request<Body> {
    mcp_post_bytes(uri, body.to_string().into_bytes())
}

pub fn mcp_post_bytes(uri: &str, body: Vec<u8>) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("Content-Type", "application/json")
        .body(Body::from(body))
        .unwrap()
}

pub fn mcp_post_modern(uri: &str, method: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("Content-Type", "application/json")
        .header("Accept", MCP_ACCEPT)
        .header("MCP-Protocol-Version", MCP_PROTOCOL)
        .header("Mcp-Method", method)
        .body(Body::from(body.to_string()))
        .unwrap()
}

pub fn mcp_post_stream(uri: &str, method: &str, tool_name: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("Content-Type", "application/json")
        .header("Accept", MCP_ACCEPT)
        .header("MCP-Protocol-Version", MCP_PROTOCOL)
        .header("Mcp-Method", method)
        .header("Mcp-Name", tool_name)
        .header("Mcp-Stream", "true")
        .body(Body::from(body.to_string()))
        .unwrap()
}

pub fn tools_call_body(name: &str, arguments: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": "call-1",
        "method": "tools/call",
        "params": {
            "name": name,
            "arguments": arguments
        }
    })
}

pub async fn mcp_tools_call(
    app: &axum::Router,
    uri: &str,
    name: &str,
    arguments: Value,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(mcp_post_legacy(uri, tools_call_body(name, arguments)))
        .await
        .unwrap();
    let status = response.status();
    (status, parse_json(response).await)
}
