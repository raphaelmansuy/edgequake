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
            // If page already has columns set by backend, still need to sort by reading order
            if !page.columns.is_empty() {
                tracing::info!(
                    "LAYOUT: Page {} has {} columns from backend, sorting blocks by reading order",
                    page.number,
                    page.columns.len()
                );
                // Sort blocks by reading order using the pre-detected columns
                self.analyzer
                    .sort_by_reading_order(&mut page.blocks, &page.columns);
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

        // OODA-19: Don't merge if a is an arxiv watermark or footnote marker
        // WHY: These are margin annotations that should remain separate blocks
        let trimmed_a = a.text.trim();
        let trimmed_a_lower = trimmed_a.to_lowercase();
        let a_is_arxiv = trimmed_a_lower.starts_with("arxiv:");
        let a_is_footnote = trimmed_a.starts_with('⋆')
            || trimmed_a.starts_with('†')
            || trimmed_a.starts_with('‡')
            || trimmed_a.starts_with('§')
            || trimmed_a.starts_with('¶');

        if a_is_arxiv || a_is_footnote {
            tracing::debug!(
                "BlockMerge: skip - a is arxiv/footnote (arxiv={}, fn={}, text='{}')",
                a_is_arxiv,
                a_is_footnote,
                safe_truncate(trimmed_a, 30)
            );
            return false;
        }

        // Don't merge if b looks like a new list item
        // WHY: Each list item should be a separate block for proper rendering
        let trimmed_b = b.text.trim();
        let trimmed_b_lower = trimmed_b.to_lowercase();

        // OODA-12: Add academic reference detection [N] pattern
        // WHY: arXiv papers have 30-60 references like "[1] Author..."
        // These must NOT be merged with preceding blocks
        let is_academic_ref = trimmed_b.len() > 2
            && trimmed_b.starts_with('[')
            && trimmed_b
                .chars()
                .skip(1)
                .take_while(|c| c.is_ascii_digit())
                .count()
                >= 1
            && trimmed_b.contains(']');

        // OODA-19: Detect arxiv identifier watermarks
        // WHY: arXiv papers have a watermark like "arXiv:2510.09244v1 [cs.AI] 10 Oct 2025"
        // These should NOT be merged with body text - they are margin annotations
        let is_arxiv_watermark = trimmed_b_lower.starts_with("arxiv:");

        // OODA-19: Detect footnote markers (⋆, †, *, ‡, §, ¶)
        // WHY: Footnotes start with special symbols and should be separate blocks
        let is_footnote_marker = trimmed_b.starts_with('⋆')
            || trimmed_b.starts_with('†')
            || trimmed_b.starts_with('‡')
            || trimmed_b.starts_with('§')
            || trimmed_b.starts_with('¶');

        if trimmed_b.starts_with("- ")
            || trimmed_b.starts_with("* ")
            || trimmed_b.starts_with("• ")
            || is_academic_ref
            || is_arxiv_watermark
            || is_footnote_marker
            || (trimmed_b.len() > 2
                && trimmed_b.chars().next().unwrap().is_ascii_digit()
                && trimmed_b.contains(". "))
        {
            tracing::debug!(
                "BlockMerge: skip - b is special (ref={}, arxiv={}, footnote={}, text='{}')",
                is_academic_ref,
                is_arxiv_watermark,
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

        // Check vertical proximity (PDF Y: higher = up)
        let vertical_gap = (a.bbox.y1 - b.bbox.y2).abs();
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

        // Horizontal alignment
        let margin_diff = (a.bbox.x1 - b.bbox.x1).abs();
        let max_margin = if a.block_type == BlockType::SectionHeader {
            stats.column_alignment_tolerance * 2.5
        } else {
            stats.column_alignment_tolerance
        };

        // Column separation: blocks in different columns shouldn't merge
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
                "BlockMerge: REJECT - margin {} > {}",
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
    /// Returns the index of the column that contains the block's center point.
    /// If no column contains the block, returns the closest column.
    fn get_block_column(&self, block: &Block, columns: &[BoundingBox]) -> usize {
        let center = block.bbox.center();

        // Find column containing the block's center
        for (idx, col) in columns.iter().enumerate() {
            if col.contains_point(&center) {
                return idx;
            }
        }

        // Fallback: find closest column by X-coordinate
        columns
            .iter()
            .enumerate()
            .min_by_key(|(_, col): &(usize, &BoundingBox)| {
                let col_center = col.center().x;
                ((col_center - center.x).abs() * 1000.0) as i32
            })
            .map(|(idx, _)| idx)
            .unwrap_or(0)
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
        tracing::info!(
            "BlockMerge: Processing {} blocks with {} columns",
            blocks.len(),
            columns.len()
        );

        // Log column bounding boxes
        for (i, col) in columns.iter().enumerate() {
            tracing::info!(
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
            tracing::info!(
                "BLOCKMERGE-START: {} blocks total, {} columns",
                blocks.len(),
                columns.len()
            );
            for (idx, block) in blocks.iter().enumerate() {
                if block.text.contains("disentangles")
                    || block.text.contains("dering")
                    || block.text.contains("independently")
                {
                    tracing::info!(
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
                tracing::info!(
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
                        tracing::info!(
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

            // OODA-13 DEBUG: Count ref blocks before merge
            let refs_before = page
                .blocks
                .iter()
                .filter(|b| {
                    let t = b.text.trim();
                    t.starts_with('[')
                        && t.chars()
                            .nth(1)
                            .map(|c| c.is_ascii_digit())
                            .unwrap_or(false)
                })
                .count();

            let blocks = std::mem::take(&mut page.blocks);
            let columns = &page.columns; // Capture columns before moving blocks
            page.blocks = self.merge_page_blocks(blocks, &stats, columns);

            // OODA-13 DEBUG: Count ref blocks after merge
            let refs_after = page
                .blocks
                .iter()
                .filter(|b| {
                    let t = b.text.trim();
                    t.starts_with('[')
                        && t.chars()
                            .nth(1)
                            .map(|c| c.is_ascii_digit())
                            .unwrap_or(false)
                })
                .count();

            let block_count_after = page.blocks.len();
            tracing::debug!(
                "BlockMergeProcessor: page {} blocks {} -> {} (columns={})",
                page.number,
                block_count_before,
                block_count_after,
                columns.len()
            );

            if refs_before > 0 || refs_after > 0 {
                eprintln!(
                    "BMP-PAGE-{}: refs {} -> {} (blocks {} -> {})",
                    page.number, refs_before, refs_after, block_count_before, block_count_after
                );
            }

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

    fn is_margin_content(
        &self,
        block: &Block,
        page_width: f32,
        _page_height: f32,
        left_margin: f32,
        right_margin: f32,
        _top_margin: f32,
        bottom_margin: f32,
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

        // Filter footer page numbers (strict 5% bottom margin)
        let in_footer = bbox.y1 <= bottom_margin;
        if in_footer && trimmed.parse::<i32>().is_ok() {
            return true;
        }

        // Extended page number detection - bottom 12% of page
        // WHY: Pandoc and other tools place page numbers at varying heights.
        // This catches standalone page numbers that aren't in the strict margin.
        let page_height = _page_height;
        let extended_footer = bbox.y1 <= page_height * 0.12;
        if extended_footer {
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
            let top_margin = page_height * 0.05;
            let bottom_margin = page_height * 0.05;

            let mut header_seen: HashSet<String> = HashSet::new();
            let mut footer_seen: HashSet<String> = HashSet::new();

            for block in &page.blocks {
                let trimmed = block.text.trim();
                if trimmed.is_empty() || trimmed.len() < 10 || trimmed.len() > 220 {
                    continue;
                }

                let bbox = &block.bbox;
                let in_header = bbox.y2 >= page_height - top_margin;
                let in_footer = bbox.y1 <= bottom_margin;

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
            let top_margin = page_height * 0.05;
            let bottom_margin = page_height * 0.05;
            let line_number_edge = page_width * 0.10;

            page.blocks.retain(|block| {
                if self.is_margin_content(
                    block,
                    page_width,
                    page_height,
                    left_margin,
                    right_margin,
                    top_margin,
                    bottom_margin,
                    line_number_edge,
                ) {
                    return false;
                }

                let trimmed = block.text.trim();
                if trimmed.is_empty() {
                    return true;
                }

                let bbox = &block.bbox;
                let in_header = bbox.y2 >= page_height - top_margin;
                let in_footer = bbox.y1 <= bottom_margin;
                let key = normalize(trimmed);

                if in_header && running_headers.contains(&key) {
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
        trimmed
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
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
            let mut merge_map: std::collections::HashMap<usize, (usize, String)> =
                std::collections::HashMap::new();

            for (sec_idx, sec_text, sec_y, sec_x) in &section_numbers {
                for (title_idx, title_block) in page.blocks.iter().enumerate() {
                    if title_idx == *sec_idx {
                        continue;
                    }

                    let title_text = title_block.text.trim();
                    let title_y_center = (title_block.bbox.y1 + title_block.bbox.y2) / 2.0;
                    let y_gap = (sec_y - title_y_center).abs();

                    if y_gap < 25.0
                        && title_block.bbox.x1 > *sec_x
                        && Self::looks_like_section_title(title_text)
                    {
                        let merged_text =
                            format!("{}. {}", sec_text.trim_end_matches('.'), title_text);
                        merge_map.insert(*sec_idx, (title_idx, merged_text));
                        break;
                    }
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

        // With 20px gap, blocks shouldn't merge
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
