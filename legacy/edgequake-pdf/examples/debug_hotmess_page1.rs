//! Debug hotmess page 1 extraction - shows exactly what elements are extracted
//! Run with: cargo run --example debug_hotmess_page1

use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::{PdfConfig, PdfExtractor};
use std::path::Path;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Enable logging
    std::env::set_var("RUST_LOG", "edgequake_pdf::backend::text_grouping=debug");
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(true)
        .init();

    let pdf_path = Path::new(
        "/Users/raphaelmansuy/Github/03-working/edgequake/zz_test_docs/hotmess_2601.23045v1.pdf",
    );

    if !pdf_path.exists() {
        eprintln!("PDF not found: {}", pdf_path.display());
        return;
    }

    // Read PDF bytes
    let pdf_bytes = std::fs::read(pdf_path).expect("Failed to read PDF");

    // Configure extraction
    let config = PdfConfig::new();
    let provider = Arc::new(MockProvider::new());
    let extractor = PdfExtractor::with_config(provider, config);

    println!("=== Extracting hotmess PDF ===\n");

    match extractor.extract_to_markdown(&pdf_bytes).await {
        Ok(markdown) => {
            println!("\n=== MARKDOWN OUTPUT (first 3000 chars) ===\n");
            let truncated = if markdown.len() > 3000 {
                &markdown[..3000]
            } else {
                &markdown
            };
            println!("{}", truncated);
        }
        Err(e) => {
            eprintln!("Extraction error: {}", e);
        }
    }
}
