//! List item hierarchy detection.
//!
//! Assigns indentation levels to list items based on their x0 (left edge)
//! coordinates, enabling proper nested list rendering in Markdown.
//!
//! ## Algorithm (ported from pymupdf4llm document_layout.py:97-151)
//!
//! ```text
//! 1. Create segments of contiguous list items (break on non-list)
//! 2. Within each segment, sort by x0 coordinate
//! 3. Assign levels: same x0 = same level, deeper x0 = deeper level
//! ```
//!
//! ## Example
//!
//! ```text
//! x0=20: • Top 1      → level 0 (no indent)
//! x0=36: • Nested 1   → level 1 (2-space indent)
//! x0=36: • Nested 2   → level 1
//! x0=20: • Top 2      → level 0
//! ```

use super::pymupdf_structs::{Block, BlockType};
use std::collections::HashMap;

/// Indentation threshold in points.
/// If a list item's x0 is more than this distance right of the segment's
/// minimum x0, it's considered a deeper level.
/// REF: pymupdf4llm uses 10pt threshold (document_layout.py:140)
const INDENT_THRESHOLD: f32 = 10.0;

/// Compute list item hierarchy levels for a sequence of blocks.
///
/// Returns a map from block index to nesting level (0-based).
/// Non-list-item blocks are not included in the map.
///
/// The algorithm detects contiguous runs of list items and assigns
/// levels based on the x0 (left edge) coordinate of each block.
pub fn compute_list_levels(blocks: &[Block]) -> HashMap<usize, u8> {
    let mut levels: HashMap<usize, u8> = HashMap::new();

    // Step 1: Find contiguous segments of list items
    let segments = find_list_segments(blocks);

    // Step 2: Assign levels within each segment
    for segment in segments {
        assign_levels_to_segment(blocks, &segment, &mut levels);
    }

    levels
}

/// Find contiguous segments of list item blocks.
///
/// A segment ends when:
/// - A non-list-item block is encountered
/// - A column break is detected (block x0 > previous block x1, or y is above previous)
fn find_list_segments(blocks: &[Block]) -> Vec<Vec<usize>> {
    let mut segments: Vec<Vec<usize>> = Vec::new();
    let mut current_segment: Vec<usize> = Vec::new();

    for (i, block) in blocks.iter().enumerate() {
        if block.block_type != BlockType::ListItem {
            if !current_segment.is_empty() {
                segments.push(current_segment);
                current_segment = Vec::new();
            }
            continue;
        }

        // Check for column break
        if let Some(&prev_idx) = current_segment.last() {
            let prev = &blocks[prev_idx];
            let breaks_column = block.x0 > prev.x1 + 50.0 || block.y1 > prev.y0 + 20.0; // Different column or above
            if breaks_column {
                segments.push(current_segment);
                current_segment = Vec::new();
            }
        }

        current_segment.push(i);
    }

    if !current_segment.is_empty() {
        segments.push(current_segment);
    }

    segments
}

/// Assign hierarchy levels within a single contiguous segment.
///
/// Algorithm:
/// 1. Find the minimum x0 in the segment (leftmost item)
/// 2. Compute distinct x0 buckets (within INDENT_THRESHOLD)
/// 3. Map each x0 bucket to a level (0, 1, 2, ...)
fn assign_levels_to_segment(blocks: &[Block], segment: &[usize], levels: &mut HashMap<usize, u8>) {
    if segment.is_empty() {
        return;
    }

    // Collect x0 values for this segment
    let mut x0_values: Vec<(usize, f32)> =
        segment.iter().map(|&idx| (idx, blocks[idx].x0)).collect();

    // Sort by x0 to find distinct levels
    x0_values.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    // Build level buckets: group x0 values within INDENT_THRESHOLD
    let mut buckets: Vec<f32> = Vec::new();
    for &(_, x0) in &x0_values {
        if buckets.is_empty() || (x0 - buckets.last().unwrap()).abs() > INDENT_THRESHOLD {
            buckets.push(x0);
        }
    }

    // Assign levels based on which bucket each block's x0 falls into
    for &idx in segment {
        let x0 = blocks[idx].x0;
        let level = buckets
            .iter()
            .position(|&bucket_x0| (x0 - bucket_x0).abs() <= INDENT_THRESHOLD)
            .unwrap_or(0) as u8;
        levels.insert(idx, level);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::pymupdf_structs::{Block, BlockType, Line, Span};

    fn make_list_block(x0: f32, y0: f32) -> Block {
        Block {
            lines: vec![Line::from_span(Span {
                text: "- item".to_string(),
                x0,
                y0,
                x1: x0 + 100.0,
                y1: y0 + 12.0,
                font_size: 12.0,
                font_name: None,
                page_num: 0,
                font_is_bold: None,
                font_is_italic: None,
                font_is_monospace: None,
            })],
            x0,
            y0,
            x1: x0 + 100.0,
            y1: y0 + 12.0,
            page_num: 0,
            block_type: BlockType::ListItem,
        }
    }

    fn make_paragraph_block(x0: f32, y0: f32) -> Block {
        Block {
            lines: vec![Line::from_span(Span {
                text: "paragraph".to_string(),
                x0,
                y0,
                x1: x0 + 100.0,
                y1: y0 + 12.0,
                font_size: 12.0,
                font_name: None,
                page_num: 0,
                font_is_bold: None,
                font_is_italic: None,
                font_is_monospace: None,
            })],
            x0,
            y0,
            x1: x0 + 100.0,
            y1: y0 + 12.0,
            page_num: 0,
            block_type: BlockType::Paragraph,
        }
    }

    #[test]
    fn test_flat_list() {
        let blocks = vec![
            make_list_block(20.0, 100.0),
            make_list_block(20.0, 85.0),
            make_list_block(20.0, 70.0),
        ];

        let levels = compute_list_levels(&blocks);
        assert_eq!(levels[&0], 0);
        assert_eq!(levels[&1], 0);
        assert_eq!(levels[&2], 0);
    }

    #[test]
    fn test_nested_list() {
        let blocks = vec![
            make_list_block(20.0, 100.0), // Top level
            make_list_block(36.0, 85.0),  // Nested (x0 increased by 16 > 10)
            make_list_block(36.0, 70.0),  // Still nested
            make_list_block(20.0, 55.0),  // Back to top
        ];

        let levels = compute_list_levels(&blocks);
        assert_eq!(levels[&0], 0);
        assert_eq!(levels[&1], 1);
        assert_eq!(levels[&2], 1);
        assert_eq!(levels[&3], 0);
    }

    #[test]
    fn test_three_level_nesting() {
        let blocks = vec![
            make_list_block(20.0, 100.0), // Level 0
            make_list_block(36.0, 85.0),  // Level 1
            make_list_block(52.0, 70.0),  // Level 2
            make_list_block(36.0, 55.0),  // Level 1
            make_list_block(20.0, 40.0),  // Level 0
        ];

        let levels = compute_list_levels(&blocks);
        assert_eq!(levels[&0], 0);
        assert_eq!(levels[&1], 1);
        assert_eq!(levels[&2], 2);
        assert_eq!(levels[&3], 1);
        assert_eq!(levels[&4], 0);
    }

    #[test]
    fn test_list_broken_by_paragraph() {
        let blocks = vec![
            make_list_block(20.0, 100.0),     // Segment 1
            make_list_block(36.0, 85.0),      // Segment 1, nested
            make_paragraph_block(20.0, 70.0), // Breaks segment
            make_list_block(20.0, 55.0),      // Segment 2
            make_list_block(36.0, 40.0),      // Segment 2, nested
        ];

        let levels = compute_list_levels(&blocks);
        // First segment
        assert_eq!(levels[&0], 0);
        assert_eq!(levels[&1], 1);
        // Paragraph not in map
        assert!(!levels.contains_key(&2));
        // Second segment (independent)
        assert_eq!(levels[&3], 0);
        assert_eq!(levels[&4], 1);
    }

    #[test]
    fn test_no_list_items() {
        let blocks = vec![
            make_paragraph_block(20.0, 100.0),
            make_paragraph_block(20.0, 85.0),
        ];

        let levels = compute_list_levels(&blocks);
        assert!(levels.is_empty());
    }

    #[test]
    fn test_single_list_item() {
        let blocks = vec![make_list_block(20.0, 100.0)];

        let levels = compute_list_levels(&blocks);
        assert_eq!(levels[&0], 0);
    }

    #[test]
    fn test_similar_x0_same_level() {
        // Items with x0 within INDENT_THRESHOLD should be same level
        let blocks = vec![
            make_list_block(20.0, 100.0),
            make_list_block(25.0, 85.0), // Only 5pt difference, same level
            make_list_block(22.0, 70.0), // Only 2pt difference, same level
        ];

        let levels = compute_list_levels(&blocks);
        assert_eq!(levels[&0], 0);
        assert_eq!(levels[&1], 0);
        assert_eq!(levels[&2], 0);
    }
}
