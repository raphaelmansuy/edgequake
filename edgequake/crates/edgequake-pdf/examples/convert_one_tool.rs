use std::path::PathBuf;
use std::sync::Arc;

use edgequake_llm::providers::mock::MockProvider;
use std::sync::Arc;
use edgequake_pdf::{PdfConfig, PdfExtractor};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // OpenAI support removed from this example; use the MockProvider
    let provider = Arc::new(MockProvider::new());

    // Configure to process first 2 pages for faster testing
    // Enable readability enhancement to fix word concatenation issues
    let config = PdfConfig::new()
        .with_max_pages(2)
        .with_image_extraction(false)
        .with_table_enhancement(false)
        .with_readability_enhancement(true);

    let extractor = PdfExtractor::with_config(provider, config);

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sample_pdf = manifest_dir.join("test-data").join("sample.pdf");
    let out_md = manifest_dir.join("test-data").join("sample.md");

    let pdf_bytes = std::fs::read(&sample_pdf)?;

    println!(
        "Extracting {} to markdown (first 6 pages)...",
        sample_pdf.display()
    );

    let markdown = extractor
        .extract_to_markdown(&pdf_bytes)
        .await
        .expect("Failed to extract markdown");

    std::fs::write(&out_md, &markdown)?;

    println!("Wrote markdown to {}", out_md.display());

    Ok(())
}
