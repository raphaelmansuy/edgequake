//! End-to-end tests for document API endpoints.
//!
//! Tests cover:
//! - Document upload (POST /api/v1/documents)
//! - List documents (GET /api/v1/documents)
//! - Get document by ID (GET /api/v1/documents/{document_id})
//! - Delete document (DELETE /api/v1/documents/{document_id})
//!
//! P-G2b: uploads always enqueue a background task (202 Accepted + pending).
//! Tests that need a fully-ingested document use `create_test_app_with_workers`
//! (which runs a real WorkerPool mirroring production) plus the
//! `upload_and_wait` / `wait_for_document_processed` polling helpers.

mod common;
use common::{
    create_test_app, create_test_app_with_workers, get_endpoint, post_json, upload_and_wait,
    upload_document_assert, wait_for_document_processed,
};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;
use std::time::Duration;
use tower::ServiceExt;

// ============================================================================
// Document Upload Tests
// ============================================================================

#[tokio::test]
async fn test_upload_document_success() {
    let workers = create_test_app_with_workers().await;
    let app = &workers.router;

    let (document_id, track_id, final_status) = upload_and_wait(
        app,
        "AI Overview",
        "This is a test document about artificial intelligence and machine learning. AI systems are becoming increasingly sophisticated.",
        Duration::from_secs(30),
    )
    .await;

    assert!(!document_id.is_empty());
    assert!(!track_id.is_empty());
    assert!(
        final_status == "completed"
            || final_status == "processed"
            || final_status == "indexed"
            || final_status == "partial_failure",
        "final status: {}",
        final_status
    );
}

#[tokio::test]
async fn test_upload_document_minimal() {
    let app = create_test_app();

    let body =
        upload_document_assert(&app, "Minimal", "A minimal document with just content.").await;
    assert!(body.get("document_id").is_some());
    // P-G2b: status is pending until the background task finishes.
    assert_eq!(body.get("status").and_then(|v| v.as_str()), Some("pending"));
}

#[tokio::test]
async fn test_upload_document_empty_content() {
    let app = create_test_app();

    let (status, _body) = post_json(&app, "/api/v1/documents", &json!({"content": ""})).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_upload_document_whitespace_only() {
    let app = create_test_app();

    let (status, _body) =
        post_json(&app, "/api/v1/documents", &json!({"content": "   \n\t   "})).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

/// Test that multipart form data is rejected on /api/v1/documents endpoint.
#[tokio::test]
async fn test_upload_document_rejects_multipart() {
    let app = create_test_app();

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

    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

/// Test that the correct endpoint (/api/v1/documents/upload) accepts multipart.
#[tokio::test]
async fn test_upload_endpoint_accepts_multipart() {
    let app = create_test_app();

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

    // file_upload.rs already enqueues + returns 201 Created.
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
            "nested": { "field": "value" }
        }
    });

    let (status, body) = post_json(&app, "/api/v1/documents", &request).await;
    assert!(
        status == StatusCode::CREATED || status == StatusCode::ACCEPTED,
        "upload status: {} | body={}",
        status,
        body
    );
    assert!(body.get("document_id").is_some());
}

// ============================================================================
// List Documents Tests
// ============================================================================

#[tokio::test]
async fn test_list_documents_empty() {
    let app = create_test_app();

    let (status, body) = get_endpoint(&app, "/api/v1/documents").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.get("documents").is_some());
    assert!(body.get("total").is_some());
    assert!(body.get("page").is_some());
    assert!(body.get("page_size").is_some());
}

#[tokio::test]
async fn test_list_documents_after_upload() {
    let workers = create_test_app_with_workers().await;
    let app = &workers.router;

    let _ = upload_and_wait(
        app,
        "Listing doc",
        "Test document for listing. Contains information about software development.",
        Duration::from_secs(30),
    )
    .await;

    // P-G2b: list endpoint filters by tenant/workspace context; provide the
    // default tenant/workspace headers so the uploaded document is visible.
    let (status, body) =
        common::get_with_tenant(app, "/api/v1/documents", "default", "default", "default").await;
    assert_eq!(status, StatusCode::OK);

    let docs = body.get("documents").and_then(|v| v.as_array());
    assert!(docs.is_some());
    assert!(
        !docs.unwrap().is_empty(),
        "uploaded document should appear in list"
    );
}

// ============================================================================
// Get Document Tests
// ============================================================================

#[tokio::test]
async fn test_get_document_success() {
    let workers = create_test_app_with_workers().await;
    let app = &workers.router;

    let (document_id, _track_id, _final_status) = upload_and_wait(
        app,
        "Retrieval doc",
        "Test document for retrieval. This document discusses programming languages.",
        Duration::from_secs(30),
    )
    .await;

    let (status, body) = get_endpoint(app, &format!("/api/v1/documents/{}", document_id)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body.get("id").and_then(|v| v.as_str()),
        Some(document_id.as_str())
    );
    assert!(body.get("status").is_some());
}

#[tokio::test]
async fn test_get_document_not_found() {
    let app = create_test_app();

    let (status, _body) = get_endpoint(&app, "/api/v1/documents/nonexistent-doc-id").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// Delete Document Tests
// ============================================================================

#[tokio::test]
async fn test_delete_document_success() {
    let workers = create_test_app_with_workers().await;
    let app = &workers.router;

    let (document_id, _track_id, _final_status) = upload_and_wait(
        app,
        "Deletion doc",
        "Document to be deleted. Contains some test content.",
        Duration::from_secs(30),
    )
    .await;

    let (status, body) =
        common::delete_endpoint(app, &format!("/api/v1/documents/{}", document_id)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body.get("document_id").and_then(|v| v.as_str()),
        Some(document_id.as_str())
    );
    assert_eq!(body.get("deleted").and_then(|v| v.as_bool()), Some(true));

    // Verify document is gone.
    let (get_status, _body) =
        get_endpoint(app, &format!("/api/v1/documents/{}", document_id)).await;
    assert_eq!(get_status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_document_not_found() {
    let app = create_test_app();

    let (status, _body) =
        common::delete_endpoint(&app, "/api/v1/documents/nonexistent-doc-id").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// Integration Flow Tests
// ============================================================================

#[tokio::test]
async fn test_complete_document_lifecycle() {
    let workers = create_test_app_with_workers().await;
    let app = &workers.router;

    // 1. Upload + wait for processing.
    let (document_id, _track_id, _final_status) = upload_and_wait(
        app,
        "AI Introduction",
        "This is a comprehensive test document about artificial intelligence. Machine learning is a subset of AI. Deep learning uses neural networks.",
        Duration::from_secs(30),
    )
    .await;

    // 2. List documents - should include new document.
    let (list_status, list_body) =
        common::get_with_tenant(app, "/api/v1/documents", "default", "default", "default").await;
    assert_eq!(list_status, StatusCode::OK);
    let listed = list_body
        .get("documents")
        .and_then(|v| v.as_array())
        .expect("documents array");
    assert!(
        listed
            .iter()
            .any(|d| d.get("id").and_then(|v| v.as_str()) == Some(document_id.as_str())),
        "uploaded document should appear in list"
    );

    // 3. Get document by ID.
    let (get_status, get_body) =
        get_endpoint(app, &format!("/api/v1/documents/{}", document_id)).await;
    assert_eq!(get_status, StatusCode::OK);
    assert_eq!(
        get_body.get("id").and_then(|v| v.as_str()),
        Some(document_id.as_str())
    );

    // 4. Delete document.
    let (delete_status, _delete_body) =
        common::delete_endpoint(app, &format!("/api/v1/documents/{}", document_id)).await;
    assert_eq!(delete_status, StatusCode::OK);

    // 5. Verify document is gone.
    let (final_status, _final_body) =
        get_endpoint(app, &format!("/api/v1/documents/{}", document_id)).await;
    assert_eq!(final_status, StatusCode::NOT_FOUND);
}

/// P-G2b contract: a plain upload (no worker pool) returns 202 + pending +
/// task_id and the document is observable via track-status polling once a
/// worker processes it.
#[tokio::test]
async fn test_upload_returns_accepted_pending_with_task_id() {
    let workers = create_test_app_with_workers().await;
    let app = &workers.router;

    let (status, body) = post_json(
        app,
        "/api/v1/documents",
        &json!({
            "content": "Contract test document for the async upload response shape.",
            "title": "P-G2b contract"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(body.get("status").and_then(|v| v.as_str()), Some("pending"));
    let track_id = body
        .get("track_id")
        .and_then(|v| v.as_str())
        .expect("track_id present")
        .to_string();
    let task_id = body
        .get("task_id")
        .and_then(|v| v.as_str())
        .map(String::from);
    assert!(
        task_id.is_some(),
        "task_id should be present for async upload"
    );

    let final_status = wait_for_document_processed(app, &track_id, Duration::from_secs(30)).await;
    assert!(
        final_status == "completed"
            || final_status == "processed"
            || final_status == "indexed"
            || final_status == "partial_failure",
        "final status: {}",
        final_status
    );
}
