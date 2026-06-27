//! SPEC-024 2.5 — HTTP query hydrates chunk text from KV when vector metadata uses content_ref.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use serde_json::{json, Value};
use tower::ServiceExt;

fn test_server_config() -> ServerConfig {
    ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: true,
    }
}

async fn extract_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("read body");
    serde_json::from_slice(&bytes).expect("parse json")
}

#[tokio::test]
async fn spec024_http_naive_query_hydrates_chunk_from_kv() {
    let state = AppState::test_state();
    let dim = 1536;
    let chunk_id = "doc-hydrate-chunk-0";
    let kv_text = "Authoritative body stored only in KV for SPEC-024 dedupe";

    state
        .storage
        .kv_storage
        .upsert(&[(
            chunk_id.to_string(),
            json!({
                "content": kv_text,
                "document_id": "doc-hydrate",
                "index": 0,
            }),
        )])
        .await
        .unwrap();

    let vector = state.storage.vector_storage.clone();
    vector.initialize().await.unwrap();
    vector
        .upsert(&[(
            chunk_id.to_string(),
            vec![1.0_f32; dim],
            json!({
                "type": "chunk",
                "document_id": "doc-hydrate",
                "content_ref": chunk_id,
            }),
        )])
        .await
        .unwrap();

    let app = Server::new(test_server_config(), state).build_router();

    let body = json!({
        "query": "hydrate dedupe authoritative",
        "mode": "naive",
        "context_only": true,
        "enable_rerank": false,
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

    let chunk_sources: Vec<_> = parsed["sources"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter(|s| s["source_type"].as_str() == Some("chunk"))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    assert!(
        !chunk_sources.is_empty(),
        "naive query must return chunk sources"
    );

    let snippet = chunk_sources[0]["snippet"].as_str().unwrap_or_default();
    assert!(
        snippet.contains("Authoritative body stored only in KV"),
        "HTTP snippet must be hydrated from KV when metadata omits inline content; got: {snippet}"
    );
}
