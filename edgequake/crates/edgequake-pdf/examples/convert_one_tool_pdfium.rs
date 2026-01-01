use std::path::PathBuf;
use std::sync::Arc;

#[cfg(feature = "pdfium")]
use edgequake_llm::providers::mock::MockProvider;
#[cfg(feature = "pdfium")]
use edgequake_pdf::{PdfConfig, PdfExtractor, PdfiumBackend};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    #[cfg(not(feature = "pdfium"))]
    {
        println!("Pdfium feature not enabled. Run with: cargo run --example convert_one_tool_pdfium --features pdfium");
        return Ok(());
    }

    #[cfg(feature = "pdfium")]
    {
        let args: Vec<String> = std::env::args().collect();
        let (sample_pdf, out_md) = if args.len() > 1 {
            let input = PathBuf::from(&args[1]);
            let mut output = input.clone();
            output.set_extension("md");
            (input, output)
        } else {
            let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let input = manifest_dir.join("test-data").join("one_tool.pdf");
            let output = manifest_dir.join("test-data").join("one_tool_pdfium.md");
            (input, output)
        };

        println!(
            "Testing PdfExtractor with PdfiumBackend on {} (SOTA quality)...",
            sample_pdf.display()
        );

        // Create PdfiumBackend and wrap in PdfExtractor to use full processor chain
        let backend = Box::new(PdfiumBackend::new()?);
        let llm_provider = Arc::new(MockProvider::new());
        let config = PdfConfig::default();
        let extractor = PdfExtractor::with_backend(backend, llm_provider, config);

        let pdf_bytes = std::fs::read(&sample_pdf)?;

        println!(
            "Extracting {} to markdown with full processor chain...",
            sample_pdf.display()
        );

        let markdown = extractor.extract_to_markdown(&pdf_bytes).await?;

        std::fs::write(&out_md, &markdown)?;

        println!("✓ Wrote markdown to {}", out_md.display());
        println!("✓ Pdfium SOTA extraction completed successfully!");

        Ok(())
    }
}
