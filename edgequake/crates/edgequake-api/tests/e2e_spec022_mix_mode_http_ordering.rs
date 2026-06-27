//! SPEC-022 P-H6 — HTTP Mix mode weight ordering mirrors engine contract.

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

async fn seed_mix_fixture(state: &AppState) {
    let dim = 1536;
    let vector = state.storage.vector_storage.clone();
    vector.initialize().await.unwrap();

    let naive_top = vec![1.0_f32; dim];
    vector
        .upsert(&[(
            "chunk_naive_top".to_string(),
            naive_top,
            json!({
                "type": "chunk",
                "content": "naive top content",
                "document_id": "doc-naive",
            }),
        )])
        .await
        .unwrap();

    let mut kg_top = vec![0.0_f32; dim];
    kg_top[1] = 0.95;
    vector
        .upsert(&[(
            "chunk_kg_top".to_string(),
            kg_top,
            json!({
                "type": "chunk",
                "content": "kg top content",
                "document_id": "doc-kg",
            }),
        )])
        .await
        .unwrap();

    vector
        .upsert(&[(
            "entity:KG_ENTITY".to_string(),
            vec![1.0_f32; dim],
            json!({
                "type": "entity",
                "entity_name": "KG_ENTITY",
                "source_chunk_ids": ["chunk_kg_top"],
            }),
        )])
        .await
        .unwrap();

    let graph = state.storage.graph_storage.clone();
    graph.initialize().await.unwrap();
    let mut props = HashMap::new();
    props.insert("entity_type".to_string(), json!("CONCEPT"));
    props.insert("description".to_string(), json!("kg entity"));
    props.insert("source_chunk_ids".to_string(), json!(["chunk_kg_top"]));
    graph.upsert_node("KG_ENTITY", props).await.unwrap();
}

async fn post_mix_query(app: &axum::Router, mix_weights: Value) -> Value {
    let body = json!({
        "query": "kg entity",
        "mode": "mix",
        "context_only": true,
        "enable_rerank": false,
        "mix_weights": mix_weights,
    });

    let response = app
        .clone()
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
    extract_json(response).await
}

#[tokio::test]
async fn spec022_http_mix_skewed_weights_change_chunk_ordering() {
    let state = AppState::test_state();
    seed_mix_fixture(&state).await;
    let app = Server::new(test_server_config(), state).build_router();

    let naive_only =
        post_mix_query(&app, json!({ "local": 0.0, "global": 0.0, "naive": 1.0 })).await;
    let local_only =
        post_mix_query(&app, json!({ "local": 1.0, "global": 0.0, "naive": 0.0 })).await;

    assert_eq!(naive_only["mode"].as_str(), Some("mix"));
    assert_eq!(local_only["mode"].as_str(), Some("mix"));

    let n_ids = chunk_source_ids(&naive_only);
    let l_ids = chunk_source_ids(&local_only);
    assert!(
        !n_ids.is_empty() && !l_ids.is_empty(),
        "HTTP mix must return chunk sources for both weight skews"
    );
    assert_ne!(
        n_ids, l_ids,
        "HTTP mix must be weight-sensitive: naive-only vs local-only ordering must differ"
    );
}

#[tokio::test]
async fn spec022_http_mix_equal_weights_matches_hybrid_chunk_set() {
    let state = AppState::test_state();
    seed_mix_fixture(&state).await;
    let app = Server::new(test_server_config(), state).build_router();

    let hybrid_body = json!({
        "query": "kg entity naive top",
        "mode": "hybrid",
        "context_only": true,
        "enable_rerank": false,
    });
    let hybrid_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from(hybrid_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(hybrid_resp.status(), StatusCode::OK);
    let hybrid = extract_json(hybrid_resp).await;

    let mix = post_mix_query(&app, json!({ "local": 1.0, "global": 1.0, "naive": 1.0 })).await;

    let h_set: std::collections::HashSet<_> = chunk_source_ids(&hybrid).into_iter().collect();
    let m_set: std::collections::HashSet<_> = chunk_source_ids(&mix).into_iter().collect();
    assert_eq!(
        h_set, m_set,
        "equal-weight HTTP mix must return the same chunk set as hybrid"
    );
}

#[test]
fn spec022_query_execute_wires_mix_weights() {
    let src = include_str!("../src/handlers/query/query_execute.rs");
    assert!(
        src.contains("mix_weights"),
        "HTTP query handler must forward mix_weights to engine request"
    );
}
