//! GitHub #277 E2E — CORS + WebSocket in production auth mode (`EDGEQUAKE_DEV_MODE=false`).
//!
//! ```bash
//! cargo test -p edgequake-api --test e2e_issue277_cors_production
//! ```

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use edgequake_api::{AppState, Server, ServerConfig};

const ALLOWED_ORIGIN: &str = "https://app.example.com";
const API_KEY: &str = "issue-277-test-key";

fn production_cors_state() -> AppState {
    let mut state = AppState::test_state();
    state.auth.config.auth_enabled = true;
    state.auth.config.dev_mode = false;
    state.auth.config.api_keys = vec![API_KEY.to_string()];
    state.security.cors_origins = Some(vec![ALLOWED_ORIGIN.to_string()]);
    state
}

fn build_production_cors_app(state: AppState) -> axum::Router {
    Server::new(
        ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            enable_cors: true,
            enable_compression: false,
            enable_swagger: true,
        },
        state,
    )
    .build_router()
}

fn cors_header<'a>(response: &'a axum::response::Response, name: &str) -> Option<&'a str> {
    response.headers().get(name).and_then(|v| v.to_str().ok())
}

#[tokio::test]
async fn issue_277_options_preflight_returns_allowed_origin() {
    let app = build_production_cors_app(production_cors_state());

    let response = app
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/api/v1/documents")
                .header("Origin", ALLOWED_ORIGIN)
                .header("Access-Control-Request-Method", "POST")
                .header(
                    "Access-Control-Request-Headers",
                    "authorization,content-type",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "preflight must not be blocked by auth middleware"
    );
    assert_eq!(
        cors_header(&response, "access-control-allow-origin"),
        Some(ALLOWED_ORIGIN)
    );
}

#[tokio::test]
async fn issue_277_openapi_json_includes_cors_for_allowed_origin() {
    let app = build_production_cors_app(production_cors_state());

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api-docs/openapi.json")
                .header("Origin", ALLOWED_ORIGIN)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        cors_header(&response, "access-control-allow-origin"),
        Some(ALLOWED_ORIGIN),
        "Swagger merge must be inside CORS layer (GitHub #277)"
    );
}

#[tokio::test]
async fn issue_277_websocket_auth_accepts_query_token() {
    let state = production_cors_state();
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("origin", ALLOWED_ORIGIN.parse().unwrap());

    assert!(edgequake_api::middleware::ws_validate_origin(&state, &headers).is_ok());
    assert!(
        edgequake_api::middleware::ws_validate_token(&state, Some(API_KEY)).await,
        "production mode must accept API key via ?token= query param"
    );
}

#[tokio::test]
async fn issue_277_websocket_auth_accepts_bearer_token_value() {
    let state = production_cors_state();
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("origin", ALLOWED_ORIGIN.parse().unwrap());

    assert!(edgequake_api::middleware::ws_validate_origin(&state, &headers).is_ok());
    assert!(
        edgequake_api::middleware::ws_validate_token(&state, Some(API_KEY)).await,
        "same token validated whether from ?token= or Authorization bearer"
    );
}

#[tokio::test]
async fn issue_277_websocket_auth_rejects_missing_token() {
    let state = production_cors_state();
    assert!(
        !edgequake_api::middleware::ws_validate_token(&state, None).await,
        "missing token must fail when auth enabled and dev_mode=false"
    );
}

#[tokio::test]
async fn issue_277_websocket_origin_rejects_disallowed() {
    let state = production_cors_state();
    let mut denied = axum::http::HeaderMap::new();
    denied.insert("origin", "https://evil.example.com".parse().unwrap());
    assert_eq!(
        edgequake_api::middleware::ws_validate_origin(&state, &denied),
        Err(StatusCode::FORBIDDEN)
    );
}

#[test]
fn issue_277_ws_validate_origin_unit() {
    let state = production_cors_state();
    let mut allowed = axum::http::HeaderMap::new();
    allowed.insert("origin", ALLOWED_ORIGIN.parse().unwrap());
    assert!(edgequake_api::middleware::ws_validate_origin(&state, &allowed).is_ok());

    let mut denied = axum::http::HeaderMap::new();
    denied.insert("origin", "https://evil.example.com".parse().unwrap());
    assert_eq!(
        edgequake_api::middleware::ws_validate_origin(&state, &denied),
        Err(StatusCode::FORBIDDEN)
    );
}
