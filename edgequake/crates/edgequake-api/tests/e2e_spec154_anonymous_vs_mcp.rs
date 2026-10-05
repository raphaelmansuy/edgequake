//! SPEC-154 EC-154-21 — auth-on: guest chat policy vs MCP write.

mod common;

use axum::http::StatusCode;
use common::spec028_mcp::{auth_enabled_mcp_state, build_mcp_app, tools_call_body};
use edgequake_api::handlers::postgres_user_bootstrap::{
    resolve_identity_bootstrap_policy, IdentityBootstrapPolicy,
};
use serde_json::json;
use tower::ServiceExt;

#[test]
fn ec_154_21_auth_on_uses_principal_not_shared_guest() {
    assert_eq!(
        resolve_identity_bootstrap_policy(true, true),
        IdentityBootstrapPolicy::UsePrincipal,
        "auth_enabled must never mint shared guest (SPEC-087 / EC-154-21)"
    );
    assert_eq!(
        resolve_identity_bootstrap_policy(true, false),
        IdentityBootstrapPolicy::UsePrincipal
    );
}

#[tokio::test]
async fn ec_154_21_mcp_write_without_creds_unauthorized() {
    let state = auth_enabled_mcp_state().await;
    let app = build_mcp_app(state);
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(
                    tools_call_body("eq_ingest", json!({"text": "nope"})).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "auth-on MCP write without credentials must be 401"
    );
}
