//! SPEC-087 / Issue #335 — shared guest identity (no per-browser anon_* growth).
//!
//! Run: `cargo test -p edgequake-api --test e2e_spec087_anonymous_guest`

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::handlers::postgres_user_bootstrap::{
    ensure_postgres_user_exists, resolve_identity_bootstrap_policy, IdentityBootstrapPolicy,
};
use edgequake_api::services::identity_storage::{
    is_anonymous_identity, shared_guest_user_id, SHARED_GUEST_EMAIL, SHARED_GUEST_USERNAME,
};
use edgequake_api::{AppState, AuthRuntime, Server, ServerConfig};
use edgequake_auth::AuthConfig;
use edgequake_core::{CreateWorkspaceRequest, Tenant, TenantPlan};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

fn test_config() -> ServerConfig {
    ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: false,
    }
}

async fn json_body(resp: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), 2 * 1024 * 1024)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap_or(json!({}))
}

#[test]
fn shared_guest_id_is_deterministic_per_tenant() {
    let t1 = Uuid::new_v4();
    let t2 = Uuid::new_v4();
    assert_eq!(shared_guest_user_id(t1), shared_guest_user_id(t1));
    assert_ne!(shared_guest_user_id(t1), shared_guest_user_id(t2));
}

#[test]
fn is_anonymous_identity_markers() {
    assert!(is_anonymous_identity(
        SHARED_GUEST_USERNAME,
        SHARED_GUEST_EMAIL,
        "anonymous"
    ));
    assert!(is_anonymous_identity(
        "anon_abcdef12",
        "abcdef12@anonymous.local",
        "anonymous"
    ));
    assert!(!is_anonymous_identity(
        "alice",
        "alice@example.com",
        "$argon2id$v=19$m=65536,t=3,p=4$abc"
    ));
}

#[test]
fn bootstrap_policy_matrix() {
    assert_eq!(
        resolve_identity_bootstrap_policy(true, false),
        IdentityBootstrapPolicy::UsePrincipal
    );
    assert_eq!(
        resolve_identity_bootstrap_policy(false, true),
        IdentityBootstrapPolicy::UseSharedGuest
    );
    assert_eq!(
        resolve_identity_bootstrap_policy(false, false),
        IdentityBootstrapPolicy::DenyAnonymous
    );
}

/// Auth off + allow_anonymous: two client UUIDs resolve to the same guest id.
#[tokio::test]
async fn two_client_uuids_map_to_one_shared_guest() {
    let state = AppState::test_state();
    assert!(!state.auth.config.auth_enabled);
    assert!(state.auth.config.allow_anonymous);

    let tenant = Tenant::new("t-087", "t-087").with_plan(TenantPlan::Pro);
    let tenant = state.workspace_service.create_tenant(tenant).await.unwrap();
    let tenant_id = tenant.tenant_id;

    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let ea = ensure_postgres_user_exists(&state, tenant_id, a)
        .await
        .unwrap();
    let eb = ensure_postgres_user_exists(&state, tenant_id, b)
        .await
        .unwrap();

    assert_eq!(ea, eb);
    assert_eq!(ea, shared_guest_user_id(tenant_id));
    assert_ne!(ea, a);
    assert_ne!(ea, b);
}

/// Auth off + allow_anonymous=false → 401, no guest resolution.
#[tokio::test]
async fn allow_anonymous_false_denies_bootstrap() {
    let mut state = AppState::test_state();
    state.auth = AuthRuntime::new(AuthConfig {
        auth_enabled: false,
        allow_anonymous: false,
        dev_mode: true,
        ..AuthConfig::default()
    });

    let tenant_id = Uuid::new_v4();
    let err = ensure_postgres_user_exists(&state, tenant_id, Uuid::new_v4())
        .await
        .unwrap_err();
    assert_eq!(err.status_code(), StatusCode::UNAUTHORIZED);
}

/// Create conversation with two X-User-ID values still succeeds (shared guest ownership).
#[tokio::test]
async fn create_conversation_two_browsers_same_guest_owner() {
    let state = AppState::test_state();
    let tenant = Tenant::new("t-087-conv", "t-087-conv").with_plan(TenantPlan::Pro);
    let tenant = state.workspace_service.create_tenant(tenant).await.unwrap();
    let _ws = state
        .workspace_service
        .create_workspace(
            tenant.tenant_id,
            CreateWorkspaceRequest {
                name: "ws".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    let app = Server::new(test_config(), state.clone()).build_router();
    let guest = shared_guest_user_id(tenant.tenant_id);

    for uuid in [Uuid::new_v4(), Uuid::new_v4()] {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/conversations")
                    .header("content-type", "application/json")
                    .header("X-Tenant-ID", tenant.tenant_id.to_string())
                    .header("X-User-ID", uuid.to_string())
                    .body(Body::from(json!({"title":"spec087"}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED, "uuid={uuid}");
        let _ = json_body(resp).await;
    }

    // Both creates resolve ownership through the same guest principal.
    assert_eq!(
        ensure_postgres_user_exists(&state, tenant.tenant_id, Uuid::new_v4())
            .await
            .unwrap(),
        guest
    );
}

/// Auth on: bootstrap returns principal (client) id unchanged — no guest remap.
#[tokio::test]
async fn auth_on_keeps_principal_user_id() {
    let mut state = AppState::test_state();
    state.auth = AuthRuntime::new(AuthConfig {
        auth_enabled: true,
        allow_anonymous: true,
        dev_mode: false,
        ..AuthConfig::default()
    });

    let tenant_id = Uuid::new_v4();
    let principal = Uuid::new_v4();
    let effective = ensure_postgres_user_exists(&state, tenant_id, principal)
        .await
        .unwrap();
    assert_eq!(effective, principal);
    assert_ne!(effective, shared_guest_user_id(tenant_id));
}
