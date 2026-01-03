//! Layout analysis and margin filtering processors.
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
use crate::schema::{Block, BlockType, Document};
use crate::Result;

use super::stats::DocumentStats;
use super::Processor;

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
            // Skip if page already has columns set (backend handled it)
            if !page.columns.is_empty() {
                tracing::debug!(
                    "Page {} already has {} columns set, skipping layout reanalysis",
                    page.number,
                    page.columns.len()
                );
                continue;
            }

            let layout = self.analyzer.analyze(&page.blocks, page.width, page.height);

            // Check if detected columns look like a table structure
            let bboxes: Vec<crate::schema::BoundingBox> =
                page.blocks.iter().map(|b| b.bbox).collect();
            let is_table = self
                .analyzer
                .column_detector()
                .is_likely_table(&bboxes, &layout.columns);

            if is_table {
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
    fn should_merge(&self, a: &Block, b: &Block, stats: &DocumentStats) -> bool {
        // Only merge text/header/list blocks
        if !matches!(
            a.block_type,
            BlockType::Text | BlockType::SectionHeader | BlockType::ListItem
        ) || !matches!(
            b.block_type,
            BlockType::Text | BlockType::SectionHeader | BlockType::ListItem
        ) {
            return false;
        }

        // Types must match
        if a.block_type != b.block_type {
            return false;
        }

        // Don't merge if b looks like a new list item
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

        // Check style compatibility
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
        if vertical_gap > vertical_threshold {
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
            return false;
        }

        margin_diff <= max_margin
    }

    fn merge_page_blocks(&self, blocks: Vec<Block>, stats: &DocumentStats) -> Vec<Block> {
        if blocks.len() < 2 {
            return blocks;
        }

        let mut merged = Vec::new();
        let mut current: Option<Block> = None;

        for block in blocks {
            if let Some(mut cur) = current.take() {
                if self.should_merge(&cur, &block, stats) {
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

        for page in &mut document.pages {
            let blocks = std::mem::take(&mut page.blocks);
            page.blocks = self.merge_page_blocks(blocks, &stats);
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
            if tokens.len() >= 6
                && tokens.iter().all(|t| t.chars().all(|c| c.is_ascii_digit()))
            {
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
            && text.chars().all(|c| c.is_ascii_digit() || c.is_ascii_uppercase())
            && (bbox.x1 < line_number_edge || bbox.x1 > page_width - line_number_edge) {
                return true;
            }

        // Filter footer page numbers
        let in_footer = bbox.y1 <= bottom_margin;
        if in_footer && trimmed.parse::<i32>().is_ok() {
            return true;
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
            text.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
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
        trimmed.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
            && trimmed.chars().any(|c| c.is_ascii_digit())
    }

    fn looks_like_section_title(text: &str) -> bool {
        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed.len() > 100 {
            return false;
        }
        trimmed.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
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

                    if y_gap < 25.0 && title_block.bbox.x1 > *sec_x
                        && Self::looks_like_section_title(title_text) {
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
        assert!(!SectionNumberMergeProcessor::is_section_number("Introduction"));
        assert!(!SectionNumberMergeProcessor::is_section_number(""));
    }

    #[test]
    fn test_section_title_detection() {
        assert!(SectionNumberMergeProcessor::looks_like_section_title("Introduction"));
        assert!(SectionNumberMergeProcessor::looks_like_section_title("Related Work"));
        assert!(!SectionNumberMergeProcessor::looks_like_section_title("lower case"));
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
        let result = processor.process(doc).unwrap();
        // Should maintain block count (no merges in this simple test doc)
        assert_eq!(result.pages[0].blocks.len(), doc.pages[0].blocks.len());
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
