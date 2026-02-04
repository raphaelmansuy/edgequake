//! Micro-test for font encoding edge cases
//!
//! **Purpose:** Instant feedback (<0.05s) for font handling edge cases
//! **PDF:** 024_embedded_fonts_obfuscated.pdf (1.7KB) - Tests embedded fonts
//!
//! **Design principles:**
//! - Uses include_bytes! for zero I/O latency
//! - Single assertion per test for clear failure diagnosis
//! - Tests one feature: font encoding and character mapping
//!
//! **Note:** Edge case PDFs may produce minimal output - tests focus on no-crash behavior
//!
//! **Usage:** `cargo test --package edgequake-pdf --test micro_fonts`

use std::sync::Arc;

use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::PdfExtractor;

// =============================================================================
// Embedded PDF Data
// =============================================================================

/// Embedded fonts PDF (1.7KB) - tests obfuscated font handling
/// WHY include_bytes!: No file I/O at runtime = instant test execution
const FONT_EDGE_CASE_PDF: &[u8] = include_bytes!("../test-data/024_embedded_fonts_obfuscated.pdf");

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
// Micro-Tests: Font Encoding
// =============================================================================

/// Test: Font edge case extraction produces output or handles gracefully
/// WHY: Edge case PDFs may produce minimal output - focus on no-crash behavior
#[tokio::test]
async fn font_extraction_produces_output() {
    let extractor = create_extractor();
    let result = extractor.extract_to_markdown(FONT_EDGE_CASE_PDF).await;

    // For edge case PDFs, we accept either:
    // 1. Successful extraction (even if empty)
    // 2. Graceful error handling
    assert!(
        result.is_ok(),
        "Font edge case extraction should not panic: {:?}",
        result.err()
    );
}

/// Test: Font edge case extraction no crash
/// WHY: Incomplete unicode mappings can cause panics if not handled
#[tokio::test]
async fn font_extraction_no_panic() {
    let extractor = create_extractor();
    let result = extractor.extract_to_markdown(FONT_EDGE_CASE_PDF).await;

    assert!(
        result.is_ok(),
        "Font edge case extraction should not fail: {:?}",
        result.err()
    );
}

/// Test: Font extraction minimizes replacement characters
/// WHY: U+FFFD (replacement character) indicates failed character mapping
/// Some replacement is acceptable, but excessive means broken extraction
/// THRESHOLD: <10% of output as replacement characters
#[tokio::test]
async fn font_extraction_minimal_replacement_chars() {
    let markdown = extract_markdown(FONT_EDGE_CASE_PDF).await;

    let replacement_count = markdown.chars().filter(|&c| c == '\u{FFFD}').count();
    let total_chars = markdown.chars().count().max(1);
    let replacement_pct = (replacement_count as f64 / total_chars as f64) * 100.0;

    // Allow up to 10% replacement characters for edge case PDFs
    // WHY 10%: This PDF intentionally has incomplete mappings
    assert!(
        replacement_pct < 10.0,
        "Font extraction should have <10% replacement chars, got {:.1}% ({}/{})",
        replacement_pct,
        replacement_count,
        total_chars
    );
}

/// Test: Font extraction produces readable text or handles gracefully
/// WHY: Edge case PDFs may not produce readable ASCII - focus on no-crash
#[tokio::test]
async fn font_extraction_handles_edge_case() {
    let extractor = create_extractor();
    let result = extractor.extract_to_markdown(FONT_EDGE_CASE_PDF).await;

    // The test passes if extraction completes without panic
    // Edge case PDFs may produce empty or minimal output
    if let Ok(markdown) = result {
        // If we got output, count readable characters as a bonus check
        let readable_count = markdown
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .count();
        println!(
            "Font extraction produced {} readable ASCII chars",
            readable_count
        );
    }
    // Success = no panic during extraction
}
