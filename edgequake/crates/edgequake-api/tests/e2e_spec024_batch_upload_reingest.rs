//! SPEC-024 pass 12 — batch upload duplicate re-ingest parity with single-file paths.

mod common;

use std::time::Duration;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

async fn batch_upload(app: &axum::Router, filename: &str, content: &str) -> serde_json::Value {
    let boundary = "----Spec024BatchBoundary";
    let body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"files\"; filename=\"{filename}\"\r\n\
         Content-Type: text/plain\r\n\r\n\
         {content}\r\n\
         --{boundary}--\r\n"
    );

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents/upload/batch")
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .expect("batch upload");

    assert_eq!(
        response.status(),
        StatusCode::ACCEPTED,
        "batch upload must enqueue async worker tasks"
    );

    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).expect("parse batch json")
}

async fn wait_for_document_terminal(
    app: &axum::Router,
    document_id: &str,
    timeout: Duration,
) -> String {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let (status, body) =
            common::get_endpoint(app, &format!("/api/v1/documents/{document_id}")).await;
        if status.is_success() {
            if let Some(doc_status) = body.get("status").and_then(|v| v.as_str()) {
                if matches!(
                    doc_status,
                    "completed" | "processed" | "indexed" | "partial_failure"
                ) {
                    return doc_status.to_string();
                }
            }
        }
        if std::time::Instant::now() >= deadline {
            return "timeout".to_string();
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::test]
async fn spec024_batch_duplicate_reingests_after_prior_completed() {
    let workers = common::create_test_app_with_workers().await;
    let app = workers.app();

    let content = "Batch re-ingest probe: Dr. Sarah Chen leads Zurich EdgeQuake lab.";
    let first = batch_upload(app, "reingest.txt", content).await;
    assert_eq!(first["processed"].as_u64(), Some(1));
    assert_eq!(first["duplicates"].as_u64(), Some(0));

    let first_doc_id = first["results"][0]["document_id"]
        .as_str()
        .expect("first document_id")
        .to_string();

    let first_status =
        wait_for_document_terminal(app, &first_doc_id, Duration::from_secs(90)).await;
    assert_ne!(
        first_status, "timeout",
        "first batch upload must complete before re-ingest test"
    );

    let second = batch_upload(app, "reingest.txt", content).await;
    assert_eq!(
        second["duplicates"].as_u64(),
        Some(0),
        "completed duplicate must re-ingest, not skip as duplicate"
    );
    assert_eq!(second["processed"].as_u64(), Some(1));

    let second_result = &second["results"][0];
    assert_eq!(
        second_result["status"].as_str(),
        Some("pending"),
        "re-ingest must enqueue a fresh async task"
    );
    let second_doc_id = second_result["document_id"]
        .as_str()
        .expect("second document_id");
    assert_ne!(
        second_doc_id, first_doc_id,
        "re-ingest must mint a new document id"
    );
}
