//! Quick Smoke Tests for PDF Extraction
//!
//! **Purpose:** Fast sanity checks that run in <5 seconds
//! **Usage:** `cargo test --package edgequake-pdf --test quick_smoke`
//!
//! **Why this split:**
//! Developers need instant feedback during development. These tests verify
//! core functionality without processing large PDF datasets.
//!
//! **Test Selection Criteria:**
//! - Uses 2-3 small PDFs (< 500KB each)
//! - Tests basic extraction pipeline (parsing, text extraction, markdown output)
//! - No comprehensive quality metrics (text/structure scoring)
//! - Focuses on: non-zero output, no crashes, basic format preservation

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::PdfExtractor;

// =============================================================================
// Test Helpers
// =============================================================================

fn test_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-data")
}

fn create_extractor() -> PdfExtractor {
    PdfExtractor::new(Arc::new(MockProvider::new()))
}

// =============================================================================
// Smoke Tests
// =============================================================================

#[tokio::test]
async fn smoke_sample_pdf() {
    let sample_path = test_data_dir().join("sample.pdf");

    if !sample_path.exists() {
        println!("⚠️  Skipping: sample.pdf not found");
        return;
    }

    let extractor = create_extractor();
    let pdf_bytes = fs::read(&sample_path).expect("Failed to read sample.pdf");

    let result = extractor.extract_to_markdown(&pdf_bytes).await;
    assert!(result.is_ok(), "Extraction should succeed for sample.pdf");

    let markdown = result.unwrap();
    assert!(!markdown.is_empty(), "Markdown output should not be empty");
    // **Why >10 chars threshold:**
    // sample.pdf is minimal ("### Dumm y PDF file" = 20 bytes)
    // We just need to verify extraction worked, not content richness
    assert!(
        markdown.len() > 10,
        "Markdown should contain some content (>10 chars)"
    );

    println!("✅ sample.pdf: {} chars extracted", markdown.len());
}

#[tokio::test]
async fn smoke_simple_text() {
    let path = test_data_dir().join("001_simple_text.pdf");

    if !path.exists() {
        println!("⚠️  Skipping: 001_simple_text.pdf not found");
        return;
    }

    let extractor = create_extractor();
    let pdf_bytes = fs::read(&path).expect("Failed to read PDF");

    let result = extractor.extract_to_markdown(&pdf_bytes).await;
    assert!(
        result.is_ok(),
        "Extraction should succeed for simple text PDF"
    );

    let markdown = result.unwrap();
    assert!(!markdown.is_empty(), "Should extract text content");

    println!("✅ 001_simple_text.pdf: {} chars", markdown.len());
}

#[tokio::test]
async fn smoke_headers_and_lists() {
    let path = test_data_dir().join("002_headers_and_lists.pdf");

    if !path.exists() {
        println!("⚠️  Skipping: 002_headers_and_lists.pdf not found");
        return;
    }

    let extractor = create_extractor();
    let pdf_bytes = fs::read(&path).expect("Failed to read PDF");

    let result = extractor.extract_to_markdown(&pdf_bytes).await;
    assert!(result.is_ok(), "Extraction should succeed");

    let markdown = result.unwrap();
    assert!(!markdown.is_empty(), "Should extract content");

    // Check for markdown structure markers (not strict - just smoke test)
    let has_structure = markdown.contains('#') || markdown.contains('-') || markdown.contains('*');
    println!(
        "✅ 002_headers_and_lists.pdf: {} chars, structure markers: {}",
        markdown.len(),
        if has_structure { "present" } else { "absent" }
    );
}

/// **Smoke Test Summary**
/// Verifies that the extraction pipeline doesn't crash on basic inputs
/// and produces non-empty output. Does NOT validate quality metrics.
#[tokio::test]
async fn smoke_test_summary() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  Smoke Test Suite Complete                                   ║");
    println!("║  Purpose: Fast sanity checks (<5 seconds)                    ║");
    println!("║  Run: cargo test --package edgequake-pdf --test quick_smoke ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
}
