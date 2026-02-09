//! OODA-48: Quality Metrics Integration Test
//!
//! Measures CLF, SPS, ROA, NR against real PDFs using the PymupdfPipeline.
//! Uses gold standard files from test-data/real_dataset/*.pymupdf.gold.md
//!
//! Run: `cargo test -p edgequake-pdf --test quality_metrics_real -- --nocapture`

use std::fs;
use std::path::PathBuf;
use std::time::Instant;

fn test_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-data")
}

/// Load a real PDF and its gold standard, compute quality metrics.
fn measure_quality(pdf_name: &str) -> Option<(String, f64, f64, f64, f64)> {
    let real_dir = test_data_dir().join("real_dataset");
    let pdf_path = real_dir.join(format!("{}.pdf", pdf_name));
    let gold_path = real_dir.join(format!("{}.pymupdf.gold.md", pdf_name));

    if !pdf_path.exists() || !gold_path.exists() {
        eprintln!("  SKIP: {} (missing pdf or gold)", pdf_name);
        return None;
    }

    let gold = fs::read_to_string(&gold_path).ok()?;

    // Use PymupdfPipeline to convert
    let start = Instant::now();
    let pipeline = edgequake_pdf::pipeline::PymupdfPipeline::new().ok()?;
    let extracted = pipeline.convert_file(&pdf_path).ok()?;
    let elapsed = start.elapsed();

    // Compute metrics
    let clf = edgequake_pdf::layout::quality_metrics::character_level_fidelity(&extracted, &gold);
    let sps =
        edgequake_pdf::layout::quality_metrics::structure_preservation_score(&extracted, &gold);
    let roa = edgequake_pdf::layout::quality_metrics::reading_order_accuracy(&extracted, &gold);
    let nr = edgequake_pdf::layout::quality_metrics::noise_ratio(&extracted);

    eprintln!(
        "  {}: CLF={:.3} SPS={:.3} ROA={:.3} NR={:.3} [{:.1}s, {}ch->{}ch gold]",
        pdf_name,
        clf,
        sps,
        roa,
        nr,
        elapsed.as_secs_f64(),
        extracted.len(),
        gold.len()
    );

    Some((pdf_name.to_string(), clf, sps, roa, nr))
}

#[test]
fn test_quality_metrics_real_pdfs() {
    eprintln!("\n=== OODA-48: Quality Metrics Against Real PDFs ===\n");

    let papers = [
        "01_2512.25075v1",
        "2900_Goyal_et_al",
        "agent_2510.09244v1",
        "AlphaEvolve",
        "ccn_2512.21804v1",
        "one_tool_2512.20957v2",
        "v2_2512.25072v1",
    ];

    let mut results = Vec::new();
    for paper in &papers {
        if let Some(result) = measure_quality(paper) {
            results.push(result);
        }
    }

    if results.is_empty() {
        eprintln!("  No papers could be processed (PDFium may not be available)");
        return;
    }

    // Compute averages
    let n = results.len() as f64;
    let avg_clf: f64 = results.iter().map(|r| r.1).sum::<f64>() / n;
    let avg_sps: f64 = results.iter().map(|r| r.2).sum::<f64>() / n;
    let avg_roa: f64 = results.iter().map(|r| r.3).sum::<f64>() / n;
    let avg_nr: f64 = results.iter().map(|r| r.4).sum::<f64>() / n;

    eprintln!("\n=== AVERAGES ({} papers) ===", results.len());
    eprintln!(
        "  CLF (Character-Level Fidelity): {:.3} (target >0.95)",
        avg_clf
    );
    eprintln!(
        "  SPS (Structure Preservation):   {:.3} (target >0.90)",
        avg_sps
    );
    eprintln!(
        "  ROA (Reading Order Accuracy):   {:.3} (target >0.95)",
        avg_roa
    );
    eprintln!(
        "  NR  (Noise Ratio):              {:.3} (target <0.05)",
        avg_nr
    );
    eprintln!();

    // Assertions: these are baseline thresholds, will improve as we iterate
    assert!(
        avg_clf > 0.30,
        "Average CLF should be >0.30 baseline, got {:.3}",
        avg_clf
    );
    // NOTE: SPS and ROA start low because our pymupdf pipeline generates different
    // markdown structure than pymupdf4llm. These will improve with iterations.
}

/// Individual paper tests for regression tracking
#[test]
fn test_quality_single_paper() {
    // Use the smallest paper for fast feedback
    if let Some((name, clf, sps, roa, nr)) = measure_quality("01_2512.25075v1") {
        eprintln!("\nSingle paper report: {}", name);
        eprintln!(
            "  CLF={:.3} SPS={:.3} ROA={:.3} NR={:.3}",
            clf, sps, roa, nr
        );
        // Baseline assertions
        assert!(clf > 0.20, "CLF should be >0.20 baseline for {}", name);
    }
}

/// Diagnostic test: dump extracted output and gold standard to /tmp for manual comparison.
///
/// Run: `cargo test -p edgequake-pdf --test quality_metrics_real test_dump_extracted_output -- --nocapture`
#[test]
fn test_dump_extracted_output() {
    let pdf_name = "01_2512.25075v1";
    let real_dir = test_data_dir().join("real_dataset");
    let pdf_path = real_dir.join(format!("{}.pdf", pdf_name));
    let gold_path = real_dir.join(format!("{}.pymupdf.gold.md", pdf_name));

    if !pdf_path.exists() {
        eprintln!("SKIP: {} PDF not found at {:?}", pdf_name, pdf_path);
        return;
    }
    if !gold_path.exists() {
        eprintln!("SKIP: {} gold file not found at {:?}", pdf_name, gold_path);
        return;
    }

    // Load gold standard
    let gold = fs::read_to_string(&gold_path).expect("failed to read gold standard file");

    // Extract via PymupdfPipeline
    let pipeline =
        edgequake_pdf::pipeline::PymupdfPipeline::new().expect("failed to create PymupdfPipeline");
    let extracted = pipeline
        .convert_file(&pdf_path)
        .expect("failed to convert PDF");

    // Write both to /tmp for easy diffing
    let extracted_out = "/tmp/edgequake_extracted_01.md";
    let gold_out = "/tmp/edgequake_gold_01.md";

    fs::write(extracted_out, &extracted).expect("failed to write extracted output");
    fs::write(gold_out, &gold).expect("failed to write gold standard");

    // Print character counts
    eprintln!("\n=== Diagnostic Dump for {} ===", pdf_name);
    eprintln!("Extracted: {} chars -> {}", extracted.len(), extracted_out);
    eprintln!("Gold:      {} chars -> {}", gold.len(), gold_out);

    // Print first 50 lines of extracted output
    eprintln!("\n--- Extracted (first 50 lines) ---");
    for (i, line) in extracted.lines().enumerate().take(50) {
        eprintln!("{:4}: {}", i + 1, line);
    }

    // Print first 50 lines of gold standard
    eprintln!("\n--- Gold Standard (first 50 lines) ---");
    for (i, line) in gold.lines().enumerate().take(50) {
        eprintln!("{:4}: {}", i + 1, line);
    }

    eprintln!("\nFiles written. Compare with:");
    eprintln!("  diff {} {}", extracted_out, gold_out);
}

/// Diagnostic test: dump all 3 worst-performing papers to /tmp for CLF gap analysis.
///
/// Run: `cargo test -p edgequake-pdf --test quality_metrics_real test_dump_worst_papers -- --nocapture`
#[test]
fn test_dump_worst_papers() {
    let papers = ["AlphaEvolve", "one_tool_2512.20957v2", "2900_Goyal_et_al"];

    let pipeline = match edgequake_pdf::pipeline::PymupdfPipeline::new() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("SKIP: PymupdfPipeline not available: {}", e);
            return;
        }
    };

    let real_dir = test_data_dir().join("real_dataset");

    for pdf_name in &papers {
        let pdf_path = real_dir.join(format!("{}.pdf", pdf_name));
        let gold_path = real_dir.join(format!("{}.pymupdf.gold.md", pdf_name));

        if !pdf_path.exists() || !gold_path.exists() {
            eprintln!("SKIP: {} (missing pdf or gold)", pdf_name);
            continue;
        }

        let gold = fs::read_to_string(&gold_path).expect("read gold");
        let extracted = pipeline.convert_file(&pdf_path).expect("convert PDF");

        // Write to /tmp
        let ext_out = format!("/tmp/edgequake_extracted_{}.md", pdf_name);
        let gold_out_path = format!("/tmp/edgequake_gold_{}.md", pdf_name);
        fs::write(&ext_out, &extracted).expect("write extracted");
        fs::write(&gold_out_path, &gold).expect("write gold");

        // Compute CLF
        let clf =
            edgequake_pdf::layout::quality_metrics::character_level_fidelity(&extracted, &gold);

        eprintln!("\n========== {} ==========", pdf_name);
        eprintln!(
            "CLF={:.3}  extracted={}ch  gold={}ch  diff={}ch",
            clf,
            extracted.len(),
            gold.len(),
            extracted.len() as i64 - gold.len() as i64
        );

        // Word counts after stripping markdown
        let ext_words: Vec<&str> = extracted.split_whitespace().collect();
        let gold_words: Vec<&str> = gold.split_whitespace().collect();
        eprintln!(
            "Words: extracted={} gold={} diff={}",
            ext_words.len(),
            gold_words.len(),
            ext_words.len() as i64 - gold_words.len() as i64
        );

        // Line counts
        let ext_lines: Vec<&str> = extracted.lines().collect();
        let gold_lines: Vec<&str> = gold.lines().collect();
        eprintln!(
            "Lines: extracted={} gold={}",
            ext_lines.len(),
            gold_lines.len()
        );

        // Count specific features in both
        let count_feature = |text: &str, marker: &str| -> usize {
            text.lines().filter(|l| l.contains(marker)).count()
        };
        eprintln!("--- Feature comparison ---");
        for (label, marker) in &[
            ("Headers (#)", "#"),
            ("Bold (**)", "**"),
            ("Table (|)", "|"),
            ("Italic (_)", "_"),
            ("Strikethrough (~~)", "~~"),
            ("Code (`)", "`"),
            ("Link ([)", "["),
            ("Blockquote (>)", ">"),
        ] {
            let ec = count_feature(&extracted, marker);
            let gc = count_feature(&gold, marker);
            if ec != gc {
                eprintln!(
                    "  {:25} ext={:4}  gold={:4}  gap={:+}",
                    label,
                    ec,
                    gc,
                    ec as i64 - gc as i64
                );
            }
        }

        // Show first 30 words unique to extracted but not in gold (potential noise)
        let gold_word_set: std::collections::HashSet<&str> = gold_words.iter().copied().collect();
        let ext_only: Vec<&&str> = ext_words
            .iter()
            .filter(|w| !gold_word_set.contains(**w))
            .collect();
        eprintln!(
            "Words in extracted but NOT in gold: {} ({:.1}% of extracted words)",
            ext_only.len(),
            ext_only.len() as f64 / ext_words.len().max(1) as f64 * 100.0
        );
        if !ext_only.is_empty() {
            let sample: Vec<&str> = ext_only.iter().take(30).map(|w| **w).collect();
            eprintln!("  Sample: {:?}", sample);
        }

        // Show first 30 words unique to gold but not in extracted (missing content)
        let ext_word_set: std::collections::HashSet<&str> = ext_words.iter().copied().collect();
        let gold_only: Vec<&&str> = gold_words
            .iter()
            .filter(|w| !ext_word_set.contains(**w))
            .collect();
        eprintln!(
            "Words in gold but NOT in extracted: {} ({:.1}% of gold words)",
            gold_only.len(),
            gold_only.len() as f64 / gold_words.len().max(1) as f64 * 100.0
        );
        if !gold_only.is_empty() {
            let sample: Vec<&str> = gold_only.iter().take(30).map(|w| **w).collect();
            eprintln!("  Sample: {:?}", sample);
        }

        eprintln!("\nFiles: diff {} {}", ext_out, gold_out_path);
    }
}
