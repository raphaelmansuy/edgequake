//! End-to-end tests for query API endpoints.
//!
//! Tests cover:
//! - Execute query (POST /api/v1/query)
//! - Streaming query (POST /api/v1/query/stream)

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use serde_json::{json, Value};
use tower::ServiceExt;

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_config() -> ServerConfig {
    ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: true,
    }
}

fn create_test_server() -> Server {
    Server::new(create_test_config(), AppState::test_state())
}

fn create_test_app() -> axum::Router {
    create_test_server().build_router()
}

async fn extract_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("Failed to read response body");
    serde_json::from_slice(&bytes).expect("Failed to parse JSON")
}

async fn upload_document(server: &Server, content: &str) -> String {
    let request = json!({
        "content": content
    });

    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = extract_json(response).await;
    body.get("document_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string()
}

// ============================================================================
// Execute Query Tests
// ============================================================================

#[tokio::test]
async fn test_query_empty() {
    let app = create_test_app();

    let request = json!({
        "query": ""
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Empty query should fail validation
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_query_simple() {
    let app = create_test_app();

    let request = json!({
        "query": "What is machine learning?"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    assert!(body.get("answer").is_some());
    assert!(body.get("mode").is_some());
    assert!(body.get("sources").is_some());
    assert!(body.get("stats").is_some());
}

#[tokio::test]
async fn test_query_with_mode_naive() {
    let app = create_test_app();

    let request = json!({
        "query": "Tell me about artificial intelligence",
        "mode": "naive"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    assert_eq!(body.get("mode").and_then(|v| v.as_str()), Some("naive"));
}

#[tokio::test]
async fn test_query_with_mode_local() {
    let app = create_test_app();

    let request = json!({
        "query": "What is quantum computing?",
        "mode": "local"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    assert_eq!(body.get("mode").and_then(|v| v.as_str()), Some("local"));
}

#[tokio::test]
async fn test_query_with_mode_global() {
    let app = create_test_app();

    let request = json!({
        "query": "How do neural networks work?",
        "mode": "global"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    assert_eq!(body.get("mode").and_then(|v| v.as_str()), Some("global"));
}

#[tokio::test]
async fn test_query_with_mode_hybrid() {
    let app = create_test_app();

    let request = json!({
        "query": "Explain deep learning",
        "mode": "hybrid"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    assert_eq!(body.get("mode").and_then(|v| v.as_str()), Some("hybrid"));
}

#[tokio::test]
async fn test_query_context_only() {
    let app = create_test_app();

    let request = json!({
        "query": "What is blockchain?",
        "context_only": true
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    assert!(body.get("sources").is_some());
}

#[tokio::test]
async fn test_query_with_max_results() {
    let app = create_test_app();

    let request = json!({
        "query": "Tell me about databases",
        "max_results": 5
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_query_stats() {
    let app = create_test_app();

    let request = json!({
        "query": "What is cloud computing?"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    let stats = body.get("stats").expect("Should have stats");
    
    assert!(stats.get("embedding_time_ms").is_some());
    assert!(stats.get("retrieval_time_ms").is_some());
    assert!(stats.get("generation_time_ms").is_some());
    assert!(stats.get("total_time_ms").is_some());
    assert!(stats.get("sources_retrieved").is_some());
}

// ============================================================================
// Stream Query Tests
// ============================================================================

#[tokio::test]
async fn test_stream_query_empty() {
    let app = create_test_app();

    let request = json!({
        "query": ""
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query/stream")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Empty query should fail validation
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_stream_query_success() {
    let app = create_test_app();

    let request = json!({
        "query": "What is machine learning?"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query/stream")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Streaming responses return 200 with SSE content type
    assert_eq!(response.status(), StatusCode::OK);
    
    // Verify content type is SSE
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    
    assert!(content_type.contains("text/event-stream"));
}

#[tokio::test]
async fn test_stream_query_with_mode() {
    let app = create_test_app();

    let request = json!({
        "query": "Explain neural networks",
        "mode": "hybrid"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query/stream")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

// ============================================================================
// Query with Document Data Tests
// ============================================================================

#[tokio::test]
async fn test_query_after_document_upload() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Upload some documents
    let _doc1 = upload_document(
        &server,
        "Machine learning is a subset of artificial intelligence. It uses algorithms to learn from data and make predictions.",
    )
    .await;

    let _doc2 = upload_document(
        &server,
        "Deep learning is a type of machine learning that uses neural networks with many layers. It excels at image recognition.",
    )
    .await;

    // Query the knowledge base
    let request = json!({
        "query": "What is the relationship between machine learning and deep learning?",
        "mode": "hybrid"
    });

    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    assert!(body.get("answer").is_some());
    
    // With mock LLM, we should still get sources
    let sources = body.get("sources").and_then(|v| v.as_array());
    assert!(sources.is_some());
}

#[tokio::test]
async fn test_query_sources_types() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Upload document with entities
    let _doc = upload_document(
        &server,
        "Google is a technology company founded by Larry Page and Sergey Brin. Google develops the Chrome browser.",
    )
    .await;

    // Query
    let request = json!({
        "query": "Who founded Google?"
    });

    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    let sources = body.get("sources").and_then(|v| v.as_array());
    
    if let Some(sources) = sources {
        for source in sources {
            let source_type = source.get("source_type").and_then(|v| v.as_str());
            // Source types should be one of: chunk, entity, relationship
            assert!(matches!(
                source_type,
                Some("chunk") | Some("entity") | Some("relationship")
            ));
        }
    }
}

// ============================================================================
// Query Modes Comparison Test
// ============================================================================

#[tokio::test]
async fn test_query_all_modes() {
    let server = Server::new(create_test_config(), AppState::test_state());

    let modes = vec!["naive", "local", "global", "hybrid", "mix"];
    let query = "What is artificial intelligence?";

    for mode in modes {
        let request = json!({
            "query": query,
            "mode": mode
        });

        let app = server.build_router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/query")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Query with mode '{}' should succeed",
            mode
        );

        let body = extract_json(response).await;
        assert!(
            body.get("answer").is_some(),
            "Query with mode '{}' should return answer",
            mode
        );
    }
}
