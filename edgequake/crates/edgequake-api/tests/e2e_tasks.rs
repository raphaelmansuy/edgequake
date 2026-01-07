//! End-to-end tests for task API endpoints.
//!
//! Tests cover:
//! - Get task (GET /api/v1/tasks/{track_id})
//! - List tasks (GET /api/v1/tasks)
//! - Cancel task (POST /api/v1/tasks/{track_id}/cancel)
//! - Retry task (POST /api/v1/tasks/{track_id}/retry)

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

async fn upload_document(server: &Server, content: &str) -> Value {
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

    extract_json(response).await
}

// ============================================================================
// Get Task Tests
// ============================================================================

#[tokio::test]
async fn test_get_task_not_found() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/tasks/nonexistent-track-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_task_response_fields() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Upload a document to create a task
    let _upload_body = upload_document(
        &server,
        "Document content for task testing. This will create a processing task.",
    )
    .await;

    // The task should be created (even if processed synchronously in test mode)
    // Try to get tasks list to find a task id
    let app = server.build_router();
    let list_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/tasks")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(list_response.status(), StatusCode::OK);

    let list_body = extract_json(list_response).await;
    let tasks = list_body.get("tasks").and_then(|v| v.as_array());

    if let Some(tasks) = tasks {
        if let Some(first_task) = tasks.first() {
            // Verify task has required fields
            assert!(first_task.get("track_id").is_some());
            assert!(first_task.get("task_type").is_some());
            assert!(first_task.get("status").is_some());
            assert!(first_task.get("created_at").is_some());
            assert!(first_task.get("updated_at").is_some());
        }
    }
}

// ============================================================================
// List Tasks Tests
// ============================================================================

#[tokio::test]
async fn test_list_tasks_empty() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/tasks")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    assert!(body.get("tasks").is_some());
    assert!(body.get("pagination").is_some());
    assert!(body.get("statistics").is_some());

    let pagination = body.get("pagination").unwrap();
    assert!(pagination.get("total").is_some());
    assert!(pagination.get("page").is_some());
    assert!(pagination.get("page_size").is_some());
    assert!(pagination.get("total_pages").is_some());

    let stats = body.get("statistics").unwrap();
    assert!(stats.get("pending").is_some());
    assert!(stats.get("processing").is_some());
    assert!(stats.get("indexed").is_some());
    assert!(stats.get("failed").is_some());
    assert!(stats.get("cancelled").is_some());
}

#[tokio::test]
async fn test_list_tasks_with_pagination() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/tasks?page=1&page_size=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    let pagination = body.get("pagination").unwrap();
    assert_eq!(pagination.get("page").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(
        pagination.get("page_size").and_then(|v| v.as_u64()),
        Some(10)
    );
}

#[tokio::test]
async fn test_list_tasks_page_size_limit() {
    let app = create_test_app();

    // Request page_size > 100, should be capped at 100
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/tasks?page_size=200")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    let pagination = body.get("pagination").unwrap();
    // Should be capped at 100
    assert!(
        pagination
            .get("page_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            <= 100
    );
}

#[tokio::test]
async fn test_list_tasks_with_status_filter() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/tasks?status=pending")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_tasks_with_task_type_filter() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/tasks?task_type=upload")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_tasks_with_sort() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/tasks?sort=created_at&order=desc")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_tasks_all_filters() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/tasks?status=indexed&task_type=upload&page=1&page_size=50&sort=updated_at&order=asc")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

// ============================================================================
// Cancel Task Tests
// ============================================================================

#[tokio::test]
async fn test_cancel_task_not_found() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tasks/nonexistent-task/cancel")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// Note: Testing actual task cancellation requires creating a task
// and catching it in a cancellable state, which is timing-dependent.
// In a real test suite, you'd use test fixtures or mock task storage.

// ============================================================================
// Retry Task Tests
// ============================================================================

#[tokio::test]
async fn test_retry_task_not_found() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tasks/nonexistent-task/retry")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// Note: Testing actual task retry requires a failed task, which requires
// integration with a failing LLM or storage layer.

// ============================================================================
// Task Statistics Tests
// ============================================================================

#[tokio::test]
async fn test_task_statistics_structure() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/tasks")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    let stats = body.get("statistics").expect("Should have statistics");

    // All statistics should be non-negative integers
    let pending = stats.get("pending").and_then(|v| v.as_u64());
    let processing = stats.get("processing").and_then(|v| v.as_u64());
    let indexed = stats.get("indexed").and_then(|v| v.as_u64());
    let failed = stats.get("failed").and_then(|v| v.as_u64());
    let cancelled = stats.get("cancelled").and_then(|v| v.as_u64());

    assert!(pending.is_some());
    assert!(processing.is_some());
    assert!(indexed.is_some());
    assert!(failed.is_some());
    assert!(cancelled.is_some());
}

// ============================================================================
// Integration Test
// ============================================================================

#[tokio::test]
async fn test_tasks_after_document_upload() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Get initial task count
    let app = server.build_router();
    let initial_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/tasks")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let initial_body = extract_json(initial_response).await;
    let initial_total = initial_body
        .get("pagination")
        .and_then(|p| p.get("total"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // Upload some documents
    for i in 0..3 {
        let _body = upload_document(
            &server,
            &format!(
                "Document {} for task integration test. Contains various content.",
                i
            ),
        )
        .await;
    }

    // Check task count (may or may not increase depending on implementation)
    let app = server.build_router();
    let final_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/tasks")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(final_response.status(), StatusCode::OK);

    let final_body = extract_json(final_response).await;
    let final_total = final_body
        .get("pagination")
        .and_then(|p| p.get("total"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // In synchronous test mode, tasks may be processed immediately
    // Just verify the endpoint works
    assert!(final_total >= initial_total);
}
