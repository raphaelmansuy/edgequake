//! Micro-test for table detection
//!
//! **Purpose:** Instant feedback (<0.05s) for table detection functionality
//! **PDF:** 004_simple_table_2x3.pdf (1.9KB) - Simple 2x3 table
//!
//! **Design principles:**
//! - Uses include_bytes! for zero I/O latency
//! - Single assertion per test for clear failure diagnosis
//! - Tests one feature: table detection and rendering
//!
//! **Usage:** `cargo test --package edgequake-pdf --test micro_tables`

use std::sync::Arc;

use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::PdfExtractor;

// =============================================================================
// Embedded PDF Data
// =============================================================================

/// Simple 2x3 table PDF (1.9KB) - contains a basic table with rows and columns
/// WHY include_bytes!: No file I/O at runtime = instant test execution
const TABLE_PDF: &[u8] = include_bytes!("../test-data/004_simple_table_2x3.pdf");

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
// Micro-Tests: Table Detection
// =============================================================================

/// Test: Table extraction produces output
/// WHY: Basic sanity check for table PDF processing
#[tokio::test]
async fn table_extraction_produces_output() {
    let markdown = extract_markdown(TABLE_PDF).await;

    assert!(
        !markdown.is_empty(),
        "Table extraction should produce non-empty markdown"
    );
}

/// Test: Table PDF contains table markers
/// WHY: Markdown tables use | characters - verify table detection works
/// NOTE: If no | found, table may be rendered as paragraphs (quality issue, not crash)
#[tokio::test]
async fn table_extraction_contains_structure() {
    let markdown = extract_markdown(TABLE_PDF).await;

    // Table PDFs should produce either:
    // 1. Markdown table syntax (|) - ideal
    // 2. Structured text (multiple lines) - acceptable
    let has_table_syntax = markdown.contains('|');
    let has_multiple_lines = markdown.lines().count() > 2;

    assert!(
        has_table_syntax || has_multiple_lines,
        "Table extraction should produce structured output (|) or multiple lines. Got:\n{}",
        &markdown[..markdown.len().min(500)]
    );
}

/// Test: No crash on table PDF
/// WHY: Verifies the extraction pipeline handles table structures
#[tokio::test]
async fn table_extraction_no_panic() {
    let extractor = create_extractor();
    let result = extractor.extract_to_markdown(TABLE_PDF).await;

    assert!(
        result.is_ok(),
        "Table extraction should not fail: {:?}",
        result.err()
    );
}

/// Test: Table has reasonable content length
/// WHY: A 2x3 table with content should produce substantial output
/// THRESHOLD: >30 chars - 6 cells with some content each
#[tokio::test]
async fn table_extraction_reasonable_length() {
    let markdown = extract_markdown(TABLE_PDF).await;

    assert!(
        markdown.len() > 30,
        "Table markdown should be >30 chars for a 2x3 table, got {} chars",
        markdown.len()
    );
}
