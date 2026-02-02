//! Debug tool to trace merge_line behavior
//!
//! Usage: cargo run --bin debug_merge -- <pdf_path>

use edgequake_pdf::backend::extraction_engine::ExtractionEngine;
use edgequake_pdf::backend::PdfBackend;
use std::env;
use std::path::Path;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <pdf_path>", args[0]);
        std::process::exit(1);
    }

    let pdf_path = Path::new(&args[1]);
    println!("Loading PDF: {}", pdf_path.display());

    // Read file bytes
    let pdf_bytes = std::fs::read(pdf_path).expect("Failed to read PDF file");

    let engine = ExtractionEngine::new();

    match engine.extract(&pdf_bytes).await {
        Ok(doc) => {
            println!("Extracted {} pages", doc.pages.len());
            if let Some(page) = doc.pages.first() {
                println!("\n=== Page 1 blocks ===\n");

                // Find blocks containing "ABOUT" or "Raphaël"
                for (i, block) in page.blocks.iter().enumerate() {
                    if block.text.contains("ABOUT")
                        || block.text.contains("Raphaël")
                        || block.text.contains("AUTHOR")
                    {
                        println!("\nBlock {} - Text: {:?}", i + 1, block.text);
                        println!("  Type: {:?}", block.block_type);
                        println!(
                            "  BBox: [{:.1}, {:.1}, {:.1}, {:.1}]",
                            block.bbox.x1, block.bbox.y1, block.bbox.x2, block.bbox.y2
                        );
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {:?}", e);
        }
    }
}
