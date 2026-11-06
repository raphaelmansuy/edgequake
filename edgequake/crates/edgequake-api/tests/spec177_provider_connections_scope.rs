//! SPEC-177 — a fleet connection is `scope=deployment`, not "the tenant header was absent".
//!
//! Login writes the JWT tenant into the request even when the browser omits `X-Tenant-ID`.
//! Create, update, delete and test therefore refuse a resolved tenant, and accept the fleet
//! only when the caller says `scope=deployment` and is a platform admin.

#[cfg(feature = "postgres")]
#[path = "../../edgequake-storage/tests/support/postgres_access_pool.rs"]
#[allow(dead_code)]
mod postgres_access_pool;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use edgequake_api::handlers::auth::{ApiRequireAdmin, RequestAuthContext};
use edgequake_api::handlers::connections::{
    create_connection, delete_connection, test_stored_connection, update_connection,
    ConnectionScopeQuery, DeleteConfirm, UpsertConnection,
};
use edgequake_api::middleware::TenantContext;
use edgequake_api::state::AppState;
use edgequake_api::ApiError;
use edgequake_auth::Role;
use uuid::Uuid;

fn principal(role: Role) -> ApiRequireAdmin {
    ApiRequireAdmin(RequestAuthContext {
        user_id: "spec177".into(),
        role,
    })
}

fn tenant() -> TenantContext {
    TenantContext {
        tenant_id: Some("00000000-0000-0000-0000-000000000002".into()),
        ..TenantContext::default()
    }
}

fn scope(value: Option<&str>) -> Query<ConnectionScopeQuery> {
    Query(ConnectionScopeQuery {
        scope: value.map(str::to_string),
    })
}

fn ollama(slug: &str) -> UpsertConnection {
    UpsertConnection {
        slug: slug.into(),
        display_name: slug.into(),
        api_shape: "ollama".into(),
        base_url: "http://127.0.0.1:9".into(),
        locality: Some("local".into()),
        auth_scheme: None,
        api_key: None,
        credential_region: None,
        timeout_secs: None,
        allow_private_network: Some(true),
    }
}

#[cfg(feature = "postgres")]
fn short_slug(prefix: &str) -> String {
    let id = Uuid::new_v4().simple().to_string();
    format!("{prefix}-{}", &id[..8])
}

fn assert_code(err: ApiError, code: &str, status: StatusCode) {
    assert_eq!(err.code(), code, "{err}");
    assert_eq!(err.status_code(), status);
}

#[tokio::test]
async fn create_with_tenant_header_and_no_scope_is_422() {
    let err = create_connection(
        State(AppState::test_state()),
        principal(Role::Admin),
        tenant(),
        scope(None),
        Json(ollama("spec177-refused")),
    )
    .await
    .expect_err("tenant create");
    assert_code(
        err,
        "connection_scope_global",
        StatusCode::UNPROCESSABLE_ENTITY,
    );
}

#[tokio::test]
async fn scope_deployment_without_admin_role_is_403() {
    let state = AppState::test_state();
    let id = Uuid::nil();
    let create = create_connection(
        State(state.clone()),
        principal(Role::User),
        tenant(),
        scope(Some("deployment")),
        Json(ollama("spec177-member")),
    )
    .await
    .expect_err("create");
    assert_code(create, "deployment_admin_required", StatusCode::FORBIDDEN);

    let update = update_connection(
        State(state.clone()),
        principal(Role::User),
        tenant(),
        scope(Some("DEPLOYMENT")),
        Path(id),
        Json(ollama("spec177-member")),
    )
    .await
    .expect_err("update");
    assert_code(update, "deployment_admin_required", StatusCode::FORBIDDEN);

    let delete = delete_connection(
        State(state.clone()),
        principal(Role::User),
        tenant(),
        Query(DeleteConfirm {
            confirm: true,
            scope: Some("deployment".into()),
        }),
        Path(id),
    )
    .await
    .expect_err("delete");
    assert_code(delete, "deployment_admin_required", StatusCode::FORBIDDEN);

    let probe = test_stored_connection(
        State(state),
        principal(Role::User),
        tenant(),
        scope(Some("deployment")),
        Path(id),
    )
    .await
    .expect_err("test");
    assert_code(probe, "deployment_admin_required", StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn deployment_scope_is_not_rejected_as_a_tenant_write() {
    // No database: the handler must get past the scope gate and then miss Postgres.
    let err = create_connection(
        State(AppState::test_state()),
        principal(Role::Admin),
        tenant(),
        scope(Some("deployment")),
        Json(ollama("spec177-gate")),
    )
    .await
    .expect_err("no database");
    assert_ne!(err.code(), "connection_scope_global");
    assert_eq!(err.status_code(), StatusCode::SERVICE_UNAVAILABLE);
}

#[cfg(feature = "postgres")]
fn install_key() {
    let hex: String = [9u8; 32].iter().map(|byte| format!("{byte:02x}")).collect();
    std::env::set_var("EDGEQUAKE_SECRETS_KEY", hex);
    std::env::set_var("EDGEQUAKE_SECRETS_KEY_ID", "spec177");
}

#[cfg(feature = "postgres")]
fn service_account() -> String {
    r#"{"type":"service_account","client_email":"vertex-express@mobioos-506015.iam.gserviceaccount.com","private_key":"-----BEGIN PRIVATE KEY-----\nnot-a-real-key\n-----END PRIVATE KEY-----\n","project_id":"mobioos-506015","token_uri":"https://oauth2.googleapis.com/token"}"#.into()
}

#[cfg(feature = "postgres")]
#[tokio::test]
async fn deployment_scope_writes_a_fleet_row_and_seals_the_secret() {
    let Some(pool) =
        postgres_access_pool::test_pool_with_acquire_timeout(std::time::Duration::from_secs(15))
            .await
    else {
        return;
    };
    install_key();
    let state = AppState::test_state_with_pg_pool(pool.clone());
    let slug = short_slug("spec177");

    let (status, Json(created)) = create_connection(
        State(state.clone()),
        principal(Role::Admin),
        tenant(),
        scope(Some("deployment")),
        Json(ollama(&slug)),
    )
    .await
    .expect("fleet create");
    assert_eq!(status, StatusCode::CREATED);
    assert!(created.tenant_id.is_none());
    let id = created.id;

    let refused = update_connection(
        State(state.clone()),
        principal(Role::Admin),
        tenant(),
        scope(None),
        Path(id),
        Json(ollama(&slug)),
    )
    .await
    .expect_err("tenant update of a fleet row");
    assert_code(
        refused,
        "connection_scope_global",
        StatusCode::UNPROCESSABLE_ENTITY,
    );

    let Json(updated) = update_connection(
        State(state.clone()),
        principal(Role::Admin),
        tenant(),
        scope(Some("deployment")),
        Path(id),
        Json(ollama(&slug)),
    )
    .await
    .expect("deployment update");
    assert!(updated.tenant_id.is_none());

    let refused_test = test_stored_connection(
        State(state.clone()),
        principal(Role::Admin),
        tenant(),
        scope(None),
        Path(id),
    )
    .await
    .expect_err("tenant test of a fleet row");
    assert_code(
        refused_test,
        "connection_scope_global",
        StatusCode::UNPROCESSABLE_ENTITY,
    );

    let Json(probe) = test_stored_connection(
        State(state.clone()),
        principal(Role::Admin),
        tenant(),
        scope(Some("deployment")),
        Path(id),
    )
    .await
    .expect("deployment test");
    assert!(!probe.ok, "a closed port must not look connected");

    let refused_delete = delete_connection(
        State(state.clone()),
        principal(Role::Admin),
        tenant(),
        Query(DeleteConfirm {
            confirm: false,
            scope: None,
        }),
        Path(id),
    )
    .await
    .expect_err("tenant delete of a fleet row");
    assert_code(
        refused_delete,
        "connection_scope_global",
        StatusCode::UNPROCESSABLE_ENTITY,
    );

    let gone = delete_connection(
        State(state.clone()),
        principal(Role::Admin),
        tenant(),
        Query(DeleteConfirm {
            confirm: true,
            scope: Some("deployment".into()),
        }),
        Path(id),
    )
    .await
    .expect("deployment delete");
    assert_eq!(gone, StatusCode::NO_CONTENT);

    let vertex = UpsertConnection {
        slug: short_slug("spec177-vx"),
        display_name: "Vertex spec177".into(),
        api_shape: "vertexai".into(),
        base_url: "https://aiplatform.googleapis.com".into(),
        locality: Some("cloud".into()),
        auth_scheme: None,
        api_key: Some(service_account()),
        credential_region: Some("us-central1".into()),
        timeout_secs: None,
        allow_private_network: Some(false),
    };
    let (status, Json(saved)) = create_connection(
        State(state.clone()),
        principal(Role::Admin),
        tenant(),
        scope(Some("deployment")),
        Json(vertex),
    )
    .await
    .expect("vertex create");
    assert_eq!(status, StatusCode::CREATED);
    assert!(saved.tenant_id.is_none());
    assert!(saved.key_configured);
    assert_eq!(saved.secret_backend.as_deref(), Some("database"));
    assert_eq!(saved.auth_scheme, "google_service_account");
    let echoed = serde_json::to_string(&saved).expect("view");
    assert!(!echoed.contains("api_key"));
    assert!(!echoed.contains("BEGIN PRIVATE KEY"));
    assert!(!echoed.contains("not-a-real-key"));

    let secret_ref: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT secret_ref FROM provider_connections WHERE id = $1")
            .bind(saved.id)
            .fetch_one(&pool)
            .await
            .expect("secret ref");
    let ciphertext: Option<Vec<u8>> =
        sqlx::query_scalar("SELECT api_key_ciphertext FROM provider_connections WHERE id = $1")
            .bind(saved.id)
            .fetch_one(&pool)
            .await
            .expect("ciphertext");
    assert!(
        ciphertext.is_none(),
        "new keys are a reference, not a column"
    );
    let opened = edgequake_api::providers::secret_manager::reveal_connection_key(
        &pool, None, secret_ref, None, None, None,
    )
    .await
    .expect("reveal")
    .expect("material");
    assert!(opened.contains("BEGIN PRIVATE KEY"));

    let _ = delete_connection(
        State(state),
        principal(Role::Admin),
        TenantContext::default(),
        Query(DeleteConfirm {
            confirm: true,
            scope: Some("deployment".into()),
        }),
        Path(saved.id),
    )
    .await;
}
