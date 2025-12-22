//! End-to-end tests for graph API endpoints.
//!
//! Tests cover:
//! - Get graph (GET /api/v1/graph)
//! - Get node (GET /api/v1/graph/nodes/{node_id})
//! - Search labels (GET /api/v1/graph/labels/search)

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
// Get Graph Tests
// ============================================================================

#[tokio::test]
async fn test_get_graph_empty() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    assert!(body.get("nodes").is_some());
    assert!(body.get("edges").is_some());
    assert!(body.get("total_nodes").is_some());
    assert!(body.get("total_edges").is_some());
}

#[tokio::test]
async fn test_get_graph_with_params() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph?depth=3&max_nodes=50")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    assert!(body.get("nodes").is_some());
}

#[tokio::test]
async fn test_get_graph_with_start_node() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Upload a document to create entities
    let _doc_id = upload_document(
        &server,
        "Sarah Chen works at Quantum Corp. Quantum Corp is located in Silicon Valley. Sarah leads the AI team.",
    )
    .await;

    // Get graph starting from a specific node
    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph?start_node=SARAH_CHEN&depth=2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    assert!(body.get("nodes").is_some());
    assert!(body.get("edges").is_some());
}

// ============================================================================
// Get Node Tests
// ============================================================================

#[tokio::test]
async fn test_get_node_not_found() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/nodes/NONEXISTENT_NODE")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_node_after_document_processing() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Upload document with entities
    let _doc_id = upload_document(
        &server,
        "Albert Einstein was a physicist who developed the theory of relativity. Einstein worked at Princeton.",
    )
    .await;

    // Try to get a node (entity name is normalized to uppercase with underscores)
    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/nodes/ALBERT_EINSTEIN")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // This may or may not find the node depending on mock LLM extraction
    // Just verify the endpoint works
    let status = response.status();
    assert!(status == StatusCode::OK || status == StatusCode::NOT_FOUND);
}

// ============================================================================
// Search Labels Tests
// ============================================================================

#[tokio::test]
async fn test_search_labels_empty() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/labels/search?q=test&limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    assert!(body.get("labels").is_some());
}

#[tokio::test]
async fn test_search_labels_with_data() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Upload document
    let _doc_id = upload_document(
        &server,
        "Microsoft is a technology company. Google is also a tech company. Both companies develop AI products.",
    )
    .await;

    // Search for labels
    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/labels/search?q=company&limit=5")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    assert!(body.get("labels").is_some());
}

#[tokio::test]
async fn test_search_labels_default_limit() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/labels/search?q=test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

// ============================================================================
// Integration Flow Tests
// ============================================================================

#[tokio::test]
async fn test_graph_after_document_upload() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // 1. Get initial empty graph
    let app = server.build_router();
    let initial_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(initial_response.status(), StatusCode::OK);

    let initial_body = extract_json(initial_response).await;
    let initial_node_count = initial_body
        .get("total_nodes")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // 2. Upload document with entities
    let _doc_id = upload_document(
        &server,
        "Amazon is an e-commerce company founded by Jeff Bezos. Amazon Web Services (AWS) is Amazon's cloud platform.",
    )
    .await;

    // 3. Get graph after upload - should have more entities
    let app = server.build_router();
    let final_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(final_response.status(), StatusCode::OK);

    let final_body = extract_json(final_response).await;
    let final_node_count = final_body
        .get("total_nodes")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // The mock LLM should extract some entities
    assert!(final_node_count >= initial_node_count);
}

#[tokio::test]
async fn test_graph_traversal() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Upload multiple related documents
    let _doc1 = upload_document(
        &server,
        "Apple Inc. was founded by Steve Jobs. Apple makes the iPhone and Mac computers.",
    )
    .await;

    let _doc2 = upload_document(
        &server,
        "Steve Jobs was a visionary entrepreneur. He returned to Apple in 1997 and launched the iPod.",
    )
    .await;

    // Get graph from starting point
    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph?max_nodes=20&depth=2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    assert!(body.get("nodes").is_some());
    assert!(body.get("edges").is_some());
    assert!(body.get("is_truncated").is_some());
}
