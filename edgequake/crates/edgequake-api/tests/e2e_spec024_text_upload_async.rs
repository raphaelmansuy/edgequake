//! SPEC-024 pass 12 — JSON text upload always async via worker queue.

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_storage::EntityId;
use std::time::Duration;
use tower::ServiceExt;

#[tokio::test]
async fn spec024_text_upload_returns_202_and_persists_via_worker() {
    let workers = common::create_test_app_with_workers().await;
    let app = workers.app();

    let body = serde_json::json!({
        "content": "Text upload async: Dr. Sarah Chen coordinates EdgeQuake research.",
        "title": "spec024-text.txt",
        "async_processing": false
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents")
                .header("Content-Type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::ACCEPTED,
        "text upload must enqueue worker even when async_processing=false (deprecated)"
    );

    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let track_id = parsed["track_id"].as_str().unwrap();

    let final_status =
        common::wait_for_document_processed(app, track_id, Duration::from_secs(90)).await;
    assert_eq!(final_status, "completed");

    let node_id = EntityId::new("Sarah Chen").as_graph_node_id().to_string();
    assert!(workers
        .graph_storage
        .get_node(&node_id)
        .await
        .expect("graph read")
        .is_some());
}
