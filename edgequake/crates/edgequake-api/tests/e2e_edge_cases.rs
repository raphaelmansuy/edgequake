//! OODA-15: Edge case E2E tests.
//!
//! Covers boundary conditions and unusual inputs not handled by
//! e2e_data_model.rs (which covers empty/whitespace/unicode basics).
//!
//! Focus areas:
//! 1. Very large documents
//! 2. Binary/null bytes
//! 3. Path traversal in titles
//! 4. Deeply nested metadata
//! 5. Extra unknown JSON fields
//! 6. Mixed line endings
//! 7. Rapid sequential uploads
//! 8. Code-like content with special characters

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
    let bytes = axum::body::to_bytes(response.into_body(), 2 * 1024 * 1024)
        .await
        .expect("Failed to read response body");
    serde_json::from_slice(&bytes).expect("Failed to parse JSON")
}

async fn upload_raw(app: &axum::Router, body: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents")
                .header("Content-Type", "application/json")
                .body(Body::from(body.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = extract_json(response).await;
    (status, body)
}

async fn upload_json(app: &axum::Router, payload: &Value) -> (StatusCode, Value) {
    upload_raw(app, &serde_json::to_string(payload).unwrap()).await
}

// ============================================================================
// Large Document Tests
// ============================================================================

/// OODA-15: Upload a large document (~50 KB of text).
/// WHY: Validates chunking works for large inputs without OOM or timeout.
#[tokio::test]
async fn test_large_document_upload() {
    let result = with_timeout(Duration::from_secs(30), async {
        let app = create_test_app();

        // Generate ~50KB of text (repeated paragraphs)
        let paragraph = "The quick brown fox jumps over the lazy dog. \
            Sphinx of black quartz, judge my vow. Pack my box with five dozen liquor jugs. \
            How vexingly quick daft zebras jump. The five boxing wizards jump quickly.\n\n";
        let large_content: String = paragraph.repeat(300); // ~50KB
        assert!(large_content.len() > 40_000, "Content should be >40KB");

        let payload = json!({
            "content": large_content,
            "title": "Large Document Test"
        });

        let (status, body) = upload_json(&app, &payload).await;
        assert_eq!(
            status,
            StatusCode::CREATED,
            "Large doc should upload successfully"
        );
        assert!(body["document_id"].is_string());
        assert_eq!(body["status"].as_str(), Some("processed"));

        body
    })
    .await;

    assert!(result.is_ok(), "Large doc: {}", result.unwrap_err());
}

// ============================================================================
// Content with null bytes / binary data
// ============================================================================

/// OODA-15: Content containing null bytes should be handled gracefully.
/// WHY: Binary data in text uploads must not crash the pipeline.
#[tokio::test]
async fn test_content_with_null_bytes() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        // Content with null bytes embedded
        let content = "Normal text\0with null\0bytes embedded.";
        let payload = json!({
            "content": content,
            "title": "Null Bytes Test"
        });

        let (status, body) = upload_json(&app, &payload).await;

        // Should either succeed or return a clear error, but NOT panic
        assert!(
            status == StatusCode::CREATED
                || status == StatusCode::OK
                || status == StatusCode::BAD_REQUEST
                || status == StatusCode::UNPROCESSABLE_ENTITY,
            "Null bytes should be handled, got {}",
            status
        );

        // If created, should have a document_id
        if status == StatusCode::CREATED {
            assert!(body["document_id"].is_string());
        }

        (status, body)
    })
    .await;

    assert!(result.is_ok(), "Null bytes: {}", result.unwrap_err());
}

// ============================================================================
// Path Traversal in Title
// ============================================================================

/// OODA-15: Title with path traversal characters should not escape storage.
/// WHY: Security — ensure ../../ in titles doesn't cause directory traversal.
#[tokio::test]
async fn test_title_path_traversal() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        let payload = json!({
            "content": "Safe content for path traversal test.",
            "title": "../../etc/passwd"
        });

        let (status, body) = upload_json(&app, &payload).await;
        // Should succeed (title is just metadata, not a file path)
        assert_eq!(
            status,
            StatusCode::CREATED,
            "Path traversal in title should be treated as plain text"
        );
        assert!(body["document_id"].is_string());

        body
    })
    .await;

    assert!(result.is_ok(), "Path traversal: {}", result.unwrap_err());
}

/// OODA-15: Very long title (1000+ chars) should be handled.
#[tokio::test]
async fn test_very_long_title() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        let long_title = "A".repeat(2000);
        let payload = json!({
            "content": "Content with a very long title.",
            "title": long_title
        });

        let (status, body) = upload_json(&app, &payload).await;
        // Should either succeed or return validation error, not crash
        assert!(
            status == StatusCode::CREATED
                || status == StatusCode::BAD_REQUEST
                || status == StatusCode::UNPROCESSABLE_ENTITY,
            "Long title should be handled, got {}",
            status
        );

        (status, body)
    })
    .await;

    assert!(result.is_ok(), "Long title: {}", result.unwrap_err());
}

// ============================================================================
// Metadata Edge Cases
// ============================================================================

/// OODA-15: Deeply nested metadata should not cause stack overflow.
#[tokio::test]
async fn test_deeply_nested_metadata() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        // Build 20 levels of nesting
        let mut nested = json!({"leaf": true});
        for i in 0..20 {
            nested = json!({ format!("level_{}", i): nested });
        }

        let payload = json!({
            "content": "Content with deeply nested metadata.",
            "title": "Nested Metadata",
            "metadata": nested
        });

        let (status, body) = upload_json(&app, &payload).await;
        assert_eq!(
            status,
            StatusCode::CREATED,
            "Deeply nested metadata should be accepted"
        );
        assert!(body["document_id"].is_string());

        body
    })
    .await;

    assert!(result.is_ok(), "Nested meta: {}", result.unwrap_err());
}

/// OODA-15: Extra unknown fields in JSON body should be ignored.
/// WHY: Forward compatibility — clients may send newer fields.
#[tokio::test]
async fn test_extra_unknown_json_fields() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        let payload = json!({
            "content": "Content with extra fields.",
            "title": "Extra Fields Test",
            "unknown_field_1": "should be ignored",
            "unknown_field_2": 42,
            "future_feature": {"enabled": true}
        });

        let (status, body) = upload_json(&app, &payload).await;

        // Should succeed — serde(deny_unknown_fields) should NOT be used on public API
        assert_eq!(
            status,
            StatusCode::CREATED,
            "Extra fields should be silently ignored, got {} body: {}",
            status,
            body
        );

        body
    })
    .await;

    assert!(result.is_ok(), "Extra fields: {}", result.unwrap_err());
}

// ============================================================================
// Line Ending & Whitespace Edge Cases
// ============================================================================

/// OODA-15: Content with mixed line endings (\r\n, \n, \r).
/// WHY: Windows/Mac/Unix line endings in same document.
#[tokio::test]
async fn test_mixed_line_endings() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        let content = "Line one\r\nLine two\nLine three\rLine four\r\n\r\nParagraph two.";
        let payload = json!({
            "content": content,
            "title": "Mixed Line Endings"
        });

        let (status, body) = upload_json(&app, &payload).await;
        assert_eq!(
            status,
            StatusCode::CREATED,
            "Mixed line endings should be handled"
        );
        assert_eq!(body["status"].as_str(), Some("processed"));

        body
    })
    .await;

    assert!(result.is_ok(), "Line endings: {}", result.unwrap_err());
}

/// OODA-15: Content with only newlines and tabs (no visible text).
#[tokio::test]
async fn test_content_only_newlines_tabs() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        let content = "\n\n\t\t\n\t\n\n";
        let payload = json!({
            "content": content,
            "title": "Only Newlines"
        });

        let (status, _body) = upload_json(&app, &payload).await;

        // Should either succeed (content is technically non-empty) or reject
        assert!(
            status == StatusCode::CREATED
                || status == StatusCode::BAD_REQUEST
                || status == StatusCode::UNPROCESSABLE_ENTITY,
            "Newline-only content should be handled, got {}",
            status
        );

        status
    })
    .await;

    assert!(result.is_ok(), "Newlines: {}", result.unwrap_err());
}

// ============================================================================
// Rapid Sequential Upload Test
// ============================================================================

/// OODA-15: Rapid sequential uploads should not corrupt state.
/// WHY: Tests that in-memory storage handles serial writes correctly.
#[tokio::test]
async fn test_rapid_sequential_uploads() {
    let result = with_timeout(Duration::from_secs(30), async {
        let app = create_test_app();

        let mut doc_ids = Vec::new();
        for i in 0..10 {
            let content = format!(
                "Rapid upload document number {}. Each has unique content to avoid dedup.",
                i
            );
            let payload = json!({
                "content": content,
                "title": format!("Rapid {}", i)
            });

            let (status, body) = upload_json(&app, &payload).await;
            assert_eq!(status, StatusCode::CREATED, "Upload {} should succeed", i);
            doc_ids.push(body["document_id"].as_str().unwrap().to_string());
        }

        // All IDs should be unique
        let unique: std::collections::HashSet<&str> = doc_ids.iter().map(|s| s.as_str()).collect();
        assert_eq!(doc_ids.len(), unique.len(), "All doc IDs should be unique");

        // List documents should show them all
        let list_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/documents")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(list_resp.status(), StatusCode::OK);
        let list = extract_json(list_resp).await;
        let docs = list["documents"].as_array().unwrap();
        assert!(
            docs.len() >= 10,
            "Should have at least 10 documents, got {}",
            docs.len()
        );

        doc_ids
    })
    .await;

    assert!(result.is_ok(), "Rapid uploads: {}", result.unwrap_err());
}

/// OODA-15: Content resembling code with many special chars.
/// WHY: Code snippets in documents must not break JSON parsing or storage.
#[tokio::test]
async fn test_code_content_special_chars() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        let code_content = "fn main() {\n\
            let x: Vec<&str> = vec![\"hello\", \"world\"];\n\
            println!(\"{:?}\", x);\n\
            let json_str = \"{\\\"key\\\": \\\"value\\\"}\";\n\
            // Special chars: < > & \" ' \\\\ / \\n \\t\n\
            let regex = \"\\\\d+\\\\.\\\\d+\";\n\
            assert!(x.len() == 2);\n\
        }";

        let payload = json!({
            "content": code_content,
            "title": "Code Content with <angle> & 'quotes'"
        });

        let (status, body) = upload_json(&app, &payload).await;
        assert_eq!(
            status,
            StatusCode::CREATED,
            "Code content should be accepted"
        );
        assert!(body["document_id"].is_string());

        body
    })
    .await;

    assert!(result.is_ok(), "Code content: {}", result.unwrap_err());
}
