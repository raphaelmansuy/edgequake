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
//! - Paragraphs are NOT table cells (wide blocks with long text)

use crate::schema::{Block, BlockType, BoundingBox, Document};
use crate::Result;
use regex::Regex;

use super::Processor;

// =============================================================================
// Paragraph Detection (OODA-21)
// =============================================================================

/// Detect if a block is a paragraph (NOT a table cell).
///
/// **First Principles (from Markitdown analysis):**
/// - Tables contain SHORT data cells, not flowing prose
/// - Paragraphs span significant page width (>55%)
/// - Paragraphs have many characters (>60)
///
/// **Thresholds:**
/// - 55% page width: Markitdown threshold, columns are typically 40-45% wide
/// - 60 characters: Table cells rarely exceed this, paragraphs always do
///
/// **OODA-21:** Adding this check prevents prose blocks from being
/// incorrectly classified as table rows.
fn is_paragraph(block: &Block, page_width: f32) -> bool {
    let block_width = block.bbox.x2 - block.bbox.x1;
    let text_len = block.text.chars().count();

    // WHY 55%: Typical column is 40-45% of page width.
    // A block wider than 55% must be spanning content, not a table cell.
    // WHY 60 chars: Table cells are short labels/numbers, paragraphs are sentences.
    block_width > page_width * 0.55 && text_len > 60
}

/// Check if any block in a row is a paragraph.
/// Used to stop table extent when encountering prose content.
fn row_contains_paragraph(row: &[usize], blocks: &[Block], page_width: f32) -> bool {
    row.iter()
        .any(|&idx| is_paragraph(&blocks[idx], page_width))
}

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

            tracing::info!(
                "TableDetectionProcessor: processing page {} with {} blocks",
                page.number,
                page.blocks.len()
            );

            // OODA-34 FIX: SKIP table detection for multi-column pages with backend-set columns
            //
            // WHY: The table detection algorithm sorts blocks by Y-coordinate (group_blocks_by_row),
            // then iterates through Y-sorted rows to create new_blocks. This destroys the
            // column-aware reading order established by text_grouping.rs and extraction_engine.rs.
            //
            // For multi-column pages:
            // - Correct order: [left_col_block_1, left_col_block_2, ..., right_col_block_1, ...]
            // - After Y-sort: [left_y100, right_y100, left_y112, right_y112, ...] (INTERLEAVED!)
            //
            // The OODA-12 and OODA-29 fixes preserved reading order in extraction_engine and
            // LayoutProcessor, but TableDetectionProcessor was still re-sorting and breaking it.
            //
            // FIX: Skip table detection entirely for pages that have columns set by the backend.
            // Tables within columns are rare and not worth the cost of destroying reading order.
            if page.columns.len() > 1 {
                tracing::debug!(
                    "OODA-34: Skipping table detection for {}-column page {} (preserving reading order)",
                    page.columns.len(),
                    page.number
                );
                continue;
            }

            // OODA-16: Enable table detection for multi-column pages with stricter criteria
            // WHY: Tables can appear within multi-column layouts (e.g., AlphaEvolve Table 1)
            // STRICT MODE: Use tighter Y-tolerance and text length checks to avoid
            // false positives from column text that happens to align horizontally
            let strict_mode = page.columns.len() > 1;
            if strict_mode {
                tracing::info!(
                    "  Multi-column page ({} columns) - using strict table detection",
                    page.columns.len()
                );
            }

            let rows = self.group_blocks_by_row(page, strict_mode);
            tracing::info!("  Grouped into {} rows", rows.len());
            let new_blocks = self.detect_tables(page, rows, strict_mode);
            tracing::info!("  Produced {} blocks", new_blocks.len());
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
    ///
    /// # OODA-16: Strict Mode for Multi-Column Pages
    ///
    /// In strict mode, use tighter Y-tolerance (2pt vs 10pt) to distinguish
    /// precise table rows from approximate column text alignment.
    fn group_blocks_by_row(
        &self,
        page: &crate::schema::Page,
        strict_mode: bool,
    ) -> Vec<Vec<usize>> {
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

                // OODA-16: Stricter Y-tolerance in multi-column mode
                // WHY: Column text may have slight Y variations (different line heights)
                // Table cells are precisely aligned (same row = same Y)
                // - Normal mode: 10pt tolerance for slight extraction misalignment
                // - Strict mode: 2pt tolerance to require precise table alignment
                let y_tolerance = if strict_mode { 2.0 } else { 10.0 };

                // WHY 0.5 overlap: blocks on same row should have >50% vertical overlap
                if overlap_y > min_h * 0.5 || (b1.bbox.y1 - block.bbox.y1).abs() < y_tolerance {
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
    fn detect_tables(
        &self,
        page: &crate::schema::Page,
        rows: Vec<Vec<usize>>,
        strict_mode: bool,
    ) -> Vec<Block> {
        let mut new_blocks = Vec::new();
        let mut i = 0;

        // OODA-21: Get page width for paragraph detection
        let page_width = page.width;

        while i < rows.len() {
            // OODA-21: Skip rows that contain paragraphs (not table candidates)
            // WHY: Paragraphs are prose content, not tabular data
            if row_contains_paragraph(&rows[i], &page.blocks, page_width) {
                for &block_idx in &rows[i] {
                    new_blocks.push(page.blocks[block_idx].clone());
                }
                i += 1;
                continue;
            }

            // Table candidate: row with multiple blocks
            if rows[i].len() > 1 {
                let table_rows = self.find_table_extent(&rows, i, page);
                tracing::debug!(
                    "  Row {} has {} blocks, table extent = {} rows",
                    i,
                    rows[i].len(),
                    table_rows.len()
                );

                if self.is_likely_table(&table_rows, &rows, page, strict_mode) {
                    tracing::info!("  ✓ Creating table from {} rows", table_rows.len());
                    let table_block = self.create_table_block(&table_rows, &rows, page);
                    new_blocks.push(table_block);
                    i = table_rows.last().copied().unwrap_or(i) + 1;
                } else {
                    tracing::debug!("  ✗ Not a table (failed is_likely_table)");
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
    ///
    /// **OODA-21:** Added paragraph detection to stop table extent when
    /// encountering prose blocks. Tables should only contain short data cells.
    fn find_table_extent(
        &self,
        rows: &[Vec<usize>],
        start: usize,
        page: &crate::schema::Page,
    ) -> Vec<usize> {
        let mut table_rows = vec![start];
        let mut j = start + 1;

        // OODA-21: Get page width for paragraph detection
        // WHY: We need to determine if blocks span >55% of page width
        let page_width = page.width;

        while j < rows.len() {
            let current_row_blocks = &rows[j];

            // OODA-21: Stop table if this row contains a paragraph
            // WHY: Paragraphs are flowing text, not table cells
            if row_contains_paragraph(current_row_blocks, &page.blocks, page_width) {
                tracing::debug!(
                    "  OODA-21: Stopping table extent at row {} - paragraph detected",
                    j
                );
                break;
            }

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
    ///
    /// # OODA-16: Strict Mode for Multi-Column Pages
    ///
    /// In strict mode, add text length check to avoid false positives from
    /// column text that happens to align at the same Y-coordinate.
    ///
    /// # OODA-32: Author Block Rejection
    ///
    /// Author blocks near the top of page 1 should NOT be detected as tables.
    /// They have short text fragments (names, affiliations) that look like table
    /// cells but are actually metadata. Detect via email patterns (@) or
    /// academic affiliation patterns (superscript numbers, university names).
    fn is_likely_table(
        &self,
        table_rows: &[usize],
        rows: &[Vec<usize>],
        page: &crate::schema::Page,
        strict_mode: bool,
    ) -> bool {
        let has_multi_col = table_rows.iter().any(|&r| rows[r].len() > 1);

        // WHY: Require multiple rows with columns to avoid false positives
        // OODA FIX 2026-01-04: Relaxed thresholds to detect smaller tables
        // - 3+ rows with 2+ columns (simple tables like 2x3)
        // - 4+ rows with 3+ columns (moderate tables)
        // - 6+ rows with any multi-col (large tables)
        let base_check = table_rows.len() >= 3 && has_multi_col;

        if !base_check {
            return false;
        }

        // OODA-32: Reject author blocks on page 1 near the top
        // WHY: Author blocks contain short text fragments (names, affiliations, emails)
        // that look like table cells but are NOT tabular data. They typically:
        // - Appear in the top 30% of page 1
        // - Contain @ for emails or superscript numbers (¹²³) for affiliations
        // - Contain university/institution names
        if page.number == 1 {
            // Check if candidate table is in the top 30% of the page
            let mut min_y = f32::MAX;
            let mut combined_text = String::new();

            for &row_idx in table_rows {
                for &block_idx in &rows[row_idx] {
                    let block = &page.blocks[block_idx];
                    min_y = min_y.min(block.bbox.y1);
                    combined_text.push_str(&block.text);
                    combined_text.push(' ');
                }
            }

            // WHY: 200.0 = approximately top 25% of a 792pt page
            let is_near_top = min_y < 200.0;

            if is_near_top {
                // Check for author block patterns:
                // - Email addresses (@)
                // - Superscript affiliation numbers (¹²³⁴⁵⁶⁷⁸⁹)
                // - Common affiliation words (University, Institut, Department, School)
                let text_lower = combined_text.to_lowercase();
                let has_author_pattern = combined_text.contains('@')
                    || combined_text.contains('¹')
                    || combined_text.contains('²')
                    || combined_text.contains('³')
                    || combined_text.contains('⁴')
                    || combined_text.contains('⁵')
                    || combined_text.contains('⁶')
                    || combined_text.contains('⁷')
                    || combined_text.contains('⁸')
                    || combined_text.contains('⁹')
                    || text_lower.contains("university")
                    || text_lower.contains("universitat")
                    || text_lower.contains("universität")
                    || text_lower.contains("institut")
                    || text_lower.contains("department")
                    || text_lower.contains("school of")
                    || text_lower.contains(".edu");

                if has_author_pattern {
                    tracing::debug!(
                        "  ✗ Rejected: author block pattern detected on page 1 (y={:.1})",
                        min_y
                    );
                    return false;
                }
            }
        }

        // OODA-16: In strict mode (multi-column pages), add text length filter
        // WHY: Tables have short cells (typically <100 chars each)
        //      Column paragraphs have long sentences (typically 100-300 chars)
        //      This distinguishes real tables from coincidental Y-alignment
        if strict_mode {
            let mut total_chars = 0usize;
            let mut total_blocks = 0usize;

            for &row_idx in table_rows {
                for &block_idx in &rows[row_idx] {
                    let block = &page.blocks[block_idx];
                    total_chars += block.text.len();
                    total_blocks += 1;
                }
            }

            if total_blocks == 0 {
                return false;
            }

            let avg_text_length = total_chars as f32 / total_blocks as f32;

            // WHY 100 chars: Typical table cell = 10-50 chars (short values)
            // Typical paragraph = 100-300 chars (full sentences)
            // 100 chars is a clear dividing line
            if avg_text_length > 100.0 {
                tracing::debug!(
                    "  ✗ Rejected: avg text length {:.1} chars > 100 (likely column text)",
                    avg_text_length
                );
                return false;
            }

            tracing::debug!(
                "  ✓ Passed strict mode: avg text length {:.1} chars <= 100",
                avg_text_length
            );
        }

        true
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
    ///
    /// ## OODA-IT10: Comma-Formatted Number Support
    ///
    /// WHY: Table data often contains comma-formatted numbers (e.g., 2,017,886).
    /// Standard f64 parsing rejects these, causing table rows to be missed.
    /// Solution: Strip commas before parsing numbers.
    fn table_like_score(text: &str) -> i32 {
        let t = text.trim();
        if t.is_empty() {
            return 0;
        }

        let pipes = t.matches('|').count();
        let has_multi_space = t.contains("  ") || t.contains('\t');
        let cleaned = Self::sanitize_line(t);

        // OODA-IT10: Count numeric tokens, supporting comma-formatted numbers
        // WHY: "2,017,886" is a valid number in table data
        let num_tokens = cleaned
            .split_whitespace()
            .filter(|tok| {
                // Strip commas and try parsing
                let no_commas = tok.replace(',', "");
                no_commas.parse::<f64>().is_ok()
            })
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
    /// Returns (prefix, [nums...]) for patterns like "Total Tokens 2,017,886 2,306,535 5,081,069"
    ///
    /// OODA-IT10: Enhanced to handle multiple comma-formatted numbers.
    /// WHY: Academic tables often have 4+ numeric columns with comma formatting.
    fn parse_numeric_suffix(line: &str) -> Option<(String, Vec<String>)> {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            return None;
        }

        // OODA-IT10: Try to find ALL numeric tokens at the end
        // Numeric tokens: integers, floats, or comma-formatted numbers like "2,017,886"
        let is_numeric = |s: &str| {
            // Strip commas for parsing
            let clean = s.replace(',', "");
            clean.parse::<f64>().is_ok()
        };

        // Find where numeric suffix starts
        let mut num_start = tokens.len();
        for i in (0..tokens.len()).rev() {
            if is_numeric(tokens[i]) {
                num_start = i;
            } else {
                break;
            }
        }

        if num_start >= tokens.len() {
            // No numeric suffix found - try old fallback
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

            return None;
        }

        // We found numeric suffix
        let prefix = tokens[..num_start].join(" ");
        let nums: Vec<String> = tokens[num_start..].iter().map(|s| s.to_string()).collect();

        if nums.is_empty() || prefix.is_empty() {
            return None;
        }

        Some((prefix, nums))
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

            // OODA-IT10: Log when we find a table caption
            tracing::info!(
                "TextTableReconstruction: Found caption at block {} on page {}: '{}'",
                i,
                page.number,
                block.text.chars().take(50).collect::<String>()
            );

            // Check if structured table already exists nearby
            let has_existing_table = self.has_existing_table(page, i, page_idx, page_table_bboxes);

            if has_existing_table {
                tracing::info!("  → Existing table found nearby, skipping");
                new_blocks.push(block.clone());
                i += 1;
                continue;
            }

            // Scan for table content
            let (table_block, consumed) = self.scan_for_table(page, i);

            if let Some(table) = table_block {
                tracing::info!(
                    "  → Reconstructed table with {} children (consumed {} blocks)",
                    table.children.len(),
                    consumed - i
                );
                new_blocks.push(block.clone()); // Keep caption
                new_blocks.push(table);
                i = consumed;
            } else {
                tracing::info!("  → No table content found after caption");
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
            })
        {
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

        // OODA-IT10: Debug logging to trace scan behavior
        tracing::debug!(
            "  scan_for_table: starting at idx={}, blocks={}",
            caption_idx,
            page.blocks.len()
        );

        for j in (caption_idx + 1)..page.blocks.len().min(caption_idx + 1 + MAX_SCAN) {
            let b = &page.blocks[j];
            let t = b.text.trim();

            // OODA-37 FIX: Stop scanning when hitting Figure captions
            // WHY: Figure captions like "Figure 4. Cam×Time dataset..." were being consumed
            // as table content because only "Table N" patterns triggered the break.
            // This caused Figure 4 and Figure 7 to disappear from 01_2512 output.
            let is_figure_caption = t.starts_with("Figure ")
                && t.len() > 7
                && t.chars().nth(7).is_some_and(|c| c.is_ascii_digit());

            // OODA-IT10: Check if this is a "Table N mentions" text, not a caption
            // WHY: "Table 4 presents statistical information..." is prose ABOUT the table,
            // not another table caption.
            //
            // DETECTION LOGIC:
            // - "Table 4:" or "Table 4." at START = caption (colon/period after number)
            // - "Table 4 presents..." = prose reference (space after number, then word)
            //
            // We check if char immediately after "Table N" is a space followed by a letter
            // (not colon, period, or another number).
            let is_table_reference = if t.starts_with("Table ") && t.len() > 10 {
                // Get char after "Table N" (skip "Table " + digits)
                let after_table = t.chars().skip(6).skip_while(|c| c.is_ascii_digit());
                let first_char = after_table.clone().next();
                let second_char = after_table.skip(1).next();

                // Pattern: "Table N X..." where X is a letter (not : or .)
                // This indicates prose like "Table 4 presents..." or "Table 4 shows..."
                matches!(first_char, Some(' '))
                    && matches!(second_char, Some(c) if c.is_alphabetic())
            } else {
                false
            };

            let looks_caption = Self::looks_like_table_caption(t);
            let is_actual_caption = looks_caption && !is_table_reference;

            if t.is_empty() || Self::is_hard_break(b) || is_actual_caption || is_figure_caption {
                break;
            }

            let score = Self::table_like_score(t);
            tracing::debug!(
                "    idx={}: score={} text='{}'",
                j,
                score,
                t.chars().take(40).collect::<String>()
            );

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

        // OODA-IT10: Table row with large numbers should have score >= 2
        // Real example from LightRAG paper Table 4
        let score = TextTableReconstructionProcessor::table_like_score(
            "Total Tokens 2,017,886 2,306,535 5,081,069 619,009",
        );
        assert!(
            score >= 2,
            "Expected score >= 2 for numeric table row, got {}",
            score
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

    #[test]
    fn test_numeric_suffix_parsing_comma_numbers() {
        // OODA-IT10: Test comma-formatted numbers from academic tables
        let result = TextTableReconstructionProcessor::parse_numeric_suffix(
            "Total Tokens 2,017,886 2,306,535 5,081,069 619,009",
        );
        assert!(result.is_some(), "Should parse comma-formatted numbers");
        let (prefix, nums) = result.unwrap();
        assert_eq!(prefix, "Total Tokens");
        assert_eq!(nums.len(), 4);
        assert_eq!(nums[0], "2,017,886");
        assert_eq!(nums[1], "2,306,535");
        assert_eq!(nums[2], "5,081,069");
        assert_eq!(nums[3], "619,009");
    }

    #[test]
    fn test_table_caption_edge_cases() {
        assert!(!TextTableReconstructionProcessor::looks_like_table_caption(
            ""
        ));
        assert!(!TextTableReconstructionProcessor::looks_like_table_caption(
            "Random text"
        ));
        assert!(TextTableReconstructionProcessor::looks_like_table_caption(
            "table 5"
        ));
    }

    #[test]
    fn test_table_like_score_edge_cases() {
        // Empty string
        assert_eq!(TextTableReconstructionProcessor::table_like_score(""), 0);

        // Pure pipes but no content
        assert!(TextTableReconstructionProcessor::table_like_score("|||") >= 1);
    }

    #[test]
    fn test_numeric_suffix_parsing_no_numbers() {
        let result = TextTableReconstructionProcessor::parse_numeric_suffix("Method A");
        assert!(result.is_none());
    }

    #[test]
    fn test_numeric_suffix_parsing_edge_cases() {
        // Empty string returns None
        let result = TextTableReconstructionProcessor::parse_numeric_suffix("");
        assert!(result.is_none());

        // Single number (no prefix) returns None - needs at least 2 tokens
        let result = TextTableReconstructionProcessor::parse_numeric_suffix("1.0");
        assert!(result.is_none());

        // Valid: prefix + number
        let result = TextTableReconstructionProcessor::parse_numeric_suffix("Method 1.0");
        assert!(result.is_some());
        let (prefix, nums) = result.unwrap();
        assert_eq!(prefix, "Method");
        assert_eq!(nums, vec!["1.0"]);
    }
}
