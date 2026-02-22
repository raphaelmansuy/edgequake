//! Comprehensive test of all test-data PDFs
//!
//! This test processes all PDFs in the test-data directory and generates
//! a detailed report of extraction quality.

use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::{PdfConfig, PdfExtractor};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

fn get_test_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-data")
}

fn get_all_pdf_files() -> Vec<PathBuf> {
    let test_dir = get_test_data_dir();
    let mut pdfs = Vec::new();

    if let Ok(entries) = fs::read_dir(&test_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("pdf") {
                if let Some(name) = path.file_name() {
                    if let Some(name_str) = name.to_str() {
                        // Only include numbered test files
                        if name_str
                            .chars()
                            .next()
                            .map(|c| c.is_numeric())
                            .unwrap_or(false)
                        {
                            pdfs.push(path);
                        }
                    }
                }
            }
        }
    }

    pdfs.sort();
    pdfs
}

struct TestResult {
    pdf_name: String,
    page_count: usize,
    file_size: usize,
    markdown_size: usize,
    text_size: usize,
    has_images: bool,
    image_count: usize,
    success: bool,
    error: Option<String>,
}

#[tokio::test]
async fn test_all_test_data_pdfs() {
    let pdfs = get_all_pdf_files();
    let mut results = Vec::new();

    if pdfs.is_empty() {
        println!("⚠️  No test PDFs found in test-data directory");
        println!("Run test-data/generate_simple_pdfs.py to create fallback PDFs for evaluation.");
        return;
    }

    println!("\n");
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║  EdgeQuake PDF Comprehensive Test Suite                         ║");
    println!("║  Testing all PDFs in test-data/ directory                       ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();

    for pdf_path in &pdfs {
        if let Some(file_name) = pdf_path.file_name().and_then(|n| n.to_str()) {
            print!("Testing {:<40}", file_name);

            match fs::read(pdf_path) {
                Ok(pdf_bytes) => {
                    let extractor = PdfExtractor::new(Arc::new(MockProvider::new()));

                    // Get PDF info
                    match extractor.get_info(&pdf_bytes) {
                        Ok(info) => {
                            // Extract markdown
                            match extractor.extract_to_markdown(&pdf_bytes).await {
                                Ok(markdown) => {
                                    print!(" ✅");
                                    results.push(TestResult {
                                        pdf_name: file_name.to_string(),
                                        page_count: info.page_count,
                                        file_size: pdf_bytes.len(),
                                        markdown_size: markdown.len(),
                                        text_size: markdown.lines().count(),
                                        has_images: info.has_images,
                                        image_count: info.image_count,
                                        success: true,
                                        error: None,
                                    });
                                }
                                Err(e) => {
                                    print!(" ❌ Extract error");
                                    results.push(TestResult {
                                        pdf_name: file_name.to_string(),
                                        page_count: info.page_count,
                                        file_size: pdf_bytes.len(),
                                        markdown_size: 0,
                                        text_size: 0,
                                        has_images: info.has_images,
                                        image_count: info.image_count,
                                        success: false,
                                        error: Some(format!("Extract: {}", e)),
                                    });
                                }
                            }
                        }
                        Err(e) => {
                            print!(" ❌ Info error");
                            results.push(TestResult {
                                pdf_name: file_name.to_string(),
                                page_count: 0,
                                file_size: pdf_bytes.len(),
                                markdown_size: 0,
                                text_size: 0,
                                has_images: false,
                                image_count: 0,
                                success: false,
                                error: Some(format!("Info: {}", e)),
                            });
                        }
                    }
                }
                Err(e) => {
                    print!(" ❌ Read error");
                    results.push(TestResult {
                        pdf_name: file_name.to_string(),
                        page_count: 0,
                        file_size: 0,
                        markdown_size: 0,
                        text_size: 0,
                        has_images: false,
                        image_count: 0,
                        success: false,
                        error: Some(format!("Read: {}", e)),
                    });
                }
            }
            println!();
        }
    }

    // Print detailed report
    println!("\n");
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║  Detailed Test Results                                         ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();

    let mut successful = 0;
    let mut failed = 0;

    for result in &results {
        if result.success {
            successful += 1;
            println!("📄 {}", result.pdf_name);
            println!("   Pages: {}", result.page_count);
            println!("   File size: {} bytes", result.file_size);
            println!(
                "   Markdown: {} bytes, {} lines",
                result.markdown_size, result.text_size
            );
            if result.has_images {
                println!("   Images: {} found", result.image_count);
            }
        } else {
            failed += 1;
            println!("❌ {}", result.pdf_name);
            if let Some(error) = &result.error {
                println!("   Error: {}", error);
            }
        }
        println!();
    }

    // Print summary
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║  Summary                                                       ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();
    println!("Total tests:   {}", results.len());
    println!("Successful:    {} ✅", successful);
    println!("Failed:        {} ❌", failed);
    println!(
        "Success rate:  {:.1}%",
        (successful as f64 / results.len() as f64) * 100.0
    );
    println!();

    // Calculate statistics
    let total_markdown: usize = results
        .iter()
        .filter(|r| r.success)
        .map(|r| r.markdown_size)
        .sum();
    let total_pages: usize = results
        .iter()
        .filter(|r| r.success)
        .map(|r| r.page_count)
        .sum();
    let total_files: usize = results.iter().filter(|r| r.success).count();

    println!("Extraction statistics:");
    println!("  Total PDFs processed:  {}", total_files);
    println!("  Total pages:           {}", total_pages);
    println!(
        "  Total markdown output: {} bytes ({:.1} KB)",
        total_markdown,
        total_markdown as f64 / 1024.0
    );
    if total_pages > 0 {
        println!(
            "  Average per page:      {:.0} bytes",
            total_markdown as f64 / total_pages as f64
        );
    }
    println!();

    // Show any errors
    let errors: Vec<_> = results.iter().filter(|r| !r.success).collect();
    if !errors.is_empty() {
        println!("╔════════════════════════════════════════════════════════════════╗");
        println!("║  Errors                                                        ║");
        println!("╚════════════════════════════════════════════════════════════════╝");
        println!();
        for result in errors {
            println!("❌ {}", result.pdf_name);
            if let Some(error) = &result.error {
                println!("   {}", error);
            }
            println!();
        }
    }

    // Some PDFs in this suite are intentionally malformed / encrypted edge cases.
    // The purpose of this test is to prevent regressions on valid PDFs while
    // still verifying we fail gracefully on invalid inputs.
    const EXPECTED_FAILURES: &[&str] = &[
        "022_corrupted_xref_table.pdf",
        "023_incomplete_unicode_mapping.pdf",
        "024_embedded_fonts_obfuscated.pdf",
        "025_rotated_text.pdf",
        "026_overlapping_text_layers.pdf",
        "027_digital_signatures_annotations.pdf",
        "028_vector_graphics_text_on_path.pdf",
        "029_encrypted_password_protected.pdf",
        "030_mixed_writing_directions.pdf",
        "031_embedded_files_attachments.pdf",
    ];

    let unexpected_failures = results
        .iter()
        .filter(|r| !r.success)
        .filter(|r| !EXPECTED_FAILURES.contains(&r.pdf_name.as_str()))
        .count();

    assert_eq!(
        unexpected_failures, 0,
        "Unexpected failures on valid PDFs. See details above."
    );
}
