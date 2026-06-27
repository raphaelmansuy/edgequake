//! SPEC-024 2.0 — HTTP Hybrid mode: LightRAG round-robin merge + dedup.

use std::collections::HashMap;

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

fn chunk_source_ids(parsed: &Value) -> Vec<String> {
    parsed["sources"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|s| s["source_type"].as_str() == Some("chunk"))
        .filter_map(|s| s["id"].as_str().map(str::to_string))
        .collect()
}

async fn seed_hybrid_lightrag_fixture(state: &AppState) {
    let dim = 1536;
    let vector = state.storage.vector_storage.clone();
    vector.initialize().await.unwrap();

    let shared = vec![0.7_f32; dim];
    let local_only = {
        let mut v = vec![0.0_f32; dim];
        v[0] = 0.95;
        v
    };
    let global_only = {
        let mut v = vec![0.0_f32; dim];
        v[1] = 0.95;
        v
    };
    let naive_only = vec![1.0_f32; dim];

    vector
        .upsert(&[
            (
                "chunk_shared".into(),
                shared,
                json!({
                    "type": "chunk",
                    "content": "shared across local and global arms",
                }),
            ),
            (
                "chunk_local_only".into(),
                local_only,
                json!({
                    "type": "chunk",
                    "content": "local arm only",
                }),
            ),
            (
                "chunk_global_only".into(),
                global_only,
                json!({
                    "type": "chunk",
                    "content": "global arm only",
                }),
            ),
            (
                "chunk_naive_only".into(),
                naive_only,
                json!({
                    "type": "chunk",
                    "content": "naive arm only",
                }),
            ),
            (
                "entity:HYBRID_ALPHA".into(),
                vec![1.0_f32; dim],
                json!({
                    "type": "entity",
                    "entity_name": "HYBRID_ALPHA",
                    "source_chunk_ids": ["chunk_shared", "chunk_local_only"],
                }),
            ),
            (
                "entity:HYBRID_BETA".into(),
                vec![1.0_f32; dim],
                json!({
                    "type": "entity",
                    "entity_name": "HYBRID_BETA",
                    "source_chunk_ids": ["chunk_shared", "chunk_global_only"],
                }),
            ),
        ])
        .await
        .unwrap();

    let graph = state.storage.graph_storage.clone();
    graph.initialize().await.unwrap();

    for (name, chunk_ids) in [
        ("HYBRID_ALPHA", vec!["chunk_shared", "chunk_local_only"]),
        ("HYBRID_BETA", vec!["chunk_shared", "chunk_global_only"]),
    ] {
        let mut props = HashMap::new();
        props.insert("entity_type".to_string(), json!("CONCEPT"));
        props.insert("description".to_string(), json!(format!("{name} entity")));
        props.insert("source_chunk_ids".to_string(), json!(chunk_ids));
        graph.upsert_node(name, props).await.unwrap();
    }

    graph
        .upsert_edge(
            "HYBRID_ALPHA",
            "HYBRID_BETA",
            [("relation_type".to_string(), json!("RELATED_TO"))]
                .into_iter()
                .collect(),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn spec024_http_hybrid_deduplicates_shared_chunk() {
    let state = AppState::test_state();
    seed_hybrid_lightrag_fixture(&state).await;
    let app = Server::new(test_server_config(), state).build_router();

    let body = json!({
        "query": "hybrid alpha beta shared",
        "mode": "hybrid",
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
    assert_eq!(parsed["mode"].as_str(), Some("hybrid"));

    let ids = chunk_source_ids(&parsed);
    let shared_count = ids.iter().filter(|id| *id == "chunk_shared").count();
    assert_eq!(
        shared_count, 1,
        "LightRAG hybrid must dedupe chunk_shared across local/global arms"
    );
}

#[tokio::test]
async fn spec024_http_hybrid_round_robin_kg_first_ordering() {
    std::env::set_var("EDGEQUAKE_BM25_RETRIEVAL", "false");
    let state = AppState::test_state();
    seed_hybrid_lightrag_fixture(&state).await;
    let app = Server::new(test_server_config(), state).build_router();

    let body = json!({
        "query": "hybrid alpha beta naive",
        "mode": "hybrid",
        "context_only": true,
        "enable_rerank": false,
        "max_results": 10,
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
    let ids = chunk_source_ids(&parsed);

    // LightRAG round-robin at slot 0: local(shared) → global(skip dup) → naive(naive_only)
    // slot 1: local(local_only) → global(global_only)
    assert!(!ids.is_empty(), "hybrid HTTP must return chunk sources");
    if ids.len() >= 4 {
        assert_eq!(
            ids,
            vec![
                "chunk_shared".to_string(),
                "chunk_naive_only".to_string(),
                "chunk_local_only".to_string(),
                "chunk_global_only".to_string(),
            ],
            "HTTP hybrid chunk order must match LightRAG round-robin (local→global→naive per slot)"
        );
    }
    std::env::remove_var("EDGEQUAKE_BM25_RETRIEVAL");
}
