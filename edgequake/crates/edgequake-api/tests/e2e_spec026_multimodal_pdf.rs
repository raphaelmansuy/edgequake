//! SPEC-026 Phase 4b — PDF inline multimodal E2E (LightRAG analyze → process subset).

mod common;

use common::spec026_multimodal::{
    allow_tiny_images_in_tests, markdown_with_data_uri_image, markdown_with_drawing_tag,
    restore_vlm_image_limits, MOCK_VLM_SARAH_JSON,
};
use edgequake_api::services::enrich_markdown_with_vlm;
use edgequake_llm::MockProvider;
use edgequake_storage::EntityId;
use serial_test::serial;
use std::sync::Arc;
use std::time::Duration;

/// LightRAG `analyze_multimodal` data-URI path → chunk/entity extract (process stage).
#[tokio::test]
#[serial]
async fn data_uri_enriched_markdown_ingest_extracts_entities() {
    allow_tiny_images_in_tests();
    let mock = Arc::new(MockProvider::new());
    mock.add_response(MOCK_VLM_SARAH_JSON).await;

    let raw = markdown_with_data_uri_image();
    let enriched = enrich_markdown_with_vlm(&raw, Some("i"), "spec026.pdf", mock.as_ref()).await;
    assert!(enriched.contains("Sarah Chen"));
    assert!(!enriched.contains("data:image/png;base64"));

    let workers = common::create_test_app_with_llm_responses(&[]).await;
    let app = workers.app();
    let (_doc_id, track_id, status) = common::upload_and_wait(
        app,
        "spec026-inline-enriched.md",
        &enriched,
        Duration::from_secs(90),
    )
    .await;
    assert_eq!(status, "completed");

    let node_id = EntityId::new("Sarah Chen").as_graph_node_id().to_string();
    assert!(workers
        .graph_storage
        .get_node(&node_id)
        .await
        .unwrap()
        .is_some());
    let _ = track_id;
    restore_vlm_image_limits();
}

/// LightRAG `process_options` gate: no `i` flag → markdown unchanged (no VLM calls).
#[tokio::test]
async fn drawing_tag_without_i_flag_skips_vlm() {
    let mock = MockProvider::new();
    let md = markdown_with_drawing_tag();
    let out = enrich_markdown_with_vlm(&md, None, "spec026.pdf", &mock).await;
    assert_eq!(out, md);
}

/// Drawing placeholder with `i` flag but no asset bytes/path stays unchanged (skipped).
#[tokio::test]
async fn drawing_tag_with_i_flag_keeps_placeholder_when_no_asset() {
    let mock = MockProvider::new();
    let md = markdown_with_drawing_tag();
    let out = enrich_markdown_with_vlm(&md, Some("i"), "spec026.pdf", &mock).await;
    assert!(out.contains("<drawing"));
    assert!(!out.contains("VLM pending sidecar"));
}

/// LightRAG tiny-image skip: data-URI below VLM_MIN_IMAGE_PIXEL keeps placeholder.
#[tokio::test]
#[serial]
async fn data_uri_tiny_image_skips_vlm_enrich() {
    restore_vlm_image_limits();
    let mock = MockProvider::new();
    mock.add_response(MOCK_VLM_SARAH_JSON).await;
    let md = markdown_with_data_uri_image();
    let out = enrich_markdown_with_vlm(&md, Some("i"), "spec026.pdf", &mock).await;
    assert!(out.contains("data:image/png;base64"));
    assert!(!out.contains("Sarah Chen"));
}
