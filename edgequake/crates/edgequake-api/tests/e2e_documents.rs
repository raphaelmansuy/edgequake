//! End-to-end tests for document API endpoints.
//!
//! Tests cover:
//! - Document upload (POST /api/v1/documents)
//! - List documents (GET /api/v1/documents)
//! - Get document by ID (GET /api/v1/documents/{document_id})
//! - Delete document (DELETE /api/v1/documents/{document_id})

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

// ============================================================================
// Document Upload Tests
// ============================================================================

#[tokio::test]
async fn test_upload_document_success() {
    let app = create_test_app();

    let request = json!({
        "content": "This is a test document about artificial intelligence and machine learning. AI systems are becoming increasingly sophisticated.",
        "title": "AI Overview",
        "metadata": {"source": "test"}
    });

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

    // WHY: POST /documents returns 201 Created per REST semantics (UC0001)
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = extract_json(response).await;
    assert!(body.get("document_id").is_some());
    assert_eq!(
        body.get("status").and_then(|v| v.as_str()),
        Some("processed")
    );
    assert!(body.get("chunk_count").is_some());
    assert!(body.get("entity_count").is_some());
    assert!(body.get("relationship_count").is_some());
}

#[tokio::test]
async fn test_upload_document_minimal() {
    let app = create_test_app();

    let request = json!({
        "content": "A minimal document with just content."
    });

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

    // WHY: POST /documents returns 201 Created per REST semantics (UC0001)
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = extract_json(response).await;
    assert!(body.get("document_id").is_some());
}

#[tokio::test]
async fn test_upload_document_empty_content() {
    let app = create_test_app();

    let request = json!({
        "content": ""
    });

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

    // Should fail validation
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_upload_document_whitespace_only() {
    let app = create_test_app();

    let request = json!({
        "content": "   \n\t   "
    });

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

    // Should fail validation
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

/// Test that multipart form data is rejected on /api/v1/documents endpoint.
///
/// WHY: This test documents the expected behavior that users were confused about.
/// The /api/v1/documents endpoint only accepts JSON (application/json).
/// For file uploads, users should use /api/v1/documents/upload instead.
///
/// This test ensures we catch regressions if someone accidentally makes
/// the endpoint accept multipart data, which would break API consistency.
#[tokio::test]
async fn test_upload_document_rejects_multipart() {
    let app = create_test_app();

    // Simulate a multipart request (what users were trying in the issue)
    let boundary = "----TestBoundary1234567890";
    let body = format!(
        "--{}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\n\
         Content-Type: text/plain\r\n\r\n\
         Test content\r\n\
         --{}--\r\n",
        boundary, boundary
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents")
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={}", boundary),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    // WHY: Should return 415 Unsupported Media Type
    // The endpoint expects JSON, not multipart form data
    // This is the correct HTTP status for wrong content-type
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

/// Test that the correct endpoint (/api/v1/documents/upload) accepts multipart.
///
/// WHY: This test serves as documentation showing users the correct way
/// to upload files. If they see test failures, they can look here to
/// understand which endpoint to use.
#[tokio::test]
async fn test_upload_endpoint_accepts_multipart() {
    let app = create_test_app();

    // This is the CORRECT way to upload files
    let boundary = "----TestBoundary1234567890";
    let body = format!(
        "--{}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\n\
         Content-Type: text/plain\r\n\r\n\
         Test content about artificial intelligence\r\n\
         --{}--\r\n",
        boundary, boundary
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents/upload")
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={}", boundary),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    // WHY: Should succeed with 201 Created
    // This is the correct endpoint for file uploads
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_upload_document_with_metadata() {
    let app = create_test_app();

    let request = json!({
        "content": "Document with rich metadata about quantum computing.",
        "title": "Quantum Computing Intro",
        "metadata": {
            "author": "Test Author",
            "version": 1,
            "tags": ["quantum", "computing", "physics"],
            "nested": {
                "field": "value"
            }
        }
    });

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

    // WHY: POST /documents returns 201 Created per REST semantics (UC0001)
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = extract_json(response).await;
    assert!(body.get("document_id").is_some());
}

// ============================================================================
// List Documents Tests
// ============================================================================

#[tokio::test]
async fn test_list_documents_empty() {
    let app = create_test_app();

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

    let body = extract_json(response).await;
    assert!(body.get("documents").is_some());
    assert!(body.get("total").is_some());
    assert!(body.get("page").is_some());
    assert!(body.get("page_size").is_some());
}

#[tokio::test]
async fn test_list_documents_after_upload() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Upload a document first
    let upload_request = json!({
        "content": "Test document for listing. Contains information about software development."
    });

    let app = server.build_router();
    let upload_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&upload_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // WHY: POST /documents returns 201 Created per REST semantics (UC0001)
    assert_eq!(upload_response.status(), StatusCode::CREATED);

    // Now list documents
    let app = server.build_router();
    let list_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/documents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(list_response.status(), StatusCode::OK);

    let body = extract_json(list_response).await;
    let docs = body.get("documents").and_then(|v| v.as_array());
    assert!(docs.is_some());
}

// ============================================================================
// Get Document Tests
// ============================================================================

#[tokio::test]
async fn test_get_document_success() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Upload a document first
    let upload_request = json!({
        "content": "Test document for retrieval. This document discusses programming languages."
    });

    let app = server.build_router();
    let upload_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&upload_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // WHY: POST /documents returns 201 Created per REST semantics (UC0001)
    assert_eq!(upload_response.status(), StatusCode::CREATED);

    let upload_body = extract_json(upload_response).await;
    let document_id = upload_body
        .get("document_id")
        .and_then(|v| v.as_str())
        .expect("Should have document_id");

    // Now get the document
    let app = server.build_router();
    let get_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/v1/documents/{}", document_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::OK);

    let body = extract_json(get_response).await;
    assert_eq!(body.get("id").and_then(|v| v.as_str()), Some(document_id));
    assert!(body.get("chunk_count").is_some());
    assert!(body.get("status").is_some());
}

#[tokio::test]
async fn test_get_document_not_found() {
    let app = create_test_app();

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

// ============================================================================
// Delete Document Tests
// ============================================================================

#[tokio::test]
async fn test_delete_document_success() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Upload a document first
    let upload_request = json!({
        "content": "Document to be deleted. Contains some test content."
    });

    let app = server.build_router();
    let upload_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&upload_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // WHY: POST /documents returns 201 Created per REST semantics (UC0001)
    assert_eq!(upload_response.status(), StatusCode::CREATED);

    let upload_body = extract_json(upload_response).await;
    let document_id = upload_body
        .get("document_id")
        .and_then(|v| v.as_str())
        .expect("Should have document_id");

    // Now delete the document
    let app = server.build_router();
    let delete_response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/v1/documents/{}", document_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(delete_response.status(), StatusCode::OK);

    let body = extract_json(delete_response).await;
    assert_eq!(
        body.get("document_id").and_then(|v| v.as_str()),
        Some(document_id)
    );
    assert_eq!(body.get("deleted").and_then(|v| v.as_bool()), Some(true));

    // Verify document is gone
    let app = server.build_router();
    let get_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/v1/documents/{}", document_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_document_not_found() {
    let app = create_test_app();

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

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Integration Flow Tests
// ============================================================================

#[tokio::test]
async fn test_complete_document_lifecycle() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // 1. Upload document
    let upload_request = json!({
        "content": "This is a comprehensive test document about artificial intelligence. Machine learning is a subset of AI. Deep learning uses neural networks.",
        "title": "AI Introduction",
        "metadata": {"test": true}
    });

    let app = server.build_router();
    let upload_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&upload_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // WHY: POST /documents returns 201 Created per REST semantics (UC0001)
    assert_eq!(upload_response.status(), StatusCode::CREATED);

    let upload_body = extract_json(upload_response).await;
    let document_id = upload_body
        .get("document_id")
        .and_then(|v| v.as_str())
        .expect("Should have document_id")
        .to_string();

    // 2. List documents - should include new document
    let app = server.build_router();
    let list_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/documents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(list_response.status(), StatusCode::OK);

    // 3. Get document by ID
    let app = server.build_router();
    let get_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/v1/documents/{}", document_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::OK);

    let get_body = extract_json(get_response).await;
    assert_eq!(
        get_body.get("id").and_then(|v| v.as_str()),
        Some(document_id.as_str())
    );

    // 4. Delete document
    let app = server.build_router();
    let delete_response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/v1/documents/{}", document_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(delete_response.status(), StatusCode::OK);

    // 5. Verify document is gone
    let app = server.build_router();
    let final_get = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/v1/documents/{}", document_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(final_get.status(), StatusCode::NOT_FOUND);
}
