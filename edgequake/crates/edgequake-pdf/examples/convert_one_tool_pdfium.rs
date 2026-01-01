use std::path::PathBuf;

#[cfg(feature = "pdfium")]
use edgequake_pdf::PdfiumExtractor;

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
            "Testing PdfiumExtractor on {} (SOTA quality)...",
            sample_pdf.display()
        );

        let extractor = PdfiumExtractor::new()?;

        let pdf_bytes = std::fs::read(&sample_pdf)?;

        println!(
            "Extracting {} to markdown with Pdfium (character-level word detection)...",
            sample_pdf.display()
        );

        let markdown = extractor.extract_to_markdown(&pdf_bytes)?;

        std::fs::write(&out_md, &markdown)?;

        println!("✓ Wrote markdown to {}", out_md.display());
        println!("✓ Pdfium SOTA extraction completed successfully!");

        Ok(())
    }
}
