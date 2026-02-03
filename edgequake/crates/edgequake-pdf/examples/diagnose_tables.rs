//! Diagnose table detection for AlphaEvolve PDF
//! Focus: Find where Table 1 content is located

use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::schema::BlockType;
use edgequake_pdf::PdfExtractor;
use std::fs;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let path = "crates/edgequake-pdf/test-data/real_dataset/AlphaEvolve.pdf";
    let pdf_bytes = fs::read(path).expect("Read failed");

    let extractor = PdfExtractor::new(Arc::new(MockProvider::new()));
    let document = extractor
        .extract_document(&pdf_bytes)
        .await
        .expect("Extraction failed");

    println!("=== Searching for Table 1 content (FunSearch vs AlphaEvolve) ===\n");

    // Search all pages for table content
    for page in &document.pages {
        let mut found_table_content = false;
        for block in &page.blocks {
            if block.text.contains("FunSearch")
                || block.text.contains("evolves single")
                || block.text.contains("evolves entire")
                || block.text.contains("evolves up to")
            {
                if !found_table_content {
                    println!("=== Page {} ===", page.number);
                    found_table_content = true;
                }
                println!(
                    "  {:?} @ bbox({:.1},{:.1},{:.1},{:.1}): '{}'",
                    block.block_type,
                    block.bbox.x1,
                    block.bbox.y1,
                    block.bbox.x2,
                    block.bbox.y2,
                    &block.text.chars().take(80).collect::<String>()
                );
            }
        }
        if found_table_content {
            // Show all blocks on this page with their coordinates
            println!("\n  All blocks on page {}:", page.number);
            for (i, block) in page.blocks.iter().enumerate() {
                let text_preview: String = block.text.chars().take(50).collect();
                println!(
                    "    [{}] x1={:>5.1} y1={:>5.1} w={:>5.1} {:?}: '{}'",
                    i,
                    block.bbox.x1,
                    block.bbox.y1,
                    block.bbox.x2 - block.bbox.x1,
                    block.block_type,
                    text_preview
                );
            }
        }
    }

    // Count tables detected across all pages
    let table_count = document
        .pages
        .iter()
        .flat_map(|p| &p.blocks)
        .filter(|b| b.block_type == BlockType::Table)
        .count();

    println!("\n=== Total Tables Detected: {} ===", table_count);
}
