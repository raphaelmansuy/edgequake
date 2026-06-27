//! SPEC-024 Phase 1.1 — async multipart file upload via worker + IngestionPersister.

mod common;

use std::time::Duration;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_storage::EntityId;
use tower::ServiceExt;

#[tokio::test]
async fn spec024_async_file_upload_persists_graph_via_worker() {
    let workers = common::create_test_app_with_workers().await;
    let app = workers.app();

    let boundary = "----Spec024Boundary";
    let body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"spec024.txt\"\r\n\
         Content-Type: text/plain\r\n\r\n\
         Dr. Sarah Chen leads the EdgeQuake research lab in Zurich.\r\n\
         --{boundary}--\r\n"
    );

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents/upload")
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .expect("upload oneshot");

    assert_eq!(
        response.status(),
        StatusCode::ACCEPTED,
        "file upload must enqueue async worker (202)"
    );

    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let track_id = parsed["track_id"]
        .as_str()
        .expect("track_id for polling")
        .to_string();
    let doc_id = parsed["document_id"].as_str().unwrap().to_string();

    let final_status =
        common::wait_for_document_processed(app, &track_id, Duration::from_secs(90)).await;
    assert_eq!(
        final_status, "completed",
        "async file upload must complete via worker (doc={doc_id})"
    );

    let node_id = EntityId::new("Sarah Chen").as_graph_node_id().to_string();
    assert!(
        workers
            .graph_storage
            .get_node(&node_id)
            .await
            .expect("graph read")
            .is_some(),
        "doc {doc_id} must persist SARAH_CHEN after async file upload"
    );
}
