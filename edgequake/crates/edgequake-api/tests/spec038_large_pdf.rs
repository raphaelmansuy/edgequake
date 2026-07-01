//! SPEC-038: Large PDF ingestion — auto-routing, profile SSOT, failure classes.
//!
//! Run:
//! `cargo test -p edgequake-api --features postgres --test spec038_large_pdf`

use edgequake_api::services::{
    classify_ingestion_failure, IngestionFailureClass, LargeDocumentProfile,
};
use edgequake_pdf::PdfParserBackend;

const SIMPLE_TEXT_PDF: &[u8] =
    include_bytes!("../../../../legacy/edgequake-pdf/test-data/001_simple_text.pdf");

#[test]
fn spec038_classify_circuit_breaker_failure() {
    let class = classify_ingestion_failure(
        "Circuit breaker tripped after 3 consecutive timeouts. Last error: timeout",
    );
    assert_eq!(class, IngestionFailureClass::CircuitBreaker);
    assert_eq!(class.as_str(), "circuit_breaker");
}

#[test]
fn spec038_classify_document_too_large() {
    let class = classify_ingestion_failure("Document too large: 12.00MB. Maximum allowed: 50MB");
    assert_eq!(class, IngestionFailureClass::DocumentTooLarge);
}

#[test]
fn spec038_profile_reproducer_603_edgeparse_eta() {
    let profile = LargeDocumentProfile::new(603, 11_043_120);
    let est = profile.ingestion_estimate(PdfParserBackend::EdgeParse, "mock");
    assert_eq!(est.recommended_backend, "edgeparse");
    assert!(est.convert_seconds < 600);
    assert!(est.total_seconds_pessimistic >= 7200);
}

#[test]
fn spec038_should_try_edgeparse_only_for_implicit_vision() {
    assert!(LargeDocumentProfile::should_try_edgeparse_before_vision(
        PdfParserBackend::Vision,
        false
    ));
    assert!(!LargeDocumentProfile::should_try_edgeparse_before_vision(
        PdfParserBackend::Vision,
        true
    ));
}

#[test]
fn spec038_gleaning_disabled_at_500_pages() {
    let profile = LargeDocumentProfile::new(603, 1_000_000);
    assert!(profile.should_disable_gleaning());
    let small = LargeDocumentProfile::new(100, 1_000_000);
    assert!(!small.should_disable_gleaning());
}

#[tokio::test]
async fn spec038_auto_route_edgeparse_on_simple_pdf() {
    let markdown =
        edgequake_api::services::try_edgeparse_fast_path(SIMPLE_TEXT_PDF, 1, "001_simple_text.pdf")
            .await;
    assert!(
        markdown.is_some(),
        "born-digital fixture should produce markdown via EdgeParse fast path"
    );
    let md = markdown.unwrap();
    assert!(LargeDocumentProfile::markdown_has_text_layer(&md, 1));
}

#[tokio::test]
async fn spec038_reproducer_fixture_edgeparse_if_present() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/spec038/guide_2606.24937v1-opt.pdf");
    let reproducer = std::env::var("SPEC038_REPRODUCER_PDF")
        .ok()
        .map(std::path::PathBuf::from)
        .filter(|p| p.exists())
        .or_else(|| fixture.exists().then_some(fixture));

    let Some(path) = reproducer else {
        eprintln!("SKIP spec038_reproducer_fixture: no PDF at tests/fixtures/spec038 or SPEC038_REPRODUCER_PDF");
        return;
    };

    let bytes = std::fs::read(&path).expect("read reproducer PDF");
    let page_count = 603usize;

    let markdown = edgequake_api::services::try_edgeparse_fast_path(
        &bytes,
        page_count,
        "guide_2606.24937v1-opt.pdf",
    )
    .await;

    assert!(
        markdown.is_some(),
        "reproducer {:?} should auto-route via EdgeParse",
        path
    );
    let md = markdown.unwrap();
    assert!(
        md.len() > 500_000,
        "expected >500KB markdown, got {}",
        md.len()
    );
    assert!(LargeDocumentProfile::markdown_has_text_layer(
        &md, page_count
    ));
}
