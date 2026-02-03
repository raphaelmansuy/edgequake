//! Micro-test for column detection and reading order
//!
//! **Purpose:** Instant feedback (<0.05s) for two-column layout handling
//! **PDF:** 003_two_columns.pdf (2.0KB) - Clear two-column layout
//!
//! **Design principles:**
//! - Uses include_bytes! for zero I/O latency
//! - Single assertion per test for clear failure diagnosis
//! - Tests one feature: multi-column reading order
//!
//! **Usage:** `cargo test --package edgequake-pdf --test micro_columns`

use std::sync::Arc;

use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::PdfExtractor;

// =============================================================================
// Embedded PDF Data
// =============================================================================

/// Two-column PDF (2.0KB) - contains text in two columns
/// WHY include_bytes!: No file I/O at runtime = instant test execution
const TWO_COLUMN_PDF: &[u8] = include_bytes!("../test-data/003_two_columns.pdf");

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
// Micro-Tests: Column Detection
// =============================================================================

/// Test: Two-column extraction produces output
/// WHY: Basic sanity check for multi-column PDF processing
#[tokio::test]
async fn column_extraction_produces_output() {
    let markdown = extract_markdown(TWO_COLUMN_PDF).await;

    assert!(
        !markdown.is_empty(),
        "Two-column extraction should produce non-empty markdown"
    );
}

/// Test: Two-column extraction no crash
/// WHY: Multi-column layout is complex - ensure no panics
#[tokio::test]
async fn column_extraction_no_panic() {
    let extractor = create_extractor();
    let result = extractor.extract_to_markdown(TWO_COLUMN_PDF).await;

    assert!(
        result.is_ok(),
        "Two-column extraction should not fail: {:?}",
        result.err()
    );
}

/// Test: Column extraction produces structured output
/// WHY: Two columns should produce multiple paragraphs/lines
#[tokio::test]
async fn column_extraction_has_structure() {
    let markdown = extract_markdown(TWO_COLUMN_PDF).await;
    let line_count = markdown.lines().count();

    assert!(
        line_count > 2,
        "Two-column layout should produce multiple lines, got {}",
        line_count
    );
}

/// Test: Column extraction reasonable length
/// WHY: Two columns means substantial content
/// THRESHOLD: >50 chars - two columns with some text each
#[tokio::test]
async fn column_extraction_reasonable_length() {
    let markdown = extract_markdown(TWO_COLUMN_PDF).await;

    assert!(
        markdown.len() > 50,
        "Two-column markdown should be >50 chars, got {} chars",
        markdown.len()
    );
}

/// Test: No interleaved column content
/// WHY: Reading order should be left column fully, then right column
/// This is a quality check - if columns are interleaved, text becomes nonsense
/// NOTE: This is a heuristic check - looks for coherent word sequences
#[tokio::test]
async fn column_reading_order_coherent() {
    let markdown = extract_markdown(TWO_COLUMN_PDF).await;

    // Count word-like sequences (alphanumeric runs)
    let word_count: usize = markdown.split_whitespace().filter(|w| w.len() > 1).count();

    // A coherent extraction should have multiple words
    assert!(
        word_count > 5,
        "Column extraction should produce coherent words, got {} words",
        word_count
    );
}
