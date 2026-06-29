//! SPEC-027 OIDC HTTP E2E — mock IdP via wiremock (phase 54b).

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use edgequake_api::services::oidc_flow::OidcFlowService;
use edgequake_api::{AppState, Server, ServerConfig};
use edgequake_auth::OidcConfig;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde_json::{json, Value};
use tower::ServiceExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn server_config() -> ServerConfig {
    ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: false,
    }
}

fn build_app(state: AppState) -> axum::Router {
    Server::new(server_config(), state).build_router()
}

async fn parse_json(response: axum::response::Response) -> Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body)
        .unwrap_or_else(|_| json!({ "raw": String::from_utf8_lossy(&body) }))
}

fn enable_oidc_on_state(
    state: &mut AppState,
    issuer_url: &str,
    redirect_uri: &str,
    client_secret: &str,
) {
    let oidc_config = OidcConfig {
        enabled: true,
        issuer_url: issuer_url.to_string(),
        client_id: "spec027-oidc-client".into(),
        client_secret: Some(client_secret.into()),
        redirect_uri: redirect_uri.to_string(),
        success_redirect_url: None,
    };
    state.auth.oidc_config = oidc_config.clone();
    state.auth.oidc_service = Some(Arc::new(OidcFlowService::new(oidc_config)));
}

async fn mount_oidc_discovery(mock: &MockServer, signing_alg: &str) {
    let issuer = mock.uri();
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_json(json!({
                "issuer": issuer,
                "authorization_endpoint": format!("{issuer}/authorize"),
                "token_endpoint": format!("{issuer}/token"),
                "jwks_uri": format!("{issuer}/jwks"),
                "response_types_supported": ["code"],
                "subject_types_supported": ["public"],
                "id_token_signing_alg_values_supported": [signing_alg],
                "scopes_supported": ["openid", "email"],
                "claims_supported": ["sub", "iss", "aud", "exp", "iat", "nonce", "email"],
                "token_endpoint_auth_methods_supported": ["client_secret_post", "client_secret_basic"],
            })),
        )
        .mount(mock)
        .await;

    Mock::given(method("GET"))
        .and(path("/jwks"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_json(json!({ "keys": [] })),
        )
        .mount(mock)
        .await;
}

fn sign_hs256_id_token(
    issuer: &str,
    client_id: &str,
    client_secret: &str,
    nonce: &str,
    subject: &str,
    email: &str,
) -> String {
    let now = chrono::Utc::now().timestamp();
    let claims = json!({
        "iss": issuer,
        "sub": subject,
        "aud": client_id,
        "nonce": nonce,
        "email": email,
        "preferred_username": "oidc_user",
        "exp": now + 3600,
        "iat": now,
    });
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(client_secret.as_bytes()),
    )
    .expect("id_token encode")
}

#[tokio::test]
async fn spec027_oidc_disabled_login_returns_503() {
    let app = build_app(AppState::test_state());
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/oidc/login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn spec027_oidc_disabled_callback_returns_503() {
    let app = build_app(AppState::test_state());
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/oidc/callback?code=abc&state=xyz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn spec027_oidc_login_redirects_to_idp_when_enabled() {
    let mock = MockServer::start().await;
    mount_oidc_discovery(&mock, "HS256").await;

    let redirect_uri = "http://localhost/api/v1/auth/oidc/callback";
    let mut state = AppState::test_state();
    enable_oidc_on_state(&mut state, &mock.uri(), redirect_uri, "spec027-secret");
    let app = build_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/oidc/login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = response
        .headers()
        .get(header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .expect("Location header");
    assert!(location.contains("/authorize"));
    assert!(location.contains("client_id=spec027-oidc-client"));
    assert!(location.contains("code_challenge="));
}

#[tokio::test]
async fn spec027_oidc_callback_missing_pending_returns_401() {
    let mock = MockServer::start().await;
    mount_oidc_discovery(&mock, "HS256").await;

    let redirect_uri = "http://localhost/api/v1/auth/oidc/callback";
    let mut state = AppState::test_state();
    enable_oidc_on_state(&mut state, &mock.uri(), redirect_uri, "spec027-secret");
    let app = build_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/oidc/callback?code=unused&state=missing-state")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn spec027_oidc_callback_roundtrip_issues_jwt() {
    let mock = MockServer::start().await;
    mount_oidc_discovery(&mock, "HS256").await;

    let client_secret = "spec027-secret";
    let redirect_uri = "http://localhost/api/v1/auth/oidc/callback";
    let mut state = AppState::test_state();
    enable_oidc_on_state(&mut state, &mock.uri(), redirect_uri, client_secret);
    let app = build_app(state.clone());

    let login_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/oidc/login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login_response.status(), StatusCode::SEE_OTHER);

    let location = login_response
        .headers()
        .get(header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .expect("Location")
        .to_string();

    let parsed = url::Url::parse(&location).expect("authorize url");
    let state_param = parsed
        .query_pairs()
        .find(|(k, _)| k == "state")
        .map(|(_, v)| v.to_string())
        .expect("state in authorize url");
    let nonce = parsed
        .query_pairs()
        .find(|(k, _)| k == "nonce")
        .map(|(_, v)| v.to_string())
        .expect("nonce in authorize url");

    let id_token = sign_hs256_id_token(
        &mock.uri(),
        "spec027-oidc-client",
        client_secret,
        &nonce,
        "oidc-subject-1",
        "oidc-roundtrip@edgequake.test",
    );

    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "provider-access",
            "token_type": "Bearer",
            "id_token": id_token,
        })))
        .mount(&mock)
        .await;

    let callback_uri = format!(
        "/api/v1/auth/oidc/callback?code=spec027-code&state={}",
        urlencoding::encode(&state_param)
    );
    let app = build_app(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&callback_uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await;
    assert!(body["access_token"]
        .as_str()
        .map(|s| !s.is_empty())
        .unwrap_or(false));
    assert_eq!(body["token_type"], "Bearer");
    assert!(body["refresh_token"]
        .as_str()
        .map(|s| !s.is_empty())
        .unwrap_or(false));
    assert_eq!(body["user"]["email"], "oidc-roundtrip@edgequake.test");
}

#[tokio::test]
async fn spec027_v2_jobs_reject_unauthenticated_when_auth_enabled() {
    let mut state = AppState::test_state();
    state.auth.config.auth_enabled = true;
    state.auth.config.dev_mode = false;
    let app = build_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v2/workspaces/default/jobs/catalog")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn spec027_health_reports_oidc_enabled_when_configured() {
    let mock = MockServer::start().await;
    let redirect_uri = "http://localhost/api/v1/auth/oidc/callback";
    let mut state = AppState::test_state();
    enable_oidc_on_state(&mut state, &mock.uri(), redirect_uri, "spec027-secret");
    let app = build_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await;
    let caps = body["capabilities"].as_object().expect("capabilities");
    assert_eq!(caps["oauth2_oidc_builtin"], true);
    assert_eq!(caps["external_sso_pattern"], "builtin-oidc");
    let mechanisms = caps["auth_mechanisms"].as_array().expect("mechanisms");
    assert!(mechanisms.iter().any(|m| m == "oidc"));
}
