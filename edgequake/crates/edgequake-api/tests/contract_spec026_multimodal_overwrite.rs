//! SPEC-026 Phase 4h — re-analyze overwrites prior results (LightRAG E05).

use edgequake_api::services::{analyze_multimodal_images, MultimodalProviders};
use edgequake_llm::MockProvider;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn reanalyze_overwrites_prior_image_description() {
    std::env::set_var("VLM_PROCESS_ENABLE", "true");
    std::env::set_var("VLM_MIN_IMAGE_PIXEL", "1");
    let b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
    let md = format!("![x](data:image/png;base64,{b64})");
    let mock = MockProvider::new();
    mock.add_response(
        r#"{"name":"first_pass","type":"Chart","description":"First analyze pass."}"#,
    )
    .await;
    let first = analyze_multimodal_images(
        &md,
        Some("i"),
        "doc.pdf",
        MultimodalProviders::single(&mock),
        None,
        None,
    )
    .await;
    assert!(first.markdown.contains("First analyze pass"));
    let first_desc = first.manifest.items[0]
        .analyze_result
        .as_ref()
        .unwrap()
        .description
        .clone()
        .unwrap();
    assert_eq!(first_desc, "First analyze pass.");

    mock.add_response(
        r#"{"name":"second_pass","type":"Chart","description":"Second analyze pass overwrites."}"#,
    )
    .await;
    let second = analyze_multimodal_images(
        &md,
        Some("i"),
        "doc.pdf",
        MultimodalProviders::single(&mock),
        None,
        None,
    )
    .await;
    assert!(second.markdown.contains("Second analyze pass overwrites"));
    assert!(!second.markdown.contains("First analyze pass"));
    let second_desc = second.manifest.items[0]
        .analyze_result
        .as_ref()
        .unwrap()
        .description
        .clone()
        .unwrap();
    assert_eq!(second_desc, "Second analyze pass overwrites.");

    std::env::remove_var("VLM_PROCESS_ENABLE");
    std::env::remove_var("VLM_MIN_IMAGE_PIXEL");
}

#[tokio::test]
#[serial]
async fn reanalyze_table_overwrites_prior_sidecar_result() {
    let md = r#"Intro <table id="tb-1" format="html"><tr><td>Revenue</td></tr></table> end"#;
    let mock = MockProvider::new();
    mock.add_response(
        r#"{"name":"table_v1","type":"Table","description":"Version one table summary."}"#,
    )
    .await;
    let first = analyze_multimodal_images(
        md,
        Some("t"),
        "doc.pdf",
        MultimodalProviders::single(&mock),
        None,
        None,
    )
    .await;
    assert!(first.markdown.contains("Version one table summary"));

    mock.add_response(
        r#"{"name":"table_v2","type":"Table","description":"Version two table summary."}"#,
    )
    .await;
    let second = analyze_multimodal_images(
        md,
        Some("t"),
        "doc.pdf",
        MultimodalProviders::single(&mock),
        None,
        None,
    )
    .await;
    assert!(second.markdown.contains("Version two table summary"));
    assert!(!second.markdown.contains("Version one table summary"));
    let record = second.manifest.items[0].analyze_result.as_ref().unwrap();
    assert_eq!(record.name.as_deref(), Some("table_v2"));
}
