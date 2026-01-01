use std::path::PathBuf;

#[cfg(feature = "pdfium")]
use edgequake_pdf::{PdfBackend, PdfiumBackend};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    #[cfg(not(feature = "pdfium"))]
    {
        println!("Pdfium feature not enabled. Run with: cargo run --example debug_blocks --features pdfium");
        return Ok(());
    }

    #[cfg(feature = "pdfium")]
    {
        let args: Vec<String> = std::env::args().collect();
        let sample_pdf = if args.len() > 1 {
            PathBuf::from(&args[1])
        } else {
            let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            manifest_dir.join("test-data/real_dataset/one_tool_2512.20957v2.pdf")
        };

        println!("Analyzing blocks in {}...\n", sample_pdf.display());

        let backend = PdfiumBackend::new()?;
        let pdf_bytes = std::fs::read(&sample_pdf)?;
        let document = backend.extract(&pdf_bytes).await?;

        // Only analyze first page
        if let Some(page) = document.pages.first() {
            println!("Page 1 dimensions: {}x{}", page.width, page.height);
            println!("Number of blocks: {}\n", page.blocks.len());

            // Show blocks that look like line numbers (single digit or short)
            println!("=== Potential line number blocks ===");
            for (i, block) in page.blocks.iter().enumerate() {
                let text = block.text.trim();
                if text.len() <= 3 && !text.is_empty() {
                    println!(
                        "[{}] text='{}' pos=({:.1}, {:.1}) - ({:.1}, {:.1})",
                        i, text, block.bbox.x1, block.bbox.y1, block.bbox.x2, block.bbox.y2
                    );
                }
            }

            println!("\n=== First 30 blocks ===");
            for (i, block) in page.blocks.iter().take(30).enumerate() {
                let text = if block.text.len() > 50 {
                    format!("{}...", &block.text[..50])
                } else {
                    block.text.clone()
                };
                println!(
                    "[{}] ({:.1},{:.1})-({:.1},{:.1}) '{}' ",
                    i,
                    block.bbox.x1,
                    block.bbox.y1,
                    block.bbox.x2,
                    block.bbox.y2,
                    text.replace('\n', "\\n")
                );
            }
        }

        Ok(())
    }
}
