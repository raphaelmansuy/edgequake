//! Evaluate extraction quality on real_dataset PDFs.
//!
//! This example runs the edgequake-pdf extraction pipeline on all PDFs in
//! `test-data/real_dataset/` and reports lightweight metrics.
//!
//! Usage:
//!   cargo run -p edgequake-pdf --example real_dataset_eval
//!   cargo run -p edgequake-pdf --example real_dataset_eval -- --write
//!
//! By default it does not overwrite existing `.mdf` files.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::{PdfConfig, PdfExtractor};
use regex::Regex;

fn tokenize_for_set(text: &str) -> HashSet<String> {
    let mut set = HashSet::new();

    let mut buf = String::with_capacity(text.len());
    for ch in text.chars() {
        let normalized = if ch.is_ascii_alphanumeric() {
            ch.to_ascii_lowercase()
        } else {
            ' '
        };
        buf.push(normalized);
    }

    for token in buf.split_whitespace() {
        if token.len() >= 2 {
            set.insert(token.to_string());
        }
    }

    set
}

fn set_f1(pred: &HashSet<String>, gold: &HashSet<String>) -> (f64, f64, f64) {
    if pred.is_empty() || gold.is_empty() {
        return (0.0, 0.0, 0.0);
    }

    let mut inter = 0usize;
    for t in pred {
        if gold.contains(t) {
            inter += 1;
        }
    }

    let precision = inter as f64 / pred.len() as f64;
    let recall = inter as f64 / gold.len() as f64;
    let f1 = if precision + recall > 0.0 {
        2.0 * precision * recall / (precision + recall)
    } else {
        0.0
    };

    (precision, recall, f1)
}

#[derive(Debug, Default)]
struct PatternCounts {
    camel_join: usize,
    hyphen_break: usize,
    double_space: usize,
    arxiv_header: usize,
}

fn count_patterns(text: &str, camel_re: &Regex) -> PatternCounts {
    let camel_join = camel_re.find_iter(text).count();
    let hyphen_break = text.matches("-\n").count();
    let double_space = text.matches("  ").count();
    let arxiv_header = text.matches("arXiv:").count();

    PatternCounts {
        camel_join,
        hyphen_break,
        double_space,
        arxiv_header,
    }
}

fn list_pdfs(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut pdfs = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("pdf") {
            pdfs.push(path);
        }
    }
    pdfs.sort();
    Ok(pdfs)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let write_outputs = std::env::args().any(|a| a == "--write");

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dataset_dir = manifest_dir.join("test-data").join("real_dataset");

    if !dataset_dir.exists() {
        anyhow::bail!("real_dataset dir not found: {}", dataset_dir.display());
    }

    // Deterministic config (no LLM enhancement).
    let config = PdfConfig::new()
        .with_page_numbers(false)
        .with_image_extraction(false)
        .with_table_enhancement(false)
        .with_readability_enhancement(false);

    let provider = Arc::new(MockProvider::new());
    let extractor = PdfExtractor::with_config(provider, config);

    let pdfs = list_pdfs(&dataset_dir)?;
    if pdfs.is_empty() {
        anyhow::bail!("No PDFs found in {}", dataset_dir.display());
    }

    let camel_re = Regex::new(r"[a-z]{2,}[A-Z][a-z]")?;

    println!("Real-dataset evaluation: {} PDFs", pdfs.len());
    println!("Write outputs: {}", write_outputs);

    for pdf_path in pdfs {
        let stem = pdf_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let bytes = fs::read(&pdf_path)?;
        let extracted = extractor.extract_to_markdown(&bytes).await?;

        let counts = count_patterns(&extracted, &camel_re);

        // Compare against existing .mdf if present.
        let gold_path = pdf_path.with_extension("mdf");
        let (p, r, f1) = if gold_path.exists() {
            let gold = fs::read_to_string(&gold_path)?;
            let pred_set = tokenize_for_set(&extracted);
            let gold_set = tokenize_for_set(&gold);
            set_f1(&pred_set, &gold_set)
        } else {
            (0.0, 0.0, 0.0)
        };

        println!(
            "- {}: chars={}, f1={:.3} (p={:.3}, r={:.3}), patterns={:?}",
            stem,
            extracted.len(),
            f1,
            p,
            r,
            counts
        );

        if write_outputs {
            let out_path = pdf_path.with_extension("mdf.gen");
            fs::write(&out_path, &extracted)?;
            println!("  wrote: {}", out_path.display());
        }
    }

    Ok(())
}
