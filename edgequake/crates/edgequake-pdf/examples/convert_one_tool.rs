use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::{PdfConfig, PdfExtractor};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup logging
    std::env::set_var("RUST_LOG", "debug");
    tracing_subscriber::fmt::init();

    // Locate the PDF file relative to the workspace root
    // We assume the command is run from the workspace root or the crate root
    let pdf_path = PathBuf::from("../../test-data/real_dataset/one_tool_2512.20957v2.pdf");

    // Fallback if running from workspace root
    let pdf_path = if pdf_path.exists() {
        pdf_path
    } else {
        PathBuf::from("test-data/real_dataset/one_tool_2512.20957v2.pdf")
    };

    if !pdf_path.exists() {
        eprintln!("Error: PDF file not found at {:?}", pdf_path);
        // Try to list files to help debug
        if let Ok(entries) = fs::read_dir(".") {
            println!("Current directory entries:");
            for entry in entries {
                if let Ok(entry) = entry {
                    println!("  {:?}", entry.path());
                }
            }
        }
        return Ok(());
    }

    println!("Processing {:?}", pdf_path);

    // Configure extractor with SOTA backend (default when lopdf feature is enabled)
    let config = PdfConfig::new();

    // Use MockProvider for now as we are testing the backend extraction logic
    let provider = Arc::new(MockProvider::new());

    let extractor = PdfExtractor::with_config(provider, config);

    // Read PDF bytes
    let bytes = fs::read(&pdf_path)?;

    // Extract
    let start = Instant::now();
    let markdown = extractor.extract_to_markdown(&bytes).await?;
    let duration = start.elapsed();

    println!("Extraction took {:?}", duration);

    // Save output
    let output_path = PathBuf::from("one_tool_output.md");
    fs::write(&output_path, &markdown)?;

    println!("Output saved to {:?}", output_path);

    // Print first 500 chars to verify
    println!("\n--- Preview ---\n");
    println!("{}", &markdown.chars().take(500).collect::<String>());

    Ok(())
}
