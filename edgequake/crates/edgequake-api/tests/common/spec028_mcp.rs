//! SPEC-028 MCP E2E harness (DRY — shared by transport, tool, and OAuth tests).

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use edgequake_auth::Role;
use serde_json::{json, Value};
use tower::ServiceExt;

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

pub async fn auth_enabled_mcp_state() -> AppState {
    let mut state = AppState::test_state();
    state.auth.config.auth_enabled = true;
    state.auth.config.dev_mode = false;
    // SPEC-154: break-glass is master_api_key only (not EDGEQUAKE_API_KEYS).
    state.auth.config.master_api_key = Some("master-mcp-test-key".to_string());
    state.auth.config.api_keys = vec!["master-mcp-test-key".to_string()];
    state.workspace_service.seed_default_workspace().await;
    // An owned workspace must still use test providers, regardless of local credentials.
    std::env::set_var("EDGEQUAKE_ALLOW_MOCK_PROVIDER", "1");
    state
        .workspace_service
        .update_workspace(
            edgequake_api::middleware::default_workspace_uuid(),
            edgequake_core::UpdateWorkspaceRequest {
                llm_provider: Some("mock".into()),
                llm_model: Some("mock-model".into()),
                embedding_provider: Some("mock".into()),
                embedding_model: Some("mock-model".into()),
                embedding_dimension: Some(1536),
                ..Default::default()
            },
        )
        .await
        .expect("mock workspace providers");
    state
        .workspace_service
        .add_membership(edgequake_core::Membership::new(
            edgequake_api::middleware::default_user_uuid(),
            edgequake_api::middleware::default_tenant_uuid(),
            edgequake_core::MembershipRole::Owner,
        ))
        .await
        .expect("fixture membership");
    state.operational_stores.identity = Some(std::sync::Arc::new(
        super::identity_fixture::FixtureIdentityStore::new(state.workspace_service.clone()),
    ));
    state
}

/// Issue an EdgeQuake JWT for MCP Bearer auth e2e (aud = resource, full scopes).
pub fn issue_test_jwt(state: &AppState, role: Role) -> String {
    issue_mcp_jwt(state, role, "edgequake:read edgequake:query")
}

/// Issue an MCP-bound JWT with explicit OAuth scopes.
pub fn issue_mcp_jwt(state: &AppState, role: Role, scope: &str) -> String {
    use edgequake_auth::Claims;

    let resource = std::env::var("EDGEQUAKE_PUBLIC_URL")
        .ok()
        .map(|s| format!("{}/mcp", s.trim().trim_end_matches('/')))
        .unwrap_or_else(|| "http://127.0.0.1:8080/mcp".to_string());

    let claims = Claims::new(edgequake_api::middleware::default_user_uuid(), role, 3600)
        .with_audience(vec![resource])
        .with_scope(scope.to_string());
    state
        .auth
        .jwt
        .generate_token_with_claims(claims)
        .expect("sign mcp test jwt")
}

/// Issue a web-session JWT (no MCP aud / scope) — SPEC-154 EC-154-02.
pub fn issue_web_session_jwt(state: &AppState, role: Role) -> String {
    use edgequake_auth::Claims;

    let claims = Claims::new(edgequake_api::middleware::default_user_uuid(), role, 3600);
    state
        .auth
        .jwt
        .generate_token_with_claims(claims)
        .expect("sign web session jwt")
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

/// Extract CallToolResult.structuredContent (MCP 2026-07-28); fall back to raw result.
pub fn tool_structured(body: &Value) -> &Value {
    body.get("result")
        .and_then(|r| r.get("structuredContent"))
        .or_else(|| body.get("result"))
        .unwrap_or(body)
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
