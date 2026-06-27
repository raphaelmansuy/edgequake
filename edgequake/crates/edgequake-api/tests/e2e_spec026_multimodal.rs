//! SPEC-026 Phase 4 — multimodal ingest E2E tests.

mod common;

use common::spec026_multimodal::{
    allow_tiny_images_in_tests, parse_accepted_upload, png_upload_request,
    restore_vlm_image_limits, text_upload_request, MOCK_VLM_SARAH_JSON,
};
use edgequake_storage::EntityId;
use serial_test::serial;
use std::time::Duration;
use tower::ServiceExt;

#[tokio::test]
#[serial]
async fn image_upload_vlm_describe_completes() {
    allow_tiny_images_in_tests();
    let workers = common::create_test_app_with_llm_responses(&[MOCK_VLM_SARAH_JSON]).await;
    let app = workers.app();

    let (doc_id, track_id) = parse_accepted_upload(
        app.clone()
            .oneshot(png_upload_request("----spec026pngboundary", "spec026.png"))
            .await
            .unwrap(),
    )
    .await;

    assert_eq!(
        common::wait_for_document_processed(app, &track_id, Duration::from_secs(90)).await,
        "completed"
    );

    let meta = common::doc_metadata_from_kv(&workers.kv_storage, &doc_id)
        .await
        .expect("metadata");
    assert_eq!(meta.get("multimodal").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        meta.get("ingest_mode").and_then(|v| v.as_str()),
        Some("vlm_describe")
    );
    assert_eq!(
        meta.get("source_type").and_then(|v| v.as_str()),
        Some("image")
    );

    let manifest_key = format!("{doc_id}-multimodal-manifest");
    let manifest = workers.kv_storage.get_by_id(&manifest_key).await.unwrap();
    assert!(
        manifest.is_some(),
        "standalone image should persist KV manifest"
    );
    let item_count = manifest
        .as_ref()
        .and_then(|v| v.get("items"))
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    assert_eq!(item_count, 1);

    restore_vlm_image_limits();
}

#[tokio::test]
#[serial]
async fn image_upload_entities_extracted() {
    allow_tiny_images_in_tests();
    let workers = common::create_test_app_with_llm_responses(&[MOCK_VLM_SARAH_JSON]).await;
    let app = workers.app();

    let (_, track_id) = parse_accepted_upload(
        app.clone()
            .oneshot(png_upload_request("----spec026pngentity", "spec026.png"))
            .await
            .unwrap(),
    )
    .await;

    assert_eq!(
        common::wait_for_document_processed(app, &track_id, Duration::from_secs(90)).await,
        "completed"
    );

    let node_id = EntityId::new("Sarah Chen").as_graph_node_id().to_string();
    assert!(workers
        .graph_storage
        .get_node(&node_id)
        .await
        .unwrap()
        .is_some());
    restore_vlm_image_limits();
}

#[tokio::test]
#[serial]
async fn image_upload_metadata_ingest_mode() {
    allow_tiny_images_in_tests();
    let workers = common::create_test_app_with_llm_responses(&[MOCK_VLM_SARAH_JSON]).await;
    let app = workers.app();

    let (doc_id, track_id) = parse_accepted_upload(
        app.clone()
            .oneshot(png_upload_request("----spec026ingestmode", "spec026.png"))
            .await
            .unwrap(),
    )
    .await;

    assert_eq!(
        common::wait_for_document_processed(app, &track_id, Duration::from_secs(90)).await,
        "completed"
    );
    let meta = common::doc_metadata_from_kv(&workers.kv_storage, &doc_id)
        .await
        .expect("metadata");
    assert_eq!(
        meta.get("ingest_mode").and_then(|v| v.as_str()),
        Some("vlm_describe")
    );
    restore_vlm_image_limits();
}

/// LightRAG `test_analyze_multimodal_skips_tiny_image_without_vlm_call` parity.
#[tokio::test]
#[serial]
async fn tiny_image_upload_skips_vlm_with_default_limits() {
    restore_vlm_image_limits();
    let workers = common::create_test_app_with_workers().await;
    let app = workers.app();

    let (doc_id, track_id) = parse_accepted_upload(
        app.clone()
            .oneshot(png_upload_request("----spec026tinyskip", "tiny.png"))
            .await
            .unwrap(),
    )
    .await;

    assert_eq!(
        common::wait_for_document_processed(app, &track_id, Duration::from_secs(90)).await,
        "completed"
    );

    let meta = common::doc_metadata_from_kv(&workers.kv_storage, &doc_id)
        .await
        .expect("metadata");
    assert_eq!(
        meta.get("ingest_mode").and_then(|v| v.as_str()),
        Some("vlm_skipped")
    );
}

#[tokio::test]
#[serial]
async fn text_upload_not_multimodal() {
    let workers = common::create_test_app_with_workers().await;
    let app = workers.app();

    let (doc_id, track_id) = parse_accepted_upload(
        app.clone()
            .oneshot(text_upload_request(
                "spec026-non-multimodal.txt",
                "Plain text: Dr. Sarah Chen.",
            ))
            .await
            .unwrap(),
    )
    .await;

    assert_eq!(
        common::wait_for_document_processed(app, &track_id, Duration::from_secs(90)).await,
        "completed"
    );
    let meta = common::doc_metadata_from_kv(&workers.kv_storage, &doc_id)
        .await
        .expect("metadata");
    assert!(meta.get("multimodal").is_none());
}
