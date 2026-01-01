//! Document processors for transforming documents.
//!
//! Processors implement a chain-of-responsibility pattern for document
//! transformation. Each processor can modify the document structure.

use crate::layout::LayoutAnalyzer;
use crate::schema::{Block, BlockType, Document};
use crate::Result;
use regex::Regex;

/// Trait for document processors.
pub trait Processor: Send + Sync {
    /// Process a document, returning the modified document.
    fn process(&self, document: Document) -> Result<Document>;

    /// Get the processor name for debugging.
    fn name(&self) -> &str;
}

/// Chain of processors.
pub struct ProcessorChain {
    processors: Vec<Box<dyn Processor>>,
}

impl ProcessorChain {
    /// Create a new empty processor chain.
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
        }
    }

    /// Add a processor to the chain.
    pub fn add<P: Processor + 'static>(mut self, processor: P) -> Self {
        self.processors.push(Box::new(processor));
        self
    }

    /// Process a document through the chain.
    pub fn process(&self, mut document: Document) -> Result<Document> {
        for processor in &self.processors {
            tracing::debug!("Running processor: {}", processor.name());
            document = processor.process(document)?;
        }
        Ok(document)
    }

    /// Get the number of processors.
    pub fn len(&self) -> usize {
        self.processors.len()
    }

    /// Check if the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.processors.is_empty()
    }
}

impl Default for ProcessorChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Layout processor - applies layout analysis to pages.
pub struct LayoutProcessor {
    analyzer: LayoutAnalyzer,
}

impl LayoutProcessor {
    /// Create a new layout processor.
    pub fn new() -> Self {
        Self {
            analyzer: LayoutAnalyzer::new(),
        }
    }
}

impl Default for LayoutProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for LayoutProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        for page in &mut document.pages {
            let layout = self.analyzer.analyze(&page.blocks, page.width, page.height);

            page.columns = layout.columns;

            // Sort blocks by reading order
            self.analyzer
                .sort_by_reading_order(&mut page.blocks, &page.columns);
        }

        Ok(document)
    }

    fn name(&self) -> &str {
        "LayoutProcessor"
    }
}
/// Table detection processor - identifies tables from block layout.
pub struct TableDetectionProcessor;

impl TableDetectionProcessor {
    /// Create a new table detection processor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for TableDetectionProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for TableDetectionProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        for page in &mut document.pages {
            if page.blocks.is_empty() {
                continue;
            }

            // Group blocks by Y coordinate (rows)
            let mut rows: Vec<Vec<usize>> = Vec::new();
            for (idx, block) in page.blocks.iter().enumerate() {
                let mut found = false;
                for row in rows.iter_mut() {
                    let first_idx = row[0];
                    // If Y coordinates are close, they are on the same row
                    if (page.blocks[first_idx].bbox.y1 - block.bbox.y1).abs() < 8.0 {
                        row.push(idx);
                        found = true;
                        break;
                    }
                }
                if !found {
                    rows.push(vec![idx]);
                }
            }

            // Sort each row by X coordinate
            for row in rows.iter_mut() {
                row.sort_by(|&a, &b| {
                    page.blocks[a]
                        .bbox
                        .x1
                        .partial_cmp(&page.blocks[b].bbox.x1)
                        .unwrap()
                });
            }

            // Sort rows by Y coordinate (top to bottom)
            rows.sort_by(|a, b| {
                page.blocks[a[0]]
                    .bbox
                    .y1
                    .partial_cmp(&page.blocks[b[0]].bbox.y1)
                    .unwrap()
            });

            // Identify table regions
            let mut new_blocks = Vec::new();
            let mut i = 0;
            while i < rows.len() {
                // A potential table row has multiple blocks
                if rows[i].len() > 1 {
                    // Check if this row has very large gaps (likely columns, not a table)
                    let mut has_large_gaps = false;
                    for k in 0..rows[i].len() - 1 {
                        let b1 = &page.blocks[rows[i][k]];
                        let b2 = &page.blocks[rows[i][k + 1]];
                        let gap = b2.bbox.x1 - b1.bbox.x2;
                        if gap > 100.0 {
                            has_large_gaps = true;
                            break;
                        }
                    }

                    if has_large_gaps {
                        for &block_idx in &rows[i] {
                            new_blocks.push(page.blocks[block_idx].clone());
                        }
                        i += 1;
                        continue;
                    }

                    let mut table_rows = vec![i];
                    let mut j = i + 1;
                    while j < rows.len() && rows[j].len() > 1 {
                        // Check if column count is similar (allow +/- 1 for merged cells)
                        let diff = (rows[j].len() as i32 - rows[i].len() as i32).abs();
                        if diff <= 1 {
                            table_rows.push(j);
                            j += 1;
                        } else {
                            break;
                        }
                    }

                    // If we have at least 2 rows with multiple columns, it's a table
                    if table_rows.len() >= 2 {
                        let mut table_bbox = page.blocks[rows[table_rows[0]][0]].bbox.clone();
                        for &row_idx in &table_rows {
                            for &block_idx in &rows[row_idx] {
                                table_bbox = table_bbox.union(&page.blocks[block_idx].bbox);
                            }
                        }

                        let mut table_block = Block::new(BlockType::Table, table_bbox);
                        table_block.page = page.number as usize - 1;

                        for &row_idx in &table_rows {
                            for &block_idx in &rows[row_idx] {
                                table_block.children.push(page.blocks[block_idx].clone());
                            }
                        }
                        new_blocks.push(table_block);
                        i = j;
                    } else {
                        // Not a table, just push the blocks
                        for &block_idx in &rows[i] {
                            new_blocks.push(page.blocks[block_idx].clone());
                        }
                        i += 1;
                    }
                } else {
                    // Single block row
                    for &block_idx in &rows[i] {
                        new_blocks.push(page.blocks[block_idx].clone());
                    }
                    i += 1;
                }
            }
            page.blocks = new_blocks;
        }
        Ok(document)
    }

    fn name(&self) -> &str {
        "TableDetectionProcessor"
    }
}
/// Block merge processor - merges adjacent blocks into paragraphs.
pub struct BlockMergeProcessor {
    /// Maximum vertical gap for merging
    max_vertical_gap: f32,
    /// Maximum horizontal alignment difference
    max_margin_diff: f32,
}

impl BlockMergeProcessor {
    /// Create a new block merge processor.
    pub fn new() -> Self {
        Self {
            max_vertical_gap: 15.0,
            max_margin_diff: 20.0,
        }
    }

    /// Create with custom parameters.
    pub fn with_params(max_vertical_gap: f32, max_margin_diff: f32) -> Self {
        Self {
            max_vertical_gap,
            max_margin_diff,
        }
    }

    /// Check if two blocks should be merged.
    fn should_merge(&self, a: &Block, b: &Block) -> bool {
        // Only merge text blocks
        if a.block_type != BlockType::Text || b.block_type != BlockType::Text {
            return false;
        }

        // Check vertical proximity
        let vertical_gap = b.bbox.y1 - a.bbox.y2;
        if vertical_gap < 0.0 || vertical_gap > self.max_vertical_gap {
            return false;
        }

        // Check horizontal alignment
        let margin_diff = (a.bbox.x1 - b.bbox.x1).abs();
        margin_diff <= self.max_margin_diff
    }

    /// Merge blocks on a page.
    fn merge_page_blocks(&self, blocks: Vec<Block>) -> Vec<Block> {
        if blocks.len() < 2 {
            return blocks;
        }

        let mut merged = Vec::new();
        let mut current: Option<Block> = None;

        for block in blocks {
            if let Some(mut cur) = current.take() {
                if self.should_merge(&cur, &block) {
                    cur.merge(&block);
                    current = Some(cur);
                } else {
                    merged.push(cur);
                    current = Some(block);
                }
            } else {
                current = Some(block);
            }
        }

        if let Some(cur) = current {
            merged.push(cur);
        }

        // Update positions
        for (i, block) in merged.iter_mut().enumerate() {
            block.position = i;
        }

        merged
    }
}

impl Default for BlockMergeProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for BlockMergeProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        for page in &mut document.pages {
            let blocks = std::mem::take(&mut page.blocks);
            page.blocks = self.merge_page_blocks(blocks);
            page.update_stats();
        }

        document.update_stats();
        Ok(document)
    }

    fn name(&self) -> &str {
        "BlockMergeProcessor"
    }
}

/// Post-processor for text cleanup.
pub struct PostProcessor {
    /// Normalize whitespace
    normalize_whitespace: bool,
    /// Fix common OCR errors
    fix_ocr_errors: bool,
    /// Consolidate headers
    consolidate_headers: bool,
}

impl PostProcessor {
    /// Create a new post-processor.
    pub fn new() -> Self {
        Self {
            normalize_whitespace: true,
            fix_ocr_errors: true,
            consolidate_headers: true,
        }
    }

    /// Enable/disable whitespace normalization.
    pub fn with_normalize_whitespace(mut self, enabled: bool) -> Self {
        self.normalize_whitespace = enabled;
        self
    }

    /// Enable/disable OCR error fixing.
    pub fn with_fix_ocr_errors(mut self, enabled: bool) -> Self {
        self.fix_ocr_errors = enabled;
        self
    }

    /// Enable/disable header consolidation.
    pub fn with_consolidate_headers(mut self, enabled: bool) -> Self {
        self.consolidate_headers = enabled;
        self
    }

    /// Normalize whitespace in text.
    fn normalize_text(&self, text: &str) -> String {
        if !self.normalize_whitespace {
            return text.to_string();
        }

        // Collapse multiple horizontal spaces but preserve newlines
        let mut result = String::new();
        let mut prev_space = false;

        for c in text.chars() {
            if c == ' ' || c == '\t' {
                if !prev_space {
                    result.push(' ');
                    prev_space = true;
                }
            } else {
                result.push(c);
                prev_space = false;
            }
        }

        result.trim().to_string()
    }

    /// Fix common OCR errors.
    fn fix_ocr_text(&self, text: &str) -> String {
        if !self.fix_ocr_errors {
            return text.to_string();
        }

        let mut result = text.to_string();

        // Common OCR replacements
        let replacements = [
            ("ﬁ", "fi"),
            ("ﬂ", "fl"),
            ("ﬀ", "ff"),
            ("ﬃ", "ffi"),
            ("ﬄ", "ffl"),
            ("ǎ", "a"),
            ("ǐ", "i"),
            ("ǒ", "o"),
            ("ǔ", "u"),
            ("\u{2018}", "'"),   // Left single quote
            ("\u{2019}", "'"),   // Right single quote
            ("\u{201C}", "\""),  // Left double quote
            ("\u{201D}", "\""),  // Right double quote
            ("\u{2013}", "-"),   // En dash
            ("\u{2014}", "-"),   // Em dash
            ("\u{2026}", "..."), // Ellipsis
            ("Þle", "file"),     // Common misread 'fi' ligature
            ("Þ", "fi"),
        ];

        for (from, to) in &replacements {
            result = result.replace(from, to);
        }

        result
    }

    /// Fix concatenated words (e.g., "methodsThe" -> "methods The")
    fn fix_concatenated_words(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Fix lowercase immediately followed by uppercase
        if let Ok(re) = Regex::new(r"([a-z])([A-Z][a-z])") {
            result = re.replace_all(&result, "$1 $2").to_string();
        }

        // Fix "etal." -> "et al."
        result = result.replace("etal.", "et al.");
        result = result.replace("etal,", "et al.,");

        // Fix common academic paper patterns
        let literal_fixes = [
            ("ofthe", "of the"),
            ("tothe", "to the"),
            ("inthe", "in the"),
            ("onthe", "on the"),
            ("forthe", "for the"),
            ("bythe", "by the"),
            ("atthe", "at the"),
            ("asthe", "as the"),
            ("isthe", "is the"),
            ("canbe", "can be"),
            ("willbe", "will be"),
            ("hasbeen", "has been"),
            ("isused", "is used"),
            ("areused", "are used"),
            ("basedon", "based on"),
            ("focuson", "focus on"),
            ("resultin", "result in"),
            ("leadto", "lead to"),
            ("dueto", "due to"),
        ];

        for (from, to) in &literal_fixes {
            result = result.replace(from, to);
        }

        result
    }

    /// Cleanup citation formatting
    fn cleanup_citations(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Fix patterns like "(Name,2024)" -> "(Name, 2024)"
        if let Ok(re) = Regex::new(r"\(([^)]+),(\d{4})\)") {
            result = re.replace_all(&result, "($1, $2)").to_string();
        }

        // Fix patterns like ",2024)" -> ", 2024)"
        if let Ok(re) = Regex::new(r",(\d{4})\)") {
            result = re.replace_all(&result, ", $1)").to_string();
        }

        result
    }

    /// Process a block.
    fn process_block(&self, block: &mut Block) {
        if block.block_type.has_text() {
            block.text = self.normalize_text(&block.text);
            block.text = self.fix_ocr_text(&block.text);
            block.text = self.fix_concatenated_words(&block.text);
            block.text = self.cleanup_citations(&block.text);
        }

        // Process children
        for child in &mut block.children {
            self.process_block(child);
        }
    }
}

impl Default for PostProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for PostProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        for page in &mut document.pages {
            for block in &mut page.blocks {
                self.process_block(block);
            }
        }

        Ok(document)
    }

    fn name(&self) -> &str {
        "PostProcessor"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{BoundingBox, Page};

    fn create_test_document() -> Document {
        let mut doc = Document::new();
        let mut page = Page::new(1, 612.0, 792.0);

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

    #[test]
    fn test_processor_chain() {
        let chain = ProcessorChain::new()
            .add(LayoutProcessor::new())
            .add(BlockMergeProcessor::new())
            .add(PostProcessor::new());

        assert_eq!(chain.len(), 3);

        let doc = create_test_document();
        let result = chain.process(doc);
        assert!(result.is_ok());
    }

    #[test]
    fn test_block_merge_processor() {
        let processor = BlockMergeProcessor::new();
        let doc = create_test_document();
        let result = processor.process(doc).unwrap();

        // With 20px gap between blocks, they shouldn't merge
        assert_eq!(result.pages[0].blocks.len(), 2);
    }

    #[test]
    fn test_post_processor_normalize() {
        let processor = PostProcessor::new();

        let normalized = processor.normalize_text("Hello    world   test");
        assert_eq!(normalized, "Hello world test");

        let fixed = processor.fix_ocr_text("ﬁnd the ﬂow");
        assert_eq!(fixed, "find the flow");
    }

    #[test]
    fn test_layout_processor() {
        let processor = LayoutProcessor::new();
        let doc = create_test_document();
        let result = processor.process(doc).unwrap();

        assert!(!result.pages.is_empty());
    }
}
