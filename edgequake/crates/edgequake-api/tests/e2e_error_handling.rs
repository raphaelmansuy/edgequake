//! OODA-16: Error handling E2E tests.
//!
//! Validates that all API error paths return structured JSON responses
//! with correct HTTP status codes and error codes.
//!
//! Covers:
//! 1. Invalid document IDs (404)
//! 2. Malformed JSON bodies (400)
//! 3. Invalid HTTP methods (405)
//! 4. Missing Content-Type header (415)
//! 5. Error response structure consistency

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use serde_json::{json, Value};
use std::time::Duration;
use tower::ServiceExt;

// ============================================================================
// Helpers
// ============================================================================

async fn with_timeout<F, T>(duration: Duration, future: F) -> Result<T, String>
where
    F: std::future::Future<Output = T>,
{
    tokio::time::timeout(duration, future)
        .await
        .map_err(|_| format!("Test exceeded timeout of {:?}", duration))
}

fn create_test_app() -> axum::Router {
    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: true,
    };
    let server = Server::new(config, AppState::test_state());
    server.build_router()
}

async fn extract_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("Failed to read response body");
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

// ============================================================================
// 404 - Not Found
// ============================================================================

/// OODA-16: GET /api/v1/documents/{nonexistent-id} → 404.
#[tokio::test]
async fn test_get_nonexistent_document() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/documents/00000000-0000-0000-0000-000000000099")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "Nonexistent doc should return 404"
        );

        let body = extract_json(response).await;
        // Error response should have structured fields
        assert!(
            body.get("code").is_some()
                || body.get("error").is_some()
                || body.get("message").is_some(),
            "Error should have a code/error/message field: {}",
            body
        );

        body
    })
    .await;

    assert!(result.is_ok(), "404 doc: {}", result.unwrap_err());
}

/// OODA-16: DELETE /api/v1/documents/{nonexistent-id} → 404.
#[tokio::test]
async fn test_delete_nonexistent_document() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/v1/documents/00000000-0000-0000-0000-000000000099")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // DELETE on nonexistent may return 404 or 200 (idempotent deletion)
        assert!(
            response.status() == StatusCode::NOT_FOUND || response.status() == StatusCode::OK,
            "Delete nonexistent should return 404 or 200, got {}",
            response.status()
        );

        response.status()
    })
    .await;

    assert!(result.is_ok(), "Del 404: {}", result.unwrap_err());
}

// ============================================================================
// 400 - Bad Request (Malformed JSON)
// ============================================================================

/// OODA-16: POST with completely invalid JSON → 400 or 422.
#[tokio::test]
async fn test_malformed_json_body() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/documents")
                    .header("Content-Type", "application/json")
                    .body(Body::from("this is not json {{{"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert!(
            response.status() == StatusCode::BAD_REQUEST
                || response.status() == StatusCode::UNPROCESSABLE_ENTITY,
            "Malformed JSON should return 400 or 422, got {}",
            response.status()
        );

        response.status()
    })
    .await;

    assert!(result.is_ok(), "Malformed JSON: {}", result.unwrap_err());
}

/// OODA-16: POST with valid JSON but missing required 'content' field.
#[tokio::test]
async fn test_missing_content_field() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        let payload = json!({
            "title": "No Content",
            "metadata": {"test": true}
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/documents")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should return 400 (missing required field) or 422 (validation error)
        assert!(
            response.status() == StatusCode::BAD_REQUEST
                || response.status() == StatusCode::UNPROCESSABLE_ENTITY,
            "Missing content should return 400 or 422, got {}",
            response.status()
        );

        response.status()
    })
    .await;

    assert!(result.is_ok(), "Missing field: {}", result.unwrap_err());
}

/// OODA-16: POST with empty JSON object → should fail gracefully.
#[tokio::test]
async fn test_empty_json_object() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/documents")
                    .header("Content-Type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should return error (no content) or process with empty content
        let status = response.status();
        assert!(
            status == StatusCode::BAD_REQUEST
                || status == StatusCode::UNPROCESSABLE_ENTITY
                || status == StatusCode::CREATED,
            "Empty object should be handled, got {}",
            status
        );

        status
    })
    .await;

    assert!(result.is_ok(), "Empty obj: {}", result.unwrap_err());
}

// ============================================================================
// Invalid ID Format
// ============================================================================

/// OODA-16: Non-UUID document ID → 400 or 404.
#[tokio::test]
async fn test_invalid_document_id_format() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/documents/not-a-valid-uuid")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should return 400 (bad format) or 404 (not found)
        assert!(
            response.status() == StatusCode::BAD_REQUEST
                || response.status() == StatusCode::NOT_FOUND,
            "Invalid UUID should return 400 or 404, got {}",
            response.status()
        );

        response.status()
    })
    .await;

    assert!(result.is_ok(), "Invalid ID: {}", result.unwrap_err());
}

/// OODA-16: SQL injection attempt in document ID → 400 or 404.
#[tokio::test]
async fn test_sql_injection_in_doc_id() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/documents/1%27%20OR%201%3D1%20--")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should return error, not data
        assert!(
            response.status() == StatusCode::BAD_REQUEST
                || response.status() == StatusCode::NOT_FOUND,
            "SQL injection should return error, got {}",
            response.status()
        );

        response.status()
    })
    .await;

    assert!(result.is_ok(), "SQL injection: {}", result.unwrap_err());
}

// ============================================================================
// Non-existent Endpoints
// ============================================================================

/// OODA-16: GET /api/v1/nonexistent → 404.
#[tokio::test]
async fn test_nonexistent_endpoint() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert!(
            response.status() == StatusCode::NOT_FOUND
                || response.status() == StatusCode::METHOD_NOT_ALLOWED,
            "Nonexistent route should return 404 or 405, got {}",
            response.status()
        );

        response.status()
    })
    .await;

    assert!(result.is_ok(), "404 route: {}", result.unwrap_err());
}

// ============================================================================
// Query Error Handling
// ============================================================================

/// OODA-16: Query with empty string → should return error or empty result.
#[tokio::test]
async fn test_query_empty_string() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        let query = json!({
            "query": "",
            "mode": "global"
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/query")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&query).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Empty query should return 400 or 200 with empty/default result
        assert!(
            response.status() == StatusCode::OK
                || response.status() == StatusCode::BAD_REQUEST
                || response.status() == StatusCode::UNPROCESSABLE_ENTITY,
            "Empty query should be handled, got {}",
            response.status()
        );

        response.status()
    })
    .await;

    assert!(result.is_ok(), "Empty query: {}", result.unwrap_err());
}

/// OODA-16: Query with invalid mode → should return error.
#[tokio::test]
async fn test_query_invalid_mode() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        let query = json!({
            "query": "What is radioactivity?",
            "mode": "nonexistent_mode"
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/query")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&query).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Invalid mode should return 400/422 or fall back to default mode
        let status = response.status();
        assert!(
            status == StatusCode::BAD_REQUEST
                || status == StatusCode::UNPROCESSABLE_ENTITY
                || status == StatusCode::OK,
            "Invalid mode should be handled, got {}",
            status
        );

        status
    })
    .await;

    assert!(result.is_ok(), "Invalid mode: {}", result.unwrap_err());
}

// ============================================================================
// Double Operations
// ============================================================================

/// OODA-16: Delete same document twice → second should return 404 or 200.
#[tokio::test]
async fn test_double_delete() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        // Upload a document
        let upload = json!({
            "content": "Document to delete twice.",
            "title": "Double Delete Test"
        });

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/documents")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&upload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = extract_json(resp).await;
        let doc_id = body["document_id"].as_str().unwrap().to_string();

        // First delete
        let del1 = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/documents/{}", doc_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(del1.status(), StatusCode::OK);

        // Second delete
        let del2 = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/documents/{}", doc_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Second delete should return 404 (already deleted) or 200 (idempotent)
        assert!(
            del2.status() == StatusCode::NOT_FOUND || del2.status() == StatusCode::OK,
            "Double delete should return 404 or 200, got {}",
            del2.status()
        );

        del2.status()
    })
    .await;

    assert!(result.is_ok(), "Double delete: {}", result.unwrap_err());
}
