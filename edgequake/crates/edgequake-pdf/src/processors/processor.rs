//! Document processors for transforming documents.
//!
//! Processors implement a chain-of-responsibility pattern for document
//! transformation. Each processor can modify the document structure.

use crate::layout::LayoutAnalyzer;
use crate::schema::{Block, BlockType, Document};
use crate::Result;
use regex::Regex;

/// Processor to filter out margin content like line numbers.
///
/// Academic papers (especially arXiv) often have line numbers in the left margin.
/// This processor removes blocks that are positioned in page margins.
pub struct MarginFilterProcessor {
    /// Left margin threshold (blocks with x < this are filtered)
    left_margin: f32,
    /// Right margin threshold (blocks with x > page_width - this are filtered)
    right_margin: f32,
    /// Top margin threshold
    top_margin: f32,
    /// Bottom margin threshold  
    bottom_margin: f32,
}

impl MarginFilterProcessor {
    /// Create with default margins for academic papers.
    pub fn new() -> Self {
        Self {
            left_margin: 50.0,   // Filter content in first 50pt (line numbers)
            right_margin: 30.0,  // Filter right margin content
            top_margin: 40.0,    // Filter header area
            bottom_margin: 40.0, // Filter footer area
        }
    }

    /// Create with custom margins.
    pub fn with_margins(left: f32, right: f32, top: f32, bottom: f32) -> Self {
        Self {
            left_margin: left,
            right_margin: right,
            top_margin: top,
            bottom_margin: bottom,
        }
    }

    /// Check if a block is in the margin area.
    fn is_margin_content(&self, block: &Block, page_width: f32, page_height: f32) -> bool {
        let bbox = &block.bbox;

        // Check if block is entirely in left margin
        if bbox.x2 < self.left_margin {
            // Also check if it's short text (likely line number)
            if block.text.trim().len() <= 3 {
                tracing::debug!("Filtering left margin content: '{}'", block.text.trim());
                return true;
            }
        }

        // Check if block is entirely in right margin
        if bbox.x1 > page_width - self.right_margin {
            if block.text.trim().len() <= 3 {
                tracing::debug!("Filtering right margin content: '{}'", block.text.trim());
                return true;
            }
        }

        // Check if block is single digit/letter at edge of content (likely line number)
        let text = block.text.trim();
        if text.len() <= 2
            && text
                .chars()
                .all(|c| c.is_ascii_digit() || c.is_ascii_uppercase())
        {
            // If it's positioned far from main content area, filter it
            if bbox.x1 < 60.0 || bbox.x1 > page_width - 60.0 {
                tracing::debug!("Filtering likely line number: '{}'", text);
                return true;
            }
        }

        // Check top margin (headers)
        if bbox.y2 < self.top_margin && block.text.len() < 100 {
            // Only filter short texts in header (not full header lines)
            // Skip - we want to keep page headers
        }

        // Check bottom margin (footers)
        if bbox.y1 > page_height - self.bottom_margin && block.text.len() < 100 {
            // Check for page number pattern
            let trimmed = block.text.trim();
            if trimmed.parse::<i32>().is_ok() {
                tracing::debug!("Filtering footer page number: '{}'", trimmed);
                return true;
            }
        }

        false
    }
}

impl Default for MarginFilterProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for MarginFilterProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        for page in &mut document.pages {
            let page_width = page.width;
            let page_height = page.height;

            page.blocks
                .retain(|block| !self.is_margin_content(block, page_width, page_height));
        }

        Ok(document)
    }

    fn name(&self) -> &str {
        "MarginFilterProcessor"
    }
}

/// Processor to filter garbled/corrupted text from figure annotations.
///
/// Detects text that appears corrupted, such as:
/// - High ratio of single-character words
/// - Text that doesn't form recognizable patterns
/// - Very short isolated fragments
pub struct GarbledTextFilterProcessor {
    /// Maximum ratio of short words (≤2 chars) allowed
    max_short_word_ratio: f32,
    /// Minimum number of words to apply the ratio check
    min_word_count: usize,
}

impl GarbledTextFilterProcessor {
    pub fn new() -> Self {
        Self {
            max_short_word_ratio: 0.35,
            min_word_count: 4,
        }
    }

    /// Check if text appears garbled/corrupted.
    fn is_garbled(&self, text: &str) -> bool {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return false;
        }

        // Filter very short isolated fragments that look like figure labels
        // e.g., ",w", "v", "x u", "l i d"
        // But not valid short content like "1.", "a)", etc.
        if trimmed.len() <= 6 && trimmed.split_whitespace().count() >= 2 {
            // Multiple words in ≤6 chars = likely garbled
            // Except if it looks like a numbered item or reference
            let has_digit = trimmed.chars().any(|c| c.is_ascii_digit());
            let looks_like_item = has_digit && (trimmed.contains('.') || trimmed.contains(')'));
            if !looks_like_item {
                tracing::debug!("Filtering short garbled fragment: '{}'", trimmed);
                return true;
            }
        }

        let words: Vec<&str> = trimmed.split_whitespace().collect();
        if words.len() < self.min_word_count {
            return false;
        }

        // Count short words (≤2 chars) excluding common valid short words
        let valid_short_words = ["a", "an", "as", "at", "be", "by", "do", "go", "he", "if", 
                                 "in", "is", "it", "me", "my", "no", "of", "on", "or", "so", 
                                 "to", "up", "us", "we", "i", "1", "2", "3", "4", "5", "6", "7", "8", "9"];
        let short_count = words.iter()
            .filter(|w| w.len() <= 2 && !valid_short_words.contains(&w.to_lowercase().as_str()))
            .count();
        let ratio = short_count as f32 / words.len() as f32;

        if ratio > self.max_short_word_ratio {
            tracing::debug!(
                "Filtering garbled text ({}% unusual short words): '{}'",
                (ratio * 100.0) as i32,
                if trimmed.len() > 50 { &trimmed[..50] } else { trimmed }
            );
            return true;
        }

        // Check for patterns like missing letters: "a iliar tools hich o erlook"
        // This has MANY isolated letters that aren't common words
        let isolated_letters = words.iter()
            .filter(|w| w.len() == 1 && w.chars().all(|c| c.is_alphabetic()))
            .filter(|w| !valid_short_words.contains(&w.to_lowercase().as_str()))
            .count();
        // Need at least 4 isolated non-common letters AND high ratio
        if isolated_letters >= 4 && ratio > 0.30 {
            tracing::debug!("Filtering text with isolated letters: '{}'", if trimmed.len() > 50 { &trimmed[..50] } else { trimmed });
            return true;
        }

        // Check for OCR-garbled text pattern: words that look like word fragments
        // Pattern: single letter + space + lowercase fragment (e.g., "a iliar" = "familiar" with missing "f")
        // "hich" = "which" missing "w", "erlook" = "overlook" missing "ov", "ec" = "exec" missing "ex"
        // These are unusual non-words that suggest OCR corruption
        let non_word_fragments = words.iter().filter(|w| {
            let w_lower = w.to_lowercase();
            let len = w_lower.len();
            // Check for likely fragments: 4-7 chars that start with unusual patterns
            if len >= 4 && len <= 8 && w_lower.chars().all(|c| c.is_alphabetic()) {
                // Check for patterns that suggest missing first letter(s)
                // Common garbled patterns from the PDF
                let garbled_patterns = ["iliar", "hich", "erlook", "ec", "tion", "xec"];
                garbled_patterns.iter().any(|p| w_lower.starts_with(p) || w_lower == *p)
            } else {
                false
            }
        }).count();

        if non_word_fragments >= 2 {
            tracing::debug!("Filtering text with OCR fragments: '{}'", if trimmed.len() > 50 { &trimmed[..50] } else { trimmed });
            return true;
        }

        false
    }
}

impl Default for GarbledTextFilterProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for GarbledTextFilterProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        for page in &mut document.pages {
            page.blocks.retain(|block| !self.is_garbled(&block.text));
        }

        Ok(document)
    }

    fn name(&self) -> &str {
        "GarbledTextFilterProcessor"
    }
}

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

            // Check if the detected columns look like a table structure
            // If so, don't apply column-based reading order
            let bboxes: Vec<crate::schema::BoundingBox> =
                page.blocks.iter().map(|b| b.bbox).collect();
            let is_table = self
                .analyzer
                .column_detector()
                .is_likely_table(&bboxes, &layout.columns);

            if is_table {
                // For table-like layouts, use single column to preserve natural order
                page.columns = vec![];
                tracing::debug!("Detected table-like layout, skipping column-based reading order");
            } else {
                page.columns = layout.columns;
                // Sort blocks by reading order only for actual multi-column layouts
                self.analyzer
                    .sort_by_reading_order(&mut page.blocks, &page.columns);
            }
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

            // If we detected multiple columns, skip table detection to preserve reading order
            // This is a simplification, but prevents column text from being treated as a table
            if page.columns.len() > 1 {
                continue;
            }

            // Group blocks by Y coordinate (rows) with a more generous tolerance
            // to handle multi-line cells that might be slightly misaligned
            let mut rows: Vec<Vec<usize>> = Vec::new();
            let mut sorted_indices: Vec<usize> = (0..page.blocks.len()).collect();
            sorted_indices.sort_by(|&a, &b| {
                page.blocks[a]
                    .bbox
                    .y1
                    .partial_cmp(&page.blocks[b].bbox.y1)
                    .unwrap()
            });

            for idx in sorted_indices {
                let block = &page.blocks[idx];
                let mut found = false;
                for row in rows.iter_mut() {
                    let first_idx = row[0];
                    // If Y coordinates overlap significantly, they are on the same row
                    let b1 = &page.blocks[first_idx];
                    let overlap_y = b1.bbox.y2.min(block.bbox.y2) - b1.bbox.y1.max(block.bbox.y1);
                    let min_h = (b1.bbox.y2 - b1.bbox.y1).min(block.bbox.y2 - block.bbox.y1);

                    if overlap_y > min_h * 0.5 || (b1.bbox.y1 - block.bbox.y1).abs() < 10.0 {
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

            tracing::debug!("Detected {} rows", rows.len());
            for (idx, row) in rows.iter().enumerate() {
                tracing::debug!("Row {}: {} blocks", idx, row.len());
            }

            // Identify table regions
            let mut new_blocks = Vec::new();
            let mut i = 0;
            while i < rows.len() {
                // A potential table row has multiple blocks
                if rows[i].len() > 1 {
                    let mut table_rows = vec![i];
                    let mut j = i + 1;

                    // Look ahead for more rows that look like they belong to the same table
                    while j < rows.len() {
                        let current_row_blocks = &rows[j];

                        // If it's a single block, it might be a multi-line cell continuation
                        // or the end of the table.
                        if current_row_blocks.len() > 1 {
                            // Check gap between blocks
                            let mut max_gap: f32 = 0.0;
                            for k in 0..current_row_blocks.len() - 1 {
                                let b1 = &page.blocks[current_row_blocks[k]];
                                let b2 = &page.blocks[current_row_blocks[k + 1]];
                                max_gap = max_gap.max(b2.bbox.x1 - b1.bbox.x2);
                            }

                            // If gap is too large, it's probably columns, not a table
                            // Increased to 150.0 to handle wider tables
                            if max_gap > 150.0 {
                                break;
                            }

                            table_rows.push(j);
                            j += 1;
                        } else if current_row_blocks.len() == 1 {
                            // Check if this single block aligns with one of the columns in the table
                            let block = &page.blocks[current_row_blocks[0]];
                            let mut aligns = false;
                            for &prev_row_idx in &table_rows {
                                for &prev_block_idx in &rows[prev_row_idx] {
                                    let prev_block = &page.blocks[prev_block_idx];
                                    let overlap_x = prev_block.bbox.x2.min(block.bbox.x2)
                                        - prev_block.bbox.x1.max(block.bbox.x1);
                                    let min_w = (prev_block.bbox.x2 - prev_block.bbox.x1)
                                        .min(block.bbox.x2 - block.bbox.x1);
                                    if overlap_x > min_w * 0.8 {
                                        aligns = true;
                                        break;
                                    }
                                }
                                if aligns {
                                    break;
                                }
                            }

                            if aligns {
                                table_rows.push(j);
                                j += 1;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    // If we have at least 2 rows and some multi-column rows, it's a table
                    let has_multi_col = table_rows.iter().any(|&r| rows[r].len() > 1);

                    // A table should have at least 4 rows or multiple rows with 3+ columns to be sure it's not just a random alignment
                    let is_likely_table = (table_rows.len() >= 6 && has_multi_col)
                        || (table_rows.len() >= 4
                            && table_rows.iter().any(|&r| rows[r].len() >= 4));

                    if is_likely_table {
                        let mut table_bbox = page.blocks[rows[table_rows[0]][0]].bbox.clone();
                        for &row_idx in &table_rows {
                            for &block_idx in &rows[row_idx] {
                                table_bbox = table_bbox.union(&page.blocks[block_idx].bbox);
                            }
                        }

                        let mut table_block = Block::new(BlockType::Table, table_bbox);
                        table_block.page = page.number as usize - 1;

                        // Group blocks into cells based on X alignment
                        // This is a simplification; a real SOTA extractor would do better
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
        // Only merge text, header, or list item blocks
        if !matches!(
            a.block_type,
            BlockType::Text | BlockType::SectionHeader | BlockType::ListItem
        ) || !matches!(
            b.block_type,
            BlockType::Text | BlockType::SectionHeader | BlockType::ListItem
        ) {
            return false;
        }

        // If types are different, don't merge
        if a.block_type != b.block_type {
            return false;
        }

        // Don't merge if b looks like a start of a new list item
        let trimmed_b = b.text.trim();
        if trimmed_b.starts_with("- ")
            || trimmed_b.starts_with("* ")
            || trimmed_b.starts_with("• ")
            || (trimmed_b.len() > 2
                && trimmed_b.chars().next().unwrap().is_ascii_digit()
                && trimmed_b.contains(". "))
        {
            return false;
        }

        // Don't merge if style changes significantly
        if let (Some(span_a), Some(span_b)) = (a.spans.last(), b.spans.first()) {
            let size_a = span_a.style.size.unwrap_or(0.0);
            let size_b = span_b.style.size.unwrap_or(0.0);
            if (size_a - size_b).abs() > 1.5 {
                return false;
            }

            let weight_a = span_a.style.weight.unwrap_or(400);
            let weight_b = span_b.style.weight.unwrap_or(400);
            if (weight_a >= 600) != (weight_b >= 600) {
                return false;
            }
        }

        // For list items, don't merge if the second one starts with a bullet/number
        if a.block_type == BlockType::ListItem {
            let trimmed_b = b.text.trim();
            if trimmed_b.starts_with("- ")
                || trimmed_b.starts_with("* ")
                || trimmed_b.starts_with("• ")
                || (trimmed_b.len() > 2
                    && trimmed_b.chars().next().unwrap().is_ascii_digit()
                    && trimmed_b.contains(". "))
            {
                return false;
            }
        }

        // Check vertical proximity
        let vertical_gap = b.bbox.y1 - a.bbox.y2;
        // For headers, be more strict with vertical gap but allow for multi-line headers
        let max_gap = if a.block_type == BlockType::SectionHeader {
            25.0
        } else {
            self.max_vertical_gap
        };

        if vertical_gap < -2.0 || vertical_gap > max_gap {
            return false;
        }

        // Check horizontal alignment
        let margin_diff = (a.bbox.x1 - b.bbox.x1).abs();
        let max_margin = if a.block_type == BlockType::SectionHeader {
            50.0
        } else {
            self.max_margin_diff
        };
        margin_diff <= max_margin
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

    /// Fix soft hyphen patterns and control characters.
    /// PDF extraction can produce control characters (like \x02) to indicate soft hyphens.
    /// Common patterns:
    /// - "modifi\x02 cation" (control char between word parts)
    /// - "modifi \x02 cation" (space + control char + space between parts)
    fn fix_soft_hyphens(&self, text: &str) -> String {
        let mut result = String::new();
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            let c = chars[i];

            // Check for control characters that indicate soft hyphen/line break
            // These are commonly \x02 (STX), \x1F (unit separator), \xAD (soft hyphen)
            if c == '\x02' || c == '\x1F' || c == '\u{00AD}' {
                // Look backward to find the last alphabetic character (skipping spaces from result string)
                let result_trimmed = result.trim_end();
                let prev_is_letter = result_trimmed
                    .chars()
                    .last()
                    .map(|c| c.is_alphabetic())
                    .unwrap_or(false);

                // Look forward past any spaces and control chars
                let mut j = i + 1;
                while j < len && (chars[j] == ' ' || chars[j] == '\x02' || chars[j] == '\x1F') {
                    j += 1;
                }

                // Check if next real char is lowercase (word continuation)
                let next_is_lower = j < len && chars[j].is_lowercase();

                if prev_is_letter && next_is_lower {
                    // This is a soft hyphen - remove trailing spaces from result and skip control/spaces
                    while result.ends_with(' ') {
                        result.pop();
                    }
                    i = j;
                    continue;
                } else {
                    // Not a soft hyphen pattern - replace with space
                    result.push(' ');
                }
            } else {
                result.push(c);
            }
            i += 1;
        }

        result
    }

    /// Normalize whitespace in text.
    fn normalize_text(&self, text: &str) -> String {
        if !self.normalize_whitespace {
            return text.to_string();
        }

        // First fix soft hyphens
        let text = self.fix_soft_hyphens(text);

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
            // Process main text
            block.text = self.normalize_text(&block.text);
            block.text = self.fix_ocr_text(&block.text);
            block.text = self.fix_concatenated_words(&block.text);
            block.text = self.cleanup_citations(&block.text);
            
            // Also process spans since MarkdownRenderer uses spans if present
            for span in &mut block.spans {
                span.text = self.normalize_text(&span.text);
                span.text = self.fix_ocr_text(&span.text);
            }
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

/// Processor to detect headers based on font size and weight.
pub struct HeaderDetectionProcessor {}

impl HeaderDetectionProcessor {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for HeaderDetectionProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for HeaderDetectionProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        // 1. Calculate font size statistics
        let mut size_counts = std::collections::HashMap::new();
        for page in &document.pages {
            for block in &page.blocks {
                if let Some(span) = block.spans.first() {
                    let size = (span.style.size.unwrap_or(10.0) * 10.0).round() as i32; // Round to 0.1
                    *size_counts.entry(size).or_insert(0) += block.text.len();
                }
            }
        }

        // Find body size (most common)
        let body_size_int = size_counts
            .iter()
            .max_by_key(|&(_, count)| count)
            .map(|(s, _)| *s)
            .unwrap_or(100);
        let body_size = body_size_int as f32 / 10.0;

        // 2. Detect headers
        for page in &mut document.pages {
            for block in &mut page.blocks {
                if block.block_type != BlockType::Text {
                    continue;
                }

                if let Some(span) = block.spans.first() {
                    let size = span.style.size.unwrap_or(10.0);
                    let weight = span.style.weight.unwrap_or(400);
                    let is_bold = weight >= 600;

                    // H1: Very large (e.g. > 1.6x body)
                    // H2: Large (> 1.3x)
                    // H3: Slightly larger (> 1.1x)
                    // H4: Bold and same size

                    if size > body_size * 1.6 {
                        block.block_type = BlockType::SectionHeader;
                        block.level = Some(1);
                    } else if size > body_size * 1.3 {
                        block.block_type = BlockType::SectionHeader;
                        block.level = Some(2);
                    } else if size > body_size * 1.1 {
                        block.block_type = BlockType::SectionHeader;
                        block.level = Some(3);
                    } else if is_bold && size >= body_size {
                        // Maybe H4? Or just bold text?
                        // If it's a short line, likely a header.
                        // Also check if it doesn't end with punctuation (except :)
                        let text = block.text.trim();
                        if text.len() < 80 && !text.ends_with('.') {
                            block.block_type = BlockType::SectionHeader;
                            block.level = Some(4);
                        }
                    }
                }
            }
        }

        Ok(document)
    }

    fn name(&self) -> &str {
        "HeaderDetectionProcessor"
    }
}

/// Processor to detect captions for figures and tables.
pub struct CaptionDetectionProcessor {}

impl CaptionDetectionProcessor {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for CaptionDetectionProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for CaptionDetectionProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        let caption_regex = Regex::new(r"^(Figure|Fig\.|Table|Tab\.)\s*\d+[:.]").unwrap();

        for page in &mut document.pages {
            for block in &mut page.blocks {
                if block.block_type != BlockType::Text {
                    continue;
                }

                let text = block.text.trim();
                if caption_regex.is_match(text) {
                    // It looks like a caption
                    block.block_type = BlockType::Caption;
                }
            }
        }
        Ok(document)
    }

    fn name(&self) -> &str {
        "CaptionDetectionProcessor"
    }
}

/// Processor to detect list items and their indentation.
pub struct ListDetectionProcessor {}

impl ListDetectionProcessor {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for ListDetectionProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for ListDetectionProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        let bullet_regex = Regex::new(r"^[-*•]\s+").unwrap();
        let number_regex = Regex::new(r"^\d+[\.)]\s+").unwrap();

        for page in &mut document.pages {
            // Find the minimum x-coordinate (left margin) to calculate indentation
            let min_x = page
                .blocks
                .iter()
                .map(|b| b.bbox.x1)
                .fold(f32::MAX, |a, b| a.min(b));

            for block in &mut page.blocks {
                if block.block_type != BlockType::Text {
                    continue;
                }

                let text = block.text.trim();
                if bullet_regex.is_match(text) || number_regex.is_match(text) {
                    block.block_type = BlockType::ListItem;

                    // Calculate indentation level
                    // Assume 20 points per level
                    let indent = block.bbox.x1 - min_x;
                    let level = (indent / 20.0).round() as i32;

                    // Store indentation in metadata for renderer
                    block
                        .metadata
                        .insert("indent".to_string(), serde_json::json!(indent));
                    block
                        .metadata
                        .insert("level".to_string(), serde_json::json!(level));
                }
            }
        }
        Ok(document)
    }

    fn name(&self) -> &str {
        "ListDetectionProcessor"
    }
}

/// Processor to detect and merge code blocks.
pub struct CodeBlockDetectionProcessor {}

impl CodeBlockDetectionProcessor {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for CodeBlockDetectionProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for CodeBlockDetectionProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        for page in &mut document.pages {
            // 1. Identify code blocks
            for block in &mut page.blocks {
                if block.block_type != BlockType::Text {
                    continue;
                }

                // Check if all spans look like code
                let all_code = !block.spans.is_empty()
                    && block.spans.iter().all(|s| s.style.looks_like_code());

                if all_code {
                    block.block_type = BlockType::Code;
                }
            }

            // 2. Merge consecutive code blocks
            let mut merged = Vec::new();
            let mut current_code: Option<Block> = None;

            for block in std::mem::take(&mut page.blocks) {
                if block.block_type == BlockType::Code {
                    if let Some(mut cur) = current_code.take() {
                        // Merge with current code block
                        // Add newline between lines
                        cur.text.push('\n');
                        cur.text.push_str(&block.text);

                        // Merge spans
                        // Add a newline span if needed, or just append
                        cur.spans.extend(block.spans);

                        // Update bbox
                        cur.bbox = cur.bbox.union(&block.bbox);

                        current_code = Some(cur);
                    } else {
                        current_code = Some(block);
                    }
                } else {
                    if let Some(cur) = current_code.take() {
                        merged.push(cur);
                    }
                    merged.push(block);
                }
            }

            if let Some(cur) = current_code {
                merged.push(cur);
            }

            page.blocks = merged;
        }
        Ok(document)
    }

    fn name(&self) -> &str {
        "CodeBlockDetectionProcessor"
    }
}

/// Processor to fix hyphenated words at line breaks.
///
/// Academic papers often use hyphenation for word wrapping. This processor
/// detects patterns like "modifi-\ncation" and joins them to "modification".
pub struct HyphenContinuationProcessor {}

impl HyphenContinuationProcessor {
    pub fn new() -> Self {
        Self {}
    }

    /// Check if a block text ends with a hyphenated word fragment.
    /// Returns the fragment if found.
    fn ends_with_hyphen(&self, text: &str) -> Option<String> {
        let trimmed = text.trim_end();

        // Check for explicit hyphen at end
        if trimmed.ends_with('-') {
            // Get the word fragment before the hyphen
            let without_hyphen = &trimmed[..trimmed.len() - 1];
            let last_word = without_hyphen.split_whitespace().last()?;
            return Some(last_word.to_string());
        }

        // Check for trailing space followed by word fragment (common PDF extraction issue)
        // Pattern: "modifi " at end of line where next block starts with "cation"
        if let Some(last_word) = trimmed.split_whitespace().last() {
            // If the last word is short (< 8 chars) and looks like a word fragment
            // (no sentence-ending punctuation), it might be hyphenated
            if last_word.len() >= 2
                && last_word.len() < 8
                && last_word.chars().all(|c| c.is_alphabetic())
                && !trimmed.ends_with('.')
                && !trimmed.ends_with('!')
                && !trimmed.ends_with('?')
                && !trimmed.ends_with(',')
                && !trimmed.ends_with(':')
                && !trimmed.ends_with(';')
            {
                return Some(last_word.to_string());
            }
        }

        None
    }

    /// Check if text starts with a continuation of a hyphenated word.
    fn starts_with_continuation(&self, text: &str) -> bool {
        let trimmed = text.trim_start();
        if trimmed.is_empty() {
            return false;
        }

        // Must start with lowercase letter
        let first_char = trimmed.chars().next().unwrap();
        if !first_char.is_lowercase() {
            return false;
        }

        // Get first word
        let first_word = trimmed.split_whitespace().next().unwrap_or("");

        // Should be a word fragment (no spaces, just letters)
        first_word.chars().all(|c| c.is_alphabetic())
    }

    /// Join two text blocks, removing hyphenation.
    fn join_hyphenated(&self, first: &str, second: &str) -> String {
        let first_trimmed = first.trim_end();
        let second_trimmed = second.trim_start();

        // If first ends with hyphen, remove it and join directly
        if first_trimmed.ends_with('-') {
            let base = &first_trimmed[..first_trimmed.len() - 1];
            return format!("{}{}", base, second_trimmed);
        }

        // Otherwise, we're dealing with a "word " + "fragment" pattern
        // Remove trailing spaces from first, then join with second
        let words: Vec<&str> = first_trimmed.split_whitespace().collect();
        if words.is_empty() {
            return second_trimmed.to_string();
        }

        let first_word = second_trimmed.split_whitespace().next().unwrap_or("");
        let rest_of_second: String = second_trimmed[first_word.len()..].trim_start().to_string();

        // Join the last word fragment with the continuation
        let last_word = words.last().unwrap();
        let prefix: String = words[..words.len() - 1].join(" ");

        if prefix.is_empty() {
            format!("{}{} {}", last_word, first_word, rest_of_second)
                .trim()
                .to_string()
        } else {
            format!("{} {}{} {}", prefix, last_word, first_word, rest_of_second)
                .trim()
                .to_string()
        }
    }
}

impl Default for HyphenContinuationProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for HyphenContinuationProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        for page in &mut document.pages {
            let mut i = 0;
            while i < page.blocks.len() {
                let should_join = if i + 1 < page.blocks.len() {
                    let current = &page.blocks[i];
                    let next = &page.blocks[i + 1];

                    // Only consider joining text blocks that are vertically adjacent
                    if current.block_type != BlockType::Text || next.block_type != BlockType::Text {
                        false
                    } else {
                        // Check if they're on consecutive lines (small vertical gap)
                        let vertical_gap = next.bbox.y1 - current.bbox.y2;
                        if vertical_gap > 20.0 || vertical_gap < -5.0 {
                            false
                        } else {
                            // Check for hyphenation
                            self.ends_with_hyphen(&current.text).is_some()
                                && self.starts_with_continuation(&next.text)
                        }
                    }
                } else {
                    false
                };

                if should_join {
                    let current = &page.blocks[i];
                    let next = &page.blocks[i + 1];

                    let joined_text = self.join_hyphenated(&current.text, &next.text);
                    let joined_bbox = current.bbox.union(&next.bbox);

                    page.blocks[i].text = joined_text;
                    page.blocks[i].bbox = joined_bbox;
                    page.blocks.remove(i + 1);
                    // Don't increment i, check if there are more continuations
                } else {
                    i += 1;
                }
            }
        }

        Ok(document)
    }

    fn name(&self) -> &str {
        "HyphenContinuationProcessor"
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
