//! SPEC-018 observability proof tests.

use axum::http::HeaderMap;
use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::get,
    Router,
};
use edgequake_observability::{
    init_metrics, parse_trace_id_from_traceparent, resolve_request_id, trace_id_from_request_id,
    REQUEST_ID_HEADER, TRACEPARENT_HEADER,
};
use tower::ServiceExt;

#[tokio::test]
async fn spec018_honors_inbound_request_id_header() {
    init_metrics();
    let app = Router::new()
        .route("/health", get(|| async { (StatusCode::OK, "ok") }))
        .layer(axum::middleware::from_fn(
            edgequake_api::observability_middleware::observability_middleware,
        ));

    let inbound = "550e8400-e29b-41d4-a716-446655440000";
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .header(REQUEST_ID_HEADER, inbound)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let header = response
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert_eq!(header, inbound);
}

#[tokio::test]
async fn spec018_api_error_includes_explicit_details() {
    use axum::{response::IntoResponse, routing::get, Router};
    use edgequake_api::error::ApiError;
    use tower::ServiceExt;

    let app = Router::new()
        .route(
            "/fail",
            get(|| async { ApiError::NotFound("doc-x".into()).into_response() }),
        )
        .layer(axum::middleware::from_fn(
            edgequake_api::observability_middleware::observability_middleware,
        ));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/fail")
                .header(REQUEST_ID_HEADER, "proof-req-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["code"], "NOT_FOUND");
    let details = &json["details"];
    assert_eq!(details["request_id"], "proof-req-id");
    assert_eq!(details["error_code"], "NOT_FOUND");
    assert!(details["diagnostics"].is_object());
}

#[tokio::test]
async fn spec018_synthesizes_traceparent_from_request_id() {
    init_metrics();
    let app = Router::new()
        .route("/health", get(|| async { (StatusCode::OK, "ok") }))
        .layer(axum::middleware::from_fn(
            edgequake_api::observability_middleware::observability_middleware,
        ));

    let inbound = "550e8400-e29b-41d4-a716-446655440000";
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .header(REQUEST_ID_HEADER, inbound)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let traceparent = response
        .headers()
        .get(TRACEPARENT_HEADER)
        .and_then(|v| v.to_str().ok())
        .expect("traceparent response header");
    assert_eq!(
        parse_trace_id_from_traceparent(traceparent),
        trace_id_from_request_id(inbound)
    );
}

#[tokio::test]
async fn spec018_server_error_includes_retryable_in_details() {
    use axum::{response::IntoResponse, routing::get, Router};
    use edgequake_api::error::ApiError;
    use tower::ServiceExt;

    let app = Router::new()
        .route(
            "/boom",
            get(|| async { ApiError::Internal("db down".into()).into_response() }),
        )
        .layer(axum::middleware::from_fn(
            edgequake_api::observability_middleware::observability_middleware,
        ));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/boom")
                .header(REQUEST_ID_HEADER, "proof-500")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["code"], "INTERNAL_ERROR");
    let details = &json["details"];
    assert_eq!(details["request_id"], "proof-500");
    assert_eq!(details["http_status"], 500);
    assert_eq!(details["retryable"], false);
    assert!(details["diagnostics"].is_object());
}

#[tokio::test]
async fn spec018_storage_error_includes_category_in_details() {
    use axum::{response::IntoResponse, routing::get, Router};
    use edgequake_api::error::ApiError;
    use edgequake_storage::error::StorageError;
    use tower::ServiceExt;

    let app = Router::new()
        .route(
            "/storage-fail",
            get(|| async {
                ApiError::Storage(StorageError::Connection("refused".into())).into_response()
            }),
        )
        .layer(axum::middleware::from_fn(
            edgequake_api::observability_middleware::observability_middleware,
        ));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/storage-fail")
                .header(REQUEST_ID_HEADER, "proof-storage")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["code"], "STORAGE_ERROR");
    let details = &json["details"];
    assert_eq!(details["request_id"], "proof-storage");
    assert_eq!(details["diagnostics"]["category"], "connection");
    assert_eq!(details["retryable"], true);
}

#[tokio::test]
async fn spec018_pipeline_error_includes_structured_diagnostics() {
    use axum::{response::IntoResponse, routing::get, Router};
    use edgequake_api::error::ApiError;
    use edgequake_pipeline::error::PipelineError;
    use tower::ServiceExt;

    let app = Router::new()
        .route(
            "/pipeline-fail",
            get(|| async {
                ApiError::Pipeline(PipelineError::CircuitBreakerOpen {
                    failures: 5,
                    retry_after_secs: 30,
                })
                .into_response()
            }),
        )
        .layer(axum::middleware::from_fn(
            edgequake_api::observability_middleware::observability_middleware,
        ));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/pipeline-fail")
                .header(REQUEST_ID_HEADER, "proof-pipeline")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["code"], "PIPELINE_ERROR");
    let diag = &json["details"]["diagnostics"];
    assert_eq!(diag["category"], "circuit_breaker_open");
    assert_eq!(diag["retryable"], true);
    assert_eq!(diag["failures"], 5);
}

#[tokio::test]
async fn spec018_auth_failure_includes_action_and_reason() {
    use axum::{response::IntoResponse, routing::get, Router};
    use edgequake_api::error::ApiError;
    use tower::ServiceExt;

    let app = Router::new()
        .route(
            "/auth-fail",
            get(|| async {
                ApiError::auth_unauthorized("login", "invalid_password", Some("alice"))
                    .into_response()
            }),
        )
        .layer(axum::middleware::from_fn(
            edgequake_api::observability_middleware::observability_middleware,
        ));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/auth-fail")
                .header(REQUEST_ID_HEADER, "proof-auth")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["code"], "UNAUTHORIZED");
    let diag = &json["details"]["diagnostics"];
    assert_eq!(diag["action"], "login");
    assert_eq!(diag["reason"], "invalid_password");
    assert_eq!(diag["subject"], "alice");
    assert_eq!(json["details"]["source"], "auth");
}

#[test]
fn spec018_resolve_request_id_unit() {
    let mut headers = HeaderMap::new();
    headers.insert(
        REQUEST_ID_HEADER,
        axum::http::HeaderValue::from_static("client-req-99"),
    );
    assert_eq!(resolve_request_id(&headers), "client-req-99");
}
