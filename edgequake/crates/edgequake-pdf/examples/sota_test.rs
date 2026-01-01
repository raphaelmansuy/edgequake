//! Test extraction of the one_tool paper for SOTA quality validation.

use std::path::PathBuf;
use std::sync::Arc;

use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::{PdfConfig, PdfExtractor};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for debug output
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "edgequake_pdf=info".to_string()),
        )
        .init();

    // OpenAI support removed from this crate: use MockProvider for examples
    let provider = Arc::new(MockProvider::new());

    // Configure for full document extraction without AI enhancements
    let config = PdfConfig::new()
        .with_max_pages(10) // First 10 pages for testing
        .with_image_extraction(false)
        .with_table_enhancement(false)
        .with_readability_enhancement(false);

    let extractor = PdfExtractor::with_config(provider, config);

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let pdf_path = manifest_dir
        .join("test-data")
        .join("real_dataset")
        .join("one_tool_2512.20957v2.pdf");
    let out_path = manifest_dir
        .join("test-data")
        .join("real_dataset")
        .join("one_tool_2512.20957v2.md");

    println!("Input: {}", pdf_path.display());
    println!("Output: {}", out_path.display());

    let pdf_bytes = std::fs::read(&pdf_path)?;
    println!("PDF size: {} bytes", pdf_bytes.len());

    println!("Extracting...");
    let markdown = extractor
        .extract_to_markdown(&pdf_bytes)
        .await
        .expect("Failed to extract markdown");

    std::fs::write(&out_path, &markdown)?;

    println!("Wrote {} bytes to {}", markdown.len(), out_path.display());
    println!("\n=== First 2000 chars of output ===\n");
    println!("{}", &markdown[..markdown.len().min(2000)]);

    Ok(())
}
