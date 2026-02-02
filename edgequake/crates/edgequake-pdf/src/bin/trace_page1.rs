//! Diagnostic tool to trace text extraction from page 1
//!
//! Usage: cargo run --bin trace_page1 -- <pdf_path>

use edgequake_pdf::backend::extraction_engine::ExtractionEngine;
use edgequake_pdf::backend::PdfBackend;
use std::env;
use std::path::Path;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

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
            println!("\n=== Document has {} pages ===\n", doc.pages.len());
            
            // Only look at page 1
            if let Some(page) = doc.pages.first() {
                println!("Page 1 has {} blocks\n", page.blocks.len());
                
                for (i, block) in page.blocks.iter().enumerate() {
                    println!("Block {} - Type: {:?}", i + 1, block.block_type);
                    println!("  BBox: [{:.1}, {:.1}, {:.1}, {:.1}]", 
                        block.bbox.x1, block.bbox.y1, block.bbox.x2, block.bbox.y2);
                    println!("  Text ({} chars): '{}'", 
                        block.text.len(),
                        &block.text.chars().take(100).collect::<String>());
                    println!();
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {:?}", e);
        }
    }
}
