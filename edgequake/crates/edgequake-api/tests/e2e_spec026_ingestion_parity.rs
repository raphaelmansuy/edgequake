//! SPEC-026 Phase 2 — ingestion parity E2E tests.

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_storage::EntityId;
use std::time::Duration;
use tower::ServiceExt;

#[tokio::test]
async fn text_upload_recursive_strategy_default() {
    let workers = common::create_test_app_with_workers().await;
    let app = workers.app();

    let content = format!(
        "{}\n\nDr. Sarah Chen leads EdgeQuake research.",
        fixture_plain_en_multi_paragraph()
    );
    let body = serde_json::json!({
        "content": content,
        "title": "spec026-default-recursive.txt",
    });

    let response = post_document(app.clone(), &body).await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);

    let parsed: serde_json::Value =
        serde_json::from_slice(&common::spec026_multimodal::response_body_bytes(response).await)
            .unwrap();
    let doc_id = parsed["document_id"].as_str().unwrap();
    let track_id = parsed["track_id"].as_str().unwrap();
    assert_eq!(
        common::wait_for_document_processed(app, track_id, Duration::from_secs(90)).await,
        "completed"
    );

    let meta = common::doc_metadata_from_kv(&workers.kv_storage, doc_id)
        .await
        .expect("metadata");
    assert_eq!(
        meta.get("chunking_strategy").and_then(|v| v.as_str()),
        Some("recursive"),
        "upload without chunk_strategy must default to recursive (LightRAG R parity)"
    );

    let node_id = EntityId::new("Sarah Chen").as_graph_node_id().to_string();
    assert!(workers
        .graph_storage
        .get_node(&node_id)
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn text_upload_recursive_strategy() {
    let workers = common::create_test_app_with_workers().await;
    let app = workers.app();

    let content = fixture_plain_en_multi_paragraph();

    let body = serde_json::json!({
        "content": content,
        "title": "spec026-recursive.txt",
        "chunk_strategy": "recursive",
        "chunk_options": {
            "chunk_token_size": 15,
            "chunk_overlap_token_size": 0
        }
    });

    let response = post_document(app.clone(), &body).await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);

    let parsed: serde_json::Value =
        serde_json::from_slice(&common::spec026_multimodal::response_body_bytes(response).await)
            .unwrap();
    let doc_id = parsed["document_id"].as_str().unwrap();
    let track_id = parsed["track_id"].as_str().unwrap();
    assert_eq!(
        common::wait_for_document_processed(app, track_id, Duration::from_secs(90)).await,
        "completed"
    );

    let meta = common::doc_metadata_from_kv(&workers.kv_storage, doc_id)
        .await
        .expect("metadata");
    assert_eq!(
        meta.get("chunking_strategy").and_then(|v| v.as_str()),
        Some("recursive"),
        "final metadata must preserve recursive strategy"
    );
    let chunk_count = meta
        .get("chunk_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(
        chunk_count >= 3,
        "recursive should split multi-paragraph fixture into >=3 chunks, got {chunk_count}"
    );
}

#[tokio::test]
async fn recursive_splits_more_paragraphs_than_fixed_e2e() {
    let workers = common::create_test_app_with_workers().await;
    let app = workers.app();
    let base = fixture_plain_en_multi_paragraph();
    let fixed_content = format!("FIXED-MARKER\n{base}");
    let rec_content = format!("RECURSIVE-MARKER\n{base}");

    let (status, fixed_body) = common::upload_document_with_options(
        app,
        "spec026-fixed-compare.txt",
        &fixed_content,
        Some(serde_json::json!({
            "chunk_strategy": "fixed",
            "chunk_options": { "chunk_token_size": 15, "chunk_overlap_token_size": 0 }
        })),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let fixed_id = fixed_body["document_id"].as_str().unwrap();
    let fixed_track = fixed_body["track_id"].as_str().unwrap();
    assert_eq!(
        common::wait_for_document_processed(app, fixed_track, Duration::from_secs(90)).await,
        "completed"
    );

    let (status2, rec_body) = common::upload_document_with_options(
        app,
        "spec026-recursive-compare.txt",
        &rec_content,
        Some(serde_json::json!({
            "chunk_strategy": "recursive",
            "chunk_options": { "chunk_token_size": 15, "chunk_overlap_token_size": 0 }
        })),
    )
    .await;
    assert_eq!(status2, StatusCode::ACCEPTED);
    let rec_id = rec_body["document_id"].as_str().unwrap();
    let rec_track = rec_body["track_id"].as_str().unwrap();
    assert_eq!(
        common::wait_for_document_processed(app, rec_track, Duration::from_secs(90)).await,
        "completed"
    );

    let fixed_meta = common::doc_metadata_from_kv(&workers.kv_storage, fixed_id)
        .await
        .expect("fixed doc metadata");
    let rec_meta = common::doc_metadata_from_kv(&workers.kv_storage, rec_id)
        .await
        .expect("recursive doc metadata");
    assert_eq!(
        fixed_meta.get("chunking_strategy").and_then(|v| v.as_str()),
        Some("fixed")
    );
    assert_eq!(
        rec_meta.get("chunking_strategy").and_then(|v| v.as_str()),
        Some("recursive")
    );
    let fixed_chunks = fixed_meta
        .get("chunk_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let rec_chunks = rec_meta
        .get("chunk_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(
        rec_chunks >= 3 && fixed_chunks >= 1,
        "fixed={fixed_chunks} recursive={rec_chunks}"
    );
}

#[tokio::test]
async fn markdown_upload_auto_strategy() {
    let workers = common::create_test_app_with_workers().await;
    let app = workers.app();

    let body = serde_json::json!({
        "content": "# Guide\n\nIntro text about EdgeQuake.\n\n## Team\n\nDr. Sarah Chen leads the team.",
        "title": "spec026-guide.md",
    });

    let response = post_document(app.clone(), &body).await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);

    let parsed: serde_json::Value =
        serde_json::from_slice(&common::spec026_multimodal::response_body_bytes(response).await)
            .unwrap();
    let track_id = parsed["track_id"].as_str().unwrap();
    assert_eq!(
        common::wait_for_document_processed(app, track_id, Duration::from_secs(90)).await,
        "completed"
    );

    let node_id = EntityId::new("Sarah Chen").as_graph_node_id().to_string();
    assert!(workers
        .graph_storage
        .get_node(&node_id)
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn file_upload_default_recursive_strategy() {
    let workers = common::create_test_app_with_workers().await;
    let app = workers.app();

    let boundary = "----spec026defaultboundary";
    let content = "Default recursive file upload: Dr. Sarah Chen verified.\n\nSecond paragraph.";
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"spec026-default.txt\"\r\nContent-Type: text/plain\r\n\r\n{content}\r\n--{boundary}--\r\n"
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
        .unwrap();

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let parsed: serde_json::Value =
        serde_json::from_slice(&common::spec026_multimodal::response_body_bytes(response).await)
            .unwrap();
    let doc_id = parsed["document_id"].as_str().unwrap();
    let track_id = parsed["track_id"].as_str().unwrap();
    assert_eq!(
        common::wait_for_document_processed(app, track_id, Duration::from_secs(90)).await,
        "completed"
    );

    let meta = common::doc_metadata_from_kv(&workers.kv_storage, doc_id)
        .await
        .expect("metadata after file upload");
    assert_eq!(
        meta.get("chunking_strategy").and_then(|v| v.as_str()),
        Some("recursive"),
        "file upload without chunk_strategy must default to recursive"
    );
}

#[tokio::test]
async fn file_upload_recursive_strategy() {
    let workers = common::create_test_app_with_workers().await;
    let app = workers.app();

    let boundary = "----spec026boundary";
    let content = "File upload recursive: Dr. Sarah Chen verified.\n\nSecond paragraph.";
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"spec026.txt\"\r\nContent-Type: text/plain\r\n\r\n{content}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"chunk_strategy\"\r\n\r\nrecursive\r\n--{boundary}--\r\n"
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
        .unwrap();

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let parsed: serde_json::Value =
        serde_json::from_slice(&common::spec026_multimodal::response_body_bytes(response).await)
            .unwrap();
    let track_id = parsed["track_id"].as_str().unwrap();
    assert_eq!(
        common::wait_for_document_processed(app, track_id, Duration::from_secs(90)).await,
        "completed"
    );

    let doc_id = parsed["document_id"].as_str().unwrap();
    let meta = common::doc_metadata_from_kv(&workers.kv_storage, doc_id)
        .await
        .expect("metadata after file upload");
    assert_eq!(
        meta.get("chunking_strategy").and_then(|v| v.as_str()),
        Some("recursive")
    );
}

#[tokio::test]
async fn batch_upload_mixed_strategies() {
    let workers = common::create_test_app_with_workers().await;
    let app = workers.app();

    let (status, body) = common::upload_document_with_options(
        app,
        "spec026-fixed-batch.txt",
        "Batch fixed: Dr. Sarah Chen leads EdgeQuake.",
        Some(serde_json::json!({ "chunk_strategy": "fixed" })),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let track_id = body["track_id"].as_str().unwrap();
    assert_eq!(
        common::wait_for_document_processed(app, track_id, Duration::from_secs(90)).await,
        "completed"
    );

    let (status2, body2) = common::upload_document_with_options(
        app,
        "spec026-recursive-batch.txt",
        "Batch recursive: Dr. Sarah Chen verified chunk splits.\n\nParagraph two.",
        Some(serde_json::json!({ "chunk_strategy": "recursive" })),
    )
    .await;
    assert_eq!(status2, StatusCode::ACCEPTED);
    let track_id2 = body2["track_id"].as_str().unwrap();
    assert_eq!(
        common::wait_for_document_processed(app, track_id2, Duration::from_secs(90)).await,
        "completed"
    );

    let _ = workers;
}

async fn post_document(app: axum::Router, body: &serde_json::Value) -> axum::response::Response {
    app.oneshot(
        Request::builder()
            .method("POST")
            .uri("/api/v1/documents")
            .header("Content-Type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await
    .unwrap()
}

fn fixture_plain_en_multi_paragraph() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../edgequake-pipeline/tests/fixtures/spec026/plain_en.txt");
    std::fs::read_to_string(&path).unwrap_or_else(|_| {
        "Paragraph one has enough text for testing.\n\nParagraph two continues the document with more content.\n\nParagraph three finishes the sample.".to_string()
    })
}
