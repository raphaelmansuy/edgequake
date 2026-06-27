//! SPEC-026 Phase 4e — asset loader + drawing path contract tests.

use edgequake_api::services::{analyze_multimodal_images, MultimodalProviders};
use edgequake_llm::MockProvider;
use std::path::PathBuf;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/spec026")
}

#[tokio::test]
async fn drawing_tag_loads_asset_from_fixture_dir() {
    std::env::set_var("VLM_PROCESS_ENABLE", "true");
    std::env::set_var("VLM_MIN_IMAGE_PIXEL", "1");
    std::env::set_var("EDGEQUAKE_VISION_PROVIDER", "__test_no_vision__");

    let md =
        std::fs::read_to_string(fixture_dir().join("mineru_drawing_tag.md")).expect("fixture md");
    let mock = MockProvider::new();
    mock.add_response(
        r#"{"name":"fixture_chart","type":"Chart","description":"Loaded from assets dir."}"#,
    )
    .await;

    let assets_dir = fixture_dir();
    let outcome = analyze_multimodal_images(
        &md,
        Some("i"),
        "spec026.pdf",
        MultimodalProviders::single(&mock),
        Some(&assets_dir),
        None,
    )
    .await;

    assert!(outcome.markdown.contains("fixture chart"));
    assert!(outcome.summary.success >= 1);
    assert!(!outcome.markdown.contains("VLM pending sidecar"));

    std::env::remove_var("VLM_PROCESS_ENABLE");
    std::env::remove_var("VLM_MIN_IMAGE_PIXEL");
    std::env::remove_var("EDGEQUAKE_VISION_PROVIDER");
}
