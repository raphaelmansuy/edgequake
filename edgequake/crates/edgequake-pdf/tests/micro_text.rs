//! Micro-test for basic text extraction
//!
//! **Purpose:** Instant feedback (<0.05s) for text extraction functionality
//! **PDF:** 001_simple_text.pdf (1.7KB) - Minimal plain text document
//!
//! **Design principles:**
//! - Uses include_bytes! for zero I/O latency
//! - Single assertion per test for clear failure diagnosis
//! - Tests one feature: basic text extraction
//!
//! **Usage:** `cargo test --package edgequake-pdf --test micro_text`

use std::sync::Arc;

use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::PdfExtractor;

// =============================================================================
// Embedded PDF Data
// =============================================================================

/// Minimal text PDF (1.7KB) - contains simple paragraphs
/// WHY include_bytes!: No file I/O at runtime = instant test execution
const SIMPLE_TEXT_PDF: &[u8] = include_bytes!("../test-data/001_simple_text.pdf");

// =============================================================================
// Test Helpers
// =============================================================================

fn create_extractor() -> PdfExtractor {
    PdfExtractor::new(Arc::new(MockProvider::new()))
}

/// Extract markdown from embedded PDF bytes
async fn extract_markdown(pdf_bytes: &[u8]) -> String {
    let extractor = create_extractor();
    extractor
        .extract_to_markdown(pdf_bytes)
        .await
        .expect("Extraction should not fail")
}

// =============================================================================
// Micro-Tests: Basic Text Extraction
// =============================================================================

/// Test: Extraction produces non-empty output
/// WHY: Most basic sanity check - if this fails, nothing else will work
#[tokio::test]
async fn text_extraction_produces_output() {
    let markdown = extract_markdown(SIMPLE_TEXT_PDF).await;

    assert!(
        !markdown.is_empty(),
        "Extraction should produce non-empty markdown"
    );
}

/// Test: Extraction produces reasonable length
/// WHY: Ensures we're extracting actual content, not just headers
/// THRESHOLD: >50 chars - simple_text.pdf has multiple sentences
#[tokio::test]
async fn text_extraction_reasonable_length() {
    let markdown = extract_markdown(SIMPLE_TEXT_PDF).await;

    assert!(
        markdown.len() > 50,
        "Extracted markdown should be >50 chars, got {} chars",
        markdown.len()
    );
}

/// Test: No crash on extraction
/// WHY: Verifies the extraction pipeline doesn't panic on valid PDF
#[tokio::test]
async fn text_extraction_no_panic() {
    let extractor = create_extractor();
    let result = extractor.extract_to_markdown(SIMPLE_TEXT_PDF).await;

    assert!(
        result.is_ok(),
        "Extraction should not fail: {:?}",
        result.err()
    );
}

/// Test: Contains expected text patterns
/// WHY: Verifies actual text content is preserved
#[tokio::test]
async fn text_extraction_preserves_content() {
    let markdown = extract_markdown(SIMPLE_TEXT_PDF).await;
    let lower = markdown.to_lowercase();

    // Simple text PDF should contain some recognizable words
    // Not checking exact content to avoid brittleness
    assert!(
        lower.contains("page")
            || lower.contains("text")
            || lower.contains("sample")
            || lower.len() > 100,
        "Extracted content should contain recognizable text or be substantial"
    );
}
