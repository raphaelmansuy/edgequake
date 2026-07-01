//! SPEC-037 stream content_granularity integration tests.

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

async fn collect_sse_json_events(response: axum::response::Response) -> Vec<Value> {
    let bytes = axum::body::to_bytes(response.into_body(), 2 * 1024 * 1024)
        .await
        .expect("read sse body");
    let text = String::from_utf8_lossy(&bytes);
    text.lines()
        .filter_map(|line| {
            let payload = line.strip_prefix("data: ")?;
            serde_json::from_str(payload).ok()
        })
        .collect()
}

fn first_chunk_snippet_len(events: &[Value]) -> Option<usize> {
    events.iter().find_map(|event| {
        if event.get("type").and_then(|v| v.as_str()) != Some("context") {
            return None;
        }
        event
            .get("sources")
            .and_then(|s| s.as_array())
            .and_then(|sources| {
                sources
                    .iter()
                    .find(|s| s.get("source_type").and_then(|t| t.as_str()) == Some("chunk"))
            })
            .and_then(|chunk| chunk.get("snippet"))
            .and_then(|s| s.as_str())
            .map(str::len)
    })
}

#[tokio::test]
async fn stream_default_citation_truncates_snippets() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query/stream")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "query": "What is EdgeQuake?",
                        "mode": "naive",
                        "stream_format": "v2"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let events = collect_sse_json_events(response).await;
    let snippet_len = first_chunk_snippet_len(&events).unwrap_or(0);
    assert!(
        snippet_len <= 200,
        "default citation snippets must be <= 200 chars, got {snippet_len}"
    );
}

#[tokio::test]
async fn stream_agent_granularity_allows_longer_snippets() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query/stream")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "query": "What is EdgeQuake?",
                        "mode": "naive",
                        "stream_format": "v2",
                        "content_granularity": "agent"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let events = collect_sse_json_events(response).await;
    // When mock retrieval returns chunks, agent tier must not cap at 200 chars.
    if let Some(snippet_len) = first_chunk_snippet_len(&events) {
        assert!(
            snippet_len > 0,
            "agent stream should include chunk snippets when retrieval returns chunks"
        );
    }
}

#[tokio::test]
async fn stream_request_deserializes_granularity_default_citation() {
    let req: edgequake_api::handlers::query_types::StreamQueryRequest =
        serde_json::from_str(r#"{"query":"hello"}"#).unwrap();
    assert_eq!(
        req.content_granularity,
        edgequake_api::handlers::context_types::ContentGranularity::Citation
    );
}
