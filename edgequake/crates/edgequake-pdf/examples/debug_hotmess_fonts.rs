//! Debug font size from ContentParser for hotmess PDF
use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::{PdfConfig, PdfExtractor};
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Just process page 1 of hotmess
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    tracing_subscriber::fmt::init();

    let pdf_path = PathBuf::from(
        "/Users/raphaelmansuy/Github/03-working/edgequake/zz_test_docs/hotmess_2601.23045v1.pdf",
    );

    let config = PdfConfig::new();
    let provider = Arc::new(MockProvider::new());
    let extractor = PdfExtractor::with_config(provider, config);

    // Read file bytes
    let pdf_bytes = std::fs::read(&pdf_path)?;
    let result = extractor.extract_full(&pdf_bytes).await?;

    // Print first page content
    if let Some(page) = result.pages.first() {
        println!("\n=== First 500 chars of page 1 ===");
        let text = &page.text;
        println!("{}", &text[..500.min(text.len())]);
    }

    Ok(())
}
