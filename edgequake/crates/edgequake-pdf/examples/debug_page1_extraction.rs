//! Extract just page 1 of hotmess PDF using the backend directly
//! Run with: cargo run --example debug_page1_extraction

use edgequake_pdf::backend::{ContentParser, ElementProcessor, ExtractionEngine};
use lopdf::Document;
use std::path::Path;

fn main() {
    let pdf_path = Path::new(
        "/Users/raphaelmansuy/Github/03-working/edgequake/zz_test_docs/hotmess_2601.23045v1.pdf",
    );

    println!("=== Page 1 Extraction Debug ===\n");

    let doc = Document::load(pdf_path).expect("Failed to load PDF");
    let engine = ExtractionEngine::new(true, 40.0);

    // Extract page 1
    match engine.extract_page(&doc, 1) {
        Ok(page_content) => {
            println!(
                "Extracted {} blocks from page 1\n",
                page_content.blocks.len()
            );

            for (i, block) in page_content.blocks.iter().enumerate() {
                // Print first 10 blocks
                if i < 10 {
                    let text = &block.content;
                    let preview = if text.len() > 80 {
                        format!("{}...", &text[..80])
                    } else {
                        text.clone()
                    };
                    println!(
                        "Block {}: Y={:.1} type={:?} '{}'",
                        i, block.y, block.block_type, preview
                    );
                }
            }

            println!(
                "\n... and {} more blocks",
                page_content.blocks.len().saturating_sub(10)
            );

            // Look for title
            let has_title = page_content
                .blocks
                .iter()
                .any(|b| b.content.contains("HOT") || b.content.contains("MESS"));
            println!("\nTitle found: {}", has_title);

            // Look for published line
            let has_published = page_content
                .blocks
                .iter()
                .any(|b| b.content.contains("Published") || b.content.contains("ICLR"));
            println!("Published line found: {}", has_published);
        }
        Err(e) => {
            eprintln!("Extraction error: {}", e);
        }
    }
}
