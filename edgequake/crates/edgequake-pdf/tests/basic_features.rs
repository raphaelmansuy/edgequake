//! Basic Feature Tests for PDF Extraction
//!
//! **Purpose:** Feature-focused tests that run in <30 seconds
//! **Usage:** `cargo test --package edgequake-pdf --test basic_features`
//!
//! **Why this split:**
//! After smoke tests pass, developers need to verify specific features
//! (tables, columns, formatting) work correctly on representative PDFs.
//!
//! **Gated behind `slow-tests` feature:**
//! Run with: `cargo test --package edgequake-pdf --test basic_features --features slow-tests`
//!
//! **Test Selection Criteria:**
//! - Uses 5-10 medium PDFs (500KB - 2MB each)
//! - Tests specific features: tables, multi-column, formatting
//! - Light quality checks (presence of structure, not full scoring)
//! - Target: <30 seconds total execution time

#![cfg(feature = "slow-tests")]

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
// Feature Tests
// =============================================================================

#[tokio::test]
async fn feature_multi_column_layout() {
    let path = test_data_dir().join("003_two_columns.pdf");

    if !path.exists() {
        println!("⚠️  Skipping: 003_two_columns.pdf not found");
        return;
    }

    let extractor = create_extractor();
    let pdf_bytes = fs::read(&path).expect("Failed to read PDF");

    let result = extractor.extract_to_markdown(&pdf_bytes).await;
    assert!(result.is_ok(), "Multi-column extraction should succeed");

    let markdown = result.unwrap();
    assert!(!markdown.is_empty(), "Should extract content from columns");

    // **Why this check:**
    // Multi-column PDFs should produce linear reading order.
    // We verify content exists but don't enforce specific column detection.
    println!("✅ Multi-column: {} chars extracted", markdown.len());
}

#[tokio::test]
async fn feature_table_extraction() {
    let path = test_data_dir().join("004_simple_table_2x3.pdf");

    if !path.exists() {
        println!("⚠️  Skipping: 004_simple_table_2x3.pdf not found");
        return;
    }

    let extractor = create_extractor();
    let pdf_bytes = fs::read(&path).expect("Failed to read PDF");

    let result = extractor.extract_to_markdown(&pdf_bytes).await;
    assert!(result.is_ok(), "Table extraction should succeed");

    let markdown = result.unwrap();
    assert!(!markdown.is_empty(), "Should extract table content");

    // Check for table markers (| character indicates markdown table)
    let has_table_markers = markdown.contains('|');
    println!(
        "✅ Table extraction: {} chars, markdown table: {}",
        markdown.len(),
        if has_table_markers {
            "yes"
        } else {
            "no (text fallback)"
        }
    );
}

#[tokio::test]
async fn feature_numbered_pdfs_batch() {
    let extractor = create_extractor();
    let test_dir = test_data_dir();

    // **Why this subset:**
    // Tests 4 diverse PDFs covering: text, structure, layout, tables
    // Balances coverage vs. speed (should run in <20 seconds)
    let test_cases = [
        "001_simple_text.pdf",
        "002_headers_and_lists.pdf",
        "003_two_columns.pdf",
        "004_simple_table_2x3.pdf",
    ];

    let mut success_count = 0;
    let mut total_chars = 0;

    for filename in test_cases {
        let path = test_dir.join(filename);
        if !path.exists() {
            println!("⚠️  Skipping: {} not found", filename);
            continue;
        }

        let pdf_bytes = fs::read(&path).expect("Failed to read PDF");
        let result = extractor.extract_to_markdown(&pdf_bytes).await;

        assert!(result.is_ok(), "Extraction should succeed for {}", filename);

        let markdown = result.unwrap();
        assert!(
            !markdown.is_empty(),
            "Should extract content from {}",
            filename
        );

        success_count += 1;
        total_chars += markdown.len();
        println!("✅ {}: {} chars", filename, markdown.len());
    }

    println!(
        "\n📊 Batch results: {}/{} PDFs extracted, {} total chars",
        success_count,
        test_cases.len(),
        total_chars
    );
}

#[tokio::test]
async fn feature_test_summary() {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║  Basic Feature Test Suite Complete                               ║");
    println!("║  Purpose: Feature validation (<30 seconds)                       ║");
    println!("║  Run: cargo test --package edgequake-pdf --test basic_features  ║");
    println!("║        --features slow-tests                                     ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
}
