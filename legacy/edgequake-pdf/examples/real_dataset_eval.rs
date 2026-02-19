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

fn preview_line(line: &str, max_chars: usize) -> String {
    let trimmed = line.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let mut out = String::new();
    for ch in trimmed.chars().take(max_chars) {
        out.push(ch);
    }
    out.push_str("…");
    out
}

#[derive(Debug, Clone)]
struct LineDiff {
    line_no: usize,
    token_count: usize,
    diff_count: usize,
    diff_ratio: f64,
    preview: String,
}

fn line_diff_report(
    pred_text: &str,
    gold_text: &str,
    top_n: usize,
    min_tokens: usize,
    preview_chars: usize,
) -> (Vec<LineDiff>, Vec<LineDiff>) {
    let pred_set = tokenize_for_set(pred_text);
    let gold_set = tokenize_for_set(gold_text);

    let mut worst_extra = Vec::new();
    for (idx, line) in pred_text.lines().enumerate() {
        let line_tokens = tokenize_for_set(line);
        let token_count = line_tokens.len();
        if token_count < min_tokens {
            continue;
        }

        let mut extra = 0usize;
        for t in &line_tokens {
            if !gold_set.contains(t) {
                extra += 1;
            }
        }
        if extra == 0 {
            continue;
        }

        worst_extra.push(LineDiff {
            line_no: idx + 1,
            token_count,
            diff_count: extra,
            diff_ratio: extra as f64 / token_count as f64,
            preview: preview_line(line, preview_chars),
        });
    }

    let mut worst_missing = Vec::new();
    for (idx, line) in gold_text.lines().enumerate() {
        let line_tokens = tokenize_for_set(line);
        let token_count = line_tokens.len();
        if token_count < min_tokens {
            continue;
        }

        let mut missing = 0usize;
        for t in &line_tokens {
            if !pred_set.contains(t) {
                missing += 1;
            }
        }
        if missing == 0 {
            continue;
        }

        worst_missing.push(LineDiff {
            line_no: idx + 1,
            token_count,
            diff_count: missing,
            diff_ratio: missing as f64 / token_count as f64,
            preview: preview_line(line, preview_chars),
        });
    }

    worst_extra.sort_by(|a, b| {
        b.diff_ratio
            .partial_cmp(&a.diff_ratio)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.diff_count.cmp(&a.diff_count))
            .then_with(|| b.token_count.cmp(&a.token_count))
    });
    worst_missing.sort_by(|a, b| {
        b.diff_ratio
            .partial_cmp(&a.diff_ratio)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.diff_count.cmp(&a.diff_count))
            .then_with(|| b.token_count.cmp(&a.token_count))
    });

    worst_extra.truncate(top_n);
    worst_missing.truncate(top_n);

    (worst_extra, worst_missing)
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
    let mut write_outputs = false;
    let mut only_stem: Option<String> = None;
    let mut diff_report = false;
    let mut diff_top_lines: usize = 10;
    let mut diff_min_tokens: usize = 6;
    let mut diff_preview_chars: usize = 160;

    let mut args = std::env::args().skip(1).peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--write" => write_outputs = true,
            "--diff" => diff_report = true,
            "--only" => {
                let Some(stem) = args.next() else {
                    anyhow::bail!("--only requires a PDF stem (filename without extension)");
                };
                only_stem = Some(stem);
            }
            "--diff-top-lines" => {
                let Some(v) = args.next() else {
                    anyhow::bail!("--diff-top-lines requires a number");
                };
                diff_top_lines = v.parse()?;
            }
            "--diff-min-tokens" => {
                let Some(v) = args.next() else {
                    anyhow::bail!("--diff-min-tokens requires a number");
                };
                diff_min_tokens = v.parse()?;
            }
            "--diff-preview-chars" => {
                let Some(v) = args.next() else {
                    anyhow::bail!("--diff-preview-chars requires a number");
                };
                diff_preview_chars = v.parse()?;
            }
            "--help" | "-h" => {
                println!(
                    "Usage: cargo run -p edgequake-pdf --example real_dataset_eval -- [--write] [--only STEM] [--diff]\n\
                     \n\
                     Flags:\n\
                       --write                    Write .mdf.gen outputs\n\
                       --only STEM                Run only one PDF (by filename stem)\n\
                       --diff                     Print a line-level diff report vs gold\n\
                       --diff-top-lines N          Lines to print per report section (default: 10)\n\
                       --diff-min-tokens N         Skip short lines (default: 6)\n\
                       --diff-preview-chars N      Preview chars per line (default: 160)\n"
                );
                return Ok(());
            }
            other => {
                anyhow::bail!("Unknown arg: {other}");
            }
        }
    }

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
    if let Some(stem) = &only_stem {
        println!("Only: {}", stem);
    }
    println!("Diff report: {}", diff_report);

    for pdf_path in pdfs {
        let stem = pdf_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        if let Some(only) = &only_stem {
            if stem != only {
                continue;
            }
        }

        let bytes = fs::read(&pdf_path)?;
        let extracted = extractor.extract_to_markdown(&bytes).await?;

        let counts = count_patterns(&extracted, &camel_re);

        // Compare against existing .gold.md or .md if present.
        let gold_path = pdf_path.with_extension("gold.md");
        let gold_path = if gold_path.exists() {
            gold_path
        } else {
            pdf_path.with_extension("md")
        };

        let (p, r, f1) = if gold_path.exists() {
            let gold = fs::read_to_string(&gold_path)?;
            let pred_set = tokenize_for_set(&extracted);
            let gold_set = tokenize_for_set(&gold);
            set_f1(&pred_set, &gold_set)
        } else {
            (0.0, 0.0, 0.0)
        };

        println!(
            "- {}: chars={}, f1={:.3} (p={:.3}, r={:.3}), patterns={{camel_join={}, hyphen_break={}, double_space={}, arxiv_header={}}}",
            stem,
            extracted.len(),
            f1,
            p,
            r,
            counts.camel_join,
            counts.hyphen_break,
            counts.double_space,
            counts.arxiv_header
        );

        if write_outputs {
            let out_path = pdf_path.with_extension("mdf.gen");
            fs::write(&out_path, &extracted)?;
            println!("  wrote: {}", out_path.display());
        }

        if diff_report && gold_path.exists() {
            let gold = fs::read_to_string(&gold_path)?;
            let (worst_extra, worst_missing) = line_diff_report(
                &extracted,
                &gold,
                diff_top_lines,
                diff_min_tokens,
                diff_preview_chars,
            );

            println!("  diff: worst extra lines (precision killers)");
            if worst_extra.is_empty() {
                println!("    (none)");
            }
            for item in worst_extra {
                println!(
                    "    L{:>5} extra={:>3}/{:>3} ({:.0}%)  {}",
                    item.line_no,
                    item.diff_count,
                    item.token_count,
                    item.diff_ratio * 100.0,
                    item.preview
                );
            }

            println!("  diff: worst missing lines (recall gaps)");
            if worst_missing.is_empty() {
                println!("    (none)");
            }
            for item in worst_missing {
                println!(
                    "    L{:>5} miss={:>3}/{:>3} ({:.0}%)  {}",
                    item.line_no,
                    item.diff_count,
                    item.token_count,
                    item.diff_ratio * 100.0,
                    item.preview
                );
            }
        }
    }

    Ok(())
}
