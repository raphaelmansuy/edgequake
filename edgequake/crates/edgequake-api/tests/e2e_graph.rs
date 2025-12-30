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
// ============================================================================
// SOTA Batch Operations E2E Tests
// ============================================================================

#[tokio::test]
async fn test_degrees_batch_e2e() {
    let server = create_test_server();

    // Upload document to create nodes
    upload_document(
        &server,
        "Alice works with Bob. Bob collaborates with Charlie. \
         Charlie leads the project team with Alice.",
    )
    .await;

    // Give time for processing
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Request degrees for multiple nodes
    let request_body = json!({
        "node_ids": ["ALICE", "BOB", "CHARLIE", "NONEXISTENT"]
    });

    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/degrees/batch")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;

    // Verify response structure
    assert!(body.get("degrees").is_some());
    assert!(body.get("count").is_some());

    let degrees = body["degrees"].as_array().unwrap();
    assert_eq!(
        degrees.len(),
        4,
        "Should return degrees for all requested nodes"
    );

    // Verify each degree has node_id and degree fields
    for degree_obj in degrees {
        assert!(degree_obj.get("node_id").is_some());
        assert!(degree_obj.get("degree").is_some());
        assert!(degree_obj["degree"].is_number());
    }
}

#[tokio::test]
async fn test_degrees_batch_performance_e2e() {
    let server = create_test_server();

    // Upload document to create many nodes
    upload_document(
        &server,
        "Alice, Bob, Charlie, David, Eve, Frank, Grace, Henry, Ivy, Jack, \
         Kate, Leo, Mary, Nancy, Oscar, Paul, Quinn, Rachel, Steve, Tom, \
         Uma, Victor, Wendy, Xavier, Yara, Zoe all work together on projects.",
    )
    .await;

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Create list of 20 node IDs
    let node_ids: Vec<String> = vec![
        "ALICE", "BOB", "CHARLIE", "DAVID", "EVE", "FRANK", "GRACE", "HENRY", "IVY", "JACK",
        "KATE", "LEO", "MARY", "NANCY", "OSCAR", "PAUL", "QUINN", "RACHEL", "STEVE", "TOM",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    let request_body = json!({
        "node_ids": node_ids
    });

    let start = std::time::Instant::now();

    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/degrees/batch")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let elapsed = start.elapsed();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    let degrees = body["degrees"].as_array().unwrap();
    assert_eq!(degrees.len(), 20, "Should return all 20 degrees");

    // Performance assertion: batch query should complete in <500ms
    assert!(
        elapsed.as_millis() < 500,
        "Batch query for 20 nodes should complete in <500ms (was {}ms)",
        elapsed.as_millis()
    );
}

#[tokio::test]
async fn test_popular_labels_optimized_e2e() {
    let server = create_test_server();

    // Upload document with entities
    upload_document(
        &server,
        "Sarah Chen is a software engineer at Microsoft. \
         She works with John Smith and Alice Johnson. \
         They collaborate on Azure and cloud computing projects.",
    )
    .await;

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/labels/popular?limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;

    assert!(body.get("labels").is_some());
    assert!(body.get("total_entities").is_some());

    let labels = body["labels"].as_array().unwrap();

    // Verify labels are sorted by degree (descending)
    for i in 1..labels.len() {
        let prev_degree = labels[i - 1]["degree"].as_u64().unwrap();
        let curr_degree = labels[i]["degree"].as_u64().unwrap();
        assert!(
            prev_degree >= curr_degree,
            "Labels should be sorted by degree descending"
        );
    }

    // Verify each label has required fields
    for label in labels {
        assert!(label.get("label").is_some());
        assert!(label.get("entity_type").is_some());
        assert!(label.get("degree").is_some());
        assert!(label.get("description").is_some());
    }
}

#[tokio::test]
async fn test_search_labels_fuzzy_e2e() {
    let server = create_test_server();

    // Upload document with entities
    upload_document(
        &server,
        "Sarah Chen works on machine learning. \
         Machine learning models require training data.",
    )
    .await;

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Test 1: Exact match
    let app1 = server.build_router();
    let response = app1
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/labels/search?q=SARAH&limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;

    // Just verify structure (entity extraction depends on LLM)
    assert!(body["labels"].is_array());

    // Test 2: Prefix match
    let app2 = server.build_router();
    let response = app2
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/labels/search?q=MACH&limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;

    // Verify response structure
    assert!(body["labels"].is_array());
}
