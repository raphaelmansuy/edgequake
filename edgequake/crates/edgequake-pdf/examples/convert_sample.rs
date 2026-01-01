use std::path::PathBuf;
use std::sync::Arc;

use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::PdfExtractor;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let provider = Arc::new(MockProvider::new());
    let extractor = PdfExtractor::new(provider);

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sample_pdf = manifest_dir.join("test-data").join("sample.pdf");
    let out_md = manifest_dir.join("test-data").join("sample.md");

    let pdf_bytes = std::fs::read(&sample_pdf)?;

    let markdown = extractor
        .extract_to_markdown(&pdf_bytes)
        .await
        .expect("Failed to extract markdown");

    std::fs::write(&out_md, &markdown)?;

    println!("Wrote markdown to {}", out_md.display());

    Ok(())
}
