//! SPEC-026 Phase 4d — PDF pipeline multimodal analyze E2E (honest stage SSOT).

mod common;

use common::spec026_multimodal::{
    allow_tiny_images_in_tests, markdown_with_data_uri_image, restore_vlm_image_limits,
    MOCK_VLM_SARAH_JSON,
};
use edgequake_api::services::run_multimodal_analyze_stage;
use edgequake_llm::MockProvider;
use edgequake_storage::EntityId;
use serial_test::serial;
use std::sync::Arc;
use std::time::Duration;

/// Exercises [`run_multimodal_analyze_stage`] then full ingest (PDF processor uses same SSOT).
#[tokio::test]
#[serial]
async fn analyze_stage_then_ingest_extracts_entities() {
    allow_tiny_images_in_tests();
    std::env::set_var("EDGEQUAKE_VISION_PROVIDER", "__test_no_vision__");
    let mock = Arc::new(MockProvider::new());
    mock.add_response(MOCK_VLM_SARAH_JSON).await;

    let raw = markdown_with_data_uri_image();
    let enriched = run_multimodal_analyze_stage(
        raw,
        Some("i"),
        "spec026-pipeline.pdf",
        None,
        uuid::Uuid::nil(),
        mock,
        None,
        None,
        None,
    )
    .await;
    assert!(enriched.contains("Sarah Chen"));

    let workers = common::create_test_app_with_llm_responses(&[]).await;
    let app = workers.app();
    let (_doc_id, _track_id, status) = common::upload_and_wait(
        app,
        "spec026-pipeline-enriched.md",
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
    restore_vlm_image_limits();
    std::env::remove_var("EDGEQUAKE_VISION_PROVIDER");
}
