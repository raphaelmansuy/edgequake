//! Layout analysis and margin filtering processors.
//!
//! @implements FEAT1003
//!
//! **Single Responsibility:** Spatial layout processing.
//!
//! This module contains processors for layout-related operations:
//! - `LayoutProcessor`: Column detection and reading order
//! - `BlockMergeProcessor`: Merge adjacent text blocks
//! - `MarginFilterProcessor`: Remove margin content (line numbers, headers)
//! - `SectionNumberMergeProcessor`: Join split section numbers with titles
//!
//! **First Principles:**
//! - Layout uses adaptive thresholds from document statistics
//! - No magic numbers - margins are percentages of page dimensions
//! - Reading order follows column structure (left-to-right, top-to-bottom)

use crate::layout::LayoutAnalyzer;
use crate::schema::{Block, BlockType, BoundingBox, Document};
use crate::Result;

use super::stats::DocumentStats;
use super::structure_detection::starts_with_bullet;
use super::Processor;

/// WHY: UTF-8 safe string truncation.
///
/// Direct byte slicing like `&s[..15]` can panic if byte 15 falls in the middle
/// of a multi-byte character (e.g., box-drawing '─' is 3 bytes). This function
/// finds the nearest valid char boundary at or before `max_bytes`.
///
/// OODA-04: Fix byte index panics in layout_processing.rs (block text logging).
fn safe_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

// =============================================================================
// LayoutProcessor
// =============================================================================

/// Detects page layout (columns) and sorts blocks by reading order.
///
/// **Column Detection:**
/// - Analyzes block x-positions for column boundaries
/// - Handles single-column, two-column, and three-column layouts
///
/// **Reading Order:**
/// - Column-by-column, top-to-bottom within each column
///
/// **WHY adaptive:**
/// Academic papers use varied layouts. We detect, not assume.
pub struct LayoutProcessor {
    analyzer: LayoutAnalyzer,
}

impl LayoutProcessor {
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
            // OODA-29 FIX: If page already has columns set by backend, skip re-sorting.
            // WHY: The extraction_engine already establishes correct reading order:
            // - For multi-column pages, text_grouping.rs sorts each column by Y
            // - Then concatenates columns in correct order (left → right)
            // - OODA-12 in extraction_engine skips Y-sort to preserve this order
            // Re-sorting here was destroying the column-aware order and causing
            // interleaving of left/right column content.
            if !page.columns.is_empty() {
                tracing::debug!(
                    "LAYOUT: Page {} has {} columns from backend, SKIPPING re-sort (OODA-29)",
                    page.number,
                    page.columns.len()
                );

                continue;
            }

            let layout = self.analyzer.analyze(&page.blocks, page.width, page.height);

            // WHY: Check for bullet list markers to avoid misclassifying lists as tables
            // Bullet lists have short items in rows but are NOT tables
            let has_bullets = page.blocks.iter().any(|b| {
                let text = b.text.trim();
                text.starts_with("•")
                    || text.starts_with("*")
                    || text.starts_with("-")
                    || text.starts_with("1.")
                    || text.starts_with("2.")
            });

            // Check if detected columns look like a table structure
            let bboxes: Vec<crate::schema::BoundingBox> =
                page.blocks.iter().map(|b| b.bbox).collect();
            let is_table = self
                .analyzer
                .column_detector()
                .is_likely_table(&bboxes, &layout.columns);

            if is_table && !has_bullets {
                // Table-like layouts: use single column to preserve natural order
                page.columns = vec![];
                tracing::debug!("Detected table-like layout, skipping column-based reading order");
            } else {
                page.columns = layout.columns;
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

// =============================================================================
// BlockMergeProcessor
// =============================================================================

/// Merges adjacent text blocks that belong together.
///
/// **Merge Criteria:**
/// - Same block type (text + text, not text + header)
/// - Vertical gap within adaptive threshold (2.5x typical line spacing)
/// - Horizontal alignment within column tolerance
/// - No style change (font size, weight)
///
/// **WHY adaptive thresholds:**
/// Different documents have different spacing. We calculate from stats.
pub struct BlockMergeProcessor {}

impl BlockMergeProcessor {
    pub fn new() -> Self {
        Self {}
    }

    /// Check if two blocks should be merged.
    ///
    /// **WHY columns parameter:** Prevents merging blocks from different columns in multi-column layouts.
    /// This is critical to preserve reading order (left column → right column).
    fn should_merge(
        &self,
        a: &Block,
        b: &Block,
        stats: &DocumentStats,
        columns: &[BoundingBox],
    ) -> bool {
        // **CRITICAL**: Never merge blocks from different columns
        // WHY: Multi-column layouts must preserve left→right reading order
        if columns.len() >= 2 {
            let a_column = self.get_block_column(a, columns);
            let b_column = self.get_block_column(b, columns);

            if a_column != b_column {
                tracing::debug!(
                    "BlockMerge: REJECT - different columns (col {} vs col {})",
                    a_column,
                    b_column
                );
                return false;
            }
        }

        // Only merge text/header/list blocks
        if !matches!(
            a.block_type,
            BlockType::Text | BlockType::SectionHeader | BlockType::ListItem
        ) || !matches!(
            b.block_type,
            BlockType::Text | BlockType::SectionHeader | BlockType::ListItem
        ) {
            tracing::debug!(
                "BlockMerge: skip - not mergeable types {:?}/{:?}",
                a.block_type,
                b.block_type
            );
            return false;
        }

        // Types must match
        if a.block_type != b.block_type {
            tracing::debug!(
                "BlockMerge: skip - type mismatch {:?} vs {:?}",
                a.block_type,
                b.block_type
            );
            return false;
        }

        // OODA-25: Generic watermark/identifier detection
        // WHY: Documents have margin identifiers (DOI, version strings, timestamps)
        // These are separate from body text and should not be merged
        let trimmed_a = a.text.trim();

        // Generic identifier pattern: "prefix:value" format (e.g., "DOI:10.xxx", "arXiv:2510.xxx")
        let a_is_identifier =
            trimmed_a.contains(':') && trimmed_a.len() < 60 && !trimmed_a.contains(' ');

        // Generic footnote marker detection (Unicode symbols commonly used)
        let a_is_footnote = trimmed_a.starts_with('⋆')
            || trimmed_a.starts_with('†')
            || trimmed_a.starts_with('‡')
            || trimmed_a.starts_with('§')
            || trimmed_a.starts_with('¶')
            || trimmed_a.starts_with('*');

        if a_is_identifier || a_is_footnote {
            tracing::debug!(
                "BlockMerge: skip - a is identifier/footnote (id={}, fn={}, text='{}')",
                a_is_identifier,
                a_is_footnote,
                safe_truncate(trimmed_a, 30)
            );
            return false;
        }

        // Don't merge if b looks like a new structural element
        let trimmed_b = b.text.trim();
        let trimmed_b_lower = trimmed_b.to_lowercase();

        // OODA-25: Generic reference detection [N] pattern
        // WHY: Academic references, citations, footnotes use [N] format
        // This is universal across document types, not just arXiv
        let is_bracketed_reference = trimmed_b.len() > 2
            && trimmed_b.starts_with('[')
            && trimmed_b
                .chars()
                .skip(1)
                .take_while(|c| c.is_ascii_digit())
                .count()
                >= 1
            && trimmed_b.contains(']');

        // OODA-25: Generic identifier watermark (prefix:value pattern)
        let is_identifier_watermark = trimmed_b.contains(':')
            && trimmed_b.len() < 60
            && trimmed_b.split_whitespace().count() <= 3
            && (trimmed_b_lower.contains("doi")
                || trimmed_b
                    .chars()
                    .next()
                    .map(|c| c.is_lowercase())
                    .unwrap_or(false)
                || trimmed_b.starts_with(|c: char| c.is_ascii_digit()));

        // Generic footnote markers
        let is_footnote_marker = trimmed_b.starts_with('⋆')
            || trimmed_b.starts_with('†')
            || trimmed_b.starts_with('‡')
            || trimmed_b.starts_with('§')
            || trimmed_b.starts_with('¶')
            || trimmed_b.starts_with('*');

        // OODA-IT12: Use comprehensive bullet detection (530+ Unicode bullets)
        // WHY: Previous check only handled "• " (bullet + space), but PDFs like
        // LightRAG paper have "•General" (bullet + uppercase, no space).
        // The starts_with_bullet helper handles all cases:
        // - Bullet + space ("• text")
        // - Bullet + uppercase ("•General") - list item sentence start
        // - Bullet + asterisk ("•**bold**") - markdown bold markers
        let is_bullet_list = starts_with_bullet(trimmed_b)
            || trimmed_b.starts_with("- ")
            || trimmed_b.starts_with("* ");

        if is_bullet_list
            || is_bracketed_reference
            || is_identifier_watermark
            || is_footnote_marker
            || (trimmed_b.len() > 2
                && trimmed_b.chars().next().unwrap().is_ascii_digit()
                && trimmed_b.contains(". "))
        {
            tracing::debug!(
                "BlockMerge: skip - b is special (bullet={}, ref={}, id={}, footnote={}, text='{}')",
                is_bullet_list,
                is_bracketed_reference,
                is_identifier_watermark,
                is_footnote_marker,
                safe_truncate(trimmed_b, 30)
            );
            return false;
        }

        // Check style compatibility
        if let (Some(span_a), Some(span_b)) = (a.spans.last(), b.spans.first()) {
            let size_a = span_a.style.size.unwrap_or(0.0);
            let size_b = span_b.style.size.unwrap_or(0.0);
            if (size_a - size_b).abs() > 1.5 {
                tracing::debug!(
                    "BlockMerge: skip - font size diff {:.1} vs {:.1}",
                    size_a,
                    size_b
                );
                return false;
            }

            let weight_a = span_a.style.weight.unwrap_or(400);
            let weight_b = span_b.style.weight.unwrap_or(400);
            if (weight_a >= 600) != (weight_b >= 600) {
                tracing::debug!(
                    "BlockMerge: skip - weight mismatch {} vs {}",
                    weight_a,
                    weight_b
                );
                return false;
            }
        }

        // === ADAPTIVE THRESHOLDS ===

        // Vertical gap threshold: 2.5x typical line spacing
        let max_vertical_gap = stats.typical_line_spacing * 2.5;
        let vertical_threshold = if a.block_type == BlockType::SectionHeader {
            max_vertical_gap * 1.5 // Headers can span more
        } else {
            max_vertical_gap
        };

        // Check vertical proximity
        // OODA-30 FIX: In document coordinates (Y=0 at TOP), blocks are ordered top-to-bottom.
        // Block 'a' is ABOVE block 'b', so gap = b.y1 - a.y2 (bottom of a to top of b)
        // WHY: Previous calculation (a.y1 - b.y2) was wrong - it compared top of a to bottom of b,
        // giving incorrect gap measurements (e.g., 20pt instead of 2pt).
        let vertical_gap = (b.bbox.y1 - a.bbox.y2).max(0.0);
        tracing::debug!(
            "BlockMerge: '{}...' vs '{}...' gap={:.1} threshold={:.1}",
            safe_truncate(&a.text, 15),
            safe_truncate(&b.text, 15),
            vertical_gap,
            vertical_threshold
        );
        if vertical_gap > vertical_threshold {
            tracing::debug!("BlockMerge: REJECT - gap too large");
            return false;
        }

        // OODA-21: Horizontal alignment with generous indent tolerance
        // WHY: Academic papers use various indentation patterns:
        // - First-line indent (~10-20pt for paragraph starts)
        // - Abstract indentation (entire block indented)
        // - Block quotes (25-40pt indent)
        // - Wrapped lines may start at different X due to font kerning
        //
        // The column_alignment_tolerance (2pt typical) is for detecting column
        // alignment patterns, NOT for merge decisions. For merging, we need to
        // allow standard typographic indentation.
        let margin_diff = (a.bbox.x1 - b.bbox.x1).abs();

        // Max indent tolerance: ~1 inch (72pt) or 5x body font size, whichever smaller
        // WHY: Standard paragraph indent is 0.25-0.5 inch (18-36pt)
        // Block quotes can be 0.5-1 inch. Beyond that, likely different column.
        let max_indent_tolerance = (stats.body_font_size * 5.0).min(72.0);

        // For headers, allow even more (headers may span columns)
        let max_margin = if a.block_type == BlockType::SectionHeader {
            max_indent_tolerance * 1.5
        } else {
            max_indent_tolerance
        };

        // Column separation: blocks clearly in different columns shouldn't merge
        // WHY: Column gap is typically 15-20% of page width
        let horizontal_zone_threshold = stats.page_width * 0.15;
        if margin_diff > horizontal_zone_threshold {
            tracing::debug!(
                "BlockMerge: REJECT - horizontal zone {} > {}",
                margin_diff,
                horizontal_zone_threshold
            );
            return false;
        }

        let accept = margin_diff <= max_margin;
        if !accept {
            tracing::debug!(
                "BlockMerge: REJECT - margin {} > {} (indent tolerance)",
                margin_diff,
                max_margin
            );
        } else {
            tracing::debug!("BlockMerge: ACCEPT - merging blocks");
        }
        accept
    }

    /// Determine which column a block belongs to.
    ///
    /// **WHY:** Allows BlockMergeProcessor to respect column boundaries.
    ///
    /// **OODA-20 FIX:** Use block's LEFT EDGE (x1) for column assignment, not center.
    /// Block widths vary wildly (narrow "ROI." vs wide "Elitizon designs...").
    /// Using center point caused adjacent blocks to be assigned to different columns
    /// when one is narrow and another is wide. The left edge represents where the
    /// block STARTS in the document, which is the true column indicator.
    fn get_block_column(&self, block: &Block, columns: &[BoundingBox]) -> usize {
        let left_edge_point =
            crate::schema::Point::new(block.bbox.x1, block.bbox.y1 + block.bbox.height() / 2.0);

        // Find column containing the block's left edge
        for (idx, col) in columns.iter().enumerate() {
            if col.contains_point(&left_edge_point) {
                return idx;
            }
        }

        // Fallback: find closest column by left edge X-coordinate
        let result = columns
            .iter()
            .enumerate()
            .min_by_key(|(_, col): &(usize, &BoundingBox)| {
                // Use left edge of column, not center
                ((col.x1 - block.bbox.x1).abs() * 1000.0) as i32
            })
            .map(|(idx, _)| idx)
            .unwrap_or(0);

        result
    }

    /// OODA-23: Merge cross-column hyphenated words.
    ///
    /// WHY: In multi-column layouts, words at column boundaries may be hyphenated:
    /// - "reposito-" at end of left column
    /// - "ries remains" at start of right column
    ///
    /// Standard block merge rejects cross-column pairs. This post-processor:
    /// 1. Finds blocks ending with hyphen in column N
    /// 2. Finds blocks starting with lowercase in column N+1
    /// 3. Validates the join makes linguistic sense (word fragment + continuation)
    /// 4. Merges them if at similar Y positions (same line visually)
    fn merge_cross_column_hyphenation(
        &self,
        blocks: Vec<Block>,
        columns: &[BoundingBox],
    ) -> Vec<Block> {
        if blocks.is_empty() || columns.len() < 2 {
            return blocks;
        }

        let result = blocks;
        let mut merged_indices: std::collections::HashSet<usize> = std::collections::HashSet::new();
        let mut new_blocks: Vec<(usize, Block)> = Vec::new(); // (left_idx, merged_block)

        // Group blocks by column
        let mut column_blocks: Vec<Vec<(usize, &Block)>> = vec![Vec::new(); columns.len()];
        for (idx, block) in result.iter().enumerate() {
            let col = self.get_block_column(block, columns);
            column_blocks[col].push((idx, block));
        }

        // Look for cross-column hyphenation pairs
        for col_idx in 0..columns.len() - 1 {
            let left_col = &column_blocks[col_idx];
            let right_col = &column_blocks[col_idx + 1];

            for &(left_idx, left_block) in left_col {
                let left_text = left_block.text.trim_end();
                if !left_text.ends_with('-') {
                    continue;
                }

                // Extract the word fragment before the hyphen
                // "...software reposito-" -> "reposito"
                let word_fragment = left_text
                    .trim_end_matches('-')
                    .split_whitespace()
                    .last()
                    .unwrap_or("");

                if word_fragment.is_empty() || word_fragment.len() < 3 {
                    continue;
                }

                // Find continuation in right column (starts with lowercase, similar Y)
                let left_y = left_block.bbox.y2; // Bottom of left block

                for &(right_idx, right_block) in right_col {
                    let right_text = right_block.text.trim();
                    if right_text.is_empty() {
                        continue;
                    }

                    let first_char = right_text.chars().next().unwrap();
                    if !first_char.is_lowercase() {
                        continue;
                    }

                    // Extract the continuation word
                    // "ries remains limited" -> "ries"
                    let continuation = right_text
                        .split(|c: char| c.is_whitespace() || c == '.' || c == ',')
                        .next()
                        .unwrap_or("");

                    if continuation.is_empty() {
                        continue;
                    }

                    // OODA-23 FIX: Validate the join makes sense
                    // The fragment + continuation should form a plausible word
                    // "reposito" + "ries" = "repositories" ✓
                    // "reposito" + "tory" = "repositotory" ✗ (likely wrong match)
                    let combined = format!("{}{}", word_fragment, continuation);

                    // Reject if combined word has duplicate/overlapping syllables
                    // This catches cases where we're matching the wrong continuation
                    // E.g., "reposito" should match "ries", not "tory"
                    // "reposi" in caption matches "tory", creating "repositotory" (wrong!)
                    let is_likely_wrong = {
                        // Check for repeated character sequences at the join point
                        let frag_suffix = if word_fragment.len() >= 2 {
                            &word_fragment[word_fragment.len() - 2..]
                        } else {
                            word_fragment
                        };
                        let cont_prefix = if continuation.len() >= 2 {
                            &continuation[..2]
                        } else {
                            continuation
                        };

                        // "to" at end of "reposito" vs "to" at start of "tory" - likely overlap
                        // But "to" at end of "reposito" vs "ri" at start of "ries" - likely correct
                        let has_overlap = frag_suffix
                            .chars()
                            .last()
                            .map(|c| cont_prefix.starts_with(c))
                            .unwrap_or(false);

                        // Also check if continuation starts with part of fragment
                        // "reposito" ends with "to", continuation "tory" starts with "to"
                        let frag_last_two = if word_fragment.len() >= 2 {
                            &word_fragment[word_fragment.len() - 2..]
                        } else {
                            ""
                        };
                        let cont_starts_with_frag_end =
                            !frag_last_two.is_empty() && continuation.starts_with(frag_last_two);

                        has_overlap || cont_starts_with_frag_end
                    };

                    if is_likely_wrong {
                        tracing::debug!(
                            "OODA-23: Rejecting unlikely match: '{}' + '{}' = '{}'",
                            word_fragment,
                            continuation,
                            combined
                        );
                        continue;
                    }

                    // Check Y proximity (within ~20% of page height)
                    let right_y = right_block.bbox.y1; // Top of right block
                    let y_diff = (left_y - right_y).abs();

                    // Allow significant Y difference since columns may be offset
                    // Academic papers often have figure/title pushing content down
                    if y_diff > 400.0 {
                        continue;
                    }

                    // Found a match! Merge the blocks
                    tracing::debug!(
                        "OODA-23: Cross-column hyphenation merge: '{}' + '{}' = '{}' (Y diff: {:.0})",
                        word_fragment,
                        continuation,
                        combined,
                        y_diff
                    );

                    // Create merged block
                    let mut merged = result[left_idx].clone();
                    merged.merge(&result[right_idx]);
                    new_blocks.push((left_idx, merged)); // Track which index to replace
                    merged_indices.insert(left_idx);
                    merged_indices.insert(right_idx);

                    break; // Only merge with first matching block
                }
            }
        }

        // If no merges happened, return original
        if merged_indices.is_empty() {
            return result;
        }

        // OODA-26 FIX: Build final result PRESERVING original reading order
        // WHY: The extraction engine established column-first order (OODA-12).
        // Sorting by Y would interleave columns, destroying reading order.
        // Instead, insert merged blocks at the position of their LEFT component.
        let new_blocks_map: std::collections::HashMap<usize, Block> =
            new_blocks.into_iter().collect();

        let mut final_blocks: Vec<Block> = Vec::with_capacity(result.len());

        for (idx, block) in result.into_iter().enumerate() {
            if merged_indices.contains(&idx) {
                // Check if this was the LEFT block of a merge
                if let Some(merged_block) = new_blocks_map.get(&idx) {
                    // Insert merged block at this position
                    final_blocks.push(merged_block.clone());
                }
                // Skip RIGHT blocks (already merged into left)
            } else {
                final_blocks.push(block);
            }
        }

        // NO Y-SORT! Preserve the column-first reading order from extraction engine.
        // WHY: OODA-12 specifically skips Y-sort for multi-column pages to ensure
        // left column is read completely before right column.

        final_blocks
    }

    fn merge_page_blocks(
        &self,
        blocks: Vec<Block>,
        stats: &DocumentStats,
        columns: &[BoundingBox],
    ) -> Vec<Block> {
        if blocks.len() < 2 {
            return blocks;
        }

        // Log column count for debugging
        tracing::debug!(
            "BlockMerge: Processing {} blocks with {} columns",
            blocks.len(),
            columns.len()
        );

        // Log column bounding boxes
        for (i, col) in columns.iter().enumerate() {
            tracing::debug!(
                "BlockMerge: Column {} bbox: x1={:.1} y1={:.1} x2={:.1} y2={:.1}",
                i,
                col.x1,
                col.y1,
                col.x2,
                col.y2
            );
        }

        let mut merged = Vec::new();
        let mut current: Option<Block> = None;

        // DEBUG: Log blocks at BlockMerge start for blocks containing key text
        let is_debug_page = blocks.iter().any(|b| b.text.contains("disentangles space"));
        if is_debug_page {
            tracing::debug!(
                "BLOCKMERGE-START: {} blocks total, {} columns",
                blocks.len(),
                columns.len()
            );
            for (idx, block) in blocks.iter().enumerate() {
                if block.text.contains("disentangles")
                    || block.text.contains("dering")
                    || block.text.contains("independently")
                {
                    tracing::debug!(
                        "BLOCKMERGE-KEY idx={}: x1={:.0} y1={:.0} len={} FULL: '{}'",
                        idx,
                        block.bbox.x1,
                        block.bbox.y1,
                        block.text.len(),
                        &block.text
                    );
                }
            }
        }

        for (idx, block) in blocks.into_iter().enumerate() {
            // DEBUG: Log blocks that contain "ren-" or "dering" or "independently"
            if block.text.contains("ren-")
                || block.text.contains("dering")
                || block.text.starts_with("independently")
            {
                tracing::debug!(
                    "MERGE-TRACE block {}: '{}...' x1={:.0}",
                    idx,
                    safe_truncate(&block.text, 50),
                    block.bbox.x1
                );
            }

            if let Some(mut cur) = current.take() {
                if self.should_merge(&cur, &block, stats, columns) {
                    // DEBUG: Log merges involving our target blocks
                    if cur.text.contains("ren-")
                        || block.text.contains("dering")
                        || block.text.starts_with("independently")
                    {
                        tracing::debug!(
                            "MERGE-HAPPENING: '{}...' + '{}...'",
                            &cur.text[cur.text.len().saturating_sub(20)..],
                            safe_truncate(&block.text, 20)
                        );
                    }
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

        // OODA-23: Cross-column hyphenation post-processing
        // WHY: In multi-column layouts, a word may be hyphenated at the end of one column
        // and continue at the start of the next column. Example: "reposito-" in left column,
        // "ries remains" in right column. Standard merge rejects cross-column merges, but
        // hyphenated words should be joined regardless of column boundaries.
        if columns.len() >= 2 {
            merged = self.merge_cross_column_hyphenation(merged, columns);
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
        let stats = DocumentStats::from_document(&document);
        tracing::debug!(
            "BlockMergeProcessor: line_spacing={:.1}",
            stats.typical_line_spacing
        );

        for page in &mut document.pages {
            let block_count_before = page.blocks.len();

            let blocks = std::mem::take(&mut page.blocks);
            let columns = &page.columns; // Capture columns before moving blocks
            page.blocks = self.merge_page_blocks(blocks, &stats, columns);

            let block_count_after = page.blocks.len();
            tracing::debug!(
                "BlockMergeProcessor: page {} blocks {} -> {} (columns={})",
                page.number,
                block_count_before,
                block_count_after,
                columns.len()
            );

            page.update_stats();
        }

        document.update_stats();
        Ok(document)
    }

    fn name(&self) -> &str {
        "BlockMergeProcessor"
    }
}

// =============================================================================
// MarginFilterProcessor
// =============================================================================

/// Filters margin content (line numbers, headers, footers).
///
/// **Adaptive Margins:**
/// - Left margin: 8% of page width
/// - Right margin: 5% of page width
/// - Top/bottom: 5% of page height
///
/// **Running Header/Footer Detection:**
/// - Text appearing on 50%+ of pages is likely running content
///
/// **WHY percentages:**
/// Page sizes vary (A4, Letter, etc.). Percentages scale appropriately.
pub struct MarginFilterProcessor {}

impl MarginFilterProcessor {
    pub fn new() -> Self {
        Self {}
    }

    /// Check if a block is margin content that should be filtered.
    ///
    /// OODA-28: Uses normalized coordinate system where Y=0 is at TOP of page.
    /// - header_threshold: blocks with Y1 <= this are in top margin
    /// - footer_threshold: blocks with Y2 >= this are in bottom margin
    fn is_margin_content(
        &self,
        block: &Block,
        page_width: f32,
        page_height: f32,
        left_margin: f32,
        right_margin: f32,
        _header_threshold: f32, // Not used here, checked separately
        footer_threshold: f32,
        line_number_edge: f32,
    ) -> bool {
        let bbox = &block.bbox;

        // Filter left margin
        if bbox.x2 < left_margin {
            return true;
        }
        // Filter right margin
        if bbox.x1 > page_width - right_margin {
            return true;
        }

        // Detect line number runs at page edges
        let trimmed = block.text.trim();
        let edge_adjacent = bbox.x1 < line_number_edge || bbox.x2 > page_width - line_number_edge;
        if edge_adjacent {
            let tokens: Vec<&str> = trimmed.split_whitespace().collect();
            if tokens.len() >= 6 && tokens.iter().all(|t| t.chars().all(|c| c.is_ascii_digit())) {
                let nums: Vec<i32> = tokens.iter().filter_map(|t| t.parse().ok()).collect();
                if nums.len() == tokens.len() {
                    let all_same = nums.iter().all(|n| *n == nums[0]);
                    let consecutive = nums.windows(2).all(|w| w[1] == w[0].saturating_add(1));
                    if all_same || consecutive {
                        return true;
                    }
                }
            }
        }

        // Filter isolated digits/letters at page edge (likely line numbers)
        let text = trimmed;
        if text.len() <= 2
            && text
                .chars()
                .all(|c| c.is_ascii_digit() || c.is_ascii_uppercase())
            && (bbox.x1 < line_number_edge || bbox.x1 > page_width - line_number_edge)
        {
            return true;
        }

        // OODA-24: Filter standalone single digits that are likely superscripts
        // WHY: Author affiliation markers (¹, ², ³) are rendered as separate text elements
        // at superscript positions. They appear as standalone "1", "2", "3" blocks.
        // Detection: Single digit, very small bbox (superscript size), not at page edges
        // Heuristic: bbox height < 8pt indicates superscript positioning
        let bbox_height = bbox.y2 - bbox.y1;
        if text.len() == 1 && text.chars().all(|c| c.is_ascii_digit()) && bbox_height < 8.0 {
            tracing::debug!(
                "OODA-24: Filtering standalone digit '{}' (height={:.1}, likely superscript)",
                text,
                bbox_height
            );
            return true;
        }

        // OODA-28: Filter footer page numbers using normalized coordinates
        // High Y = bottom of page in normalized system
        let in_footer = bbox.y2 >= footer_threshold;
        if in_footer && trimmed.parse::<i32>().is_ok() {
            return true;
        }

        // Extended page number detection - bottom 12% of page
        // WHY: Pandoc and other tools place page numbers at varying heights.
        // OODA-28: Use normalized coordinates - high Y = bottom
        let extended_footer_threshold = page_height * 0.88; // Top 88% = bottom 12%
        let in_extended_footer = bbox.y2 >= extended_footer_threshold;
        if in_extended_footer {
            // Standalone page number: 1-4 digits only
            if let Ok(num) = trimmed.parse::<u32>() {
                if num <= 9999 && trimmed.len() <= 4 {
                    return true;
                }
            }
            // "Page N" or "Page N of M" format
            let text_lower = trimmed.to_lowercase();
            if text_lower.starts_with("page ") {
                let rest = text_lower.strip_prefix("page ").unwrap_or("");
                let first_word = rest.split_whitespace().next().unwrap_or("");
                if first_word.parse::<u32>().is_ok() {
                    return true;
                }
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
        use std::collections::{HashMap, HashSet};

        // First pass: collect repeated margin texts
        let mut header_counts: HashMap<String, usize> = HashMap::new();
        let mut footer_counts: HashMap<String, usize> = HashMap::new();

        let normalize = |text: &str| -> String {
            text.split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase()
        };

        for page in &document.pages {
            let page_height = page.height;
            // OODA-28 FIX: Coordinate system uses Y=0 at TOP (normalized coordinates)
            // WHY: Backend normalizes coordinates so Y=0 is at top of page
            // - in_header = LOW Y (near 0 = top of page)
            // - in_footer = HIGH Y (near page_height = bottom of page)
            let header_threshold = page_height * 0.05; // Top 5% of page
            let footer_threshold = page_height * 0.95; // Bottom 5% of page

            let mut header_seen: HashSet<String> = HashSet::new();
            let mut footer_seen: HashSet<String> = HashSet::new();

            for block in &page.blocks {
                let trimmed = block.text.trim();
                if trimmed.is_empty() || trimmed.len() < 10 || trimmed.len() > 220 {
                    continue;
                }

                let bbox = &block.bbox;
                // OODA-28: Use normalized coordinate system (Y=0 at top)
                let in_header = bbox.y1 <= header_threshold; // LOW Y = top of page
                let in_footer = bbox.y2 >= footer_threshold; // HIGH Y = bottom of page

                if in_header {
                    let key = normalize(trimmed);
                    if header_seen.insert(key.clone()) {
                        *header_counts.entry(key).or_insert(0) += 1;
                    }
                }

                if in_footer && trimmed.parse::<i32>().is_err() {
                    let key = normalize(trimmed);
                    if footer_seen.insert(key.clone()) {
                        *footer_counts.entry(key).or_insert(0) += 1;
                    }
                }
            }
        }

        // Text on 50%+ pages is running header/footer
        let threshold = (document.pages.len() / 2).max(3);
        let running_headers: HashSet<String> = header_counts
            .into_iter()
            .filter(|(_, count)| *count >= threshold)
            .map(|(k, _)| k)
            .collect();
        let running_footers: HashSet<String> = footer_counts
            .into_iter()
            .filter(|(_, count)| *count >= threshold)
            .map(|(k, _)| k)
            .collect();

        // Second pass: filter
        for page in &mut document.pages {
            let page_width = page.width;
            let page_height = page.height;

            let left_margin = page_width * 0.08;
            let right_margin = page_width * 0.05;
            // OODA-28: Use normalized coordinate thresholds (Y=0 at top)
            let header_threshold = page_height * 0.05; // Top 5% of page
            let footer_threshold = page_height * 0.95; // Bottom 5% of page
            let line_number_edge = page_width * 0.10;

            page.blocks.retain(|block| {
                if self.is_margin_content(
                    block,
                    page_width,
                    page_height,
                    left_margin,
                    right_margin,
                    header_threshold,
                    footer_threshold,
                    line_number_edge,
                ) {
                    return false;
                }

                let trimmed = block.text.trim();
                if trimmed.is_empty() {
                    return true;
                }

                let bbox = &block.bbox;
                // OODA-28: Use normalized coordinate system (Y=0 at top)
                let in_header = bbox.y1 <= header_threshold; // LOW Y = top of page
                let in_footer = bbox.y2 >= footer_threshold; // HIGH Y = bottom of page
                let key = normalize(trimmed);

                // OODA-28 FIX: Don't filter large blocks as running headers
                // WHY: Running headers are small (font ~9pt), titles are large (font ~14pt)
                // Use bbox height as proxy for font size: titles are typically > 12pt tall
                // This prevents filtering out the actual paper title on page 1
                let bbox_height = bbox.y2 - bbox.y1;
                let is_large_block = bbox_height > 12.0;

                if in_header && running_headers.contains(&key) && !is_large_block {
                    return false;
                }
                if in_footer && running_footers.contains(&key) {
                    return false;
                }

                true
            });
        }

        Ok(document)
    }

    fn name(&self) -> &str {
        "MarginFilterProcessor"
    }
}

// =============================================================================
// SectionNumberMergeProcessor
// =============================================================================

/// Merges standalone section numbers with their titles.
///
/// **Problem:** Some PDFs have "1." and "Introduction" as separate blocks.
///
/// **Solution:** Detect adjacent blocks on same Y-band and merge.
///
/// **Result:** "1. Introduction" as single block.
pub struct SectionNumberMergeProcessor;

impl SectionNumberMergeProcessor {
    pub fn new() -> Self {
        Self
    }

    fn is_section_number(text: &str) -> bool {
        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed.len() > 10 {
            return false;
        }
        let all_digit_or_dot = trimmed.chars().all(|c| c.is_ascii_digit() || c == '.');
        if !all_digit_or_dot {
            return false;
        }
        trimmed
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
            && trimmed.chars().any(|c| c.is_ascii_digit())
    }

    fn looks_like_section_title(text: &str) -> bool {
        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed.len() > 100 {
            return false;
        }

        let starts_uppercase = trimmed
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false);

        if !starts_uppercase {
            return false;
        }

        // OODA-32: Filter out person name patterns
        // WHY: Author names like "Alois Knoll" start with uppercase but are NOT section titles.
        // Section titles are: Introduction, Motivation, Background, Methods, etc.
        // Person names are: 2-3 capitalized words, no common section keywords.
        //
        // Heuristic: If text is 2-4 short capitalized words without section keywords, it's a name.
        let words: Vec<&str> = trimmed.split_whitespace().collect();

        // OODA-38: ALL-CAPS text is always a section title, never a person name.
        // WHY (First Principle): Person names use Title Case ("Alois Knoll"),
        // while section titles in academic papers use ALL CAPS ("DUAL-LEVEL RETRIEVAL").
        // Checking alpha chars for all-uppercase is more robust than keyword matching.
        let alpha_chars: Vec<char> = trimmed.chars().filter(|c| c.is_alphabetic()).collect();
        let is_all_caps = !alpha_chars.is_empty() && alpha_chars.iter().all(|c| c.is_uppercase());
        if is_all_caps {
            return true; // ALL CAPS = section title
        }

        // Check if it looks like a person name (2-4 short capitalized words)
        let looks_like_person_name = !words.is_empty()
            && words.len() <= 4
            && words.iter().all(|w| {
                let first_char = w.chars().next();
                // Each word starts with uppercase and is short (person name word)
                matches!(first_char, Some(c) if c.is_uppercase()) && w.len() <= 15
            })
            && trimmed.len() <= 40; // Total name is short

        if looks_like_person_name {
            // Check if it contains any section keyword - if so, it IS a section title
            let text_lower = trimmed.to_lowercase();
            let has_section_keyword = text_lower.contains("introduction")
                || text_lower.contains("motivation")
                || text_lower.contains("background")
                || text_lower.contains("method")
                || text_lower.contains("result")
                || text_lower.contains("conclusion")
                || text_lower.contains("discussion")
                || text_lower.contains("abstract")
                || text_lower.contains("related")
                || text_lower.contains("experiment")
                || text_lower.contains("evaluation")
                || text_lower.contains("overview")
                || text_lower.contains("objective")
                || text_lower.contains("problem")
                || text_lower.contains("approach")
                || text_lower.contains("system")
                || text_lower.contains("framework")
                || text_lower.contains("analysis")
                || text_lower.contains("implementation")
                || text_lower.contains("appendix")
                || text_lower.contains("reference")
                || text_lower.contains("acknowledgment")
                || text_lower.contains("preliminaries")
                || text_lower.contains("definition")
                || text_lower.contains("theorem")
                || text_lower.contains("proof")
                || text_lower.contains("algorithm")
                || text_lower.contains("data")
                || text_lower.contains("model")
                || text_lower.contains("training")
                || text_lower.contains("inference")
                || text_lower.contains("architecture")
                || text_lower.contains("network")
                || text_lower.contains("performance")
                || text_lower.contains("benchmark")
                || text_lower.contains("comparison")
                || text_lower.contains("limitation")
                || text_lower.contains("future")
                || text_lower.contains("work")
                || text_lower.contains("contribution")
                || text_lower.contains("setup")
                || text_lower.contains("setting")
                || text_lower.contains("detail")
                || text_lower.contains("application")
                || text_lower.contains("review")
                || text_lower.contains("survey")
                || text_lower.contains("statement");

            if !has_section_keyword {
                // It's a short capitalized phrase without section keywords = likely person name
                return false;
            }
        }

        true
    }
}

impl Default for SectionNumberMergeProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for SectionNumberMergeProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        for page in &mut document.pages {
            // Collect section numbers
            let mut section_numbers: Vec<(usize, String, f32, f32)> = Vec::new();

            for (idx, block) in page.blocks.iter().enumerate() {
                let text = block.text.trim();
                if Self::is_section_number(text) {
                    let y_center = (block.bbox.y1 + block.bbox.y2) / 2.0;
                    section_numbers.push((idx, text.to_string(), y_center, block.bbox.x1));
                }
            }

            // Match with titles
            // OODA-38: Two matching modes for section number + title merging:
            //
            // Mode A: SAME LINE — title is to the RIGHT of section number
            //   ┌─────────────────────────────────┐
            //   │  "1."  │  "Introduction"         │  ← same Y-band, title.x1 > sec.x1
            //   └─────────────────────────────────┘
            //
            // Mode B: NEXT LINE — title is BELOW the section number
            //   ┌──────────────┐
            //   │  "3.2"       │  ← section number alone
            //   ├──────────────┤
            //   │  "DUAL-LEVEL │  ← title on next line, similar X position
            //   │   RETRIEVAL" │
            //   └──────────────┘
            //
            // WHY: Academic PDFs often place section numbers and titles on separate lines.
            // The original code only handled Mode A (same-line, title to the right).
            let mut merge_map: std::collections::HashMap<usize, (usize, String)> =
                std::collections::HashMap::new();

            for (sec_idx, sec_text, sec_y, sec_x) in &section_numbers {
                // Track best matches separately: Mode A (same-line) takes priority
                let mut best_same_line: Option<(usize, String, f32)> = None;
                let mut best_next_line: Option<(usize, String, f32)> = None;

                for (title_idx, title_block) in page.blocks.iter().enumerate() {
                    if title_idx == *sec_idx {
                        continue;
                    }

                    let title_text = title_block.text.trim();
                    if !Self::looks_like_section_title(title_text) {
                        continue;
                    }

                    let title_y_center = (title_block.bbox.y1 + title_block.bbox.y2) / 2.0;
                    let y_gap = (sec_y - title_y_center).abs();
                    let merged_text = format!("{}. {}", sec_text.trim_end_matches('.'), title_text);

                    // Mode A: Same line — title to the right, tight Y tolerance
                    let is_same_line = y_gap < 25.0 && title_block.bbox.x1 > *sec_x;

                    // Mode B: Next line — title below, similar or same X position
                    // WHY: title_y_center > sec_y means title is below; X within ±20pt
                    let is_next_line = !is_same_line
                        && y_gap < 40.0
                        && title_y_center > *sec_y
                        && (title_block.bbox.x1 - sec_x).abs() < 20.0;

                    if is_same_line {
                        let better = best_same_line
                            .as_ref()
                            .map(|(_, _, best_y)| y_gap < *best_y)
                            .unwrap_or(true);
                        if better {
                            best_same_line = Some((title_idx, merged_text, y_gap));
                        }
                    } else if is_next_line {
                        let better = best_next_line
                            .as_ref()
                            .map(|(_, _, best_y)| y_gap < *best_y)
                            .unwrap_or(true);
                        if better {
                            best_next_line = Some((title_idx, merged_text, y_gap));
                        }
                    }
                }

                // Mode A always wins over Mode B
                let best_match = best_same_line.or(best_next_line);
                if let Some((title_idx, merged_text, _)) = best_match {
                    merge_map.insert(*sec_idx, (title_idx, merged_text));
                }
            }

            // Apply merges
            let mut skip_indices: std::collections::HashSet<usize> =
                std::collections::HashSet::new();
            let mut merged_blocks: Vec<Block> = Vec::new();

            for (idx, block) in page.blocks.iter().enumerate() {
                if skip_indices.contains(&idx) {
                    continue;
                }

                if let Some((title_idx, merged_text)) = merge_map.get(&idx) {
                    let title_block = &page.blocks[*title_idx];
                    let mut merged = block.clone();
                    merged.text = merged_text.clone();
                    merged.spans.extend(title_block.spans.clone());
                    merged.bbox.x2 = merged.bbox.x2.max(title_block.bbox.x2);
                    merged.bbox.y1 = merged.bbox.y1.min(title_block.bbox.y1);
                    merged.bbox.y2 = merged.bbox.y2.max(title_block.bbox.y2);
                    merged_blocks.push(merged);
                    skip_indices.insert(*title_idx);
                } else {
                    merged_blocks.push(block.clone());
                }
            }

            for (pos, block) in merged_blocks.iter_mut().enumerate() {
                block.position = pos;
            }

            page.blocks = merged_blocks;
        }

        Ok(document)
    }

    fn name(&self) -> &'static str {
        "SectionNumberMergeProcessor"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processors::test_helpers::create_test_document;

    #[test]
    fn test_layout_processor() {
        let processor = LayoutProcessor::new();
        let doc = create_test_document();
        let result = processor.process(doc).unwrap();
        assert!(!result.pages.is_empty());
    }

    #[test]
    fn test_block_merge_with_gap() {
        let processor = BlockMergeProcessor::new();
        let doc = create_test_document();
        let result = processor.process(doc).unwrap();

        // OODA-30: With correct gap calculation (b.y1 - a.y2 = 150 - 130 = 20pt),
        // blocks with 20pt gap WILL merge because it's within threshold (2.5x line spacing).
        // WHY: Adjacent paragraph blocks with small gaps should merge into one.
        // To test non-merging, use create_document_with_large_gap() instead.
        assert_eq!(result.pages[0].blocks.len(), 1);
    }

    #[test]
    fn test_block_no_merge_with_large_gap() {
        use crate::schema::{Block, BoundingBox, Document, Page};

        let processor = BlockMergeProcessor::new();

        let mut doc = Document::new();
        let mut page = Page::new(1, 612.0, 792.0);

        // Block 1 at Y=100-130, Block 2 at Y=300-330 (gap = 170pt)
        // WHY: Large gap (170pt) should prevent merging regardless of threshold
        page.add_block(Block::text(
            "First paragraph.",
            BoundingBox::new(72.0, 100.0, 540.0, 130.0),
        ));
        page.add_block(Block::text(
            "Second paragraph.",
            BoundingBox::new(72.0, 300.0, 540.0, 330.0),
        ));

        doc.add_page(page);

        let result = processor.process(doc).unwrap();

        // With 170pt gap, blocks should NOT merge
        assert_eq!(result.pages[0].blocks.len(), 2);
    }

    #[test]
    fn test_section_number_detection() {
        assert!(SectionNumberMergeProcessor::is_section_number("1."));
        assert!(SectionNumberMergeProcessor::is_section_number("2"));
        assert!(SectionNumberMergeProcessor::is_section_number("1.1."));
        assert!(!SectionNumberMergeProcessor::is_section_number(
            "Introduction"
        ));
        assert!(!SectionNumberMergeProcessor::is_section_number(""));
    }

    #[test]
    fn test_section_title_detection() {
        assert!(SectionNumberMergeProcessor::looks_like_section_title(
            "Introduction"
        ));
        assert!(SectionNumberMergeProcessor::looks_like_section_title(
            "Related Work"
        ));
        assert!(!SectionNumberMergeProcessor::looks_like_section_title(
            "lower case"
        ));
        assert!(!SectionNumberMergeProcessor::looks_like_section_title(""));
    }

    #[test]
    fn test_margin_filter_basic() {
        let processor = MarginFilterProcessor::new();
        let doc = create_test_document();
        let result = processor.process(doc).unwrap();
        // Should process without errors
        assert!(!result.pages.is_empty());
    }

    #[test]
    fn test_section_number_merge_adjacency() {
        let processor = SectionNumberMergeProcessor::new();
        let doc = create_test_document();
        let initial_block_count = doc.pages[0].blocks.len();
        let result = processor.process(doc).unwrap();
        // Should maintain block count (no merges in this simple test doc)
        assert_eq!(result.pages[0].blocks.len(), initial_block_count);
    }

    #[test]
    fn test_section_number_merge_same_line() {
        // OODA-38: Mode A — section number and title on same Y-band, title to the right
        use crate::schema::{Block, BoundingBox, Document, Page};

        let processor = SectionNumberMergeProcessor::new();
        let mut doc = Document::new();
        let mut page = Page::new(1, 612.0, 792.0);

        // "1." at left, "Introduction" to the right, same Y
        page.add_block(Block::text(
            "1.",
            BoundingBox::new(72.0, 100.0, 85.0, 115.0),
        ));
        page.add_block(Block::text(
            "Introduction",
            BoundingBox::new(90.0, 100.0, 250.0, 115.0),
        ));

        doc.add_page(page);
        let result = processor.process(doc).unwrap();

        assert_eq!(result.pages[0].blocks.len(), 1);
        assert_eq!(result.pages[0].blocks[0].text, "1. Introduction");
    }

    #[test]
    fn test_section_number_merge_next_line() {
        // OODA-38: Mode B — section number alone, title on the NEXT LINE below
        // WHY: Academic PDFs like the LightRAG paper have "3.2" alone on one line
        // and "DUAL-LEVEL RETRIEVAL PARADIGM" on the next line at similar X position.
        use crate::schema::{Block, BoundingBox, Document, Page};

        let processor = SectionNumberMergeProcessor::new();
        let mut doc = Document::new();
        let mut page = Page::new(1, 612.0, 792.0);

        // "3.2" at Y=200-215, title at Y=220-235 (below, gap ~15pt, similar X)
        page.add_block(Block::text(
            "3.2",
            BoundingBox::new(72.0, 200.0, 95.0, 215.0),
        ));
        page.add_block(Block::text(
            "DUAL-LEVEL RETRIEVAL PARADIGM",
            BoundingBox::new(72.0, 220.0, 350.0, 235.0),
        ));

        doc.add_page(page);
        let result = processor.process(doc).unwrap();

        assert_eq!(result.pages[0].blocks.len(), 1);
        assert_eq!(
            result.pages[0].blocks[0].text,
            "3.2. DUAL-LEVEL RETRIEVAL PARADIGM"
        );
    }

    #[test]
    fn test_section_number_no_merge_far_below() {
        // OODA-38: Title too far below (>40pt gap) should NOT merge
        use crate::schema::{Block, BoundingBox, Document, Page};

        let processor = SectionNumberMergeProcessor::new();
        let mut doc = Document::new();
        let mut page = Page::new(1, 612.0, 792.0);

        // "3.2" at Y=200-215, text at Y=260-275 (gap=50pt, too far)
        page.add_block(Block::text(
            "3.2",
            BoundingBox::new(72.0, 200.0, 95.0, 215.0),
        ));
        page.add_block(Block::text(
            "Some Paragraph Text",
            BoundingBox::new(72.0, 260.0, 350.0, 275.0),
        ));

        doc.add_page(page);
        let result = processor.process(doc).unwrap();

        // Should NOT merge — Y gap too large
        assert_eq!(result.pages[0].blocks.len(), 2);
    }

    #[test]
    fn test_layout_processor_default() {
        let processor = LayoutProcessor::default();
        assert_eq!(processor.name(), "LayoutProcessor");
    }

    #[test]
    fn test_block_merge_processor_default() {
        let processor = BlockMergeProcessor::default();
        assert_eq!(processor.name(), "BlockMergeProcessor");
    }

    #[test]
    fn test_margin_filter_processor_default() {
        let processor = MarginFilterProcessor::default();
        assert_eq!(processor.name(), "MarginFilterProcessor");
    }

    #[test]
    fn test_section_number_merge_processor_default() {
        let processor = SectionNumberMergeProcessor::default();
        assert_eq!(processor.name(), "SectionNumberMergeProcessor");
    }
}
