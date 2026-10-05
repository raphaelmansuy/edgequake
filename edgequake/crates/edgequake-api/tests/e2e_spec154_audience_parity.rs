//! SPEC-154 Wave 1 — audience capability parity (EC-154-01 / EC-154-02).

mod common;

use axum::http::StatusCode;
use common::spec028_mcp::{
    auth_enabled_mcp_state, build_mcp_app, issue_mcp_jwt, issue_web_session_jwt, mcp_post_bearer,
    parse_json,
};
use edgequake_auth::Role;
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
async fn ec_154_01_mcp_jwt_rejected_on_rest() {
    let state = auth_enabled_mcp_state().await;
    let app = build_mcp_app(state.clone());
    let mcp_token = issue_mcp_jwt(&state, Role::User, "edgequake:read edgequake:query");

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/api/v1/documents")
                .header("Authorization", format!("Bearer {mcp_token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "MCP resource JWT must not authorize REST (LAW-154-3)"
    );
}

#[tokio::test]
async fn ec_154_02_web_session_jwt_rejected_on_mcp() {
    let state = auth_enabled_mcp_state().await;
    let app = build_mcp_app(state.clone());
    let web_token = issue_web_session_jwt(&state, Role::User);

    let response = app
        .oneshot(mcp_post_bearer(
            "/mcp",
            &web_token,
            json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let www = response
        .headers()
        .get("www-authenticate")
        .and_then(|v| v.to_str().ok())
        .expect("WWW-Authenticate on MCP 401");
    assert!(
        www.contains("resource_metadata="),
        "challenge must point at PRM: {www}"
    );
}

#[tokio::test]
async fn ec_154_web_session_jwt_accepted_on_rest() {
    let state = auth_enabled_mcp_state().await;
    let app = build_mcp_app(state.clone());
    let web_token = issue_web_session_jwt(&state, Role::User);

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/api/v1/documents")
                .header("Authorization", format!("Bearer {web_token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_ne!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "web session JWT must authorize REST; got {}",
        response.status()
    );
}

#[tokio::test]
async fn ec_154_mcp_jwt_accepted_on_mcp_tools_list() {
    let state = auth_enabled_mcp_state().await;
    let app = build_mcp_app(state.clone());
    let mcp_token = issue_mcp_jwt(&state, Role::User, "edgequake:read edgequake:query");

    let response = app
        .oneshot(mcp_post_bearer(
            "/mcp",
            &mcp_token,
            json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await;
    assert!(body.get("result").is_some(), "tools/list result: {body}");
}
