//! Detailed analysis of test-data PDFs with example outputs
//!
//! This test shows the actual markdown output for each PDF file.

use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::PdfExtractor;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

fn get_test_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-data")
}

fn get_sample_pdfs() -> Vec<PathBuf> {
    vec![
        "001_simple_text.pdf",
        "003_two_columns.pdf",
        "004_simple_table_2x3.pdf",
        "006_multi_column_layout.pdf",
        "008_multi_page_5_pages.pdf",
        "014_table_spanning_cells.pdf",
        "017_three_columns.pdf",
    ]
    .iter()
    .map(|name| get_test_data_dir().join(name))
    .collect()
}

#[tokio::test]
async fn test_detailed_output_analysis() {
    let pdfs = get_sample_pdfs();

    println!("\n");
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║  Detailed PDF Extraction Analysis                              ║");
    println!("║  Sample outputs from key test cases                            ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();

    for pdf_path in pdfs {
        if let Some(file_name) = pdf_path.file_name().and_then(|n| n.to_str()) {
            match fs::read(&pdf_path) {
                Ok(pdf_bytes) => {
                    let extractor = PdfExtractor::new(Arc::new(MockProvider::new()));

                    println!("📄 {}", file_name);
                    println!("───────────────────────────────────────────────────────────");

                    // Get PDF info
                    if let Ok(info) = extractor.get_info(&pdf_bytes) {
                        println!(
                            "Pages: {} | Size: {} bytes | Images: {}",
                            info.page_count,
                            pdf_bytes.len(),
                            info.image_count
                        );
                    }

                    // Extract markdown
                    match extractor.extract_to_markdown(&pdf_bytes).await {
                        Ok(markdown) => {
                            // Get expected markdown from file
                            let expected_path = pdf_path.with_extension("md");
                            let expected = fs::read_to_string(&expected_path)
                                .unwrap_or_else(|_| "N/A".to_string());

                            println!("\n📝 Extracted markdown ({} chars):", markdown.len());
                            println!("```");
                            // Print first 50 lines
                            for (i, line) in markdown.lines().take(50).enumerate() {
                                if i >= 50 {
                                    break;
                                }
                                println!("{}", line);
                            }
                            if markdown.lines().count() > 50 {
                                println!("... ({} more lines)", markdown.lines().count() - 50);
                            }
                            println!("```");

                            println!("\n✅ Expected markdown ({} chars):", expected.len());
                            println!("```");
                            // Print first 20 lines
                            for (i, line) in expected.lines().take(20).enumerate() {
                                if i >= 20 {
                                    break;
                                }
                                println!("{}", line);
                            }
                            if expected.lines().count() > 20 {
                                println!("... ({} more lines)", expected.lines().count() - 20);
                            }
                            println!("```");

                            // Check for exact match
                            if markdown.trim() == expected.trim() {
                                println!("\n✅ PERFECT MATCH with expected output");
                            } else {
                                let extracted_lines = markdown.lines().count();
                                let expected_lines = expected.lines().count();
                                let line_diff =
                                    (extracted_lines as i32 - expected_lines as i32).abs();
                                println!("\n⚠️  OUTPUT DIFFERS");
                                println!(
                                    "   Extracted: {} lines | Expected: {} lines (diff: {})",
                                    extracted_lines, expected_lines, line_diff
                                );
                            }
                        }
                        Err(e) => {
                            println!("❌ Extraction error: {}", e);
                        }
                    }

                    println!();
                }
                Err(e) => {
                    println!("❌ {}: Failed to read - {}", file_name, e);
                }
            }
        }
    }

    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║  Analysis Complete                                             ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
}
