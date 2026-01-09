//! End-to-end tests for file upload API endpoints.
//!
//! Tests cover:
//! - Single file upload (POST /api/v1/documents/upload)
//! - Batch file upload (POST /api/v1/documents/upload/batch)
//! - Content deduplication
//! - File type validation
//! - Error handling

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use serde_json::Value;
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

/// Create a multipart body for file upload
fn create_multipart_body(filename: &str, content: &str) -> (String, Vec<u8>) {
    let boundary = "----TestBoundary1234567890";
    let body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n\
         Content-Type: text/plain\r\n\r\n\
         {content}\r\n\
         --{boundary}--\r\n",
        boundary = boundary,
        filename = filename,
        content = content
    );
    (boundary.to_string(), body.into_bytes())
}

/// Create a multipart body for batch file upload
fn create_batch_multipart_body(files: &[(&str, &str)]) -> (String, Vec<u8>) {
    let boundary = "----TestBoundary1234567890";
    let mut body = String::new();

    for (filename, content) in files {
        body.push_str(&format!(
            "--{boundary}\r\n\
             Content-Disposition: form-data; name=\"files\"; filename=\"{filename}\"\r\n\
             Content-Type: text/plain\r\n\r\n\
             {content}\r\n",
            boundary = boundary,
            filename = filename,
            content = content
        ));
    }

    body.push_str(&format!("--{boundary}--\r\n", boundary = boundary));
    (boundary.to_string(), body.into_bytes())
}

// ============================================================================
// Single File Upload Tests
// ============================================================================

#[tokio::test]
async fn test_upload_file_success() {
    let app = create_test_app();

    let (boundary, body) = create_multipart_body(
        "test_document.txt",
        "This is a test document about artificial intelligence and machine learning.",
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

    // WHY: POST file upload returns 201 Created per REST semantics (UC0002)
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = extract_json(response).await;
    assert!(body.get("document_id").is_some());
    assert_eq!(
        body.get("filename").and_then(|v| v.as_str()),
        Some("test_document.txt")
    );
    assert!(body.get("size").is_some());
    assert!(body.get("content_hash").is_some());
    assert_eq!(
        body.get("status").and_then(|v| v.as_str()),
        Some("processed")
    );
}

#[tokio::test]
async fn test_upload_file_markdown() {
    let app = create_test_app();

    let (boundary, body) = create_multipart_body(
        "readme.md",
        "# Test Document\n\nThis is a markdown document with **bold** text.",
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

    // WHY: POST file upload returns 201 Created per REST semantics (UC0002)
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = extract_json(response).await;
    assert_eq!(
        body.get("filename").and_then(|v| v.as_str()),
        Some("readme.md")
    );
}

#[tokio::test]
async fn test_upload_file_json() {
    let app = create_test_app();

    let (boundary, body) = create_multipart_body(
        "config.json",
        r#"{"name": "test", "version": "1.0", "features": ["rag", "graph"]}"#,
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

    // WHY: POST file upload returns 201 Created per REST semantics (UC0002)
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_upload_file_unsupported_type() {
    let app = create_test_app();

    // Binary file type should be rejected
    let boundary = "----TestBoundary1234567890";
    let body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"test.exe\"\r\n\
         Content-Type: application/octet-stream\r\n\r\n\
         binary_content_here\r\n\
         --{boundary}--\r\n",
        boundary = boundary
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

    // Should fail validation for unsupported file type (returns 400 BAD_REQUEST)
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_upload_file_empty_content() {
    let app = create_test_app();

    let (boundary, body) = create_multipart_body("empty.txt", "");

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

    // Should fail validation for empty content (returns 400 BAD_REQUEST or 422)
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_upload_file_no_file() {
    let app = create_test_app();

    // Empty multipart body
    let boundary = "----TestBoundary1234567890";
    let body = format!("--{boundary}--\r\n", boundary = boundary);

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

    // Should fail - no file provided (returns 400 BAD_REQUEST)
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ============================================================================
// File Deduplication Tests
// ============================================================================

#[tokio::test]
async fn test_upload_file_deduplication() {
    let server = create_test_server();

    let content = "This is a unique document about quantum computing and neural networks.";
    let (boundary, body) = create_multipart_body("document1.txt", content);

    // Upload first file
    let app = server.build_router();
    let response1 = app
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

    // WHY: POST file upload returns 201 Created per REST semantics (UC0002)
    assert_eq!(response1.status(), StatusCode::CREATED);
    let body1 = extract_json(response1).await;
    let doc_id1 = body1.get("document_id").and_then(|v| v.as_str()).unwrap();
    let hash1 = body1.get("content_hash").and_then(|v| v.as_str()).unwrap();

    // Upload same content with different filename
    let (boundary2, body2) = create_multipart_body("document2.txt", content);

    let app = server.build_router();
    let response2 = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents/upload")
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={}", boundary2),
                )
                .body(Body::from(body2))
                .unwrap(),
        )
        .await
        .unwrap();

    // WHY: Duplicate content returns 200 OK (not 201) since no new resource created
    assert_eq!(response2.status(), StatusCode::OK);
    let body2 = extract_json(response2).await;
    let doc_id2 = body2.get("document_id").and_then(|v| v.as_str()).unwrap();
    let hash2 = body2.get("content_hash").and_then(|v| v.as_str()).unwrap();

    // Same hash
    assert_eq!(hash1, hash2);

    // Same document ID (deduplicated)
    assert_eq!(doc_id1, doc_id2);

    // Status should indicate duplicate
    assert_eq!(
        body2.get("status").and_then(|v| v.as_str()),
        Some("duplicate")
    );
}

// ============================================================================
// Batch File Upload Tests
// ============================================================================

#[tokio::test]
async fn test_upload_batch_success() {
    let app = create_test_app();

    let files = vec![
        (
            "doc1.txt",
            "First document about machine learning algorithms.",
        ),
        (
            "doc2.txt",
            "Second document about natural language processing.",
        ),
        (
            "doc3.txt",
            "Third document about computer vision techniques.",
        ),
    ];

    let (boundary, body) = create_batch_multipart_body(&files);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents/upload/batch")
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={}", boundary),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    // WHY: POST file upload returns 201 Created per REST semantics (UC0002)
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = extract_json(response).await;
    assert!(body.get("results").is_some());

    let results = body.get("results").unwrap().as_array().unwrap();
    assert_eq!(results.len(), 3);

    assert_eq!(body.get("total_files").and_then(|v| v.as_u64()), Some(3));
    assert_eq!(body.get("processed").and_then(|v| v.as_u64()), Some(3));
    assert_eq!(body.get("duplicates").and_then(|v| v.as_u64()), Some(0));
    assert_eq!(body.get("failed").and_then(|v| v.as_u64()), Some(0));
}

#[tokio::test]
async fn test_upload_batch_with_duplicates() {
    let server = create_test_server();

    let content = "Duplicate content for batch testing purposes.";

    let files = vec![
        ("unique1.txt", "First unique document about blockchain."),
        ("dup1.txt", content),
        ("dup2.txt", content), // Same content as dup1
    ];

    let (boundary, body) = create_batch_multipart_body(&files);

    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents/upload/batch")
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={}", boundary),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    // WHY: POST file upload returns 201 Created per REST semantics (UC0002)
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = extract_json(response).await;
    assert_eq!(body.get("total_files").and_then(|v| v.as_u64()), Some(3));
    assert_eq!(body.get("processed").and_then(|v| v.as_u64()), Some(2));
    assert_eq!(body.get("duplicates").and_then(|v| v.as_u64()), Some(1));
}

#[tokio::test]
async fn test_upload_batch_empty() {
    let app = create_test_app();

    // Empty batch
    let boundary = "----TestBoundary1234567890";
    let body = format!("--{boundary}--\r\n", boundary = boundary);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents/upload/batch")
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={}", boundary),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    // Empty batch should return success with 0 files
    // WHY: POST file upload returns 201 Created per REST semantics (UC0002)
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = extract_json(response).await;
    assert_eq!(body.get("total_files").and_then(|v| v.as_u64()), Some(0));
}

#[tokio::test]
async fn test_upload_batch_mixed_valid_invalid() {
    let app = create_test_app();

    let files = vec![
        ("valid.txt", "Valid text document content."),
        ("empty.txt", ""), // Invalid: empty content
    ];

    let (boundary, body) = create_batch_multipart_body(&files);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents/upload/batch")
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={}", boundary),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    // WHY: POST file upload returns 201 Created per REST semantics (UC0002)
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = extract_json(response).await;
    assert_eq!(body.get("total_files").and_then(|v| v.as_u64()), Some(2));
    assert_eq!(body.get("processed").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(body.get("failed").and_then(|v| v.as_u64()), Some(1));
}

// ============================================================================
// Content Hash Tests
// ============================================================================

#[tokio::test]
async fn test_content_hash_consistency() {
    let server = create_test_server();

    let content = "Consistent content for hash verification testing.";

    // Upload file
    let (boundary, body) = create_multipart_body("hash_test.txt", content);

    let app = server.build_router();
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

    let body = extract_json(response).await;
    let hash = body.get("content_hash").and_then(|v| v.as_str()).unwrap();

    // Hash should be 64 characters (SHA-256 hex)
    assert_eq!(hash.len(), 64);

    // Should only contain hex characters
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}

// ============================================================================
// File Size Tests
// ============================================================================

#[tokio::test]
async fn test_upload_file_size_reported() {
    let app = create_test_app();

    let content = "This is a test document with known size.";
    let (boundary, body) = create_multipart_body("size_test.txt", content);

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

    // WHY: POST file upload returns 201 Created per REST semantics (UC0002)
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = extract_json(response).await;
    let size = body.get("size").and_then(|v| v.as_u64()).unwrap();

    // Size should match content length
    assert_eq!(size, content.len() as u64);
}

// ============================================================================
// Response Structure Tests
// ============================================================================

#[tokio::test]
async fn test_upload_file_response_structure() {
    let app = create_test_app();

    let (boundary, body) = create_multipart_body(
        "structure_test.txt",
        "Document for testing response structure.",
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

    // WHY: POST file upload returns 201 Created per REST semantics (UC0002)
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = extract_json(response).await;

    // Required fields
    assert!(body.get("document_id").is_some(), "Missing document_id");
    assert!(body.get("filename").is_some(), "Missing filename");
    assert!(body.get("size").is_some(), "Missing size");
    assert!(body.get("content_hash").is_some(), "Missing content_hash");
    assert!(body.get("status").is_some(), "Missing status");

    // Processing counts (for processed status)
    if body.get("status").and_then(|v| v.as_str()) == Some("processed") {
        assert!(body.get("chunk_count").is_some(), "Missing chunk_count");
        assert!(body.get("entity_count").is_some(), "Missing entity_count");
        assert!(
            body.get("relationship_count").is_some(),
            "Missing relationship_count"
        );
    }
}

#[tokio::test]
async fn test_batch_upload_response_structure() {
    let app = create_test_app();

    let files = vec![
        ("batch_struct1.txt", "First batch document."),
        ("batch_struct2.txt", "Second batch document."),
    ];

    let (boundary, body) = create_batch_multipart_body(&files);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents/upload/batch")
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={}", boundary),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    // WHY: POST file upload returns 201 Created per REST semantics (UC0002)
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = extract_json(response).await;

    // Required fields
    assert!(body.get("results").is_some(), "Missing results array");
    assert!(body.get("total_files").is_some(), "Missing total_files");
    assert!(body.get("processed").is_some(), "Missing processed");
    assert!(body.get("duplicates").is_some(), "Missing duplicates");
    assert!(body.get("failed").is_some(), "Missing failed");

    // Each file in array should have required fields
    let results = body.get("results").unwrap().as_array().unwrap();
    for file in results {
        assert!(file.get("filename").is_some(), "File missing filename");
        assert!(file.get("status").is_some(), "File missing status");
    }
}
