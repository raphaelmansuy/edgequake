//! SPEC-026 Phase 4e — strict fail mode contract (E20/E21).

use edgequake_api::services::{analyze_multimodal_images, MultimodalProviders};
use edgequake_llm::MockProvider;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn vlm_disabled_with_i_strict_returns_hard_error() {
    std::env::set_var("VLM_PROCESS_ENABLE", "false");
    std::env::set_var("EDGEQUAKE_MULTIMODAL_FAIL_MODE", "strict");
    let b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
    let md = format!("![x](data:image/png;base64,{b64})");
    let mock = MockProvider::new();
    let out = analyze_multimodal_images(
        &md,
        Some("i"),
        "doc.pdf",
        MultimodalProviders::single(&mock),
        None,
        None,
    )
    .await;
    assert!(out.hard_error.is_some());
    assert!(out
        .hard_error
        .as_deref()
        .unwrap()
        .contains("VLM_PROCESS_ENABLE"));
    std::env::remove_var("VLM_PROCESS_ENABLE");
    std::env::remove_var("EDGEQUAKE_MULTIMODAL_FAIL_MODE");
}

#[tokio::test]
#[serial]
async fn invalid_json_strict_fails_after_retry() {
    std::env::set_var("VLM_PROCESS_ENABLE", "true");
    std::env::set_var("VLM_MIN_IMAGE_PIXEL", "1");
    std::env::set_var("EDGEQUAKE_MULTIMODAL_FAIL_MODE", "strict");
    let b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
    let md = format!("![x](data:image/png;base64,{b64})");
    let mock = MockProvider::new();
    mock.add_response("not json at all").await;
    mock.add_response("still not json").await;
    let out = analyze_multimodal_images(
        &md,
        Some("i"),
        "doc.pdf",
        MultimodalProviders::single(&mock),
        None,
        None,
    )
    .await;
    assert!(out.hard_error.is_some());
    assert!(out.summary.failed >= 1 || out.hard_error.is_some());
    std::env::remove_var("VLM_PROCESS_ENABLE");
    std::env::remove_var("VLM_MIN_IMAGE_PIXEL");
    std::env::remove_var("EDGEQUAKE_MULTIMODAL_FAIL_MODE");
}
