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

// ============================================================================
// Conversation History Tests
// ============================================================================

#[tokio::test]
async fn test_query_with_conversation_history() {
    let app = create_test_app();

    let request = json!({
        "query": "What did we discuss earlier?",
        "mode": "naive",
        "conversation_history": [
            {"role": "user", "content": "Tell me about machine learning."},
            {"role": "assistant", "content": "Machine learning is a subset of AI that enables systems to learn from data."},
            {"role": "user", "content": "How does it relate to neural networks?"},
            {"role": "assistant", "content": "Neural networks are a key technique used in machine learning for pattern recognition."}
        ]
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
    assert!(
        body.get("conversation_id").is_some(),
        "Should return conversation_id when history is provided"
    );
}

#[tokio::test]
async fn test_query_without_conversation_history_no_id() {
    let app = create_test_app();

    let request = json!({
        "query": "What is machine learning?",
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
    assert!(body.get("answer").is_some());
    // No conversation_id when no history is provided
    assert!(
        body.get("conversation_id").is_none() || body.get("conversation_id").unwrap().is_null()
    );
}

#[tokio::test]
async fn test_query_empty_conversation_history() {
    let app = create_test_app();

    let request = json!({
        "query": "What is deep learning?",
        "conversation_history": []
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
}

#[tokio::test]
async fn test_query_conversation_history_structure() {
    let app = create_test_app();

    let request = json!({
        "query": "Continue our discussion",
        "conversation_history": [
            {"role": "user", "content": "Hello, I have a question."},
            {"role": "assistant", "content": "Sure, how can I help you?"}
        ]
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

    // Verify response structure
    assert!(body.get("answer").is_some(), "Response should have answer");
    assert!(body.get("mode").is_some(), "Response should have mode");
    assert!(
        body.get("sources").is_some(),
        "Response should have sources"
    );
    assert!(body.get("stats").is_some(), "Response should have stats");
    assert!(
        body.get("conversation_id").is_some(),
        "Response should have conversation_id"
    );

    // Verify conversation_id is a valid UUID
    let conv_id = body.get("conversation_id").unwrap().as_str().unwrap();
    assert_eq!(conv_id.len(), 36, "Conversation ID should be a UUID");
}

#[tokio::test]
async fn test_query_multi_turn_context() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // First query without history
    let request1 = json!({
        "query": "Tell me about Rust programming",
        "mode": "naive"
    });

    let app = server.build_router();
    let response1 = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request1).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response1.status(), StatusCode::OK);
    let body1 = extract_json(response1).await;
    let answer1 = body1.get("answer").unwrap().as_str().unwrap();

    // Second query with history from first
    let request2 = json!({
        "query": "What are its main features?",
        "mode": "naive",
        "conversation_history": [
            {"role": "user", "content": "Tell me about Rust programming"},
            {"role": "assistant", "content": answer1}
        ]
    });

    let app = server.build_router();
    let response2 = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request2).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response2.status(), StatusCode::OK);
    let body2 = extract_json(response2).await;
    assert!(body2.get("answer").is_some());
    assert!(body2.get("conversation_id").is_some());
}

// ============================================================================
// Reranking Tests
// ============================================================================

#[tokio::test]
async fn test_query_with_reranking_enabled() {
    let app = create_test_app();

    let request = json!({
        "query": "What is machine learning?",
        "mode": "naive",
        "enable_rerank": true
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
    assert_eq!(body.get("reranked").and_then(|v| v.as_bool()), Some(true));

    // Stats should include rerank time
    let stats = body.get("stats").unwrap();
    assert!(stats.get("rerank_time_ms").is_some());
}

#[tokio::test]
async fn test_query_with_reranking_disabled() {
    let app = create_test_app();

    let request = json!({
        "query": "What is artificial intelligence?",
        "mode": "naive",
        "enable_rerank": false
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

    // Reranked should be false (and omitted from response due to skip_serializing_if)
    assert!(
        body.get("reranked").is_none()
            || body.get("reranked").and_then(|v| v.as_bool()) == Some(false)
    );

    // Rerank time should not be present
    let stats = body.get("stats").unwrap();
    assert!(stats.get("rerank_time_ms").is_none());
}

#[tokio::test]
async fn test_query_rerank_default_enabled() {
    let app = create_test_app();

    // Don't specify enable_rerank - should default to true
    let request = json!({
        "query": "What are neural networks?",
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
    assert!(body.get("answer").is_some());

    // Default should be reranked = true
    assert_eq!(body.get("reranked").and_then(|v| v.as_bool()), Some(true));
}

#[tokio::test]
async fn test_query_rerank_with_top_k() {
    let app = create_test_app();

    let request = json!({
        "query": "What is deep learning?",
        "mode": "naive",
        "enable_rerank": true,
        "rerank_top_k": 3
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
    assert_eq!(body.get("reranked").and_then(|v| v.as_bool()), Some(true));

    // Sources should be limited to top_k chunks (if there were any)
    let sources = body.get("sources").and_then(|v| v.as_array()).unwrap();
    let chunks: Vec<&Value> = sources
        .iter()
        .filter(|s| s.get("source_type").and_then(|v| v.as_str()) == Some("chunk"))
        .collect();
    assert!(
        chunks.len() <= 3,
        "Should have at most 3 chunks after rerank_top_k"
    );
}

#[tokio::test]
async fn test_query_rerank_sources_have_rerank_scores() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // First upload a document to ensure we have chunks
    let _doc_id = upload_document(
        &server,
        "Machine learning is a branch of artificial intelligence. \
         It enables systems to learn from data. \
         Deep learning uses neural networks with many layers.",
    )
    .await;

    let request = json!({
        "query": "What is machine learning?",
        "mode": "naive",
        "enable_rerank": true
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
    assert_eq!(body.get("reranked").and_then(|v| v.as_bool()), Some(true));

    // Check that chunk sources have rerank_score field
    let sources = body.get("sources").and_then(|v| v.as_array()).unwrap();
    for source in sources {
        let source_type = source.get("source_type").and_then(|v| v.as_str()).unwrap();
        if source_type == "chunk" {
            // Chunks should have rerank_score when reranking is enabled
            assert!(
                source.get("rerank_score").is_some(),
                "Chunk source should have rerank_score"
            );
        }
    }
}

#[tokio::test]
async fn test_query_rerank_with_model() {
    let app = create_test_app();

    let request = json!({
        "query": "Explain transformers",
        "mode": "naive",
        "enable_rerank": true,
        "rerank_model": "cohere-rerank-v3"
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
    // Model parameter is accepted (even if not fully implemented yet)
}
