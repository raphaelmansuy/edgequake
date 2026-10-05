//! SPEC-154 Wave 3 / gap-close — scoped API keys (EC-154-05 / EC-154-06).

mod common;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use common::spec028_mcp::{auth_enabled_mcp_state, build_mcp_app, tools_call_body};
use edgequake_api::oauth::scopes::{
    default_api_key_scopes, normalize_api_key_scopes, MCP_SCOPE_QUERY, MCP_SCOPE_READ,
    MCP_SCOPE_WRITE,
};
use edgequake_api::oauth::types::McpAuthScopes;
use edgequake_api::{AppState, Server, ServerConfig};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn parse_json(response: axum::response::Response) -> Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap_or_else(|_| json!({}))
}

fn build_app(state: AppState) -> axum::Router {
    Server::new(
        ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            enable_cors: false,
            enable_compression: false,
            enable_swagger: true,
        },
        state,
    )
    .build_router()
}

#[test]
fn normalize_legacy_read_write() {
    let n =
        normalize_api_key_scopes(&["read".to_string(), "write".to_string(), "query".to_string()]);
    assert!(n.contains(&MCP_SCOPE_READ.to_string()));
    assert!(n.contains(&MCP_SCOPE_QUERY.to_string()));
    assert!(n.contains(&MCP_SCOPE_WRITE.to_string()));
}

#[test]
fn normalize_empty_defaults_read_query_not_write() {
    let n = normalize_api_key_scopes(&[]);
    assert_eq!(
        n,
        vec![MCP_SCOPE_READ.to_string(), MCP_SCOPE_QUERY.to_string()]
    );
}

#[test]
fn scoped_api_key_denies_write() {
    let scopes = McpAuthScopes::from_api_key_scopes(vec![
        MCP_SCOPE_READ.to_string(),
        MCP_SCOPE_QUERY.to_string(),
    ]);
    assert!(scopes.allows(MCP_SCOPE_READ));
    assert!(scopes.allows(MCP_SCOPE_QUERY));
    assert!(!scopes.allows(MCP_SCOPE_WRITE));
}

#[test]
fn break_glass_allows_write() {
    assert!(McpAuthScopes::api_key_full().allows(MCP_SCOPE_WRITE));
}

#[test]
fn env_static_keys_are_not_break_glass() {
    let scopes = McpAuthScopes::from_api_key_scopes(default_api_key_scopes());
    assert!(!scopes.break_glass);
    assert!(!scopes.allows(MCP_SCOPE_WRITE));
}

#[tokio::test]
async fn ec_154_06_master_key_can_list_tools() {
    let state = auth_enabled_mcp_state().await;
    let app = build_mcp_app(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("Authorization", "Bearer master-mcp-test-key")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

/// EC-154-05: stored API key with read+query must get HTTP 403 on write tool.
#[tokio::test]
async fn ec_154_05_stored_read_query_key_ingest_forbidden() {
    let state = auth_enabled_mcp_state().await;
    let owner = common::spec028_mcp::issue_web_session_jwt(&state, edgequake_auth::Role::Admin);

    let app = build_app(state);

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/api-keys")
                .header(header::AUTHORIZATION, format!("Bearer {owner}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "name": "scoped-read",
                        "scopes": ["read", "query"]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED, "create scoped key");
    let created = parse_json(create).await;
    let raw_key = created["api_key"].as_str().expect("api_key").to_string();
    assert!(
        !created["scopes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s.as_str() == Some(MCP_SCOPE_WRITE)),
        "created key must not include write"
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header(header::AUTHORIZATION, format!("Bearer {raw_key}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    tools_call_body("eq_ingest", json!({"text": "hello"})).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "read+query stored key must not call eq_ingest"
    );
}

/// EC-154-06: master key may call write tools (scope gate passes; body may still error).
#[tokio::test]
async fn ec_154_06_master_key_write_not_insufficient_scope() {
    let state = auth_enabled_mcp_state().await;
    let app = build_mcp_app(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header(header::AUTHORIZATION, "Bearer master-mcp-test-key")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    tools_call_body("eq_ingest", json!({"text": "hello"})).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_ne!(
        response.status(),
        StatusCode::FORBIDDEN,
        "master break-glass must not get insufficient_scope on eq_ingest"
    );
    assert_ne!(response.status(), StatusCode::UNAUTHORIZED);
}

/// Env static keys are not break-glass — MCP write is 403.
#[tokio::test]
async fn ec_154_env_api_key_not_break_glass_write_forbidden() {
    let mut state = AppState::test_state();
    state.auth.config.auth_enabled = true;
    state.auth.config.dev_mode = false;
    state.auth.config.master_api_key = None;
    state.auth.config.api_keys = vec!["env-static-only-key".to_string()];
    let app = build_mcp_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header(header::AUTHORIZATION, "Bearer env-static-only-key")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    tools_call_body("eq_ingest", json!({"text": "x"})).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "EDGEQUAKE_API_KEYS alone must not be MCP write break-glass"
    );
}

#[tokio::test]
async fn ec_154_env_api_key_without_membership_cannot_list_tools() {
    let mut state = AppState::test_state();
    state.auth.config.auth_enabled = true;
    state.auth.config.dev_mode = false;
    state.auth.config.master_api_key = None;
    state.auth.config.api_keys = vec!["env-static-read-key".to_string()];
    let app = build_mcp_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header(header::AUTHORIZATION, "Bearer env-static-read-key")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
