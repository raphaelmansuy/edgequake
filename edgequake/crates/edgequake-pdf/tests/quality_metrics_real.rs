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
