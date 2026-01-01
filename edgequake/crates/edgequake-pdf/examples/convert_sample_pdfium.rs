use std::path::PathBuf;

#[cfg(feature = "pdfium")]
use edgequake_pdf::{PdfiumBackend, PdfBackend, MarkdownRenderer, Renderer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(feature = "pdfium"))]
    {
        println!("Pdfium feature not enabled. Run with: cargo run --example convert_sample_pdfium --features pdfium");
        return Ok(());
    }

    #[cfg(feature = "pdfium")]
    {
        println!("Testing PdfiumBackend on sample.pdf...");

        let backend = PdfiumBackend::new()?;

        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let sample_pdf = manifest_dir.join("test-data").join("sample.pdf");
        let out_md = manifest_dir.join("test-data").join("sample_pdfium.md");

        let pdf_bytes = std::fs::read(&sample_pdf)?;

        println!(
            "Extracting {} to markdown with Pdfium...",
            sample_pdf.display()
        );

        let document = backend.extract(&pdf_bytes).await?;
        let renderer = MarkdownRenderer::default();
        let markdown = renderer.render(&document)?;

        std::fs::write(&out_md, &markdown)?;

        println!("✓ Wrote markdown to {}", out_md.display());
        println!("✓ Pdfium extraction completed successfully!");

        Ok(())
    }
}
