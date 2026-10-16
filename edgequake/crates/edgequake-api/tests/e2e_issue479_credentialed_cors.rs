//! GitHub #479 — credentialed split-origin CORS.
//!
//! The WebUI sends `credentials: "include"`. Browsers reject
//! `Access-Control-Allow-Origin: *` and require
//! `Access-Control-Allow-Credentials: true` on the actual GET, not only OPTIONS.
//!
//! ```bash
//! cargo test -p edgequake-api --test e2e_issue479_credentialed_cors
//! ```

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use edgequake_api::{AppState, Server, ServerConfig};

const UI_ORIGIN: &str = "http://127.0.0.1:3000";
const OTHER_ORIGIN: &str = "http://127.0.0.1:3999";

fn router(mut state: AppState) -> axum::Router {
    let _ = &mut state;
    Server::new(
        ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            enable_cors: true,
            enable_compression: false,
            enable_swagger: false,
        },
        state,
    )
    .build_router()
}

fn open_dev_state() -> AppState {
    let mut state = AppState::test_state();
    state.security.cors_origins = None;
    state.security.cors_fail_closed = false;
    state
}

fn header<'a>(response: &'a axum::response::Response, name: &str) -> Option<&'a str> {
    response.headers().get(name).and_then(|v| v.to_str().ok())
}

async fn get(app: axum::Router, uri: &str, origin: &str) -> axum::response::Response {
    app.oneshot(
        Request::builder()
            .method("GET")
            .uri(uri)
            .header("Origin", origin)
            .header("Cookie", "eq_refresh=issue-479")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .expect("router must answer the credentialed GET")
}

fn assert_echoed_credentials(response: &axum::response::Response, origin: &str) {
    let allow_origin = header(response, "access-control-allow-origin");
    assert_eq!(
        allow_origin,
        Some(origin),
        "credentialed CORS must echo the request Origin"
    );
    assert_ne!(allow_origin, Some("*"));
    assert_eq!(
        header(response, "access-control-allow-credentials"),
        Some("true")
    );
}

#[tokio::test]
async fn issue_479_open_dev_get_health_echoes_origin_and_credentials() {
    let response = get(router(open_dev_state()), "/health", UI_ORIGIN).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_echoed_credentials(&response, UI_ORIGIN);
}

#[tokio::test]
async fn issue_479_open_dev_get_tenants_echoes_origin_and_credentials() {
    let response = get(router(open_dev_state()), "/api/v1/tenants", UI_ORIGIN).await;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "dev-mode tenant list is the request the dashboard picker makes"
    );
    assert_echoed_credentials(&response, UI_ORIGIN);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.get("items").is_some(), "tenant list body: {json}");
}

#[tokio::test]
async fn issue_479_wildcard_allow_list_is_not_sent() {
    let mut state = open_dev_state();
    state.security.cors_origins = Some(vec!["*".into()]);
    let response = get(router(state), "/api/v1/tenants", UI_ORIGIN).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_echoed_credentials(&response, UI_ORIGIN);
}

#[tokio::test]
async fn issue_479_fail_closed_without_allow_list_does_not_echo() {
    let mut state = AppState::test_state();
    state.security.cors_origins = None;
    state.security.cors_fail_closed = true;
    let response = get(router(state), "/api/v1/tenants", UI_ORIGIN).await;
    assert_ne!(
        header(&response, "access-control-allow-origin"),
        Some("*"),
        "strict mode must not fall open to a wildcard"
    );
    assert_ne!(
        header(&response, "access-control-allow-origin"),
        Some(UI_ORIGIN),
        "strict mode with no allow-list must not echo the request Origin"
    );
}

#[tokio::test]
async fn issue_479_fail_closed_allow_list_echoes_only_listed_origin() {
    let mut state = AppState::test_state();
    state.security.cors_origins = Some(vec![UI_ORIGIN.into(), "*".into()]);
    state.security.cors_fail_closed = true;

    let allowed = get(router(state.clone()), "/health", UI_ORIGIN).await;
    assert_eq!(allowed.status(), StatusCode::OK);
    assert_echoed_credentials(&allowed, UI_ORIGIN);

    let denied = get(router(state), "/health", OTHER_ORIGIN).await;
    assert_ne!(
        header(&denied, "access-control-allow-origin"),
        Some(OTHER_ORIGIN)
    );
    assert_ne!(header(&denied, "access-control-allow-origin"), Some("*"));
}
