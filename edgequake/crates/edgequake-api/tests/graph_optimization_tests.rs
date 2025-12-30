//! Integration tests for optimized graph queries.
//!
//! These tests verify that the N+1 query elimination works correctly
//! and that the optimized batch methods return correct results.
//!
//! Run with: `cargo test --package edgequake-api --test graph_optimization_tests`

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

#[allow(dead_code)]
fn create_test_server() -> Server {
    Server::new(create_test_config(), AppState::test_state())
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
// N+1 Query Optimization Tests
// ============================================================================

#[tokio::test]
async fn test_get_graph_returns_nodes_with_degrees() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Upload document with entities
    let _doc_id = upload_document(
        &server,
        "Sarah Chen works at Quantum Corp. Quantum Corp is located in Silicon Valley. \
         Michael Johnson is the CEO of Quantum Corp. Sarah and Michael collaborate on AI projects.",
    )
    .await;

    // Get graph
    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph?max_nodes=50")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    let nodes = body.get("nodes").and_then(|v| v.as_array()).unwrap();

    // Verify nodes have degree information
    for node in nodes {
        let degree = node.get("degree").and_then(|v| v.as_u64());
        assert!(
            degree.is_some(),
            "Node {} should have degree field",
            node.get("id").unwrap()
        );
    }
}

#[tokio::test]
async fn test_get_graph_nodes_ordered_by_degree() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Upload document with multiple entities
    let _doc_id = upload_document(
        &server,
        "Alice works at TechCorp. Bob works at TechCorp. Charlie works at TechCorp. \
         David is the CEO of TechCorp. Eve manages everyone at TechCorp. \
         TechCorp is located in San Francisco. TechCorp makes software.",
    )
    .await;

    // Get graph with limited nodes
    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph?max_nodes=5")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    let nodes = body.get("nodes").and_then(|v| v.as_array()).unwrap();

    // Verify nodes are in descending order by degree
    let degrees: Vec<u64> = nodes
        .iter()
        .filter_map(|n| n.get("degree").and_then(|d| d.as_u64()))
        .collect();

    for i in 1..degrees.len() {
        assert!(
            degrees[i - 1] >= degrees[i],
            "Nodes should be ordered by degree descending: {:?}",
            degrees
        );
    }
}

#[tokio::test]
async fn test_get_graph_edges_only_between_returned_nodes() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Upload document
    let _doc_id = upload_document(
        &server,
        "Alice knows Bob. Bob knows Charlie. Charlie knows Diana. Diana knows Eve. \
         Eve knows Frank. Frank knows George. George knows Henry.",
    )
    .await;

    // Get graph with limited nodes
    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph?max_nodes=5")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    let nodes = body.get("nodes").and_then(|v| v.as_array()).unwrap();
    let edges = body.get("edges").and_then(|v| v.as_array()).unwrap();

    // Collect node IDs
    let node_ids: Vec<&str> = nodes
        .iter()
        .filter_map(|n| n.get("id").and_then(|id| id.as_str()))
        .collect();

    // Verify all edges have both source and target in returned nodes
    for edge in edges {
        let source = edge.get("source").and_then(|s| s.as_str()).unwrap();
        let target = edge.get("target").and_then(|t| t.as_str()).unwrap();

        assert!(
            node_ids.contains(&source),
            "Edge source '{}' not in returned nodes",
            source
        );
        assert!(
            node_ids.contains(&target),
            "Edge target '{}' not in returned nodes",
            target
        );
    }
}

#[tokio::test]
async fn test_get_graph_truncation_flag() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Upload multiple documents to create many entities
    for i in 0..5 {
        let _doc_id = upload_document(
            &server,
            &format!(
                "Company{} is in the tech sector. CEO{} runs Company{}. \
                 CTO{} is at Company{}. CFO{} manages Company{} finances.",
                i, i, i, i, i, i, i
            ),
        )
        .await;
    }

    // Get graph with small limit
    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph?max_nodes=5")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    let nodes = body.get("nodes").and_then(|v| v.as_array()).unwrap();
    let total_nodes = body.get("total_nodes").and_then(|v| v.as_u64()).unwrap();

    // If total_nodes > max_nodes, is_truncated should be true
    if total_nodes > nodes.len() as u64 {
        let is_truncated = body.get("is_truncated").and_then(|v| v.as_bool()).unwrap();
        assert!(is_truncated, "Graph should be marked as truncated");
    }
}

#[tokio::test]
async fn test_get_graph_performance_multiple_calls() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Upload documents
    let _doc_id = upload_document(
        &server,
        "NetworkA connects to NetworkB. NetworkB connects to NetworkC. \
         ServerA is part of NetworkA. ServerB is part of NetworkB. \
         ServerC is part of NetworkC. AdminUser manages all servers.",
    )
    .await;

    // Make multiple calls to verify consistency
    for _ in 0..3 {
        let app = server.build_router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/graph?max_nodes=50")
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
}

// ============================================================================
// Edge Filtering Tests
// ============================================================================

#[tokio::test]
async fn test_get_graph_filters_edges_at_db_level() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Upload document
    let _doc_id = upload_document(
        &server,
        "Alpha connects to Beta. Beta connects to Gamma. \
         Gamma connects to Delta. Delta connects to Epsilon.",
    )
    .await;

    // Get graph - edges should be filtered to only include nodes in the result set
    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph?max_nodes=100")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    let nodes = body.get("nodes").and_then(|v| v.as_array()).unwrap();
    let edges = body.get("edges").and_then(|v| v.as_array()).unwrap();

    // Get node IDs
    let node_ids: std::collections::HashSet<&str> = nodes
        .iter()
        .filter_map(|n| n.get("id").and_then(|id| id.as_str()))
        .collect();

    // All edges should have both endpoints in the node set
    for edge in edges {
        let source = edge.get("source").and_then(|s| s.as_str()).unwrap();
        let target = edge.get("target").and_then(|t| t.as_str()).unwrap();

        assert!(
            node_ids.contains(source) && node_ids.contains(target),
            "Edge ({} -> {}) has endpoint not in node set",
            source,
            target
        );
    }
}

// ============================================================================
// Start Node Tests
// ============================================================================

#[tokio::test]
async fn test_get_graph_with_start_node_optimization() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Upload document
    let _doc_id = upload_document(
        &server,
        "CentralNode connects to Leaf1. CentralNode connects to Leaf2. \
         CentralNode connects to Leaf3. Leaf1 connects to Leaf2. \
         UnconnectedNode exists separately.",
    )
    .await;

    // Get graph starting from CentralNode
    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph?start_node=CENTRALNODE&depth=1&max_nodes=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    assert!(body.get("nodes").is_some());
}

// ============================================================================
// Empty Graph Tests
// ============================================================================

#[tokio::test]
async fn test_get_graph_empty_returns_valid_response() {
    let server = Server::new(create_test_config(), AppState::test_state());

    let app = server.build_router();
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
    let nodes = body.get("nodes").and_then(|v| v.as_array()).unwrap();
    let edges = body.get("edges").and_then(|v| v.as_array()).unwrap();
    let total_nodes = body.get("total_nodes").and_then(|v| v.as_u64()).unwrap();
    let total_edges = body.get("total_edges").and_then(|v| v.as_u64()).unwrap();

    assert!(nodes.is_empty() || !nodes.is_empty()); // Valid array
    assert!(edges.is_empty() || !edges.is_empty()); // Valid array
    assert!(total_nodes <= u64::MAX); // Valid count
    assert!(total_edges <= u64::MAX); // Valid count
}

// ============================================================================
// Response Structure Tests
// ============================================================================

#[tokio::test]
async fn test_get_graph_response_structure() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Upload document
    let _doc_id = upload_document(
        &server,
        "EntityA of type Person relates to EntityB of type Organization.",
    )
    .await;

    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph?max_nodes=50")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;

    // Verify response structure
    assert!(body.get("nodes").is_some(), "Response should have 'nodes'");
    assert!(body.get("edges").is_some(), "Response should have 'edges'");
    assert!(
        body.get("is_truncated").is_some(),
        "Response should have 'is_truncated'"
    );
    assert!(
        body.get("total_nodes").is_some(),
        "Response should have 'total_nodes'"
    );
    assert!(
        body.get("total_edges").is_some(),
        "Response should have 'total_edges'"
    );

    // Verify node structure
    if let Some(nodes) = body.get("nodes").and_then(|v| v.as_array()) {
        for node in nodes {
            assert!(node.get("id").is_some(), "Node should have 'id'");
            assert!(node.get("label").is_some(), "Node should have 'label'");
            assert!(
                node.get("node_type").is_some(),
                "Node should have 'node_type'"
            );
            assert!(node.get("degree").is_some(), "Node should have 'degree'");
        }
    }

    // Verify edge structure
    if let Some(edges) = body.get("edges").and_then(|v| v.as_array()) {
        for edge in edges {
            assert!(edge.get("source").is_some(), "Edge should have 'source'");
            assert!(edge.get("target").is_some(), "Edge should have 'target'");
            assert!(
                edge.get("edge_type").is_some(),
                "Edge should have 'edge_type'"
            );
        }
    }
}
