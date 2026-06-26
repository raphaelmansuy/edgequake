//! SPEC-021 P-G8-http — Bypass + Mix via HTTP `POST /api/v1/query`.

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
            enable_swagger: true,
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
async fn spec021_http_bypass_returns_direct_answer_not_apology() {
    let app = create_test_app();
    let body = json!({
        "query": "What is the meaning of life?",
        "mode": "bypass"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let parsed = extract_json(response).await;
    let answer = parsed["answer"].as_str().unwrap_or("");
    assert!(
        !answer.contains("couldn't find any relevant information"),
        "HTTP bypass must not return RAG apology: {answer}"
    );
    assert_eq!(parsed["mode"].as_str(), Some("bypass"));
}

#[tokio::test]
async fn spec021_http_mix_mode_accepted() {
    let app = create_test_app();
    let body = json!({
        "query": "edgequake knowledge graph",
        "mode": "mix",
        "context_only": true
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let parsed = extract_json(response).await;
    assert_eq!(parsed["mode"].as_str(), Some("mix"));
    assert!(
        parsed.get("stats").is_some(),
        "HTTP mix must return stats (retrieval ran); keys={:?}",
        parsed.as_object().map(|m| m.keys().collect::<Vec<_>>())
    );
}
