//! Debug: Trace the block merge process to find where "arepository" is formed

use std::path::PathBuf;
use std::sync::Arc;

#[cfg(feature = "pdfium")]
use edgequake_llm::providers::mock::MockProvider;
#[cfg(feature = "pdfium")]
use edgequake_pdf::{PdfConfig, PdfExtractor, PdfiumBackend, PdfBackend};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(feature = "pdfium"))]
    {
        println!("Pdfium feature not enabled.");
        return Ok(());
    }

    #[cfg(feature = "pdfium")]
    {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let sample_pdf = manifest_dir.join("test-data/real_dataset/one_tool_2512.20957v2.pdf");

        println!("Tracing block merge process...\n");

        let pdf_bytes = std::fs::read(&sample_pdf)?;

        // Step 1: Raw extraction
        println!("=== STEP 1: Raw Extraction ===");
        let backend = PdfiumBackend::new()?;
        let raw_doc = backend.extract(&pdf_bytes).await?;
        
        if let Some(page) = raw_doc.pages.first() {
            // Find blocks containing "as a" or "repository"
            for (i, block) in page.blocks.iter().enumerate() {
                if block.text.contains("as a") || block.text.starts_with("repository") {
                    println!(
                        "Block {}: '{}...'",
                        i,
                        if block.text.len() > 50 { &block.text[..50] } else { &block.text }
                    );
                }
            }
        }
        
        // Step 2: Final markdown
        println!("\n=== STEP 2: Final Markdown ===");
        let backend = Box::new(PdfiumBackend::new()?);
        let llm_provider = Arc::new(MockProvider::new());
        let config = PdfConfig::default();
        let extractor = PdfExtractor::with_backend(backend, llm_provider, config);
        let markdown = extractor.extract_to_markdown(&pdf_bytes).await?;
        
        // Find the problematic section
        for (i, line) in markdown.lines().enumerate() {
            if line.contains("arepository") {
                println!("Line {}: FOUND 'arepository'", i + 1);
                println!("  '{}'", if line.len() > 100 { &line[..100] } else { line });
            }
            if line.contains("as a repository") {
                println!("Line {}: FOUND 'as a repository' (CORRECT!)", i + 1);
                println!("  '{}'", if line.len() > 100 { &line[..100] } else { line });
            }
        }

        Ok(())
    }
}
