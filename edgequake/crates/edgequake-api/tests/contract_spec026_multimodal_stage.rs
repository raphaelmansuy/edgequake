//! SPEC-026 Phase 4d — multimodal analyze stage contract tests (DRY/SOLID SSOT).

use edgequake_api::services::{
    apply_process_options_to_metadata, resolve_process_options_from_metadata,
    run_multimodal_analyze_stage, should_run_image_analysis, vlm_process_enabled,
    MultimodalProcessOptions,
};
use edgequake_llm::MockProvider;
use serde_json::json;
use std::sync::Arc;

#[test]
fn metadata_roundtrip_process_options() {
    let mut obj = serde_json::Map::new();
    apply_process_options_to_metadata(&mut obj, Some("i"));
    let value = json!(obj);
    assert_eq!(
        resolve_process_options_from_metadata(&value).as_deref(),
        Some("i")
    );
}

#[test]
fn gates_default_disabled_like_lightrag() {
    std::env::remove_var("VLM_PROCESS_ENABLE");
    assert!(!vlm_process_enabled());
    let opts = MultimodalProcessOptions {
        images: true,
        ..Default::default()
    };
    assert!(!should_run_image_analysis(&opts));
}

#[tokio::test]
async fn analyze_stage_skips_without_i_flag() {
    let mock = Arc::new(MockProvider::new());
    let md = "plain text";
    let out = run_multimodal_analyze_stage(
        md.to_string(),
        None,
        "doc.pdf",
        None,
        uuid::Uuid::nil(),
        mock,
        None,
        None,
        None,
    )
    .await;
    assert_eq!(out, md);
}

#[tokio::test]
async fn analyze_stage_enriches_data_uri_with_i_flag() {
    std::env::set_var("EDGEQUAKE_VISION_PROVIDER", "__test_no_vision__");
    std::env::set_var("VLM_PROCESS_ENABLE", "true");
    std::env::set_var("VLM_MIN_IMAGE_PIXEL", "1");
    let mock = Arc::new(MockProvider::new());
    mock.add_response(
        r#"{"name":"pipeline_chart","type":"Chart","description":"Pipeline verified."}"#,
    )
    .await;
    let b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
    let md = format!("![x](data:image/png;base64,{b64})");
    let out = run_multimodal_analyze_stage(
        md,
        Some("i"),
        "pipeline.pdf",
        None,
        uuid::Uuid::nil(),
        mock,
        None,
        None,
        None,
    )
    .await;
    assert!(out.contains("Pipeline verified"));
    assert!(!out.contains("data:image/png;base64"));
    std::env::remove_var("VLM_MIN_IMAGE_PIXEL");
    std::env::remove_var("VLM_PROCESS_ENABLE");
    std::env::remove_var("EDGEQUAKE_VISION_PROVIDER");
}
