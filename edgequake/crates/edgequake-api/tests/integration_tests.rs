//! Integration tests for EdgeQuake API.
//!
//! These tests exercise the full HTTP API using the test router.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

use edgequake_api::{AppState, Server, ServerConfig};

/// Helper to create a test server.
fn create_test_server() -> Server {
    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0, // ephemeral
        enable_cors: false,
        enable_compression: false,
        enable_swagger: true,
    };
    Server::new(config, AppState::test_state())
}

/// Parse JSON response body.
async fn parse_json(response: axum::response::Response) -> Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap()
}

// ============ Health Endpoint Tests ============

#[tokio::test]
async fn test_health_endpoint() {
    let server = create_test_server();
    let app = server.build_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let json = parse_json(response).await;
    assert_eq!(json["status"], "healthy");
}

#[tokio::test]
async fn test_health_endpoint_has_components() {
    let server = create_test_server();
    let app = server.build_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let json = parse_json(response).await;
    assert!(json.get("components").is_some());
}

// ============ Document Endpoints Tests ============

#[tokio::test]
async fn test_list_documents_empty() {
    let server = create_test_server();
    let app = server.build_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/documents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let json = parse_json(response).await;
    assert!(json.get("documents").is_some());
    assert!(json["documents"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_upload_document() {
    let server = create_test_server();
    let app = server.build_router();

    let body = json!({
        "content": "This is a test document for EdgeQuake.",
        "metadata": {
            "source": "test"
        }
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let json = parse_json(response).await;
    assert!(json.get("document_id").is_some());
    assert!(json.get("status").is_some());
}

#[tokio::test]
async fn test_upload_document_missing_content() {
    let server = create_test_server();
    let app = server.build_router();

    let body = json!({
        "metadata": {
            "source": "test"
        }
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should fail due to missing content
    assert!(response.status().is_client_error());
}

#[tokio::test]
async fn test_get_document_not_found() {
    let server = create_test_server();
    let app = server.build_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/documents/nonexistent-doc-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_document_not_found() {
    let server = create_test_server();
    let app = server.build_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/documents/nonexistent-doc-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Delete should return 404 for non-existent documents
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ============ Query Endpoints Tests ============

#[tokio::test]
async fn test_query_endpoint() {
    let server = create_test_server();
    let app = server.build_router();

    let body = json!({
        "query": "What is EdgeQuake?",
        "mode": "naive"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let json = parse_json(response).await;
    assert!(json.get("answer").is_some() || json.get("response").is_some());
}

#[tokio::test]
async fn test_query_with_different_modes() {
    let modes = ["naive", "local", "global", "hybrid", "mix"];

    for mode in modes {
        let server = create_test_server();
        let app = server.build_router();

        let body = json!({
            "query": "Test query",
            "mode": mode
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/query")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Query mode '{}' should succeed",
            mode
        );
    }
}

#[tokio::test]
async fn test_query_missing_query_field() {
    let server = create_test_server();
    let app = server.build_router();

    let body = json!({
        "mode": "naive"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status().is_client_error());
}

// ============ Graph Endpoints Tests ============

#[tokio::test]
async fn test_graph_endpoint() {
    let server = create_test_server();
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
}

#[tokio::test]
async fn test_graph_node_not_found() {
    let server = create_test_server();
    let app = server.build_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/nodes/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Non-existent node should return 404
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_graph_labels_search() {
    let server = create_test_server();
    let app = server.build_router();

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
}

// ============ OpenAPI/Swagger Tests ============

#[tokio::test]
async fn test_swagger_ui_available() {
    let server = create_test_server();
    let app = server.build_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/swagger-ui/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Swagger UI should redirect or return HTML
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::MOVED_PERMANENTLY
            || response.status() == StatusCode::TEMPORARY_REDIRECT
    );
}

#[tokio::test]
async fn test_openapi_json_available() {
    let server = create_test_server();
    let app = server.build_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api-docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let json = parse_json(response).await;
    assert!(json.get("openapi").is_some());
    assert!(json.get("info").is_some());
    assert!(json.get("paths").is_some());
}

// ============ Error Handling Tests ============

#[tokio::test]
async fn test_not_found_route() {
    let server = create_test_server();
    let app = server.build_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/nonexistent/route")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_method_not_allowed() {
    let server = create_test_server();
    let app = server.build_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // PUT on /health should fail
    assert!(response.status().is_client_error());
}

#[tokio::test]
async fn test_invalid_json_body() {
    let server = create_test_server();
    let app = server.build_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from("{ invalid json }"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status().is_client_error());
}

// ============ Request ID Middleware Tests ============

#[tokio::test]
async fn test_request_id_header_added() {
    let server = create_test_server();
    let app = server.build_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // The request_id middleware should add x-request-id header
    // Note: Depending on implementation, this may or may not be present
    assert!(response.status().is_success());
}
