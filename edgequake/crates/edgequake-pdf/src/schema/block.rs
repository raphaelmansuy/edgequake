//! Block definition - the core element in document structure.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::block_types::BlockType;
use super::geometry::BoundingBox;

/// Unique identifier for a block within a document.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockId(String);

impl BlockId {
    /// Create a new block ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generate a new unique block ID.
    pub fn generate() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        Self(format!("block_{}", id))
    }

    /// Generate a block ID with page and element indices.
    pub fn with_indices(page: usize, block: usize) -> Self {
        Self(format!("p{}b{}", page, block))
    }

    /// Get the string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for BlockId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for BlockId {
    fn default() -> Self {
        Self::generate()
    }
}

/// Font style information for text blocks.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FontStyle {
    /// Font family name
    pub family: Option<String>,
    /// Font size in points
    pub size: Option<f32>,
    /// Font weight (normal=400, bold=700)
    pub weight: Option<u16>,
    /// Is italic
    pub italic: bool,
    /// Is underlined
    pub underline: bool,
    /// Is strikethrough
    pub strikethrough: bool,
    /// Is superscript
    pub superscript: bool,
    /// Is subscript
    pub subscript: bool,
    /// Text color as hex string (e.g., "#000000")
    pub color: Option<String>,
    /// Background color as hex string
    pub background_color: Option<String>,
}

impl FontStyle {
    /// Check if this style represents a heading (bold and/or larger font).
    pub fn looks_like_heading(&self, base_font_size: f32) -> bool {
        let is_bold = self.weight.map(|w| w >= 600).unwrap_or(false);
        let is_larger = self.size.map(|s| s > base_font_size * 1.2).unwrap_or(false);
        is_bold || is_larger
    }

    /// Check if this style indicates code (monospace font).
    pub fn looks_like_code(&self) -> bool {
        self.family
            .as_ref()
            .map(|f| {
                let lower = f.to_lowercase();
                lower.contains("mono")
                    || lower.contains("courier")
                    || lower.contains("consolas")
                    || lower.contains("source code")
            })
            .unwrap_or(false)
    }
}

/// Span within a text block with its own styling.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextSpan {
    /// The text content of this span
    pub text: String,
    /// Bounding box of this span within the page
    pub bbox: Option<BoundingBox>,
    /// Font style for this span
    pub style: FontStyle,
}

impl TextSpan {
    /// Create a new text span with default styling.
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            bbox: None,
            style: FontStyle::default(),
        }
    }

    /// Create a new text span with styling.
    pub fn styled(text: impl Into<String>, style: FontStyle) -> Self {
        Self {
            text: text.into(),
            bbox: None,
            style,
        }
    }
}

/// A block in the document structure.
///
/// Blocks are the fundamental elements in the document hierarchy.
/// They can represent paragraphs, headings, tables, figures, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// Unique identifier for this block
    pub id: BlockId,

    /// The type of this block (text, header, table, etc.)
    pub block_type: BlockType,

    /// Bounding box on the page (in page coordinates)
    pub bbox: BoundingBox,

    /// Page number (0-indexed)
    pub page: usize,

    /// Reading order position on the page
    pub position: usize,

    /// The text content of this block
    pub text: String,

    /// HTML representation (for tables, complex blocks)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,

    /// Structured text spans (for rich text blocks)
    pub spans: Vec<TextSpan>,

    /// Child blocks (for container blocks like tables)
    pub children: Vec<Block>,

    /// Confidence score from classification (0.0 to 1.0)
    pub confidence: f32,

    /// Section/heading level (1-6 for headers)
    pub level: Option<u8>,

    /// Original source (for debugging)
    pub source: Option<String>,

    /// Custom metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Block {
    /// Create a new block with the given type and bounding box.
    pub fn new(block_type: BlockType, bbox: BoundingBox) -> Self {
        Self {
            id: BlockId::generate(),
            block_type,
            bbox,
            page: 0,
            position: 0,
            text: String::new(),
            html: None,
            spans: Vec::new(),
            children: Vec::new(),
            confidence: 1.0,
            level: None,
            source: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a text block.
    pub fn text(text: impl Into<String>, bbox: BoundingBox) -> Self {
        let mut block = Self::new(BlockType::Text, bbox);
        block.text = text.into();
        block
    }

    /// Create a section header block.
    pub fn header(text: impl Into<String>, level: u8, bbox: BoundingBox) -> Self {
        let mut block = Self::new(BlockType::SectionHeader, bbox);
        block.text = text.into();
        block.level = Some(level);
        block
    }

    /// Create a code block.
    pub fn code(text: impl Into<String>, bbox: BoundingBox) -> Self {
        let mut block = Self::new(BlockType::Code, bbox);
        block.text = text.into();
        block
    }

    /// Create a list item block.
    pub fn list_item(text: impl Into<String>, bbox: BoundingBox) -> Self {
        let mut block = Self::new(BlockType::ListItem, bbox);
        block.text = text.into();
        block
    }

    /// Set the page number.
    pub fn on_page(mut self, page: usize) -> Self {
        self.page = page;
        self
    }

    /// Set the position (reading order).
    pub fn at_position(mut self, position: usize) -> Self {
        self.position = position;
        self
    }

    /// Set the confidence score.
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence;
        self
    }

    /// Add a child block.
    pub fn with_child(mut self, child: Block) -> Self {
        self.children.push(child);
        self
    }

    /// Add multiple child blocks.
    pub fn with_children(mut self, children: Vec<Block>) -> Self {
        self.children.extend(children);
        self
    }

    /// Set metadata value.
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Add a text span.
    pub fn add_span(&mut self, span: TextSpan) {
        self.spans.push(span);
    }

    /// Get the text content, either from spans or direct text.
    pub fn get_text(&self) -> &str {
        if !self.text.is_empty() {
            &self.text
        } else if !self.spans.is_empty() {
            // This would need to join spans - for now just return direct text
            &self.text
        } else {
            ""
        }
    }

    /// Get all text content including from children.
    pub fn get_all_text(&self) -> String {
        let mut result = self.text.clone();
        for child in &self.children {
            if !result.is_empty() && !child.get_all_text().is_empty() {
                result.push(' ');
            }
            result.push_str(&child.get_all_text());
        }
        result
    }

    /// Check if this block is empty (no text or children).
    pub fn is_empty(&self) -> bool {
        self.text.is_empty() && self.spans.is_empty() && self.children.is_empty()
    }

    /// Get the total number of blocks (including children).
    pub fn total_blocks(&self) -> usize {
        1 + self
            .children
            .iter()
            .map(|c| c.total_blocks())
            .sum::<usize>()
    }

    /// Recursively iterate over all blocks.
    pub fn iter_all(&self) -> BlockIterator<'_> {
        BlockIterator::new(self)
    }

    /// Merge with another block (combine text and expand bbox).
    pub fn merge(&mut self, other: &Block) {
        if !other.text.is_empty() {
            if !self.text.is_empty() {
                self.text.push(' ');
                // Add space span if we have spans
                if !self.spans.is_empty() || !other.spans.is_empty() {
                    self.spans.push(TextSpan::plain(" "));
                }
            }
            self.text.push_str(&other.text);
        }
        self.bbox = self.bbox.union(&other.bbox);
        self.spans.extend(other.spans.clone());
        // Keep lower confidence
        self.confidence = self.confidence.min(other.confidence);
    }

    /// Sort children by reading order (top-to-bottom, left-to-right).
    pub fn sort_children_by_position(&mut self) {
        self.children.sort_by(|a, b| {
            // Primary: vertical position
            let y_cmp = a.bbox.y1.partial_cmp(&b.bbox.y1).unwrap();
            if y_cmp != std::cmp::Ordering::Equal {
                return y_cmp;
            }
            // Secondary: horizontal position
            a.bbox.x1.partial_cmp(&b.bbox.x1).unwrap()
        });
    }
}

impl Default for Block {
    fn default() -> Self {
        Self::new(BlockType::Text, BoundingBox::default())
    }
}

/// Iterator over a block and all its children.
pub struct BlockIterator<'a> {
    stack: Vec<&'a Block>,
}

impl<'a> BlockIterator<'a> {
    fn new(block: &'a Block) -> Self {
        Self { stack: vec![block] }
    }
}

impl<'a> Iterator for BlockIterator<'a> {
    type Item = &'a Block;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(block) = self.stack.pop() {
            // Add children in reverse order so they're processed in order
            for child in block.children.iter().rev() {
                self.stack.push(child);
            }
            Some(block)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_id_generation() {
        let id1 = BlockId::generate();
        let id2 = BlockId::generate();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_block_id_with_indices() {
        let id = BlockId::with_indices(3, 7);
        assert_eq!(id.as_str(), "p3b7");
    }

    #[test]
    fn test_block_creation() {
        let bbox = BoundingBox::new(0.0, 0.0, 100.0, 50.0);
        let block = Block::text("Hello world", bbox).on_page(1).at_position(5);

        assert_eq!(block.text, "Hello world");
        assert_eq!(block.page, 1);
        assert_eq!(block.position, 5);
        assert_eq!(block.block_type, BlockType::Text);
    }

    #[test]
    fn test_block_header() {
        let bbox = BoundingBox::new(0.0, 0.0, 200.0, 30.0);
        let block = Block::header("Chapter 1", 1, bbox);

        assert_eq!(block.block_type, BlockType::SectionHeader);
        assert_eq!(block.level, Some(1));
    }

    #[test]
    fn test_block_with_children() {
        let parent_bbox = BoundingBox::new(0.0, 0.0, 300.0, 200.0);
        let child_bbox = BoundingBox::new(10.0, 10.0, 290.0, 50.0);

        let child = Block::text("Child text", child_bbox);
        let parent = Block::new(BlockType::Table, parent_bbox).with_child(child);

        assert_eq!(parent.children.len(), 1);
        assert_eq!(parent.total_blocks(), 2);
    }

    #[test]
    fn test_block_iterator() {
        let bbox = BoundingBox::new(0.0, 0.0, 100.0, 100.0);
        let child1 = Block::text("Child 1", bbox);
        let child2 = Block::text("Child 2", bbox);
        let parent = Block::new(BlockType::Page, bbox)
            .with_child(child1)
            .with_child(child2);

        let texts: Vec<_> = parent.iter_all().map(|b| b.text.as_str()).collect();
        assert_eq!(texts, vec!["", "Child 1", "Child 2"]);
    }

    #[test]
    fn test_block_merge() {
        let bbox1 = BoundingBox::new(0.0, 0.0, 100.0, 50.0);
        let bbox2 = BoundingBox::new(0.0, 50.0, 100.0, 100.0);

        let mut block1 = Block::text("First", bbox1);
        let block2 = Block::text("Second", bbox2);

        block1.merge(&block2);

        assert_eq!(block1.text, "First Second");
        assert_eq!(block1.bbox.height(), 100.0);
    }

    #[test]
    fn test_font_style_heading_detection() {
        let mut style = FontStyle::default();
        style.weight = Some(700);
        assert!(style.looks_like_heading(12.0));

        style.weight = Some(400);
        style.size = Some(18.0);
        assert!(style.looks_like_heading(12.0));
    }

    #[test]
    fn test_font_style_code_detection() {
        let mut style = FontStyle::default();
        style.family = Some("Courier New".to_string());
        assert!(style.looks_like_code());

        style.family = Some("Consolas".to_string());
        assert!(style.looks_like_code());

        style.family = Some("Arial".to_string());
        assert!(!style.looks_like_code());
    }

    #[test]
    fn test_block_serialization() {
        let bbox = BoundingBox::new(10.0, 20.0, 110.0, 70.0);
        let block = Block::text("Test text", bbox)
            .on_page(2)
            .with_metadata("custom", serde_json::json!("value"));

        let json = serde_json::to_string(&block).unwrap();
        let parsed: Block = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.text, "Test text");
        assert_eq!(parsed.page, 2);
        assert_eq!(parsed.metadata.get("custom").unwrap(), "value");
    }
}
