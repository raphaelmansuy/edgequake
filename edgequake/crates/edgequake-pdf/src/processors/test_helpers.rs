//! Shared test utilities for PDF processor tests.
//!
//! **Single Responsibility:** Test fixture creation and common assertions.
//!
//! Provides reusable functions for creating test documents, blocks, and pages
//! to reduce duplication across processor test modules.

use crate::schema::{Block, BoundingBox, Document, FontStyle, Page, TextSpan};

/// Create a minimal test document with default pages.
///
/// **Use Case:** Testing processors that just need a valid document structure.
pub fn create_test_document() -> Document {
    let mut doc = Document::new();
    let mut page = Page::new(1, 612.0, 792.0); // Standard US Letter size

    page.add_block(Block::text(
        "First paragraph.",
        BoundingBox::new(72.0, 100.0, 540.0, 130.0),
    ));
    page.add_block(Block::text(
        "Second paragraph.",
        BoundingBox::new(72.0, 150.0, 540.0, 180.0),
    ));

    doc.add_page(page);
    doc
}

/// Create a test block with plain text.
///
/// **Parameters:**
/// - `text`: Block content
/// - `bbox`: Bounding box coordinates (x1, y1, x2, y2)
pub fn text_block(text: &str, bbox: (f32, f32, f32, f32)) -> Block {
    Block::text(text, BoundingBox::new(bbox.0, bbox.1, bbox.2, bbox.3))
}

/// Create a test block with styled spans.
///
/// **Parameters:**
/// - `text`: Block content  
/// - `bbox`: Bounding box coordinates
/// - `font_size`: Font size in points
/// - `font_weight`: Font weight (400 = normal, 700 = bold)
pub fn styled_block(
    text: &str,
    bbox: (f32, f32, f32, f32),
    font_size: f32,
    font_weight: u16,
) -> Block {
    let mut block = Block::text(text, BoundingBox::new(bbox.0, bbox.1, bbox.2, bbox.3));
    block.spans = vec![TextSpan::styled(
        text,
        FontStyle {
            family: Some("Times-Roman".to_string()),
            size: Some(font_size),
            weight: Some(font_weight),
            italic: false,
            ..Default::default()
        },
    )];
    block
}

/// Create a test page with standard dimensions.
///
/// **Default:** US Letter (612 x 792 points)
pub fn test_page(page_num: usize) -> Page {
    Page::new(page_num, 612.0, 792.0)
}

/// Create a document with a single page containing given blocks.
pub fn doc_with_blocks(blocks: Vec<Block>) -> Document {
    let mut doc = Document::new();
    let mut page = test_page(1);

    for block in blocks {
        page.add_block(block);
    }

    doc.add_page(page);
    doc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_document() {
        let doc = create_test_document();
        assert_eq!(doc.pages.len(), 1);
        assert_eq!(doc.pages[0].blocks.len(), 2);
    }

    #[test]
    fn test_text_block() {
        let block = text_block("Hello", (0.0, 0.0, 100.0, 20.0));
        assert_eq!(block.text, "Hello");
    }

    #[test]
    fn test_styled_block() {
        let block = styled_block("Bold text", (0.0, 0.0, 100.0, 20.0), 14.0, 700);
        assert_eq!(block.spans.len(), 1);
        assert_eq!(block.spans[0].style.weight, Some(700));
    }
}
