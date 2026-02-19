//! Comprehensive Quality Evaluation Test Suite
//!
//! **Purpose:** Full quality metrics on real-world PDFs (accepts 2+ minutes)
//! **Usage:** `cargo test --package edgequake-pdf --test comprehensive_quality`
//!
//! **Gated behind `comprehensive-tests` feature:**
//! Run with: `cargo test --test comprehensive_quality --features comprehensive-tests`
//!
//! **Why this split:**
//! Production readiness requires thorough quality validation against gold standards.
//! This test suite processes all PDFs in real_dataset/ and calculates detailed metrics.
//!
//! **Test Selection Criteria:**
//! - Uses ALL PDFs in test-data/real_dataset/ (7+ PDFs, 500KB - 9MB each)
//! - Compares against gold markdown standards (.gold.md files)
//! - Calculates: Text Preservation Score (TPS), Structural Fidelity Score (SFS)
//! - Reports per-PDF and aggregate quality metrics
//! - Target: 74%+ overall quality (current baseline)

#![cfg(feature = "comprehensive-tests")]

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

/// **Text Preservation Score (TPS)**
///
/// **Formula:** TPS = (preserved_words / gold_words) × 100
///
/// **Why this metric:**
/// First-principles evaluation: if we can't extract the words from the PDF,
/// all downstream processing (structure, semantics) fails.
///
/// **Interpretation:**
/// - 98%+: Excellent (near-perfect text extraction)
/// - 90-97%: Good (minor losses acceptable)
/// - 80-89%: Fair (missing content, needs investigation)
/// - <80%: Poor (fundamental extraction issues)
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

/// **Structural Fidelity Score (SFS)**
///
/// **Formula:** SFS = (matched_structures / gold_structures) × 100
///
/// **Why this metric:**
/// Markdown is not just text - it's structured. We need to preserve:
/// - Headers (document hierarchy)
/// - Lists (enumeration, organization)
/// - Tables (data relationships)
///
/// **Implementation:**
/// Counts structural markers in gold vs. extracted markdown:
/// - Headers: lines starting with #
/// - Lists: lines starting with -, *, or digit+.
/// - Tables: lines containing | (markdown table syntax)
///
/// **Interpretation:**
/// - 95%+: Excellent structure preservation
/// - 85-94%: Good (minor structural losses)
/// - 70-84%: Fair (some structure lost, still usable)
/// - <70%: Poor (structure not preserved)
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
// Comprehensive Quality Test
// =============================================================================

/// **Real Dataset Quality Evaluation**
///
/// **Purpose:**
/// Validates extraction quality on real academic PDFs (arXiv papers) against
/// gold standard markdown generated by expert conversion tools.
///
/// **Dataset:**
/// - 7 PDFs from real_dataset/ (total ~27MB)
/// - Each has corresponding .gold.md reference file
/// - PDFs range from 12-page papers to 50-page documents
/// - Covers: multi-column layouts, tables, figures, citations
///
/// **Quality Threshold:**
/// - Target: 50%+ overall score (achievable without LLM enhancement)
/// - Current baseline: 74.6% (as of OODA-09)
/// - With LLM: expected 85%+ (future improvement)
///
/// **Why 50% threshold:**
/// Without LLM-based structure enhancement, we rely on heuristics.
/// 50% ensures basic functionality while allowing room for improvement.
#[tokio::test]
async fn comprehensive_real_dataset_quality() {
    let dataset_dir = real_dataset_dir();

    if !dataset_dir.exists() {
        println!("⚠️  Skipping: real_dataset/ not found");
        println!("    Expected: {}", dataset_dir.display());
        return;
    }

    let extractor = create_extractor();
    let mut results = Vec::new();

    println!("\n🔍 Processing real_dataset/ PDFs...");
    println!("   This will take 1-2 minutes for 7 PDFs\n");

    // Find all PDFs with corresponding gold files
    for entry in fs::read_dir(&dataset_dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "pdf").unwrap_or(false) {
            let stem = path.file_stem().unwrap().to_string_lossy();
            let gold_path = dataset_dir.join(format!("{}.gold.md", stem));

            if gold_path.exists() {
                print!("   Processing {}... ", stem);
                let pdf_bytes = fs::read(&path).expect("Failed to read PDF");
                let gold_md = fs::read_to_string(&gold_path).expect("Failed to read gold");

                match extractor.extract_to_markdown(&pdf_bytes).await {
                    Ok(extracted) => {
                        let text_score = text_preservation_score(&gold_md, &extracted);
                        let struct_score = structural_fidelity_score(&gold_md, &extracted);
                        let overall = (text_score + struct_score) / 2.0;

                        println!("✅ {:.1}%", overall);
                        results.push((stem.to_string(), text_score, struct_score, overall));
                    }
                    Err(e) => {
                        println!("❌ extraction failed");
                        results.push((stem.to_string(), 0.0, 0.0, 0.0));
                        eprintln!("      Error: {}", e);
                    }
                }
            }
        }
    }

    // Print detailed results
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║  Comprehensive Quality Evaluation Results                        ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let mut total_text = 0.0;
    let mut total_struct = 0.0;
    let mut total_overall = 0.0;

    for (name, text, structure, overall) in &results {
        println!("📄 {}", name);
        println!(
            "   Text: {:5.1}% | Structure: {:5.1}% | Overall: {:5.1}%",
            text, structure, overall
        );
        total_text += text;
        total_struct += structure;
        total_overall += overall;
    }

    if !results.is_empty() {
        let count = results.len() as f64;
        let avg_text = total_text / count;
        let avg_struct = total_struct / count;
        let avg_overall = total_overall / count;

        println!("\n────────────────────────────────────────────────────────────────");
        println!("📊 Average Scores:");
        println!("   Text Preservation:    {:.1}%", avg_text);
        println!("   Structural Fidelity:  {:.1}%", avg_struct);
        println!("   Overall Quality:      {:.1}%", avg_overall);
        println!("────────────────────────────────────────────────────────────────");

        // **Why 50% threshold:**
        // Without LLM enhancement, we can still extract text and some structure.
        // 50% ensures we're extracting meaningful content, not failing catastrophically.
        assert!(
            avg_overall >= 50.0,
            "Quality score {:.1}% below 50% threshold. This indicates fundamental extraction issues.",
            avg_overall
        );

        // Print target guidance
        if avg_overall < 70.0 {
            println!("\n⚠️  Quality below 70% - consider improvements:");
            println!("   - Table detection heuristics");
            println!("   - Multi-column reading order");
            println!("   - Header/list structure preservation");
        } else if avg_overall < 85.0 {
            println!("\n✅ Quality acceptable (70-85%)");
            println!("   Consider LLM enhancement for 85%+ target");
        } else {
            println!("\n🎯 Excellent quality (85%+)!");
        }
    }
}

#[tokio::test]
async fn comprehensive_test_summary() {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║  Comprehensive Test Suite Complete                               ║");
    println!("║  Purpose: Full quality validation (2+ minutes)                   ║");
    println!("║  Run: cargo test --package edgequake-pdf                         ║");
    println!("║       --test comprehensive_quality --features comprehensive-tests║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
}
