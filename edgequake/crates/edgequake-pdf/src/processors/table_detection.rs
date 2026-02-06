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

            tracing::debug!(
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
                tracing::debug!(
                    "  Multi-column page ({} columns) - using strict table detection",
                    page.columns.len()
                );
            }

            let rows = self.group_blocks_by_row(page, strict_mode);
            tracing::trace!("  Grouped into {} rows", rows.len());
            let new_blocks = self.detect_tables(page, rows, strict_mode);
            tracing::trace!("  Produced {} blocks", new_blocks.len());
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
                    tracing::debug!("  ✓ Creating table from {} rows", table_rows.len());
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

    /// Check if text is a prose reference to a table, NOT a caption.
    ///
    /// ## OODA-IT10: Distinguish "Table N mentions" from captions
    ///
    /// WHY: "Table 4 presents statistical information..." is prose ABOUT a table,
    /// not a caption for a new table. If we treat it as a caption, we'll try
    /// to scan for table rows below it and fail (since the rows are earlier).
    ///
    /// DETECTION LOGIC:
    /// - "Table 4:" or "Table 4." → caption (ends with colon/period after number)
    /// - "Table 4 presents..." → prose reference (space after number, then word)
    ///
    /// We check if the char immediately after "Table N" is a space followed
    /// by an alphabetic character (indicating prose continuation).
    pub fn is_table_reference(text: &str) -> bool {
        let t = text.trim();
        if !t.starts_with("Table ") || t.len() <= 10 {
            return false;
        }

        // Get char after "Table N" (skip "Table " + digits)
        let after_table = t.chars().skip(6).skip_while(|c| c.is_ascii_digit());
        let first_char = after_table.clone().next();
        let second_char = after_table.clone().nth(1);

        // Pattern: "Table N X..." where X is a letter (not : or .)
        // This indicates prose like "Table 4 presents..." or "Table 4 shows..."
        matches!(first_char, Some(' ')) && matches!(second_char, Some(c) if c.is_alphabetic())
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

    /// Strip commas and percent signs for numeric parsing.
    /// WHY: Table values may be formatted as "2,017,886" or "32.4%".
    fn strip_numeric_decorators(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for c in s.chars() {
            if c != ',' && c != '%' {
                out.push(c);
            }
        }
        out
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
            .filter(|tok| Self::strip_numeric_decorators(tok).parse::<f64>().is_ok())
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

        // OODA-IT33: Detect column-oriented table blocks
        // WHY: Pdfium extracts table columns as vertical blocks with one value per line.
        // Academic tables produce blocks like "Agriculture\nCS\nLegal\nMix" (column headers)
        // or "32.4%\n67.6%\n38.4%\n61.6%" (column data). These get score 0 from the
        // row-oriented checks above because each line has only one value.
        let lines: Vec<&str> = t
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();

        if lines.len() >= 3 {
            // Count lines that look like percentage values (e.g., "32.4%", "100%")
            let pct_lines = lines
                .iter()
                .filter(|l| Self::is_percentage_value(l))
                .count();
            if pct_lines >= 2 {
                score += 3;
            }

            // Count lines that look like numeric values (including comma-formatted)
            let numeric_lines = lines
                .iter()
                .filter(|l| Self::strip_numeric_decorators(l).parse::<f64>().is_ok())
                .count();
            if numeric_lines >= 2 {
                score += 2;
            }

            // Multi-line short-value blocks: table columns have many short lines
            // WHY: A column of "Agriculture\nCS\nLegal\nMix" has avg length ~7
            let avg_len: f64 =
                lines.iter().map(|l| l.len() as f64).sum::<f64>() / lines.len() as f64;
            if avg_len <= 20.0 && lines.len() >= 3 {
                score += 1;
            }
        }

        score
    }

    /// Check if a string is a percentage value like "32.4%", "100%", "0.5%"
    fn is_percentage_value(s: &str) -> bool {
        let t = s.trim();
        if !t.ends_with('%') || t.len() < 2 {
            return false;
        }
        Self::strip_numeric_decorators(&t[..t.len() - 1])
            .parse::<f64>()
            .is_ok()
    }

    fn sanitize_line(line: &str) -> String {
        line.replace('|', " ")
    }

    /// Parse numeric suffix from line.
    /// Returns (prefix, [nums...]) for patterns like "Total Tokens 2,017,886 2,306,535 5,081,069"
    ///
    /// OODA-IT10: Enhanced to handle multiple comma-formatted numbers.
    /// OODA-IT33: Enhanced to handle percentage values (e.g., "32.4%").
    /// WHY: Academic tables often have 4+ numeric columns with comma formatting or percentages.
    fn parse_numeric_suffix(line: &str) -> Option<(String, Vec<String>)> {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            return None;
        }

        // OODA-IT33: Support percentage values alongside plain numbers
        // WHY: Academic tables use "32.4%" format extensively
        let is_numeric = |s: &str| Self::strip_numeric_decorators(s).parse::<f64>().is_ok();

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
            tracing::debug!(
                "TextTableReconstruction: Found caption at block {} on page {}: '{}'",
                i,
                page.number,
                block.text.chars().take(50).collect::<String>()
            );

            // Check if structured table already exists nearby
            let has_existing_table = self.has_existing_table(page, i, page_idx, page_table_bboxes);

            if has_existing_table {
                tracing::debug!("  → Existing table found nearby, skipping");
                new_blocks.push(block.clone());
                i += 1;
                continue;
            }

            // Scan for table content
            let (table_block, consumed) = self.scan_for_table(page, i);

            if let Some(table) = table_block {
                tracing::debug!(
                    "  → Reconstructed table with {} children (consumed {} blocks)",
                    table.children.len(),
                    consumed - i
                );
                new_blocks.push(block.clone()); // Keep caption
                new_blocks.push(table);
                i = consumed;
            } else {
                tracing::debug!("  → No table content found after caption");
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
    ///
    /// OODA-IT33: Now tries column-oriented reconstruction first.
    /// WHY: Pdfium extracts table data as vertical column blocks (one value per line),
    /// not horizontal rows. Academic tables like "Table 1: Win rates..." produce blocks
    /// such as ["Agriculture\nCS\nLegal\nMix", "32.4%\n67.6%\n38.4%\n61.6%", ...].
    /// These blocks are columns, not rows, so we need to transpose them.
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

        tracing::debug!(
            "  scan_for_table: starting at idx={}, blocks={}",
            caption_idx,
            page.blocks.len()
        );

        for j in (caption_idx + 1)..page.blocks.len().min(caption_idx + 1 + MAX_SCAN) {
            let b = &page.blocks[j];
            let t = b.text.trim();

            // OODA-37 FIX: Stop scanning when hitting Figure captions
            let is_figure_caption = t.starts_with("Figure ")
                && t.len() > 7
                && t.chars().nth(7).is_some_and(|c| c.is_ascii_digit());

            let is_table_ref = Self::is_table_reference(t);
            let looks_caption = Self::looks_like_table_caption(t);
            let is_actual_caption = looks_caption && !is_table_ref;

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

        // OODA-IT33: Try column-oriented reconstruction first
        // WHY: When pdfium extracts table data, each column becomes a separate block
        // with multiple lines. Detect this pattern and transpose columns → rows.
        let block_indices: Vec<usize> = lines.iter().map(|(idx, _, _)| *idx).collect();
        if let Some((table_block, consumed)) =
            self.try_column_reconstruction(page, caption_idx, &block_indices)
        {
            return (Some(table_block), consumed);
        }

        // Fall back to row-oriented reconstruction
        let rows = self.parse_rows(&lines);

        if rows.len() < 2 {
            return (None, caption_idx + 1);
        }

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

    /// OODA-IT33: Try column-oriented table reconstruction.
    ///
    /// WHY: Pdfium extracts table columns as vertical blocks. For example, page 7 of the
    /// LightRAG paper has blocks like:
    ///   Block 1: "Agriculture\nCS\nLegal\nMix" (4 lines, column headers)
    ///   Block 2: "NaiveRAG\nLightRAG\nNaiveRAG\nLightRAG..." (8 lines, sub-headers)
    ///   Block 3: "Comprehensiveness\n32.4%\n67.6%\n..." (36 lines, data grid)
    ///
    /// Detection: When blocks have many short lines, reconstruct by parsing the linearized
    /// data pattern: [label, value, value, ..., label, value, value, ...].
    fn try_column_reconstruction(
        &self,
        page: &crate::schema::Page,
        caption_idx: usize,
        block_indices: &[usize],
    ) -> Option<(Block, usize)> {
        if block_indices.len() < 2 {
            return None;
        }

        // Collect all lines from all blocks, tracking which block they came from
        let mut all_block_lines: Vec<(usize, Vec<String>)> = Vec::new();
        let mut has_multi_line_block = false;

        for &idx in block_indices {
            let block = &page.blocks[idx];
            let lines: Vec<String> = block
                .text
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();

            if lines.len() >= 6 {
                has_multi_line_block = true;
            }
            if !lines.is_empty() {
                all_block_lines.push((idx, lines));
            }
        }

        // Need at least one multi-line block to justify column reconstruction
        if !has_multi_line_block {
            return None;
        }

        // Try linearized grid detection on multi-line blocks
        // WHY: A block like "Comprehensiveness\n32.4%\n67.6%\n...\nDiversity\n23.6%\n..."
        // is a linearized table where rows = [label, val1, val2, ...val_n, label, ...]
        let mut all_rows: Vec<Vec<String>> = Vec::new();
        let mut header_lines: Vec<Vec<String>> = Vec::new();

        for (_, lines) in &all_block_lines {
            if lines.len() >= 6 {
                // Try to parse as linearized grid
                if let Some(grid_rows) = Self::parse_linearized_grid(lines) {
                    // Check if this is a sub-table (same column count)
                    if all_rows.is_empty()
                        || grid_rows.first().map(|r| r.len()).unwrap_or(0)
                            == all_rows.first().map(|r| r.len()).unwrap_or(0)
                        || all_rows.is_empty()
                    {
                        all_rows.extend(grid_rows);
                    }
                }
            } else {
                // Short blocks are headers
                header_lines.push(lines.clone());
            }
        }

        if all_rows.len() < 2 {
            return None;
        }

        // Prepend header lines as rows
        let data_col_count = all_rows.first().map(|r| r.len()).unwrap_or(0);
        let mut final_rows: Vec<Vec<String>> = Vec::new();

        for header in &header_lines {
            // Pad or trim header to match data column count
            let mut row = header.clone();
            row.resize(data_col_count, String::new());
            final_rows.push(row);
        }
        final_rows.extend(all_rows);

        if final_rows.len() < 2 {
            return None;
        }

        tracing::debug!(
            "  → Column-oriented reconstruction: {} rows × {} cols from {} blocks",
            final_rows.len(),
            data_col_count,
            all_block_lines.len()
        );

        // Build bounding box from all blocks
        let caption_block = &page.blocks[caption_idx];
        let mut table_bbox = caption_block.bbox;
        for (idx, _) in &all_block_lines {
            table_bbox = table_bbox.union(&page.blocks[*idx].bbox);
        }

        let mut table_block = Block::new(BlockType::Table, table_bbox);
        table_block.page = page.number - 1;
        table_block.children = Self::build_table_cells(table_bbox, table_block.page, &final_rows);
        table_block
            .metadata
            .insert("reconstructed".to_string(), serde_json::json!(true));
        table_block
            .metadata
            .insert("column_oriented".to_string(), serde_json::json!(true));

        let consumed = block_indices
            .iter()
            .max()
            .map(|&idx| idx + 1)
            .unwrap_or(caption_idx + 1);

        Some((table_block, consumed))
    }

    /// OODA-IT33: Parse a linearized grid from a multi-line block.
    ///
    /// WHY: Pdfium extracts table data as a single block with many lines like:
    ///   "Comprehensiveness\n32.4%\n67.6%\n38.4%\n61.6%\n16.4%\n83.6%\n38.8%\n61.2%\n
    ///    Diversity\n23.6%\n76.4%\n..."
    ///
    /// Pattern: [label, value, value, ..., label, value, value, ...] where labels
    /// are non-numeric and values are numeric/percentage.
    ///
    /// Returns rows like [["Comprehensiveness", "32.4%", "67.6%", ...], ["Diversity", ...]]
    fn parse_linearized_grid(lines: &[String]) -> Option<Vec<Vec<String>>> {
        if lines.len() < 4 {
            return None;
        }

        // Find label positions (non-numeric, non-percentage lines)
        let label_indices: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| {
                let t = l.trim();
                !t.is_empty() && !Self::is_numeric_or_pct(t)
            })
            .map(|(i, _)| i)
            .collect();

        if label_indices.len() < 2 {
            return None;
        }

        // Check that labels are evenly spaced (consistent column count)
        let first_gap = label_indices[1] - label_indices[0];
        if first_gap < 2 {
            return None; // Each row needs at least 1 label + 1 value
        }

        let consistent = label_indices.windows(2).all(|w| {
            let gap = w[1] - w[0];
            gap == first_gap
        });

        if !consistent {
            // Check if last row might be truncated
            let mostly_consistent = if label_indices.len() >= 3 {
                label_indices
                    .windows(2)
                    .take(label_indices.len() - 1)
                    .all(|w| {
                        let gap = w[1] - w[0];
                        gap == first_gap
                    })
            } else {
                false
            };
            if !mostly_consistent {
                return None;
            }
        }

        // Parse rows: each row starts at a label index and spans first_gap lines
        let values_per_row = first_gap - 1; // Number of numeric values per row
        let mut rows: Vec<Vec<String>> = Vec::new();

        for &label_idx in &label_indices {
            let label = lines[label_idx].trim().to_string();
            let mut row = vec![label];

            for offset in 1..=values_per_row {
                let val_idx = label_idx + offset;
                if val_idx < lines.len() {
                    row.push(lines[val_idx].trim().to_string());
                } else {
                    row.push(String::new());
                }
            }
            rows.push(row);
        }

        if rows.len() >= 2 {
            Some(rows)
        } else {
            None
        }
    }

    /// Check if a string is a numeric or percentage value.
    fn is_numeric_or_pct(s: &str) -> bool {
        let t = s.trim();
        if t.is_empty() {
            return false;
        }
        // Check percentage
        if Self::is_percentage_value(t) {
            return true;
        }
        // Check numeric (with commas)
        Self::strip_numeric_decorators(t).parse::<f64>().is_ok()
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

    #[test]
    fn test_is_table_reference_vs_caption() {
        // OODA-IT10: Test distinguishing prose references from table captions

        // Prose references (should return TRUE)
        assert!(
            TextTableReconstructionProcessor::is_table_reference(
                "Table 4 presents statistical information about the datasets"
            ),
            "Prose 'Table N presents...' should be detected as reference"
        );
        assert!(
            TextTableReconstructionProcessor::is_table_reference(
                "Table 1 shows the results of our experiments"
            ),
            "Prose 'Table N shows...' should be detected as reference"
        );
        assert!(
            TextTableReconstructionProcessor::is_table_reference(
                "Table 2 summarizes the key findings"
            ),
            "Prose 'Table N summarizes...' should be detected as reference"
        );

        // Captions (should return FALSE)
        assert!(
            !TextTableReconstructionProcessor::is_table_reference("Table 1."),
            "Caption 'Table N.' should NOT be a reference"
        );
        assert!(
            !TextTableReconstructionProcessor::is_table_reference("Table 1:"),
            "Caption 'Table N:' should NOT be a reference"
        );
        assert!(
            !TextTableReconstructionProcessor::is_table_reference("Table 1: Results"),
            "Caption 'Table N: Title' should NOT be a reference"
        );
        assert!(
            !TextTableReconstructionProcessor::is_table_reference("Table 4."),
            "Caption 'Table 4.' should NOT be a reference"
        );

        // Edge cases
        assert!(
            !TextTableReconstructionProcessor::is_table_reference("Table"),
            "Short 'Table' should NOT be a reference"
        );
        assert!(
            !TextTableReconstructionProcessor::is_table_reference("Table 1"),
            "'Table N' alone (<=10 chars) should NOT be a reference"
        );
        assert!(
            !TextTableReconstructionProcessor::is_table_reference("Not a table reference"),
            "Non-table text should NOT be a reference"
        );
    }

    #[test]
    fn test_table_like_score_percentage_blocks() {
        // OODA-IT33: Multi-line blocks with percentage values should score > 0
        let score =
            TextTableReconstructionProcessor::table_like_score("32.4%\n23.6%\n32.4%\n32.4%");
        assert!(
            score >= 3,
            "Percentage block should score >= 3, got {}",
            score
        );

        // Single-line percentage should not trigger multi-line bonus
        let score = TextTableReconstructionProcessor::table_like_score("32.4%");
        assert_eq!(score, 0, "Single percentage should score 0");
    }

    #[test]
    fn test_table_like_score_short_multiline() {
        // OODA-IT33: Multi-line short-value blocks (table columns)
        let score =
            TextTableReconstructionProcessor::table_like_score("Agriculture\nCS\nLegal\nMix");
        assert!(
            score >= 1,
            "Short multi-line block should score >= 1, got {}",
            score
        );
    }

    #[test]
    fn test_is_percentage_value() {
        assert!(TextTableReconstructionProcessor::is_percentage_value(
            "32.4%"
        ));
        assert!(TextTableReconstructionProcessor::is_percentage_value(
            "100%"
        ));
        assert!(TextTableReconstructionProcessor::is_percentage_value(
            "0.5%"
        ));
        assert!(!TextTableReconstructionProcessor::is_percentage_value(
            "hello"
        ));
        assert!(!TextTableReconstructionProcessor::is_percentage_value("%"));
        assert!(!TextTableReconstructionProcessor::is_percentage_value(""));
    }

    #[test]
    fn test_is_numeric_or_pct() {
        assert!(TextTableReconstructionProcessor::is_numeric_or_pct("32.4%"));
        assert!(TextTableReconstructionProcessor::is_numeric_or_pct("100"));
        assert!(TextTableReconstructionProcessor::is_numeric_or_pct(
            "2,017,886"
        ));
        assert!(!TextTableReconstructionProcessor::is_numeric_or_pct(
            "Agriculture"
        ));
        assert!(!TextTableReconstructionProcessor::is_numeric_or_pct(""));
    }

    #[test]
    fn test_parse_linearized_grid() {
        // OODA-IT33: Parse a linearized grid pattern
        let lines: Vec<String> = vec![
            "Comprehensiveness".to_string(),
            "32.4%".to_string(),
            "67.6%".to_string(),
            "Diversity".to_string(),
            "23.6%".to_string(),
            "76.4%".to_string(),
            "Empowerment".to_string(),
            "32.4%".to_string(),
            "67.6%".to_string(),
        ];
        let rows = TextTableReconstructionProcessor::parse_linearized_grid(&lines);
        assert!(rows.is_some(), "Should parse linearized grid");
        let rows = rows.unwrap();
        assert_eq!(rows.len(), 3, "Should have 3 rows");
        assert_eq!(rows[0], vec!["Comprehensiveness", "32.4%", "67.6%"]);
        assert_eq!(rows[1], vec!["Diversity", "23.6%", "76.4%"]);
        assert_eq!(rows[2], vec!["Empowerment", "32.4%", "67.6%"]);
    }

    #[test]
    fn test_parse_linearized_grid_uneven() {
        // Not a valid grid (uneven spacing)
        let lines: Vec<String> = vec![
            "Label1".to_string(),
            "1.0".to_string(),
            "2.0".to_string(),
            "Label2".to_string(),
            "3.0".to_string(),
        ];
        // Labels at 0 and 3 with gap of 3, but last section only has 1 value
        // This should work as the last row may be truncated
        let rows = TextTableReconstructionProcessor::parse_linearized_grid(&lines);
        assert!(rows.is_some());
    }

    #[test]
    fn test_parse_linearized_grid_no_pattern() {
        // All numeric - no labels
        let lines: Vec<String> = vec![
            "1.0".to_string(),
            "2.0".to_string(),
            "3.0".to_string(),
            "4.0".to_string(),
        ];
        let rows = TextTableReconstructionProcessor::parse_linearized_grid(&lines);
        assert!(rows.is_none(), "All-numeric should not parse as grid");
    }

    #[test]
    fn test_parse_numeric_suffix_with_percentages() {
        // OODA-IT33: Test percentage values in numeric suffix
        let result = TextTableReconstructionProcessor::parse_numeric_suffix(
            "Comprehensiveness 32.4% 67.6% 38.4%",
        );
        assert!(result.is_some(), "Should parse percentage suffixes");
        let (prefix, nums) = result.unwrap();
        assert_eq!(prefix, "Comprehensiveness");
        assert_eq!(nums.len(), 3);
        assert_eq!(nums[0], "32.4%");
    }

    #[test]
    fn test_strip_numeric_decorators() {
        assert_eq!(
            TextTableReconstructionProcessor::strip_numeric_decorators("2,017,886"),
            "2017886"
        );
        assert_eq!(
            TextTableReconstructionProcessor::strip_numeric_decorators("32.4%"),
            "32.4"
        );
        assert_eq!(
            TextTableReconstructionProcessor::strip_numeric_decorators("hello"),
            "hello"
        );
    }
}
