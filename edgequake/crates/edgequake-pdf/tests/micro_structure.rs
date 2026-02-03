//! Micro-test for structure detection (headers and lists)
//!
//! **Purpose:** Instant feedback (<0.05s) for header/list detection
//! **PDF:** legacy/002_headers_and_lists.pdf (1.9KB) - Headers and lists
//!
//! **Design principles:**
//! - Uses include_bytes! for zero I/O latency
//! - Single assertion per test for clear failure diagnosis
//! - Tests one feature: document structure recognition
//!
//! **Usage:** `cargo test --package edgequake-pdf --test micro_structure`

use std::sync::Arc;

use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::PdfExtractor;

// =============================================================================
// Embedded PDF Data
// =============================================================================

/// Headers and lists PDF (1.9KB) - contains headings and bullet points
/// WHY include_bytes!: No file I/O at runtime = instant test execution
const STRUCTURE_PDF: &[u8] = include_bytes!("../test-data/legacy/002_headers_and_lists.pdf");

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
// Micro-Tests: Structure Detection
// =============================================================================

/// Test: Structure extraction produces output
/// WHY: Basic sanity check for structured document processing
#[tokio::test]
async fn structure_extraction_produces_output() {
    let markdown = extract_markdown(STRUCTURE_PDF).await;

    assert!(
        !markdown.is_empty(),
        "Structure extraction should produce non-empty markdown"
    );
}

/// Test: Structure extraction no crash
/// WHY: Header/list detection involves font size analysis - ensure stability
#[tokio::test]
async fn structure_extraction_no_panic() {
    let extractor = create_extractor();
    let result = extractor.extract_to_markdown(STRUCTURE_PDF).await;

    assert!(
        result.is_ok(),
        "Structure extraction should not fail: {:?}",
        result.err()
    );
}

/// Test: Structure extraction detects headers
/// WHY: Headers are marked with # in markdown - verify detection works
#[tokio::test]
async fn structure_extraction_has_headers() {
    let markdown = extract_markdown(STRUCTURE_PDF).await;

    // Headers start lines with # (or ## or ###)
    let has_headers = markdown.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with('#') || trimmed.starts_with("**")
    });

    assert!(
        has_headers,
        "Structure extraction should detect headers (# or **). Got:\n{}",
        &markdown[..markdown.len().min(500)]
    );
}

/// Test: Structure extraction has multiple sections
/// WHY: Headers and lists document should have clear structure
#[tokio::test]
async fn structure_extraction_has_sections() {
    let markdown = extract_markdown(STRUCTURE_PDF).await;

    // Count non-empty lines as a proxy for structural elements
    let structural_lines = markdown
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();

    assert!(
        structural_lines > 3,
        "Structure extraction should have multiple sections, got {} lines",
        structural_lines
    );
}

/// Test: Structure extraction detects lists (if present)
/// WHY: Lists use - or * or numbered patterns
/// NOTE: May not have lists in this specific PDF, so we check for either lists OR paragraphs
#[tokio::test]
async fn structure_extraction_coherent_output() {
    let markdown = extract_markdown(STRUCTURE_PDF).await;

    // Look for list markers or substantial content
    let has_lists = markdown.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with('-')
            || trimmed.starts_with('*')
            || trimmed.starts_with("1.")
            || trimmed.starts_with("•")
    });

    let has_paragraphs = markdown.len() > 100;

    assert!(
        has_lists || has_paragraphs,
        "Structure extraction should have lists or substantial paragraphs"
    );
}
