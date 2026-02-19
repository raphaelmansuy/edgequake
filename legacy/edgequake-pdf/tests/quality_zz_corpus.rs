//! Quality metrics test against the zz_test_docs/ corpus (22 diverse PDFs).
//!
//! Quick feedback: runs 7 small representative PDFs in ~20s
//! Full corpus: runs all 22 PDFs in ~2-5min
//!
//! Run quick:  `cargo test -p edgequake-pdf --test quality_zz_corpus test_quick_metrics -- --nocapture`
//! Run full:   `cargo test -p edgequake-pdf --test quality_zz_corpus test_full_corpus -- --nocapture`

use std::fs;
use std::path::PathBuf;
use std::time::Instant;

fn workspace_root() -> PathBuf {
    // edgequake/crates/edgequake-pdf -> ../../.. -> workspace root
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn zz_test_docs() -> PathBuf {
    workspace_root().join("zz_test_docs")
}

/// Measure quality for a PDF file and its gold standard.
/// Returns (short_name, clf, sps, roa, nr, extraction_time_secs).
fn measure_quality(
    pipeline: &edgequake_pdf::pipeline::PymupdfPipeline,
    pdf_path: &std::path::Path,
    gold_path: &std::path::Path,
) -> Option<(String, f64, f64, f64, f64, f64)> {
    if !pdf_path.exists() || !gold_path.exists() {
        eprintln!("  SKIP: {:?} (missing pdf or gold)", pdf_path.file_name());
        return None;
    }

    let gold = fs::read_to_string(gold_path).ok()?;
    if gold.trim().is_empty() {
        eprintln!("  SKIP: {:?} (empty gold)", gold_path.file_name());
        return None;
    }

    let start = Instant::now();
    let extracted = match pipeline.convert_file(pdf_path) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("  ERROR: {:?}: {}", pdf_path.file_name(), e);
            return None;
        }
    };
    let elapsed = start.elapsed().as_secs_f64();

    let clf = edgequake_pdf::layout::quality_metrics::character_level_fidelity(&extracted, &gold);
    let sps =
        edgequake_pdf::layout::quality_metrics::structure_preservation_score(&extracted, &gold);
    let roa = edgequake_pdf::layout::quality_metrics::reading_order_accuracy(&extracted, &gold);
    let nr = edgequake_pdf::layout::quality_metrics::noise_ratio(&extracted);

    let short_name = pdf_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // Truncate name for display
    let display_name = if short_name.len() > 35 {
        format!("{}...", &short_name[..32])
    } else {
        short_name.clone()
    };

    eprintln!(
        "  {:35} CLF={:.3} SPS={:.3} ROA={:.3} NR={:.3} [{:.1}s] w:{}/{}  p:{}/{}",
        display_name,
        clf,
        sps,
        roa,
        nr,
        elapsed,
        extracted.split_whitespace().count(),
        gold.split_whitespace().count(),
        extracted
            .split("\n\n")
            .filter(|p| p.trim().len() >= 5)
            .count(),
        gold.split("\n\n").filter(|p| p.trim().len() >= 5).count(),
    );

    Some((short_name, clf, sps, roa, nr, elapsed))
}

/// Collect all PDF files with gold standards from a directory (recursively).
fn collect_pdfs_with_gold(base_dir: &std::path::Path) -> Vec<(PathBuf, PathBuf)> {
    let mut pairs = Vec::new();
    collect_pdfs_recursive(base_dir, &mut pairs);
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    pairs
}

fn collect_pdfs_recursive(dir: &std::path::Path, pairs: &mut Vec<(PathBuf, PathBuf)>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Skip generated_output directory
            if path.file_name().map_or(false, |n| n == "generated_output") {
                continue;
            }
            collect_pdfs_recursive(&path, pairs);
        } else if path.extension().map_or(false, |e| e == "pdf") {
            // Look for corresponding .pymupdf.gold.md
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            let gold_path = path
                .parent()
                .unwrap()
                .join(format!("{}.pymupdf.gold.md", stem));
            if gold_path.exists() {
                pairs.push((path, gold_path));
            }
        }
    }
}

/// Print summary of results and return averages.
fn print_summary(results: &[(String, f64, f64, f64, f64, f64)]) -> (f64, f64, f64, f64) {
    let n = results.len() as f64;
    let avg_clf: f64 = results.iter().map(|r| r.1).sum::<f64>() / n;
    let avg_sps: f64 = results.iter().map(|r| r.2).sum::<f64>() / n;
    let avg_roa: f64 = results.iter().map(|r| r.3).sum::<f64>() / n;
    let avg_nr: f64 = results.iter().map(|r| r.4).sum::<f64>() / n;
    let total_time: f64 = results.iter().map(|r| r.5).sum();

    eprintln!(
        "\n=== AVERAGES ({} documents, {:.1}s total) ===",
        results.len(),
        total_time
    );
    eprintln!("  CLF={:.3} (target >0.95)", avg_clf);
    eprintln!("  SPS={:.3} (target >0.90)", avg_sps);
    eprintln!("  ROA={:.3} (target >0.95)", avg_roa);
    eprintln!("  NR ={:.3} (target <0.05)", avg_nr);

    // Find worst performers
    if results.len() >= 3 {
        let mut by_clf: Vec<_> = results.iter().collect();
        by_clf.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        eprintln!("\n--- Worst CLF ---");
        for r in by_clf.iter().take(3) {
            eprintln!("  {:35} CLF={:.3}", &r.0[..r.0.len().min(35)], r.1);
        }

        let mut by_roa: Vec<_> = results.iter().collect();
        by_roa.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap());
        eprintln!("--- Worst ROA ---");
        for r in by_roa.iter().take(3) {
            eprintln!("  {:35} ROA={:.3}", &r.0[..r.0.len().min(35)], r.3);
        }
    }

    (avg_clf, avg_sps, avg_roa, avg_nr)
}

/// Quick feedback test: 7 small PDFs from different categories.
/// Runs in ~20s. Use this for rapid OODA iteration.
#[test]
fn test_quick_metrics() {
    eprintln!("\n=== Quick Metrics: zz_test_docs (7 representative PDFs) ===\n");

    let pipeline = match edgequake_pdf::pipeline::PymupdfPipeline::new() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("SKIP: PymupdfPipeline not available: {}", e);
            return;
        }
    };

    let base = zz_test_docs();

    // 7 small, diverse PDFs for quick feedback (<5s conversion time)
    // Avoids slow docs: CCN (50pg/22s), national-capitals (6pg/21s table-only)
    let quick_set: Vec<(&str, &str)> = vec![
        ("technical_docs", "AI_Services__Elitizon"), // 5pg, tech doc
        ("technical_docs", "Apple-Sandbox-Guide-v1.0"), // 48pg, tech guide
        ("reference_materials", "001-BEYONG-TRANFORMER-OUTLINE-V1_1"), // 15pg, outline
        ("reference_materials", "SEAL_U_DM-i-0225-FR-V5"), // 6pg, French ref
        (
            "reference_materials",
            "Scottish SMEs Delegation - AI Learning Expedition to France - February 2026",
        ), // 5pg
        ("academic_papers", "lighrag_2410.05779v3"), // 16pg, academic
        ("academic_papers", "stackplanner_2601.05890v1"), // 16pg, academic
    ];

    let mut results = Vec::new();
    for (subdir, stem) in &quick_set {
        let pdf_path = base.join(subdir).join(format!("{}.pdf", stem));
        let gold_path = base.join(subdir).join(format!("{}.pymupdf.gold.md", stem));
        if let Some(r) = measure_quality(&pipeline, &pdf_path, &gold_path) {
            results.push(r);
        }
    }

    if results.is_empty() {
        eprintln!("  No documents could be processed");
        return;
    }

    let (avg_clf, avg_sps, avg_roa, avg_nr) = print_summary(&results);

    // Baseline assertions (will be tightened as we improve)
    assert!(avg_clf > 0.20, "CLF should be >0.20, got {:.3}", avg_clf);
    assert!(avg_nr < 0.50, "NR should be <0.50, got {:.3}", avg_nr);

    // Print one-liner for commit messages
    eprintln!(
        "\nMetrics: CLF={:.3} SPS={:.3} ROA={:.3} NR={:.3}",
        avg_clf, avg_sps, avg_roa, avg_nr
    );
}

/// Full corpus test: all 22 PDFs in zz_test_docs/.
#[test]
fn test_full_corpus() {
    eprintln!("\n=== Full Corpus: zz_test_docs (all PDFs) ===\n");

    let pipeline = match edgequake_pdf::pipeline::PymupdfPipeline::new() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("SKIP: PymupdfPipeline not available: {}", e);
            return;
        }
    };

    let base = zz_test_docs();
    let pairs = collect_pdfs_with_gold(&base);

    eprintln!("Found {} PDF+gold pairs\n", pairs.len());

    let mut results = Vec::new();
    for (pdf_path, gold_path) in &pairs {
        if let Some(r) = measure_quality(&pipeline, pdf_path, gold_path) {
            results.push(r);
        }
    }

    if results.is_empty() {
        eprintln!("  No documents could be processed");
        return;
    }

    let (avg_clf, avg_sps, avg_roa, avg_nr) = print_summary(&results);

    // Baseline assertions
    assert!(avg_clf > 0.20, "CLF should be >0.20, got {:.3}", avg_clf);
    assert!(avg_nr < 0.50, "NR should be <0.50, got {:.3}", avg_nr);

    eprintln!(
        "\nFull Metrics: CLF={:.3} SPS={:.3} ROA={:.3} NR={:.3}",
        avg_clf, avg_sps, avg_roa, avg_nr
    );
}

/// Dump extracted output for the worst-performing PDF for manual comparison.
#[test]
fn test_dump_worst() {
    let pipeline = match edgequake_pdf::pipeline::PymupdfPipeline::new() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("SKIP: PymupdfPipeline not available: {}", e);
            return;
        }
    };

    let base = zz_test_docs();
    let pairs = collect_pdfs_with_gold(&base);

    // Find worst CLF
    let mut results: Vec<_> = pairs
        .iter()
        .filter_map(|(pdf, gold)| {
            let g = fs::read_to_string(gold).ok()?;
            if g.trim().is_empty() {
                return None;
            }
            let e = pipeline.convert_file(pdf).ok()?;
            let clf = edgequake_pdf::layout::quality_metrics::character_level_fidelity(&e, &g);
            Some((pdf.clone(), gold.clone(), e, g, clf))
        })
        .collect();

    results.sort_by(|a, b| a.4.partial_cmp(&b.4).unwrap());

    for (pdf, _gold, extracted, gold_text, clf) in results.iter().take(3) {
        let name = pdf.file_stem().unwrap_or_default().to_string_lossy();
        let ext_path = format!("/tmp/zz_extracted_{}.md", name);
        let gold_out = format!("/tmp/zz_gold_{}.md", name);

        let _ = fs::write(&ext_path, extracted);
        let _ = fs::write(&gold_out, gold_text);

        eprintln!(
            "\n{}: CLF={:.3}  ext={}ch  gold={}ch",
            name,
            clf,
            extracted.len(),
            gold_text.len()
        );
        eprintln!("  diff {} {}", ext_path, gold_out);
    }
}
