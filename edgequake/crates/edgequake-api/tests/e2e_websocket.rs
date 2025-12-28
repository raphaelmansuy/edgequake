//! WebSocket E2E tests for pipeline progress streaming.
//!
//! These tests verify the WebSocket functionality for real-time progress updates.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;

use edgequake_api::{create_router, state::AppState};

/// Create a test router with fresh application state.
fn create_test_app() -> axum::Router {
    let state = AppState::test_state();
    create_router(state)
}

/// Test that the WebSocket route is accessible.
#[tokio::test]
async fn test_websocket_route_exists() {
    let app = create_test_app();

    // The WebSocket upgrade requires specific headers
    let response = app
        .oneshot(
            Request::builder()
                .uri("/ws/pipeline/progress")
                .header("Connection", "Upgrade")
                .header("Upgrade", "websocket")
                .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
                .header("Sec-WebSocket-Version", "13")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 101 Switching Protocols for valid upgrade request
    // or 400/426 if WebSocket handshake fails (depends on test setup)
    // 426 Upgrade Required is also valid when the upgrade cannot be completed
    assert!(
        response.status() == StatusCode::SWITCHING_PROTOCOLS
            || response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UPGRADE_REQUIRED,
        "Expected 101 Switching Protocols, 400 Bad Request, or 426 Upgrade Required, got: {}",
        response.status()
    );
}

/// Test that the WebSocket route returns proper error without upgrade headers.
#[tokio::test]
async fn test_websocket_route_without_upgrade() {
    let app = create_test_app();

    // Request without WebSocket upgrade headers should fail
    let response = app
        .oneshot(
            Request::builder()
                .uri("/ws/pipeline/progress")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return error status (not 2xx)
    assert!(
        !response.status().is_success(),
        "Expected non-success status without upgrade headers, got: {}",
        response.status()
    );
}

/// Test that progress broadcaster can broadcast events.
#[tokio::test]
async fn test_progress_broadcaster_integration() {
    use edgequake_api::handlers::ProgressBroadcaster;

    let broadcaster = ProgressBroadcaster::new(100);

    // Create multiple subscribers
    let mut rx1 = broadcaster.subscribe();
    let mut rx2 = broadcaster.subscribe();

    // Broadcast job started
    broadcaster.job_started("test-job", 10, 2);

    // Both subscribers should receive the event
    let event1 = rx1.recv().await.unwrap();
    let event2 = rx2.recv().await.unwrap();

    // Serialize to JSON to verify structure
    let json1 = serde_json::to_string(&event1).unwrap();
    let json2 = serde_json::to_string(&event2).unwrap();

    assert!(json1.contains("JobStarted"));
    assert!(json1.contains("test-job"));
    assert_eq!(json1, json2);
}

/// Test progress event serialization format.
#[tokio::test]
async fn test_progress_event_serialization() {
    use edgequake_api::handlers::ProgressEvent;

    // Test JobStarted
    let event = ProgressEvent::JobStarted {
        job_name: "test-job".to_string(),
        total_documents: 10,
        total_batches: 2,
    };
    let json: Value = serde_json::from_str(&serde_json::to_string(&event).unwrap()).unwrap();
    assert_eq!(json["type"], "JobStarted");
    assert_eq!(json["data"]["job_name"], "test-job");
    assert_eq!(json["data"]["total_documents"], 10);
    assert_eq!(json["data"]["total_batches"], 2);

    // Test DocumentProgress
    let event = ProgressEvent::DocumentProgress {
        document_id: "doc-123".to_string(),
        entities_extracted: 5,
        processed: 3,
        total: 10,
    };
    let json: Value = serde_json::from_str(&serde_json::to_string(&event).unwrap()).unwrap();
    assert_eq!(json["type"], "DocumentProgress");
    assert_eq!(json["data"]["document_id"], "doc-123");
    assert_eq!(json["data"]["entities_extracted"], 5);
    assert_eq!(json["data"]["processed"], 3);
    assert_eq!(json["data"]["total"], 10);

    // Test DocumentFailed
    let event = ProgressEvent::DocumentFailed {
        document_id: "doc-456".to_string(),
        error: "Parse error".to_string(),
        processed: 4,
        total: 10,
    };
    let json: Value = serde_json::from_str(&serde_json::to_string(&event).unwrap()).unwrap();
    assert_eq!(json["type"], "DocumentFailed");
    assert_eq!(json["data"]["document_id"], "doc-456");
    assert_eq!(json["data"]["error"], "Parse error");

    // Test BatchCompleted
    let event = ProgressEvent::BatchCompleted {
        batch: 1,
        total_batches: 3,
    };
    let json: Value = serde_json::from_str(&serde_json::to_string(&event).unwrap()).unwrap();
    assert_eq!(json["type"], "BatchCompleted");
    assert_eq!(json["data"]["batch"], 1);
    assert_eq!(json["data"]["total_batches"], 3);

    // Test JobFinished
    let event = ProgressEvent::JobFinished {
        total_processed: 10,
        duration_ms: 5000,
    };
    let json: Value = serde_json::from_str(&serde_json::to_string(&event).unwrap()).unwrap();
    assert_eq!(json["type"], "JobFinished");
    assert_eq!(json["data"]["total_processed"], 10);
    assert_eq!(json["data"]["duration_ms"], 5000);

    // Test StatusSnapshot
    let event = ProgressEvent::StatusSnapshot {
        is_busy: true,
        job_name: Some("processing".to_string()),
        processed_documents: 5,
        total_documents: 10,
        current_batch: 2,
        total_batches: 3,
    };
    let json: Value = serde_json::from_str(&serde_json::to_string(&event).unwrap()).unwrap();
    assert_eq!(json["type"], "StatusSnapshot");
    assert_eq!(json["data"]["is_busy"], true);
    assert_eq!(json["data"]["job_name"], "processing");

    // Test Heartbeat
    let event = ProgressEvent::Heartbeat {
        timestamp: "2024-01-01T00:00:00Z".to_string(),
    };
    let json: Value = serde_json::from_str(&serde_json::to_string(&event).unwrap()).unwrap();
    assert_eq!(json["type"], "Heartbeat");
    assert!(json["data"]["timestamp"].is_string());

    // Test Connected
    let event = ProgressEvent::Connected {
        message: "Welcome".to_string(),
    };
    let json: Value = serde_json::from_str(&serde_json::to_string(&event).unwrap()).unwrap();
    assert_eq!(json["type"], "Connected");
    assert_eq!(json["data"]["message"], "Welcome");

    // Test Message
    let event = ProgressEvent::Message {
        level: "info".to_string(),
        message: "Processing...".to_string(),
        timestamp: "2024-01-01T00:00:00Z".to_string(),
    };
    let json: Value = serde_json::from_str(&serde_json::to_string(&event).unwrap()).unwrap();
    assert_eq!(json["type"], "Message");
    assert_eq!(json["data"]["level"], "info");
    assert_eq!(json["data"]["message"], "Processing...");

    // Test CancellationRequested
    let event = ProgressEvent::CancellationRequested;
    let json: Value = serde_json::from_str(&serde_json::to_string(&event).unwrap()).unwrap();
    assert_eq!(json["type"], "CancellationRequested");
}

/// Test broadcaster channel capacity.
#[tokio::test]
async fn test_broadcaster_channel_capacity() {
    use edgequake_api::handlers::ProgressBroadcaster;

    // Create broadcaster with larger capacity to ensure events aren't dropped
    let broadcaster = ProgressBroadcaster::new(100);
    let mut rx = broadcaster.subscribe();

    // Send events
    for i in 0..5 {
        broadcaster.document_progress(&format!("doc-{}", i), i, i as u32, 10);
    }

    // Give a small delay to ensure events are sent
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // First few events should be received
    let first_event = rx.recv().await;
    assert!(first_event.is_ok(), "Should receive at least one event");
}

/// Test that state includes progress broadcaster.
#[tokio::test]
async fn test_app_state_has_broadcaster() {
    let state = AppState::test_state();

    // The broadcaster should be accessible and functional
    let mut rx = state.progress_broadcaster.subscribe();

    state
        .progress_broadcaster
        .job_started("integration-test", 5, 1);

    let event = rx.recv().await.unwrap();
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("integration-test"));
}
