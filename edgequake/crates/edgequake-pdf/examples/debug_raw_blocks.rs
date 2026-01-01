//! Debug: Extract raw blocks BEFORE processor chain to trace "arepository" issue

use std::path::PathBuf;

#[cfg(feature = "pdfium")]
use edgequake_pdf::{PdfBackend, PdfiumBackend};

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

        println!("Tracing raw block extraction...\n");

        let backend = PdfiumBackend::new()?;
        let pdf_bytes = std::fs::read(&sample_pdf)?;
        let document = backend.extract(&pdf_bytes).await?;

        // Show all blocks from page 1
        for page in &document.pages {
            println!("Page {} has {} blocks\n", page.number, page.blocks.len());
            
            if page.number > 1 {
                break;
            }
            
            // Show blocks 8-15 in detail
            for (i, block) in page.blocks.iter().enumerate() {
                if i >= 8 && i <= 15 {
                    println!(
                        "Block {}: bbox=({:.1},{:.1})-({:.1},{:.1})",
                        i, block.bbox.x1, block.bbox.y1, block.bbox.x2, block.bbox.y2
                    );
                    println!("  Text: '{}'", block.text.replace('\n', "\\n"));
                    println!();
                }
            }
        }

        Ok(())
    }
}
