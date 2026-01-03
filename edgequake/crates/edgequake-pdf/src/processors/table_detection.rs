//! Table detection and reconstruction processors.
//!
//! **Single Responsibility:** Identifying and structuring tabular data in PDFs.
//!
//! This module contains two complementary processors:
//! - `TableDetectionProcessor`: Detects tables from spatial arrangement of blocks
//! - `TextTableReconstructionProcessor`: Reconstructs tables from text patterns
//!
//! **First Principles:**
//! - Tables have columnar structure (multiple blocks per row)
//! - Tables have consistent row heights and column alignment
//! - Captions like "Table 1." indicate nearby table content

use crate::schema::{Block, BlockType, BoundingBox, Document};
use crate::Result;
use regex::Regex;

use super::Processor;

// =============================================================================
// TableDetectionProcessor
// =============================================================================

/// Detects tables from spatial arrangement of text blocks.
///
/// **Algorithm:**
/// 1. Group blocks by Y-coordinate (rows)
/// 2. Sort each row by X-coordinate
/// 3. Identify regions with multiple columns per row
/// 4. Create Table blocks with TableCell children
///
/// **Limitations:**
/// - Requires blocks to be spatially arranged in grid pattern
/// - May fail on complex merged-cell tables
/// - Skips multi-column layouts to avoid false positives
pub struct TableDetectionProcessor;

impl TableDetectionProcessor {
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

            // Skip multi-column layouts to avoid treating columns as table
            // WHY: Column text arranged side-by-side looks like table rows
            if page.columns.len() > 1 {
                continue;
            }

            let rows = self.group_blocks_by_row(page);
            let new_blocks = self.detect_tables(page, rows);
            page.blocks = new_blocks;
        }
        Ok(document)
    }

    fn name(&self) -> &str {
        "TableDetectionProcessor"
    }
}

impl TableDetectionProcessor {
    /// Group blocks into rows based on Y-coordinate overlap.
    fn group_blocks_by_row(&self, page: &crate::schema::Page) -> Vec<Vec<usize>> {
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
                let b1 = &page.blocks[first_idx];

                // Check Y-coordinate overlap
                let overlap_y = b1.bbox.y2.min(block.bbox.y2) - b1.bbox.y1.max(block.bbox.y1);
                let min_h = (b1.bbox.y2 - b1.bbox.y1).min(block.bbox.y2 - block.bbox.y1);

                // WHY 0.5 overlap: blocks on same row should have >50% vertical overlap
                // WHY 10.0 tolerance: handles slight misalignment in extracted text
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

        // Sort each row by X coordinate (left to right)
        for row in rows.iter_mut() {
            row.sort_by(|&a, &b| {
                page.blocks[a]
                    .bbox
                    .x1
                    .partial_cmp(&page.blocks[b].bbox.x1)
                    .unwrap()
            });
        }

        rows
    }

    /// Detect table regions from grouped rows.
    fn detect_tables(&self, page: &crate::schema::Page, rows: Vec<Vec<usize>>) -> Vec<Block> {
        let mut new_blocks = Vec::new();
        let mut i = 0;

        while i < rows.len() {
            // Table candidate: row with multiple blocks
            if rows[i].len() > 1 {
                let table_rows = self.find_table_extent(&rows, i, page);

                if self.is_likely_table(&table_rows, &rows) {
                    let table_block = self.create_table_block(&table_rows, &rows, page);
                    new_blocks.push(table_block);
                    i = table_rows.last().copied().unwrap_or(i) + 1;
                } else {
                    // Not a table, add blocks individually
                    for &block_idx in &rows[i] {
                        new_blocks.push(page.blocks[block_idx].clone());
                    }
                    i += 1;
                }
            } else {
                // Single block row - not part of table
                for &block_idx in &rows[i] {
                    new_blocks.push(page.blocks[block_idx].clone());
                }
                i += 1;
            }
        }

        new_blocks
    }

    /// Find extent of table starting at given row index.
    fn find_table_extent(
        &self,
        rows: &[Vec<usize>],
        start: usize,
        page: &crate::schema::Page,
    ) -> Vec<usize> {
        let mut table_rows = vec![start];
        let mut j = start + 1;

        while j < rows.len() {
            let current_row_blocks = &rows[j];

            if current_row_blocks.len() > 1 {
                // Check gap between blocks
                // WHY: Large gaps indicate separate columns, not table cells
                let mut max_gap: f32 = 0.0;
                for k in 0..current_row_blocks.len() - 1 {
                    let b1 = &page.blocks[current_row_blocks[k]];
                    let b2 = &page.blocks[current_row_blocks[k + 1]];
                    max_gap = max_gap.max(b2.bbox.x1 - b1.bbox.x2);
                }

                // WHY 150.0: Typical table cell gap < 150pt, column gap > 150pt
                if max_gap > 150.0 {
                    break;
                }

                table_rows.push(j);
                j += 1;
            } else if current_row_blocks.len() == 1 {
                // Check if single block aligns with table columns
                let block = &page.blocks[current_row_blocks[0]];
                let mut aligns = false;

                for &prev_row_idx in &table_rows {
                    for &prev_block_idx in &rows[prev_row_idx] {
                        let prev_block = &page.blocks[prev_block_idx];
                        let overlap_x = prev_block.bbox.x2.min(block.bbox.x2)
                            - prev_block.bbox.x1.max(block.bbox.x1);
                        let min_w = (prev_block.bbox.x2 - prev_block.bbox.x1)
                            .min(block.bbox.x2 - block.bbox.x1);

                        // WHY 0.8: Strong column alignment required (80% overlap)
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

        table_rows
    }

    /// Check if table candidate is likely a real table.
    fn is_likely_table(&self, table_rows: &[usize], rows: &[Vec<usize>]) -> bool {
        let has_multi_col = table_rows.iter().any(|&r| rows[r].len() > 1);

        // WHY: Require multiple rows with columns to avoid false positives
        // 6+ rows or 4+ rows with 4+ columns
        (table_rows.len() >= 6 && has_multi_col)
            || (table_rows.len() >= 4 && table_rows.iter().any(|&r| rows[r].len() >= 4))
    }

    /// Create Table block from detected rows.
    fn create_table_block(
        &self,
        table_rows: &[usize],
        rows: &[Vec<usize>],
        page: &crate::schema::Page,
    ) -> Block {
        let mut table_bbox = page.blocks[rows[table_rows[0]][0]].bbox;

        for &row_idx in table_rows {
            for &block_idx in &rows[row_idx] {
                table_bbox = table_bbox.union(&page.blocks[block_idx].bbox);
            }
        }

        let mut table_block = Block::new(BlockType::Table, table_bbox);
        table_block.page = page.number - 1;

        // Add blocks as table cells (clone block content, not bbox)
        for &row_idx in table_rows {
            for &block_idx in &rows[row_idx] {
                let mut cell = page.blocks[block_idx].clone();
                cell.block_type = BlockType::TableCell;
                table_block.children.push(cell);
            }
        }

        table_block
    }
}

// =============================================================================
// TextTableReconstructionProcessor
// =============================================================================

/// Reconstructs tables from text patterns when spatial detection fails.
///
/// **Use Case:** PDFs where table content is extracted as single text blocks
/// instead of individual cells.
///
/// **Algorithm:**
/// 1. Find table captions ("Table 1.", "Table 2.", etc.)
/// 2. Scan adjacent blocks for table-like patterns
/// 3. Parse rows from text using heuristics
/// 4. Build structured Table block with TableCell children
///
/// **Heuristics:**
/// - Pipe separators (|)
/// - Multi-space alignment
/// - Numeric suffixes (e.g., "Method 0.95 3")
pub struct TextTableReconstructionProcessor;

impl TextTableReconstructionProcessor {
    pub fn new() -> Self {
        Self
    }

    /// Check if text looks like a table caption.
    /// Matches: "Table 1.", "TABLE 2:", "Table S1", etc.
    pub fn looks_like_table_caption(text: &str) -> bool {
        let t = Self::normalize_caption(text);
        let re = Regex::new(r"(?i)^table\s*(?:\d+|s\d+)\b").unwrap();
        re.is_match(&t)
    }

    /// Check if block is a hard break (section boundary).
    fn is_hard_break(block: &Block) -> bool {
        let t = block.text.trim();
        t == "---" || block.block_type == BlockType::SectionHeader
    }

    fn normalize_caption(text: &str) -> String {
        text.trim()
            .trim_start_matches('#')
            .trim_start_matches('*')
            .trim_start()
            .to_string()
    }

    /// Check if text is a pipe-formatted markdown table.
    fn looks_like_pipe_table(text: &str) -> bool {
        let t = text.trim();
        if !t.starts_with('|') {
            return false;
        }

        let lines: Vec<&str> = t
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();

        if lines.len() < 2 {
            return false;
        }

        // Detect markdown separator: | --- | ---: |
        let has_separator = lines.iter().any(|l| {
            l.starts_with('|')
                && l.chars()
                    .all(|c| c == '|' || c == '-' || c == ':' || c == ' ' || c == '\t')
        });

        let pipe_lines = lines
            .iter()
            .filter(|l| l.starts_with('|') && l.matches('|').count() >= 2)
            .count();

        has_separator && pipe_lines >= 2
    }

    /// Score text for table-likeness.
    /// Higher score = more likely table data.
    fn table_like_score(text: &str) -> i32 {
        let t = text.trim();
        if t.is_empty() {
            return 0;
        }

        let pipes = t.matches('|').count();
        let has_multi_space = t.contains("  ") || t.contains('\t');
        let cleaned = Self::sanitize_line(t);
        let num_tokens = cleaned
            .split_whitespace()
            .filter(|tok| tok.parse::<f64>().is_ok())
            .count();
        let has_numeric_suffix = Self::parse_numeric_suffix(&cleaned).is_some();

        let mut score = 0;
        if pipes >= 2 {
            score += 3;
        }
        if has_multi_space {
            score += 2;
        }
        if has_numeric_suffix {
            score += 3;
        } else if num_tokens >= 2 {
            score += 2;
        }

        score
    }

    fn sanitize_line(line: &str) -> String {
        line.replace('|', " ")
    }

    /// Parse numeric suffix from line.
    /// Returns (prefix, [float, int]) for patterns like "Method 0.95 3"
    fn parse_numeric_suffix(line: &str) -> Option<(String, Vec<String>)> {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            return None;
        }

        // Try: <prefix> <float> <int>
        if tokens.len() >= 3 {
            let last = tokens[tokens.len() - 1];
            let prev = tokens[tokens.len() - 2];

            if last.parse::<i64>().is_ok() && prev.parse::<f64>().is_ok() {
                let prefix = tokens[..tokens.len() - 2].join(" ");
                return Some((prefix, vec![prev.to_string(), last.to_string()]));
            }
        }

        // Try: <prefix> <float>
        if tokens.len() >= 2 {
            let last = tokens[tokens.len() - 1];
            if last.parse::<f64>().is_ok() {
                let prefix = tokens[..tokens.len() - 1].join(" ");
                return Some((prefix, vec![last.to_string()]));
            }
        }

        None
    }

    /// Build table cells from row data.
    fn build_table_cells(table_bbox: BoundingBox, page: usize, rows: &[Vec<String>]) -> Vec<Block> {
        if rows.is_empty() {
            return Vec::new();
        }

        let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
        if col_count == 0 {
            return Vec::new();
        }

        let width = (table_bbox.x2 - table_bbox.x1).max(1.0);
        let col_w = width / col_count as f32;
        let row_h = 14.0;

        let mut children = Vec::new();
        for (r, row) in rows.iter().enumerate() {
            for c in 0..col_count {
                let text = row.get(c).cloned().unwrap_or_default();
                let cell_bbox = BoundingBox::new(
                    table_bbox.x1 + c as f32 * col_w,
                    table_bbox.y1 + r as f32 * row_h,
                    table_bbox.x1 + (c as f32 + 1.0) * col_w,
                    table_bbox.y1 + r as f32 * row_h + row_h,
                );

                let mut cell = Block::new(BlockType::TableCell, cell_bbox);
                cell.page = page;
                cell.text = text;
                children.push(cell);
            }
        }
        children
    }
}

impl Default for TextTableReconstructionProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for TextTableReconstructionProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        // Pre-scan for existing tables to avoid duplicates
        let page_table_bboxes: Vec<Vec<BoundingBox>> = document
            .pages
            .iter()
            .map(|p| {
                p.blocks
                    .iter()
                    .filter(|b| {
                        b.block_type == BlockType::Table || Self::looks_like_pipe_table(&b.text)
                    })
                    .map(|b| b.bbox)
                    .collect()
            })
            .collect();

        for (page_idx, page) in document.pages.iter_mut().enumerate() {
            if page.blocks.is_empty() {
                continue;
            }

            let new_blocks = self.process_page(page, page_idx, &page_table_bboxes);
            page.blocks = new_blocks;
        }

        Ok(document)
    }

    fn name(&self) -> &str {
        "TextTableReconstructionProcessor"
    }
}

impl TextTableReconstructionProcessor {
    /// Process a single page for table reconstruction.
    fn process_page(
        &self,
        page: &crate::schema::Page,
        page_idx: usize,
        page_table_bboxes: &[Vec<BoundingBox>],
    ) -> Vec<Block> {
        let mut new_blocks: Vec<Block> = Vec::with_capacity(page.blocks.len());
        let mut i = 0;

        while i < page.blocks.len() {
            let block = &page.blocks[i];

            // Not a caption - just add the block
            if !Self::looks_like_table_caption(&block.text) {
                new_blocks.push(block.clone());
                i += 1;
                continue;
            }

            // Check if structured table already exists nearby
            let has_existing_table = self.has_existing_table(page, i, page_idx, page_table_bboxes);

            if has_existing_table {
                new_blocks.push(block.clone());
                i += 1;
                continue;
            }

            // Scan for table content
            let (table_block, consumed) = self.scan_for_table(page, i);

            if let Some(table) = table_block {
                new_blocks.push(block.clone()); // Keep caption
                new_blocks.push(table);
                i = consumed;
            } else {
                new_blocks.push(block.clone());
                i += 1;
            }
        }

        new_blocks
    }

    /// Check if a structured table already exists near the caption.
    fn has_existing_table(
        &self,
        page: &crate::schema::Page,
        caption_idx: usize,
        page_idx: usize,
        page_table_bboxes: &[Vec<BoundingBox>],
    ) -> bool {
        let caption_bbox = page.blocks[caption_idx].bbox;

        let consider_table_bbox = |table_bbox: BoundingBox| -> bool {
            let overlap_x =
                (caption_bbox.x2.min(table_bbox.x2) - caption_bbox.x1.max(table_bbox.x1)).max(0.0);
            let min_w = caption_bbox.width().min(table_bbox.width()).max(1.0);
            overlap_x / min_w >= 0.30
        };

        // Check before caption
        if caption_idx > 0
            && page.blocks[..caption_idx].iter().any(|b| {
                (b.block_type == BlockType::Table || Self::looks_like_pipe_table(&b.text))
                    && consider_table_bbox(b.bbox)
            }) {
                return true;
            }

        // Check after caption
        if caption_idx + 1 < page.blocks.len()
            && page.blocks[(caption_idx + 1)..]
                .iter()
                .any(|b| b.block_type == BlockType::Table && consider_table_bbox(b.bbox))
            {
                return true;
            }

        // Check previous page
        if page_idx > 0 {
            if let Some(prev_tables) = page_table_bboxes.get(page_idx - 1) {
                if prev_tables.iter().any(|bb| consider_table_bbox(*bb)) {
                    return true;
                }
            }
        }

        false
    }

    /// Scan for table content after caption.
    /// Returns (optional table block, next index to process).
    fn scan_for_table(
        &self,
        page: &crate::schema::Page,
        caption_idx: usize,
    ) -> (Option<Block>, usize) {
        const MAX_SCAN: usize = 22;
        const MAX_ZERO_LINES: usize = 2;
        const MAX_LEADING_ZEROS: usize = 3;

        let caption_block = &page.blocks[caption_idx];
        let mut lines: Vec<(usize, String, i32)> = Vec::new();
        let mut skipped_zeros: Vec<(usize, String, i32)> = Vec::new();
        let mut started = false;
        let mut consecutive_zeros = 0;

        for j in (caption_idx + 1)..page.blocks.len().min(caption_idx + 1 + MAX_SCAN) {
            let b = &page.blocks[j];
            let t = b.text.trim();

            if t.is_empty() || Self::is_hard_break(b) || Self::looks_like_table_caption(t) {
                break;
            }

            let score = Self::table_like_score(t);

            if !started {
                if score == 0 {
                    if skipped_zeros.len() < MAX_LEADING_ZEROS {
                        skipped_zeros.push((j, t.to_string(), score));
                    }
                    continue;
                }

                // Found first positive-score line
                started = true;
                for skipped in skipped_zeros.drain(..) {
                    lines.push(skipped);
                }
                consecutive_zeros = 0;
            } else if score == 0 {
                consecutive_zeros += 1;
                if consecutive_zeros > MAX_ZERO_LINES {
                    break;
                }
                lines.push((j, t.to_string(), score));
                continue;
            } else {
                consecutive_zeros = 0;
            }

            lines.push((j, t.to_string(), score));
        }

        if lines.len() < 2 {
            return (None, caption_idx + 1);
        }

        // Build table from lines
        let rows = self.parse_rows(&lines);

        if rows.len() < 2 {
            return (None, caption_idx + 1);
        }

        // Create table block
        let mut table_bbox = caption_block.bbox;
        for (idx, _, _) in &lines {
            table_bbox = table_bbox.union(&page.blocks[*idx].bbox);
        }

        let mut table_block = Block::new(BlockType::Table, table_bbox);
        table_block.page = page.number - 1;
        table_block.children = Self::build_table_cells(table_bbox, table_block.page, &rows);
        table_block
            .metadata
            .insert("reconstructed".to_string(), serde_json::json!(true));

        let consumed = lines
            .last()
            .map(|(idx, _, _)| *idx + 1)
            .unwrap_or(caption_idx + 1);
        (Some(table_block), consumed)
    }

    /// Parse table rows from scanned lines.
    fn parse_rows(&self, lines: &[(usize, String, i32)]) -> Vec<Vec<String>> {
        if lines.is_empty() {
            return Vec::new();
        }

        let first = Self::sanitize_line(&lines[0].1);
        let header_cols: Vec<String> = first.split_whitespace().map(|s| s.to_string()).collect();

        if header_cols.len() < 2 {
            return Vec::new();
        }

        let mut rows: Vec<Vec<String>> = Vec::new();
        rows.push(header_cols.clone());

        // Parse data rows using numeric suffix heuristic
        for (_, line, _) in lines.iter().skip(1) {
            let cleaned = Self::sanitize_line(line);
            if let Some((prefix, nums)) = Self::parse_numeric_suffix(&cleaned) {
                let mut r = Vec::new();
                r.push(prefix);
                r.extend(nums);
                rows.push(r);
            }
        }

        // Normalize column count
        let col_count = header_cols.len();
        for r in rows.iter_mut() {
            if r.len() < col_count {
                r.resize(col_count, String::new());
            }
        }

        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_caption_detection() {
        assert!(TextTableReconstructionProcessor::looks_like_table_caption(
            "Table 1."
        ));
        assert!(TextTableReconstructionProcessor::looks_like_table_caption(
            "TABLE 2"
        ));
        assert!(TextTableReconstructionProcessor::looks_like_table_caption(
            "Table S1"
        ));
        assert!(TextTableReconstructionProcessor::looks_like_table_caption(
            "#### Table 1."
        ));
        assert!(!TextTableReconstructionProcessor::looks_like_table_caption(
            "Figure 1."
        ));
    }

    #[test]
    fn test_table_like_score() {
        // High score for pipe tables
        assert!(TextTableReconstructionProcessor::table_like_score("| A | B |") >= 3);

        // Score for numeric data
        assert!(TextTableReconstructionProcessor::table_like_score("Method  0.95  3") >= 2);

        // Low score for plain text
        assert_eq!(
            TextTableReconstructionProcessor::table_like_score("Hello world"),
            0
        );
    }

    #[test]
    fn test_numeric_suffix_parsing() {
        let (prefix, nums) =
            TextTableReconstructionProcessor::parse_numeric_suffix("Method A 0.95 3")
                .expect("should parse");
        assert_eq!(prefix, "Method A");
        assert_eq!(nums, vec!["0.95", "3"]);
    }
}
