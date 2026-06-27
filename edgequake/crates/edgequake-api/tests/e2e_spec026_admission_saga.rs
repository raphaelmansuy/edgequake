//! SPEC-026 Phase 2 — admission staging saga E2E.

mod common;

use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_storage::kv_keys;
use edgequake_storage::traits::KVStorage;
use std::time::Duration;
use tower::ServiceExt;

#[tokio::test]
async fn worker_success_promotes_staging() {
    let workers = common::create_test_app_with_workers().await;
    let app = workers.app();

    let body = serde_json::json!({
        "content": "Staging promote: Dr. Sarah Chen leads EdgeQuake.",
        "title": "spec026-staging-ok.txt",
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

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let doc_id = parsed["document_id"].as_str().unwrap();
    let track_id = parsed["track_id"].as_str().unwrap();

    assert_eq!(
        common::wait_for_document_processed(app, track_id, Duration::from_secs(90)).await,
        "completed"
    );

    assert!(workers
        .kv_storage
        .get_by_id(&kv_keys::doc_content(doc_id))
        .await
        .unwrap()
        .is_some());
    assert!(workers
        .kv_storage
        .get_by_id(&kv_keys::staging_doc_content(doc_id))
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn duplicate_hash_during_staging_rejected() {
    let workers = common::create_test_app_with_workers().await;
    let app = workers.app();

    let content = "Duplicate in-flight: Dr. Sarah Chen leads EdgeQuake staging saga.";
    let (status, body) = common::upload_document(app, "spec026-dup-a.txt", content).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let first_doc_id = body["document_id"].as_str().unwrap().to_string();

    let (status2, body2) = common::upload_document(app, "spec026-dup-b.txt", content).await;
    assert_eq!(status2, StatusCode::OK);
    assert_eq!(body2["status"].as_str(), Some("duplicate_processing"));
    assert_eq!(body2["document_id"].as_str().unwrap(), first_doc_id);
}

#[tokio::test]
async fn worker_failure_leaves_no_final_kv() {
    let kv: Arc<dyn KVStorage> = Arc::new(
        edgequake_storage::adapters::memory::MemoryKVStorage::new("e2e"),
    );
    let doc_id = "failed-doc-e2e";
    let ws = common::TEST_WORKSPACE_ID;
    let hash = "simulated-failure";

    kv.upsert(&[(
        kv_keys::staging_doc_content(doc_id),
        serde_json::json!({"content": "simulated failure"}),
    )])
    .await
    .unwrap();

    edgequake_api::services::rollback_staging(&kv, doc_id, ws, hash)
        .await
        .expect("rollback");

    assert!(kv
        .get_by_id(&kv_keys::doc_content(doc_id))
        .await
        .unwrap()
        .is_none());
    assert!(kv
        .get_by_id(&kv_keys::staging_doc_content(doc_id))
        .await
        .unwrap()
        .is_none());
}
