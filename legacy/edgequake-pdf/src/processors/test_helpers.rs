//! Shared test utilities for PDF processor tests.
//!
//! **Single Responsibility:** Test fixture creation and common assertions.
//!
//! **WHY centralized fixtures:**
//! - Eliminates duplicate document creation code across 10+ test modules
//! - Ensures consistent test data (same dimensions, margins, font sizes)
//! - Makes tests more readable by hiding boilerplate
//! - Simplifies maintenance when schema changes occur
//!
//! Provides reusable functions for creating test documents, blocks, and pages
//! to reduce duplication across processor test modules.

// Allow dead code since these are test utilities that may not all be used yet
#![allow(dead_code)]

use crate::schema::{Block, BlockType, BoundingBox, Document, FontStyle, Page, TextSpan};

// =============================================================================
// Document Fixtures
// =============================================================================

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

/// Create a document with multiple pages.
pub fn doc_with_pages(page_blocks: Vec<Vec<Block>>) -> Document {
    let mut doc = Document::new();

    for (i, blocks) in page_blocks.into_iter().enumerate() {
        let mut page = test_page(i + 1);
        for block in blocks {
            page.add_block(block);
        }
        doc.add_page(page);
    }

    doc
}

// =============================================================================
// Page Fixtures
// =============================================================================

/// Create a test page with standard dimensions.
///
/// **Default:** US Letter (612 x 792 points)
pub fn test_page(page_num: usize) -> Page {
    Page::new(page_num, 612.0, 792.0)
}

// =============================================================================
// Block Fixtures
// =============================================================================

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

/// Create a bold header block.
pub fn header_block(text: &str, bbox: (f32, f32, f32, f32), level: u8) -> Block {
    let mut block = styled_block(text, bbox, 14.0, 700);
    block.block_type = BlockType::SectionHeader;
    block.level = Some(level);
    block
}

/// Create a code block with monospace font.
pub fn code_block(text: &str, bbox: (f32, f32, f32, f32)) -> Block {
    let mut block = Block::text(text, BoundingBox::new(bbox.0, bbox.1, bbox.2, bbox.3));
    block.block_type = BlockType::Code;
    block.spans = vec![TextSpan::styled(
        text,
        FontStyle {
            family: Some("Courier".to_string()),
            size: Some(10.0),
            weight: Some(400),
            italic: false,
            ..Default::default()
        },
    )];
    block
}

/// Create a text block with monospace font (for testing code detection).
/// Unlike `code_block()`, this does NOT set block_type to Code.
pub fn monospace_block(text: &str, bbox: (f32, f32, f32, f32)) -> Block {
    let mut block = Block::text(text, BoundingBox::new(bbox.0, bbox.1, bbox.2, bbox.3));
    block.spans = vec![TextSpan::styled(
        text,
        FontStyle {
            family: Some("Courier".to_string()),
            size: Some(10.0),
            weight: Some(400),
            italic: false,
            ..Default::default()
        },
    )];
    block
}

/// Create a table cell block.
pub fn table_cell(text: &str, bbox: (f32, f32, f32, f32)) -> Block {
    let mut block = Block::text(text, BoundingBox::new(bbox.0, bbox.1, bbox.2, bbox.3));
    block.block_type = BlockType::TableCell;
    block
}

// =============================================================================
// Standard Bounding Boxes
// =============================================================================

/// Standard content area (72pt margins on US Letter)
pub const CONTENT_LEFT: f32 = 72.0;
pub const CONTENT_RIGHT: f32 = 540.0;
pub const CONTENT_TOP: f32 = 720.0;
pub const CONTENT_BOTTOM: f32 = 72.0;

/// Create a bounding box for a row at given Y position.
pub fn row_bbox(y: f32, height: f32) -> (f32, f32, f32, f32) {
    (CONTENT_LEFT, y, CONTENT_RIGHT, y + height)
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

    #[test]
    fn test_header_block() {
        let block = header_block("Introduction", (0.0, 0.0, 100.0, 20.0), 2);
        assert_eq!(block.block_type, BlockType::SectionHeader);
        assert_eq!(block.level, Some(2));
    }

    #[test]
    fn test_doc_with_pages() {
        let doc = doc_with_pages(vec![
            vec![text_block("Page 1", (0.0, 0.0, 100.0, 20.0))],
            vec![text_block("Page 2", (0.0, 0.0, 100.0, 20.0))],
        ]);
        assert_eq!(doc.pages.len(), 2);
    }
}
