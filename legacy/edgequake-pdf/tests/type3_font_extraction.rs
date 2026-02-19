//! Integration tests for Type3 font extraction
//!
//! Tests PDFs using Type3 fonts with ToUnicode streams that previously caused
//! 0-byte output due to incorrect OCR layer detection heuristics.
//!
//! The fix (commit 345bc157) changed OCR layer detection from:
//! - OLD: `has_ocr_layer = actual_max_y > page_height * 2.5` (absolute threshold)
//! - NEW: Bimodal Y distribution detection (looks for gap > 0.8 * page_height)
//!
//! Run with: cargo test --package edgequake-pdf type3_font --no-fail-fast -- --nocapture

use std::fs;
use std::sync::Arc;

use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::PdfExtractor;

/// Test Type3 font extraction with Qwen.pdf
///
/// This PDF uses Type3 fonts with CTM transforms that cause Y coordinates
/// to range from 265-2452 for a 792pt page. The old heuristic incorrectly
/// detected this as an OCR layer and filtered out all text.
///
/// Expected: At least 500 bytes of extracted text (contains ~600 words)
#[tokio::test]
async fn test_type3_font_extraction_qwen() {
    // Read the Qwen PDF
    let pdf_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../zz_test_docs/Qwen.pdf"
    );

    if !std::path::Path::new(pdf_path).exists() {
        println!("Skipping: Qwen.pdf not found at {}", pdf_path);
        return;
    }

    let pdf_bytes = fs::read(pdf_path).expect("Failed to read PDF");
    println!("PDF size: {} bytes", pdf_bytes.len());

    // Create extractor with mock LLM
    let provider = Arc::new(MockProvider::new());
    let extractor = PdfExtractor::new(provider);

    // Extract to markdown
    let markdown = extractor
        .extract_to_markdown(&pdf_bytes)
        .await
        .expect("Type3 font extraction should succeed");

    println!("Markdown length: {} bytes", markdown.len());
    println!("First 300 chars:\n{}", &markdown[..markdown.len().min(300)]);

    // Assert minimum content extracted
    // WHY: This PDF has ~600 words, so we expect at least 500 bytes
    assert!(
        markdown.len() >= 500,
        "Type3 font PDF should extract at least 500 bytes, got {}",
        markdown.len()
    );

    // Assert key content is present
    // WHY: These phrases appear in the PDF and validate proper font decoding
    let markdown_lower = markdown.to_lowercase();
    assert!(
        markdown_lower.contains("qwen") || markdown_lower.contains("reasoning"),
        "Should extract 'Qwen' or 'reasoning' from Type3 font PDF"
    );
}

/// Test document structure extraction for Type3 fonts
#[tokio::test]
async fn test_type3_font_document_structure() {
    let pdf_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../zz_test_docs/Qwen.pdf"
    );

    if !std::path::Path::new(pdf_path).exists() {
        println!("Skipping: Qwen.pdf not found at {}", pdf_path);
        return;
    }

    let pdf_bytes = fs::read(pdf_path).expect("Failed to read PDF");

    let provider = Arc::new(MockProvider::new());
    let extractor = PdfExtractor::new(provider);

    let doc = extractor
        .extract_document(&pdf_bytes)
        .await
        .expect("Document extraction should succeed");

    println!("Document extraction:");
    println!("  Pages: {}", doc.pages.len());

    // Assert we extracted the page
    assert_eq!(doc.pages.len(), 1, "Qwen.pdf has 1 page");

    // Assert we have blocks (not filtered by OCR layer detection)
    let block_count = doc.pages[0].blocks.len();
    println!("  Page 1: {} blocks", block_count);

    // WHY: Before the fix, this was 0 blocks due to OCR layer filtering
    assert!(
        block_count >= 5,
        "Should extract at least 5 blocks from Type3 font PDF, got {}",
        block_count
    );

    // Print first few blocks for debugging
    for (j, block) in doc.pages[0].blocks.iter().take(5).enumerate() {
        println!(
            "    Block {}: {:?} - '{}'",
            j,
            block.block_type,
            block.text.chars().take(50).collect::<String>()
        );
    }
}
