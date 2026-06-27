//! SPEC-026 Phase 4f — table + equation extract contract.

use edgequake_api::services::{
    analyze_multimodal_images, scan_manifest_items, MultimodalProviders,
};
use edgequake_llm::MockProvider;

#[test]
fn scan_manifest_items_finds_html_table() {
    let md = r#"<table id="tb-1" format="html"><tr><td>Cell</td></tr></table>"#;
    let items = scan_manifest_items(md);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].modality, "table");
}

#[test]
fn scan_manifest_items_finds_equation_with_id() {
    let md = r#"<equation id="eq-1">E=mc^2</equation>"#;
    let items = scan_manifest_items(md);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].modality, "equation");
}

#[tokio::test]
async fn table_analyze_success_when_t_enabled() {
    let md = r#"Intro <table id="tb-1" format="html"><tr><td>Revenue</td></tr></table> end"#;
    let mock = MockProvider::new();
    mock.add_response(r#"{"name":"revenue_table","type":"Table","description":"Revenue row."}"#)
        .await;
    let out = analyze_multimodal_images(
        md,
        Some("t"),
        "doc.pdf",
        MultimodalProviders::single(&mock),
        None,
        None,
    )
    .await;
    assert!(out.markdown.contains("[Table Name]revenue_table"));
    assert_eq!(out.summary.success, 1);
}

#[tokio::test]
async fn equation_analyze_success_when_e_enabled() {
    let md = r#"Text <equation id="eq-1">E=mc^2</equation> end"#;
    let mock = MockProvider::new();
    mock.add_response(
        r#"{"name":"mass_energy","equation":"E=mc^2","description":"Einstein mass-energy equivalence."}"#,
    )
    .await;
    let out = analyze_multimodal_images(
        md,
        Some("e"),
        "doc.pdf",
        MultimodalProviders::single(&mock),
        None,
        None,
    )
    .await;
    assert!(out.markdown.contains("[Equation Name]mass_energy"));
    assert!(out.markdown.contains("E=mc^2"));
    assert_eq!(out.summary.success, 1);
}
