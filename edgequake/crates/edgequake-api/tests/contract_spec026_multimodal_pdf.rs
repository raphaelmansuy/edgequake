//! SPEC-026 Phase 4b — PDF inline multimodal markdown contract tests.

use edgequake_api::services::vlm_limits::{probe_image_dimensions, validate_image_for_vlm};
use edgequake_api::services::{enrich_markdown_with_vlm, MultimodalProcessOptions};
use edgequake_llm::MockProvider;
use edgequake_pdf::inline_images::scan_inline_image_refs;

/// 1×1 PNG bytes (shared with E2E fixtures).
const TINY_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
];

#[test]
fn scan_inline_image_refs_finds_drawing_tag() {
    let md = r#"<drawing id="im-001" format="png" />"#;
    assert_eq!(scan_inline_image_refs(md).len(), 1);
}

#[test]
fn multimodal_process_options_default_disables_images() {
    let opts = MultimodalProcessOptions::default();
    assert!(!opts.images);
}

#[tokio::test]
async fn enrich_markdown_leaves_plain_text_unchanged_without_i_flag() {
    let mock = MockProvider::new();
    let md = "No placeholders here.";
    let out = enrich_markdown_with_vlm(md, None, "doc.pdf", &mock).await;
    assert_eq!(out, md);
}

#[test]
fn probe_and_validate_reject_tiny_png() {
    std::env::remove_var("VLM_MIN_IMAGE_PIXEL");
    assert_eq!(probe_image_dimensions(TINY_PNG, "image/png"), Some((1, 1)));
    assert!(validate_image_for_vlm(TINY_PNG, 1, 1).is_err());
}

#[tokio::test]
async fn enrich_skips_tiny_data_uri_without_vlm_call() {
    std::env::set_var("VLM_PROCESS_ENABLE", "true");
    std::env::remove_var("VLM_MIN_IMAGE_PIXEL");
    let b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
    let md = format!("![x](data:image/png;base64,{b64})");
    let mock = MockProvider::new();
    mock.add_response(r#"{"name":"n","type":"Chart","description":"d"}"#)
        .await;
    let out = enrich_markdown_with_vlm(&md, Some("i"), "doc.pdf", &mock).await;
    assert!(out.contains("data:image/png;base64"));
    std::env::remove_var("VLM_PROCESS_ENABLE");
}
