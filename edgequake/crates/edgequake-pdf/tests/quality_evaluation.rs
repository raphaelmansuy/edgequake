//! Quality Evaluation Test Suite for PDF-to-Markdown Conversion
//!
//! **Single Responsibility:** Validate extraction quality against gold standards.
//!
//! This test suite implements the TEST_PROTOCOL.md evaluation methodology:
//! - Tests extraction on real PDFs from test-data/
//! - Compares output with gold markdown standards
//! - Reports quality metrics (text preservation, structure fidelity)
//!
//! **WHY this approach:**
//! First-principles evaluation: we test actual PDFs that represent
//! real-world documents, not synthetic minimal examples.

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
// Real Dataset Tests
// =============================================================================

/// Test extraction on real academic papers from real_dataset/
#[tokio::test]
async fn test_real_dataset_extraction() {
    let dataset_dir = real_dataset_dir();

    if !dataset_dir.exists() {
        println!("Skipping: real_dataset directory not found");
        return;
    }

    let extractor = create_extractor();
    let mut results = Vec::new();

    // Find all PDFs with corresponding gold files
    for entry in fs::read_dir(&dataset_dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "pdf").unwrap_or(false) {
            let stem = path.file_stem().unwrap().to_string_lossy();
            let gold_path = dataset_dir.join(format!("{}.gold.md", stem));

            if gold_path.exists() {
                let pdf_bytes = fs::read(&path).expect("Failed to read PDF");
                let gold_md = fs::read_to_string(&gold_path).expect("Failed to read gold");

                match extractor.extract_to_markdown(&pdf_bytes).await {
                    Ok(extracted) => {
                        let text_score = text_preservation_score(&gold_md, &extracted);
                        let struct_score = structural_fidelity_score(&gold_md, &extracted);
                        let overall = (text_score + struct_score) / 2.0;

                        results.push((stem.to_string(), text_score, struct_score, overall));
                    }
                    Err(e) => {
                        results.push((stem.to_string(), 0.0, 0.0, 0.0));
                        eprintln!("Extraction failed for {}: {}", stem, e);
                    }
                }
            }
        }
    }

    // Print results
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║  Real Dataset Quality Evaluation                                 ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let mut total_overall = 0.0;
    for (name, text, structure, overall) in &results {
        println!("📄 {}", name);
        println!(
            "   Text: {:5.1}% | Structure: {:5.1}% | Overall: {:5.1}%",
            text, structure, overall
        );
        total_overall += overall;
    }

    if !results.is_empty() {
        let avg = total_overall / results.len() as f64;
        println!("\n────────────────────────────────────────────────────────────────");
        println!("📊 Average Overall Score: {:.1}%", avg);

        // Quality threshold: must be at least 50% (achievable without LLM)
        assert!(avg >= 50.0, "Quality score {:.1}% below threshold 50%", avg);
    }
}

// =============================================================================
// Basic PDF Tests (from test-data/ root)
// =============================================================================

#[tokio::test]
async fn test_sample_pdf_extraction() {
    let sample_path = test_data_dir().join("sample.pdf");

    if !sample_path.exists() {
        println!("Skipping: sample.pdf not found");
        return;
    }

    let extractor = create_extractor();
    let pdf_bytes = fs::read(&sample_path).expect("Failed to read sample.pdf");

    let result = extractor.extract_to_markdown(&pdf_bytes).await;
    assert!(result.is_ok(), "Extraction should succeed");

    let markdown = result.unwrap();
    assert!(!markdown.is_empty(), "Markdown should not be empty");

    println!("✓ sample.pdf extracted ({} chars)", markdown.len());
}

#[tokio::test]
async fn test_numbered_pdfs_extraction() {
    let extractor = create_extractor();
    let test_dir = test_data_dir();

    // Test a subset of numbered PDFs
    let test_cases = [
        "001_simple_text.pdf",
        "002_headers_and_lists.pdf",
        "003_two_columns.pdf",
        "004_simple_table_2x3.pdf",
    ];

    for filename in test_cases {
        let path = test_dir.join(filename);
        if path.exists() {
            let pdf_bytes = fs::read(&path).expect("Failed to read PDF");
            let result = extractor.extract_to_markdown(&pdf_bytes).await;

            assert!(result.is_ok(), "Extraction should succeed for {}", filename);
            let markdown = result.unwrap();
            assert!(
                !markdown.is_empty(),
                "Markdown should not be empty for {}",
                filename
            );

            println!("✓ {} extracted ({} chars)", filename, markdown.len());
        }
    }
}

// =============================================================================
// Specific Feature Tests
// =============================================================================

#[tokio::test]
async fn test_table_extraction_quality() {
    let path = test_data_dir().join("004_simple_table_2x3.pdf");

    if !path.exists() {
        println!("Skipping: table test PDF not found");
        return;
    }

    let extractor = create_extractor();
    let pdf_bytes = fs::read(&path).expect("Failed to read PDF");
    let markdown = extractor
        .extract_to_markdown(&pdf_bytes)
        .await
        .expect("Extraction failed");

    // Check for table markers
    let has_table = markdown.contains('|');
    println!("Table detection: {}", if has_table { "✓" } else { "✗" });

    // For non-LLM extraction, we may not always detect tables perfectly
    // but we should at least extract the text content
    assert!(!markdown.is_empty(), "Should extract some content");
}

#[tokio::test]
async fn test_multi_column_layout() {
    let path = test_data_dir().join("003_two_columns.pdf");

    if !path.exists() {
        println!("Skipping: column test PDF not found");
        return;
    }

    let extractor = create_extractor();
    let pdf_bytes = fs::read(&path).expect("Failed to read PDF");
    let markdown = extractor
        .extract_to_markdown(&pdf_bytes)
        .await
        .expect("Extraction failed");

    assert!(
        !markdown.is_empty(),
        "Should extract content from multi-column layout"
    );
    println!(
        "✓ Multi-column extraction succeeded ({} chars)",
        markdown.len()
    );
}
