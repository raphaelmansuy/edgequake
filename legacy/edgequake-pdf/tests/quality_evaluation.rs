//! Quality Evaluation Test Suite for PDF-to-Markdown Conversion
//!
//! **⚠️ DEPRECATED:** This test file has been split for performance optimization.
//!
//! **Use instead:**
//! - `quick_smoke.rs` - Fast sanity checks (<5s)
//! - `basic_features.rs` - Feature validation (<30s) [requires --features slow-tests]
//! - `comprehensive_quality.rs` - Full quality metrics (2+ min) [requires --features comprehensive-tests]
//!
//! **Why split:**
//! Original test took 116 seconds to run all 7 PDFs in real_dataset/.
//! Developers need fast feedback (<5s) for most changes.
//!
//! **Migration guide:**
//! ```bash
//! # Old (slow):
//! cargo test --package edgequake-pdf --test quality_evaluation
//!
//! # New (fast smoke tests):
//! cargo test --package edgequake-pdf --test quick_smoke
//!
//! # New (feature tests):
//! cargo test --package edgequake-pdf --test basic_features --features slow-tests
//!
//! # New (comprehensive):
//! cargo test --package edgequake-pdf --test comprehensive_quality --features comprehensive-tests
//! ```
//!
//! **This file:**
//! Kept for backward compatibility. Contains only one fast test.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::PdfExtractor;

// =============================================================================
// Test Configuration
// =============================================================================

fn test_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-data")
}

fn real_dataset_dir() -> PathBuf {
    test_data_dir().join("real_dataset")
}

fn create_extractor() -> PdfExtractor {
    PdfExtractor::new(Arc::new(MockProvider::new()))
}

// =============================================================================
// Quality Metrics
// =============================================================================

/// Calculate text preservation score (0-100%)
fn text_preservation_score(gold: &str, extracted: &str) -> f64 {
    let gold_words: std::collections::HashSet<&str> = gold.split_whitespace().collect();
    let extracted_words: std::collections::HashSet<&str> = extracted.split_whitespace().collect();

    if gold_words.is_empty() {
        return if extracted_words.is_empty() {
            100.0
        } else {
            0.0
        };
    }

    let preserved = gold_words.intersection(&extracted_words).count();
    (preserved as f64 / gold_words.len() as f64) * 100.0
}

/// Calculate structural fidelity (headers, lists, tables detected)
fn structural_fidelity_score(gold: &str, extracted: &str) -> f64 {
    let mut gold_structures = 0;
    let mut matched = 0;

    // Count headers in gold
    let gold_headers = gold.lines().filter(|l| l.starts_with('#')).count();
    let extracted_headers = extracted.lines().filter(|l| l.starts_with('#')).count();
    gold_structures += gold_headers;
    matched += gold_headers.min(extracted_headers);

    // Count list items
    let gold_lists = gold
        .lines()
        .filter(|l| {
            l.trim().starts_with('-')
                || l.trim().starts_with("* ")
                || l.trim()
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
        })
        .count();
    let extracted_lists = extracted
        .lines()
        .filter(|l| {
            l.trim().starts_with('-')
                || l.trim().starts_with("* ")
                || l.trim()
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
        })
        .count();
    gold_structures += gold_lists;
    matched += gold_lists.min(extracted_lists);

    // Count table markers
    let gold_tables = gold.lines().filter(|l| l.contains('|')).count();
    let extracted_tables = extracted.lines().filter(|l| l.contains('|')).count();
    gold_structures += gold_tables;
    matched += gold_tables.min(extracted_tables);

    if gold_structures == 0 {
        return 100.0;
    }

    (matched as f64 / gold_structures as f64) * 100.0
}

// =============================================================================
// Deprecated Tests - Kept for Backward Compatibility Only
// =============================================================================

/// **DEPRECATED:** Use `quick_smoke.rs` instead
#[tokio::test]
async fn test_sample_pdf_extraction() {
    let sample_path = test_data_dir().join("sample.pdf");

    if !sample_path.exists() {
        println!("⚠️  Skipping: sample.pdf not found");
        println!("    Run tests via: cargo test --package edgequake-pdf --test quick_smoke");
        return;
    }

    let extractor = create_extractor();
    let pdf_bytes = fs::read(&sample_path).expect("Failed to read sample.pdf");

    let result = extractor.extract_to_markdown(&pdf_bytes).await;
    assert!(result.is_ok(), "Extraction should succeed");

    let markdown = result.unwrap();
    assert!(!markdown.is_empty(), "Markdown should not be empty");

    println!("✓ sample.pdf extracted ({} chars)", markdown.len());
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║  ⚠️  DEPRECATION NOTICE                                          ║");
    println!("║  This test file has been split for better performance:          ║");
    println!("║  - quick_smoke.rs        (<5s)                                   ║");
    println!("║  - basic_features.rs     (<30s, --features slow-tests)          ║");
    println!("║  - comprehensive_quality.rs (2min, --features comprehensive)    ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
}

/// **DEPRECATED:** Use `comprehensive_quality.rs --features comprehensive-tests` instead
#[tokio::test]
#[ignore] // Ignored by default to avoid 116-second test runs
async fn test_real_dataset_extraction_deprecated() {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║  ⚠️  TEST DEPRECATED                                             ║");
    println!("║  This test took 116 seconds. Use split tests instead:           ║");
    println!("║                                                                  ║");
    println!("║  cargo test --test comprehensive_quality \\                      ║");
    println!("║              --features comprehensive-tests                      ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
}

/// **DEPRECATED:** Use `basic_features.rs --features slow-tests` instead
#[tokio::test]
#[ignore] // Ignored to avoid slow test runs by default
async fn test_numbered_pdfs_extraction_deprecated() {
    println!("⚠️  DEPRECATED: Use basic_features.rs --features slow-tests instead");
}

/// **DEPRECATED:** Use `basic_features.rs --features slow-tests` instead
#[tokio::test]
#[ignore]
async fn test_table_extraction_quality_deprecated() {
    println!("⚠️  DEPRECATED: Use basic_features.rs --features slow-tests instead");
}

/// **DEPRECATED:** Use `basic_features.rs --features slow-tests` instead  
#[tokio::test]
#[ignore]
async fn test_multi_column_layout_deprecated() {
    println!("⚠️  DEPRECATED: Use basic_features.rs --features slow-tests instead");
}
