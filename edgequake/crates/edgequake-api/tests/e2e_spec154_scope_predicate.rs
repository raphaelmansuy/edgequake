//! SPEC-154 Wave 1 — scope predicate (EC-154-03 / EC-154-04 / EC-154-20).

mod common;

use axum::http::StatusCode;
use common::spec028_mcp::{
    auth_enabled_mcp_state, build_mcp_app, issue_mcp_jwt, mcp_post_bearer, parse_json,
    tools_call_body,
};
use edgequake_api::oauth::scopes::{scopes_cover, MCP_SCOPE_QUERY, MCP_SCOPE_READ};
use edgequake_auth::Role;
use serde_json::json;
use tower::ServiceExt;

#[test]
fn ec_154_04_scopes_cover_empty_is_deny() {
    assert!(!scopes_cover(&[], MCP_SCOPE_READ));
    assert!(!scopes_cover(&[], MCP_SCOPE_QUERY));
}

#[test]
fn ec_154_20_star_covers_when_explicit() {
    let star = vec!["*".to_string()];
    assert!(scopes_cover(&star, MCP_SCOPE_READ));
}

#[tokio::test]
async fn ec_154_03_empty_scope_jwt_forbidden_on_query_tool() {
    let state = auth_enabled_mcp_state().await;
    let app = build_mcp_app(state.clone());
    // Mint MCP JWT with empty scope claim (aud ok, scopes empty).
    let token = issue_mcp_jwt(&state, Role::User, "");

    let response = app
        .oneshot(mcp_post_bearer(
            "/mcp",
            &token,
            tools_call_body(
                "eq_search",
                json!({"query": "test", "workspace_id": "00000000-0000-0000-0000-000000000001"}),
            ),
        ))
        .await
        .unwrap();

    // Gateway may return HTTP 403 with WWW-Authenticate insufficient_scope,
    // or JSON-RPC error mapped to insufficient_scope.
    let status = response.status();
    let www = response
        .headers()
        .get("www-authenticate")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let body = parse_json(response).await;

    let insufficient = status == StatusCode::FORBIDDEN
        || www.contains("insufficient_scope")
        || body
            .pointer("/error/message")
            .and_then(|m| m.as_str())
            .is_some_and(|m| m.contains("insufficient_scope") || m.contains("scope"))
        || body
            .pointer("/result/isError")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

    assert!(
        insufficient,
        "empty scope must not run query tools; status={status} www={www} body={body}"
    );
}

#[tokio::test]
async fn ec_154_read_scope_allows_tools_list() {
    let state = auth_enabled_mcp_state().await;
    let app = build_mcp_app(state.clone());
    let token = issue_mcp_jwt(&state, Role::User, "edgequake:read");

    let response = app
        .oneshot(mcp_post_bearer(
            "/mcp",
            &token,
            json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
