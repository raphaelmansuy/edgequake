//! SPEC-028 Query Context Service E2E tests.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use serde_json::{json, Value};
use tower::ServiceExt;

fn create_test_app() -> axum::Router {
    Server::new(
        ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            enable_cors: false,
            enable_compression: false,
            enable_swagger: false,
        },
        AppState::test_state(),
    )
    .build_router()
}

async fn extract_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("read body");
    serde_json::from_slice(&bytes).expect("parse json")
}

#[tokio::test]
async fn ec_empty_query_returns_422() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query/context")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"query": ""}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn ec_context_retrieve_returns_bundle() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query/context")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "query": "What is EdgeQuake?",
                        "mode": "naive",
                        "content_granularity": "agent"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    assert!(body.get("retrieval_id").is_some());
    assert!(body.get("bundle").is_some());
    assert!(body["bundle"].get("subgraph").is_some());
    assert!(body["bundle"].get("chunks").is_some());
    assert!(body.get("retrieval_quality").is_some());
    assert!(body.get("retrieval_fingerprint").is_some());
}

#[tokio::test]
async fn ec_bypass_mode_rejected() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query/context")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"query": "hello", "mode": "bypass"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn ec_search_then_fetch_roundtrip() {
    let app = create_test_app();

    let search_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query/context/search")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"query": "machine learning basics", "mode": "naive"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(search_resp.status(), StatusCode::OK);
    let search_body = extract_json(search_resp).await;
    let retrieval_id = search_body["results"][0]["retrieval_id"]
        .as_str()
        .expect("retrieval_id");

    let fetch_resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/query/context/{}", retrieval_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(fetch_resp.status(), StatusCode::OK);
    let fetch_body = extract_json(fetch_resp).await;
    assert_eq!(fetch_body["retrieval_id"], retrieval_id);
    assert_eq!(fetch_body["cached"], true);
}

#[tokio::test]
async fn ec_context_only_parity_with_legacy_query() {
    let app = create_test_app();
    let query_text = "What is retrieval augmented generation?";

    let context_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query/context")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "query": query_text,
                        "mode": "naive",
                        "content_granularity": "citation"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(context_resp.status(), StatusCode::OK);

    let legacy_resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "query": query_text,
                        "mode": "naive",
                        "context_only": true
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(legacy_resp.status(), StatusCode::OK);

    let context_body = extract_json(context_resp).await;
    let legacy_body = extract_json(legacy_resp).await;

    assert_eq!(legacy_body["answer"].as_str().unwrap(), "");
    assert_eq!(
        context_body["bundle"]["chunks"].as_array().map(|a| a.len()),
        legacy_body["sources"]
            .as_array()
            .map(|s| s.iter().filter(|x| x["source_type"] == "chunk").count())
    );
}

#[tokio::test]
async fn ec_invalid_retrieval_id_returns_400() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/query/context/not-a-valid-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn ec_retrieval_response_includes_agent_hints_when_requested() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query/context")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "query": "graph retrieval test",
                        "mode": "naive",
                        "include_agent_hints": true
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    assert!(body.get("agent_hints").is_some());
}

#[tokio::test]
async fn ec_truncation_metadata_present() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query/context")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"query": "truncation fields", "mode": "naive"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    assert!(body.get("truncation").is_some());
}

#[tokio::test]
async fn ec_unknown_retrieval_id_returns_404() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/query/context/ret_00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
