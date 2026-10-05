//! SPEC-154 Wave 2 / gap-close — membership bind on MCP (EC-154-07 / 13 / 30).

mod common;

use axum::http::StatusCode;
use common::spec028_mcp::{
    auth_enabled_mcp_state, build_mcp_app, issue_mcp_jwt, mcp_post_bearer, tools_call_body,
};
use edgequake_api::{AppState, Server, ServerConfig};
use edgequake_auth::{Claims, Role};
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn ec_154_13_mcp_header_workspace_mismatch_forbidden_under_strict_bind() {
    let mut state = auth_enabled_mcp_state().await;
    state.security.strict_tenant_bind = true;

    let user_id = edgequake_api::middleware::default_user_uuid();
    let resource = "http://127.0.0.1:8080/mcp";
    let claims = Claims::new(user_id, Role::User, 3600)
        .with_audience(vec![resource.to_string()])
        .with_scope("edgequake:read edgequake:query".to_string())
        .with_workspace_id("00000000-0000-0000-0000-0000000000aa");
    let token = state
        .auth
        .jwt
        .generate_token_with_claims(claims)
        .expect("sign");

    let app = build_mcp_app(state);
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("Authorization", format!("Bearer {token}"))
                .header("X-Workspace-Id", "00000000-0000-0000-0000-0000000000bb")
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(
                    tools_call_body(
                        "eq_search",
                        json!({"query": "mismatch", "workspace_id": "00000000-0000-0000-0000-0000000000bb"}),
                    )
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "strict bind must reject header vs JWT workspace mismatch on MCP"
    );
}

#[tokio::test]
async fn ec_154_master_key_still_reaches_mcp_under_strict_bind() {
    let mut state = auth_enabled_mcp_state().await;
    state.security.strict_tenant_bind = true;
    let app = build_mcp_app(state);

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("Authorization", "Bearer master-mcp-test-key")
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(
                    json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "master key is break-glass under strict bind (audited skip)"
    );
}

#[tokio::test]
async fn ec_154_mcp_jwt_matching_workspace_still_ok() {
    let mut state = auth_enabled_mcp_state().await;
    state.security.strict_tenant_bind = true;
    let token = issue_mcp_jwt(&state, Role::User, "edgequake:read");
    let app = build_mcp_app(state);

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

/// EC-154-30: authenticated JWT without workspace claim + tool foreign workspace_id → forbidden.
#[tokio::test]
async fn ec_154_30_tool_foreign_workspace_without_claim_forbidden() {
    let state = auth_enabled_mcp_state().await;
    let resource = "http://127.0.0.1:8080/mcp";
    let claims = Claims::new(
        edgequake_api::middleware::default_user_uuid(),
        Role::User,
        3600,
    )
    .with_audience(vec![resource.to_string()])
    .with_scope("edgequake:read edgequake:query".to_string());
    // No workspace_id claim.
    let token = state
        .auth
        .jwt
        .generate_token_with_claims(claims)
        .expect("sign");
    let app = build_mcp_app(state);

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(
                    tools_call_body(
                        "eq_search",
                        json!({
                            "query": "x",
                            "workspace_id": "00000000-0000-0000-0000-00000000dead"
                        }),
                    )
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "EC-154-30: tool workspace without claim must be forbidden"
    );
}

#[cfg(feature = "postgres")]
async fn connect_and_bootstrap() -> Option<sqlx::PgPool> {
    use edgequake_api::state::migration_bootstrap::run_postgres_migrations;
    use sqlx::postgres::PgPoolOptions;

    let database_url = common::spec013_postgres::try_database_url()?;
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&database_url)
        .await
        .expect("connect postgres");
    run_postgres_migrations(&pool)
        .await
        .expect("bootstrap migrations");
    Some(pool)
}

/// EC-154-07: JWT claims a workspace the user is not a member of → 403 under strict bind + PG.
#[cfg(feature = "postgres")]
#[tokio::test]
async fn ec_154_07_non_member_mcp_forbidden_with_pg() {
    let pool = match connect_and_bootstrap().await {
        Some(p) => p,
        None => {
            eprintln!("SKIP ec_154_07_non_member_mcp_forbidden_with_pg: DATABASE_URL not set");
            return;
        }
    };

    let mut state = AppState::test_state_with_pg_pool(pool.clone());
    state.auth.config.auth_enabled = true;
    state.auth.config.dev_mode = false;
    state.auth.config.master_api_key = Some("master-mcp-test-key".to_string());
    state.auth.config.api_keys = vec!["master-mcp-test-key".to_string()];
    state.security.strict_tenant_bind = true;
    state.security.kv_identity_mirror = false;
    // Provision authority rows before scoped identity helpers validate ownership.
    // The fixture pool is administrative; request RLS remains enabled below.
    common::provider_access::http_harness::seed_scope(
        &pool,
        edgequake_api::middleware::default_tenant_uuid(),
        edgequake_api::middleware::default_workspace_uuid(),
        "mcp-default",
    )
    .await;
    // Ensure default tenant/workspace exist; do NOT add membership for this user.
    edgequake_api::services::identity_storage::ensure_default_tenant_workspace(
        &pool,
        &state.security,
    )
    .await
    .expect("default tenant/workspace");
    state.initialize_defaults().await.expect("defaults");

    let user_id = edgequake_api::middleware::default_user_uuid();
    let tenant_id = edgequake_api::middleware::default_tenant_uuid();
    let foreign_ws = Uuid::new_v4();
    let resource = "http://127.0.0.1:8080/mcp";
    let claims = Claims::new(user_id, Role::User, 3600)
        .with_audience(vec![resource.to_string()])
        .with_scope("edgequake:read edgequake:query".to_string())
        .with_tenant_id(tenant_id.to_string())
        .with_workspace_id(foreign_ws.to_string());
    let token = state
        .auth
        .jwt
        .generate_token_with_claims(claims)
        .expect("sign");

    let app = Server::new(
        ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            enable_cors: false,
            enable_compression: false,
            enable_swagger: false,
        },
        state,
    )
    .build_router();

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("Authorization", format!("Bearer {token}"))
                .header("X-Tenant-Id", tenant_id.to_string())
                .header("X-Workspace-Id", foreign_ws.to_string())
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(
                    tools_call_body(
                        "eq_search",
                        json!({"query": "x", "workspace_id": foreign_ws.to_string()}),
                    )
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "EC-154-07: non-member under strict bind must be 403"
    );
}
