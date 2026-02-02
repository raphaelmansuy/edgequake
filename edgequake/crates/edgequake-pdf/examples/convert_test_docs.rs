//! Tool to convert test documents from zz_test_docs/ for comparison with markitdown.
//! Run: cargo run --example convert_test_docs --release

use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::{PdfConfig, PdfExtractor};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup minimal logging
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    tracing_subscriber::fmt::init();

    // Test documents in zz_test_docs/
    let test_docs = [
        "Apple-Sandbox-Guide-v1.0.pdf",
        "agentfail_2601.22984v1.pdf",
        "hotmess_2601.23045v1.pdf",
    ];

    // Base path relative to workspace root - go up from crates/edgequake-pdf to edgequake, then to zz_test_docs
    let base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../zz_test_docs");

    // Output directory
    let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test-data")
        .join("edgequake_output");
    fs::create_dir_all(&output_dir)?;

    // Configure extractor
    let config = PdfConfig::new();
    let provider = Arc::new(MockProvider::new());
    let extractor = PdfExtractor::with_config(provider, config);

    for doc in &test_docs {
        let pdf_path = base_path.join(doc);

        if !pdf_path.exists() {
            eprintln!("⚠️  File not found: {:?}", pdf_path);
            continue;
        }

        println!("\n{}", "=".repeat(60));
        println!("📄 Processing: {}", doc);
        println!("{}", "=".repeat(60));

        // Read PDF bytes
        let bytes = fs::read(&pdf_path)?;
        println!("   File size: {} bytes", bytes.len());

        // Extract markdown
        let start = Instant::now();
        let markdown = match extractor.extract_to_markdown(&bytes).await {
            Ok(md) => md,
            Err(e) => {
                eprintln!("   ❌ Extraction failed: {}", e);
                continue;
            }
        };
        let duration = start.elapsed();

        // Output file name
        let output_name = doc.replace(".pdf", "_edgequake.md");
        let output_path = output_dir.join(&output_name);
        fs::write(&output_path, &markdown)?;

        // Statistics
        let char_count = markdown.len();
        let line_count = markdown.lines().count();
        let word_count = markdown.split_whitespace().count();

        println!("   ✅ Extraction complete in {:?}", duration);
        println!("   📊 Output stats:");
        println!("      - Characters: {}", char_count);
        println!("      - Lines: {}", line_count);
        println!("      - Words: {}", word_count);
        println!("   💾 Saved to: {:?}", output_path);

        // Preview first 500 chars
        println!("\n   📝 Preview (first 500 chars):");
        println!("   {}", "-".repeat(40));
        for line in markdown.chars().take(500).collect::<String>().lines() {
            println!("   {}", line);
        }
        println!("   {}", "-".repeat(40));
    }

    println!("\n✨ All conversions complete!");
    println!("   Output directory: {:?}", output_dir);

    Ok(())
}
