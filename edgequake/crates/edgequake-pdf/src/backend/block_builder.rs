//! Block builder: converts text lines to semantic blocks.
//!
//! # WHY a separate block builder?
//!
//! After grouping text elements into lines, we need to convert them into
//! semantic blocks with:
//! - Block type detection (text, header, running header)
//! - Duplicate block removal (OCR layer overlap)
//! - Text occurrence tracking for running header detection
//! - Span preservation for style information
//!
//! This module handles the final stage of text extraction before
//! the document is assembled.

use std::collections::{BTreeMap, HashMap};
use tracing::debug;

use super::elements::TextElement;
use super::text_grouping::{MergedLine, TextGrouper};
use crate::schema::{Block, BlockId, BlockType, BoundingBox};

/// WHY: UTF-8 safe string truncation.
///
/// Direct byte slicing like `&s[..80]` can panic if byte 80 falls in the middle
/// of a multi-byte character (e.g., box-drawing '─' is 3 bytes). This function
/// finds the nearest valid char boundary at or before `max_bytes`.
///
/// OODA-04: Fix byte index panics in block_builder.rs (block text logging).
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

/// Builder for converting lines into semantic blocks.
pub struct BlockBuilder {
    text_grouper: TextGrouper,
}

impl BlockBuilder {
    pub fn new() -> Self {
        Self {
            text_grouper: TextGrouper::new(),
        }
    }

    /// Convert lines to blocks with type detection.
    ///
    /// # Arguments
    /// * `lines` - Groups of text elements representing logical lines
    /// * `page_width` - Page width for bounding box calculation
    ///
    /// # Returns
    /// Vector of blocks with type detection and duplicate removal
    pub fn build(&self, lines: Vec<Vec<TextElement>>, page_width: f32) -> Vec<Block> {
        let mut blocks = Vec::new();

        if lines.is_empty() {
            return blocks;
        }

        // Debug: Log first 10 lines being processed
        debug!("BlockBuilder: Converting {} lines to blocks", lines.len());
        for (i, line) in lines.iter().take(10).enumerate() {
            let text: String = line
                .iter()
                .map(|e| e.text.as_str())
                .collect::<Vec<_>>()
                .join("");
            let y = line.first().map(|e| e.y).unwrap_or(0.0);
            let x = line.first().map(|e| e.x).unwrap_or(0.0);
            let preview: String = text.chars().take(40).collect();
            debug!(
                "  Block input line {}: Y={:.1} X={:.1} '{}'",
                i, y, x, preview
            );
        }

        // Calculate body font size (most common)
        let _body_size = self.calculate_body_font_size(&lines);

        // Track text occurrences for running header detection
        let text_occurrences = self.build_occurrence_map(&lines);
        let line_texts: Vec<MergedLine> = lines
            .iter()
            .map(|line| self.text_grouper.merge_line(line))
            .collect();

        let mut last_bbox: Option<BoundingBox> = None;
        let mut last_text: String = String::new();

        for (idx, line) in lines.iter().enumerate() {
            if line.is_empty() {
                continue;
            }

            let merged = &line_texts[idx];
            let text = merged.text.trim();
            if text.is_empty() {
                continue;
            }

            // Get bounding box
            let bbox = self.calculate_line_bbox(line, page_width, merged.avg_font_size);

            // Deduplication: Check if this block is a duplicate of the previous one
            if self.is_duplicate_block(&bbox, text, &last_bbox, &last_text) {
                continue;
            }

            last_bbox = Some(bbox);
            last_text = text.to_string();

            // Detect block type
            let block_type = self.detect_block_type(text, &text_occurrences);

            // Build spans with bounding box
            let spans = merged
                .spans
                .iter()
                .cloned()
                .map(|mut s| {
                    s.bbox = Some(bbox);
                    s
                })
                .collect::<Vec<_>>();

            let block = Block {
                id: BlockId::with_indices(0, blocks.len()),
                block_type,
                text: text.to_string(),
                bbox,
                page: 0,
                position: blocks.len(),
                level: None,
                spans,
                ..Default::default()
            };

            // Log blocks with wide X-ranges (potential cross-column spans)
            let x_range = bbox.x2 - bbox.x1;
            if x_range > 200.0 {
                tracing::info!(
                    "BLOCK-XRANGE: pos={} bbox=[{:.1},{:.1}] range={:.1} text='{}'",
                    blocks.len(),
                    bbox.x1,
                    bbox.x2,
                    x_range,
                    safe_truncate(text, 80)
                );
            }

            blocks.push(block.clone());
        }

        blocks
    }

    /// Calculate the most common (body) font size.
    fn calculate_body_font_size(&self, lines: &[Vec<TextElement>]) -> f32 {
        let mut font_size_counts: BTreeMap<i32, usize> = BTreeMap::new();
        for line in lines {
            for elem in line {
                let key = (elem.font_size * 10.0) as i32;
                *font_size_counts.entry(key).or_insert(0) += 1;
            }
        }
        font_size_counts
            .iter()
            .max_by_key(|&(_, count)| count)
            .map(|(&size, _)| size as f32 / 10.0)
            .unwrap_or(12.0)
    }

    /// Build a map of text occurrences for running header detection.
    fn build_occurrence_map(&self, lines: &[Vec<TextElement>]) -> HashMap<String, usize> {
        let mut text_occurrences: HashMap<String, usize> = HashMap::new();
        let line_texts: Vec<MergedLine> = lines
            .iter()
            .map(|line| self.text_grouper.merge_line(line))
            .collect();

        for merged in &line_texts {
            let normalized = merged.text.trim().to_lowercase();
            if !normalized.is_empty() && normalized.len() < 100 {
                *text_occurrences.entry(normalized).or_insert(0) += 1;
            }
        }
        text_occurrences
    }

    /// Calculate bounding box for a line.
    ///
    /// # OODA-20 FIX: Compute proper x2 including text width
    ///
    /// Previously, max_x was computed as max(e.x) which is the left edge of the
    /// rightmost text element. This resulted in zero-width bboxes when all elements
    /// have the same x position (common in single-column PDFs).
    ///
    /// Now we compute max_x as max(e.x + estimated_width) to get the actual right edge.
    fn calculate_line_bbox(
        &self,
        line: &[TextElement],
        page_width: f32,
        avg_font_size: f32,
    ) -> BoundingBox {
        let min_x = line
            .iter()
            .map(|e| e.x)
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        // OODA-20: Compute right edge as x + estimated text width
        // Average char width is ~55% of font size for proportional fonts
        let max_x = line
            .iter()
            .map(|e| {
                let char_count = e.text.chars().count() as f32;
                let estimated_width = char_count * e.font_size * 0.55;
                e.x + estimated_width
            })
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(page_width);

        let y = line.first().map(|e| e.y).unwrap_or(0.0);

        BoundingBox::new(min_x, y, max_x, y + avg_font_size)
    }

    /// Check if a block is a duplicate of the previous one.
    ///
    /// Handles OCR layers that overlap visible text.
    fn is_duplicate_block(
        &self,
        bbox: &BoundingBox,
        text: &str,
        last_bbox: &Option<BoundingBox>,
        last_text: &str,
    ) -> bool {
        if let Some(prev_bbox) = last_bbox {
            // Check vertical overlap (lines are sorted by Y, so duplicates should be adjacent)
            let overlap_y = prev_bbox.y2.min(bbox.y2) - prev_bbox.y1.max(bbox.y1);
            let min_h = (prev_bbox.y2 - prev_bbox.y1).min(bbox.y2 - bbox.y1);

            if overlap_y > min_h * 0.5 {
                // Significant vertical overlap (>50%). Check text similarity.
                if text == last_text
                    || (text.len() > 5 && (text.contains(last_text) || last_text.contains(text)))
                {
                    return true;
                }
            }
        }
        false
    }

    /// Detect block type based on text content.
    fn detect_block_type(
        &self,
        text: &str,
        text_occurrences: &HashMap<String, usize>,
    ) -> BlockType {
        let normalized = text.to_lowercase();
        let trimmed = text.trim();

        // WHY: Figure/Table captions are critical content that must NEVER be classified as
        // PageHeader, even if they appear multiple times. The pattern "Figure N." or "Table N."
        // at the start of text definitively identifies a caption. This protection ensures
        // captions like "Figure 4. Cam×Time dataset..." are not filtered out in markdown.
        let is_figure_or_table_caption = {
            let starts_with_figure_pattern = trimmed.starts_with("Figure ")
                && trimmed.len() > 7
                && trimmed.chars().nth(7).map_or(false, |c| c.is_ascii_digit());
            let starts_with_table_pattern = trimmed.starts_with("Table ")
                && trimmed.len() > 6
                && trimmed.chars().nth(6).map_or(false, |c| c.is_ascii_digit());
            starts_with_figure_pattern || starts_with_table_pattern
        };

        // Short-circuit: captions are always Text, never PageHeader
        if is_figure_or_table_caption {
            return BlockType::Text;
        }

        let is_running_header = text_occurrences.get(&normalized).copied().unwrap_or(0) >= 3;

        if is_running_header {
            BlockType::PageHeader
        } else {
            BlockType::Text
        }
    }

    /// Calculate header level based on font size ratio.
    #[allow(dead_code)]
    pub fn calculate_header_level(&self, font_size: f32, body_size: f32) -> u8 {
        let ratio = font_size / body_size;
        if ratio >= 2.0 {
            1
        } else if ratio >= 1.5 {
            2
        } else if ratio >= 1.3 {
            3
        } else {
            4
        }
    }
}

impl Default for BlockBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_element(text: &str, x: f32, y: f32) -> TextElement {
        let width = text.chars().count() as f32 * 12.0 * 0.55;
        TextElement {
            text: text.to_string(),
            x,
            y,
            width,
            font_size: 12.0,
            font_name: "Times-Roman".to_string(),
            is_bold: false,
            is_italic: false,
            is_rotated: false,
        }
    }

    #[test]
    fn test_block_builder_creation() {
        let builder = BlockBuilder::new();
        let blocks = builder.build(vec![], 612.0);
        assert!(blocks.is_empty());
    }

    #[test]
    fn test_block_builder_single_line() {
        let builder = BlockBuilder::new();
        let lines = vec![vec![make_element("Hello World", 72.0, 700.0)]];
        let blocks = builder.build(lines, 612.0);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].text, "Hello World");
    }

    #[test]
    fn test_running_header_detection() {
        let builder = BlockBuilder::new();
        // Same text appearing 3+ times should be detected as running header
        let lines = vec![
            vec![make_element("Page Header", 72.0, 750.0)],
            vec![make_element("Content", 72.0, 700.0)],
            vec![make_element("Page Header", 72.0, 650.0)],
            vec![make_element("More Content", 72.0, 600.0)],
            vec![make_element("Page Header", 72.0, 550.0)],
        ];
        let blocks = builder.build(lines, 612.0);

        // Check that "Page Header" blocks are marked as PageHeader
        let header_blocks: Vec<_> = blocks
            .iter()
            .filter(|b| b.block_type == BlockType::PageHeader)
            .collect();
        assert_eq!(header_blocks.len(), 3, "Should detect 3 running headers");
    }

    #[test]
    fn test_header_level_calculation() {
        let builder = BlockBuilder::new();
        assert_eq!(builder.calculate_header_level(24.0, 12.0), 1); // 2x ratio
        assert_eq!(builder.calculate_header_level(18.0, 12.0), 2); // 1.5x ratio
        assert_eq!(builder.calculate_header_level(16.0, 12.0), 3); // 1.33x ratio
        assert_eq!(builder.calculate_header_level(13.0, 12.0), 4); // 1.08x ratio
    }

    #[test]
    fn test_body_font_size_calculation() {
        let builder = BlockBuilder::new();
        let lines = vec![
            vec![make_element("Small", 72.0, 700.0)],
            vec![make_element("Normal 1", 72.0, 680.0)],
            vec![make_element("Normal 2", 72.0, 660.0)],
            vec![make_element("Normal 3", 72.0, 640.0)],
        ];
        let body_size = builder.calculate_body_font_size(&lines);
        assert_eq!(body_size, 12.0); // All elements have font_size 12.0
    }
}
