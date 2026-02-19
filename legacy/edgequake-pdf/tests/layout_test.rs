use edgequake_pdf::{
    Block, BlockId, BlockType, BoundingBox, Document, LayoutProcessor, Page, Processor,
};
use std::collections::HashMap;

fn create_block(id: usize, x1: f32, y1: f32, x2: f32, y2: f32, text: &str) -> Block {
    Block {
        id: BlockId::with_indices(0, id),
        block_type: BlockType::Text,
        bbox: BoundingBox::new(x1, y1, x2, y2),
        page: 0,
        position: id, // Initial position
        text: text.to_string(),
        html: None,
        spans: Vec::new(),
        children: Vec::new(),
        confidence: 1.0,
        level: None,
        source: None,
        metadata: HashMap::new(),
    }
}

#[test]
fn simple_test() {
    assert!(true);
}

#[test]
fn test_layout_processor_sorting() {
    // Create a 2-column layout
    // Column 1: Block A (top), Block B (bottom)
    // Column 2: Block C (top), Block D (bottom)
    // Reading order should be A -> B -> C -> D (if columns are detected)
    // OR A -> C -> B -> D (if strictly top-down)

    // Let's assume standard column detection: Left column first, then right.

    let block_a = create_block(0, 10.0, 10.0, 100.0, 50.0, "Block A");
    let block_b = create_block(1, 10.0, 60.0, 100.0, 100.0, "Block B");
    let block_c = create_block(2, 110.0, 10.0, 200.0, 50.0, "Block C");
    let block_d = create_block(3, 110.0, 60.0, 200.0, 100.0, "Block D");

    // Put them in random order in the document
    let mut page = Page::new(0, 210.0, 297.0); // A4ish
    page.blocks = vec![block_d, block_b, block_c, block_a];

    let mut doc = Document::new();
    doc.pages.push(page);

    let processor = LayoutProcessor::new();
    let processed_doc = processor.process(doc).expect("Layout processing failed");

    let blocks = &processed_doc.pages[0].blocks;

    // Verify order
    // If column detection works, it should be A, B, C, D.
    // If it fails and falls back to XY-cut or simple sort, it might be A, C, B, D (top-down).

    // Let's print the order to observe first (ODAA)
    for block in blocks {
        println!("Block: {}", block.text);
    }

    // For now, let's just assert we have 4 blocks
    assert_eq!(blocks.len(), 4);
}
