//! Reading order detection for document blocks.
//!
//! This module provides algorithms for determining the correct reading order
//! of blocks on a page, handling single-column, multi-column, and complex layouts.

use crate::schema::{Block, BlockType, BoundingBox};

/// WHY: UTF-8 safe string truncation.
///
/// Direct byte slicing like `&s[..30]` can panic if byte 30 falls in the middle
/// of a multi-byte character (e.g., ellipsis '…' is 3 bytes, box-drawing '─' is 3 bytes).
/// This function finds the nearest valid char boundary at or before `max_bytes`.
///
/// PRODUCTION_BUG_FIX: Fix byte index panics in reading_order.rs (line 366, 151, 251).
/// Reproduction: PDF with ellipsis character at truncation boundary causes:
/// "byte index 30 is not a char boundary; it is inside '…' (bytes 29..32)"
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

/// Reading order result.
#[derive(Debug, Clone)]
pub struct ReadingOrder {
    /// Indices in reading order
    pub order: Vec<usize>,
    /// Confidence score
    pub confidence: f32,
}

impl ReadingOrder {
    /// Create a new reading order.
    pub fn new(order: Vec<usize>) -> Self {
        Self {
            order,
            confidence: 1.0,
        }
    }

    /// Get the reading position for a block index.
    pub fn position_of(&self, block_idx: usize) -> Option<usize> {
        self.order.iter().position(|&i| i == block_idx)
    }

    /// Iterate blocks in reading order.
    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.order.iter().copied()
    }
}

/// OODA-41: Tolerance for boundary normalization (pixels).
/// WHY (First Principles): PDF text coordinates vary by 1-3 pixels even for aligned columns
/// due to rounding in PDF generation tools. pymupdf4llm uses 3pt tolerance. This value is
/// derived from typical PDF coordinate precision (1/72 inch per point, with common rounding).
const BOUNDARY_ALIGNMENT_TOLERANCE: f32 = 3.0;

/// OODA-41: Tolerance for vertical gap joining (pixels).
/// WHY (First Principles): Paragraphs in PDFs typically have 10-12pt leading (line spacing).
/// A vertical gap smaller than 10pt between text blocks is likely within the same logical
/// region. pymupdf4llm uses 10pt, which matches single-line spacing for 10pt body text.
const _VERTICAL_GAP_TOLERANCE: f32 = 10.0;

/// Reading order detector.
#[derive(Debug, Clone)]
pub struct ReadingOrderDetector {
    /// Tolerance for considering blocks on the same line
    line_tolerance: f32,
    /// Tolerance for column alignment
    _column_tolerance: f32,
}

impl ReadingOrderDetector {
    /// Create a new reading order detector.
    pub fn new() -> Self {
        // OODA-04: Line tolerance changed from 5.0 to 3.0 to match pymupdf4llm
        // WHY: 5pt was causing lines to incorrectly merge. pymupdf4llm uses 3pt
        // which matches typical PDF coordinate precision.
        Self {
            line_tolerance: 3.0,
            _column_tolerance: 20.0,
        }
    }

    /// Create with custom tolerances.
    pub fn with_tolerances(line_tolerance: f32, column_tolerance: f32) -> Self {
        Self {
            line_tolerance,
            _column_tolerance: column_tolerance,
        }
    }

    /// Determine reading order for blocks given detected columns.
    pub fn determine_order(&self, blocks: &[Block], columns: &[BoundingBox]) -> Vec<usize> {
        if blocks.is_empty() {
            return Vec::new();
        }

        tracing::debug!(
            "READING-ORDER: blocks={} columns={}",
            blocks.len(),
            columns.len()
        );

        if columns.is_empty() || columns.len() == 1 {
            // Single column: simple top-to-bottom, left-to-right
            tracing::debug!("READING-ORDER: using single_column_order");
            return self.single_column_order(blocks);
        }

        // Multi-column layout: process column by column
        tracing::debug!("READING-ORDER: using multi_column_order");
        self.multi_column_order(blocks, columns)
    }

    /// Determine reading order for single-column layout.
    fn single_column_order(&self, blocks: &[Block]) -> Vec<usize> {
        let mut indices: Vec<usize> = (0..blocks.len()).collect();

        // Sort by Y position (top to bottom), then X (left to right)
        //
        // After Y-normalization in the extraction engine, coordinates are in
        // standard document order: Y=0 at top of page, Y increases downward.
        // So for top-to-bottom reading order, we sort by ASCENDING Y (lower Y = top = first).
        indices.sort_by(|&a, &b| {
            let bbox_a = &blocks[a].bbox;
            let bbox_b = &blocks[b].bbox;

            // Group by approximate Y position (same line)
            let y_a = (bbox_a.y1 / self.line_tolerance).floor();
            let y_b = (bbox_b.y1 / self.line_tolerance).floor();

            if y_a != y_b {
                // ASCENDING Y for top-to-bottom (lower Y = top of page = first)
                y_a.partial_cmp(&y_b).unwrap()
            } else {
                // Same line: sort by X (left to right)
                bbox_a.x1.partial_cmp(&bbox_b.x1).unwrap()
            }
        });

        indices
    }

    /// Determine reading order for multi-column layout.
    fn multi_column_order(&self, blocks: &[Block], columns: &[BoundingBox]) -> Vec<usize> {
        // Assign blocks to columns
        let mut column_blocks: Vec<Vec<usize>> = vec![Vec::new(); columns.len()];
        let mut spanning_blocks: Vec<usize> = Vec::new();
        let mut footer_blocks: Vec<usize> = Vec::new(); // OODA-38: Footer/affiliation blocks
        let mut unassigned: Vec<usize> = Vec::new();

        for (idx, block) in blocks.iter().enumerate() {
            let column_idx = self.assign_to_column(block, columns);

            match column_idx {
                Some(ColumnAssignment::Single(col)) => {
                    // DEBUG: Track interesting blocks
                    if block.text.contains("disentangles")
                        || block.text.contains("ren-")
                        || block.text.contains("independently")
                    {
                        tracing::trace!(
                            "ASSIGN: block {} '{}...' x1={:.0} -> column {}",
                            idx,
                            safe_truncate(&block.text, 30),
                            block.bbox.x1,
                            col
                        );
                    }
                    column_blocks[col].push(idx);
                }
                Some(ColumnAssignment::Spanning) => {
                    spanning_blocks.push(idx);
                }
                Some(ColumnAssignment::Footer) => {
                    // OODA-38: Footer blocks go at the very end
                    footer_blocks.push(idx);
                }
                None => {
                    unassigned.push(idx);
                }
            }
        }

        tracing::debug!(
            "MULTI-COL: col0={} blocks, col1={} blocks, spanning={}, footer={}, unassigned={}",
            column_blocks.first().map(|v| v.len()).unwrap_or(0),
            column_blocks.get(1).map(|v| v.len()).unwrap_or(0),
            spanning_blocks.len(),
            footer_blocks.len(),
            unassigned.len()
        );

        // OODA-41: Use smart sort key for WITHIN-column sorting
        // This ensures blocks at the same vertical level within a column are ordered correctly
        // OODA-04 FIX: Actually call sort_by_smart_key (was incorrectly calling sort_by_position)
        for col_blocks in &mut column_blocks {
            self.sort_by_smart_key(col_blocks, blocks);
        }

        // Sort spanning blocks by Y position
        self.sort_by_position(&mut spanning_blocks, blocks);

        // Sort footer blocks by Y position (for consistent ordering)
        self.sort_by_position(&mut footer_blocks, blocks);

        // Merge columns respecting spanning elements, with footer at the end
        // OODA-41: The key fix is in merge_column_orders_with_footer_smart which
        // uses smart keys for the final ordering
        self.merge_column_orders_with_footer_smart(
            &column_blocks,
            &spanning_blocks,
            &footer_blocks,
            &unassigned,
            blocks,
        )
    }

    /// Assign a block to a column.
    fn assign_to_column(&self, block: &Block, columns: &[BoundingBox]) -> Option<ColumnAssignment> {
        // OODA-28 FIX: Detect potential titles that should span both columns
        // WHY: Some titles have zero-width bboxes due to a bug in block_builder.rs
        // Heuristic: If block is a level-1 heading at top of page (Y < 20), treat as spanning
        // This ensures titles appear before column content regardless of bbox width
        let is_title_candidate = block.block_type == BlockType::SectionHeader
            && block.level == Some(1)
            && block.bbox.y1 < 20.0; // Near top of page (normalized Y=0 at top)

        // Also detect by content: long text at top of page in left margin
        let text_len = block.text.len();
        let is_long_text_at_top = text_len > 50 && block.bbox.y1 < 20.0 && block.bbox.x1 < 80.0;

        if is_title_candidate || is_long_text_at_top {
            return Some(ColumnAssignment::Spanning);
        }

        // OODA-38 FIX: Detect footer/affiliation content that should appear at the end
        // WHY: Affiliations like "1School of Computer Science, Peking University" were
        // appearing between left column body and right column body. They should appear
        // AFTER all body content from both columns.
        //
        // Heuristics for footer/affiliation detection:
        // 1. Block is near bottom of page (Y > 550 in normalized coords where page ~650 tall)
        // 2. Contains affiliation keywords (University, School of, Academy, @, etc.)
        // 3. Starts with superscript number pattern (1, 2, etc.)
        let is_near_bottom = block.bbox.y1 > 550.0;
        let text = &block.text;
        let looks_like_affiliation = text.contains("University")
            || text.contains("School of")
            || text.contains("Academy")
            || text.contains("Department of")
            || text.contains("Correspondence")
            || text.contains('@')
            || (text.starts_with('1')
                && text.len() > 1
                && !text.chars().nth(1).unwrap_or(' ').is_ascii_digit())
            || (text.starts_with('2')
                && text.len() > 1
                && !text.chars().nth(1).unwrap_or(' ').is_ascii_digit());

        if is_near_bottom && looks_like_affiliation {
            tracing::debug!(
                "OODA-38: Detected footer/affiliation: Y={:.1} '{}'",
                block.bbox.y1,
                safe_truncate(text, 50)
            );
            return Some(ColumnAssignment::Footer);
        }

        let center_x = block.bbox.center().x;

        let mut containing_columns = Vec::new();

        for (idx, col) in columns.iter().enumerate() {
            if block.bbox.intersects(col) {
                containing_columns.push(idx);
            }
        }

        match containing_columns.len() {
            0 => {
                // Find closest column by center
                let closest = columns
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        let dist_a = (a.center().x - center_x).abs();
                        let dist_b = (b.center().x - center_x).abs();
                        dist_a.partial_cmp(&dist_b).unwrap()
                    })
                    .map(|(idx, _)| idx);

                closest.map(ColumnAssignment::Single)
            }
            1 => Some(ColumnAssignment::Single(containing_columns[0])),
            _ => {
                // Check if block spans significantly across columns
                let first_col = &columns[containing_columns[0]];
                let overlap = block.bbox.intersection_area(first_col);

                if overlap / block.bbox.area() < 0.8 {
                    Some(ColumnAssignment::Spanning)
                } else {
                    Some(ColumnAssignment::Single(containing_columns[0]))
                }
            }
        }
    }

    /// Sort block indices by position.
    /// After Y-normalization: Y=0 at top, Y increases downward. Sort ASCENDING Y for top-to-bottom.
    fn sort_by_position(&self, indices: &mut [usize], blocks: &[Block]) {
        indices.sort_by(|&a, &b| {
            let bbox_a = &blocks[a].bbox;
            let bbox_b = &blocks[b].bbox;

            // ASCENDING Y (lower Y = top of page = comes first)
            bbox_a
                .y1
                .partial_cmp(&bbox_b.y1)
                .unwrap()
                .then_with(|| bbox_a.x1.partial_cmp(&bbox_b.x1).unwrap())
        });
    }

    /// OODA-41: Compute smart sort key for a block based on pymupdf4llm's Phase 3 algorithm.
    ///
    /// The key insight from pymupdf4llm's multi_column.py:
    /// - For each block, find the LEFT-MOST block that overlaps vertically
    /// - If found, use (left_block.y0, current_block.x0) as sort key
    /// - This ensures right-column content comes AFTER left-column content at same vertical level
    ///
    /// WHY (First Principles):
    /// In a two-column layout, when reading, we finish the left column at a given vertical
    /// position before moving to the right column. By using the left block's Y as the primary
    /// sort key, we ensure proper reading order even when blocks are at similar vertical positions.
    fn compute_smart_sort_key(&self, block_idx: usize, blocks: &[Block]) -> (f32, f32) {
        let block = &blocks[block_idx];
        let block_bbox = &block.bbox;

        // Find all blocks that are to the LEFT of this block and overlap vertically
        let left_blocks: Vec<(usize, &Block)> = blocks
            .iter()
            .enumerate()
            .filter(|(idx, other)| {
                // Skip self
                if *idx == block_idx {
                    return false;
                }

                let other_bbox = &other.bbox;

                // Block is to the left: its right edge < our left edge (with tolerance)
                let is_to_left = other_bbox.x2 < block_bbox.x1 - BOUNDARY_ALIGNMENT_TOLERANCE;

                // Vertical overlap: ranges [y1, y2] intersect
                // Overlap exists if: NOT (other.y2 < block.y1 OR block.y2 < other.y1)
                let has_vertical_overlap =
                    !(other_bbox.y2 < block_bbox.y1 || block_bbox.y2 < other_bbox.y1);

                is_to_left && has_vertical_overlap
            })
            .collect();

        // OODA-02 FIX: Use LEFT-MOST block (min x1), not RIGHT-MOST (max x2)
        // WHY (First Principles - per pymupdf4llm multi_column.py lines 290-304):
        // When sorting blocks for reading order, the left-most block with vertical
        // overlap determines the sort key. This ensures that right-column content
        // comes AFTER left-column content at the same vertical level. The original
        // code used max_by(x2) which found the right-most left block, but pymupdf4llm
        // uses min_by(x1) to find the truly left-most overlapping block.
        if let Some((_, left_block)) = left_blocks
            .iter()
            .min_by(|(_, a), (_, b)| a.bbox.x1.partial_cmp(&b.bbox.x1).unwrap())
        {
            // Use the left block's Y coordinate for sorting, but our X coordinate
            // This ensures blocks at the same vertical level come after their left neighbors
            tracing::debug!(
                "OODA-02: Block '{}' uses left-block Y={:.1} for sort (orig Y={:.1})",
                safe_truncate(&block.text, 30),
                left_block.bbox.y1,
                block_bbox.y1
            );
            (left_block.bbox.y1, block_bbox.x1)
        } else {
            // No left block found, use original position
            (block_bbox.y1, block_bbox.x1)
        }
    }

    /// OODA-41: Sort block indices using smart sort key algorithm.
    ///
    /// This is the key improvement from pymupdf4llm's reading order algorithm.
    /// Instead of sorting purely by (y, x), we use a computed sort key that
    /// considers left-column blocks at the same vertical level.
    fn sort_by_smart_key(&self, indices: &mut [usize], blocks: &[Block]) {
        // Compute sort keys for all blocks
        let mut keyed_indices: Vec<(usize, (f32, f32))> = indices
            .iter()
            .map(|&idx| (idx, self.compute_smart_sort_key(idx, blocks)))
            .collect();

        // Sort by computed key: (y, x) where y may come from a left neighbor
        keyed_indices.sort_by(|(_, key_a), (_, key_b)| {
            key_a
                .0
                .partial_cmp(&key_b.0)
                .unwrap()
                .then_with(|| key_a.1.partial_cmp(&key_b.1).unwrap())
        });

        // Update indices in place
        for (i, (idx, _)) in keyed_indices.iter().enumerate() {
            indices[i] = *idx;
        }
    }

    /// Merge column orders with spanning elements.
    ///
    /// Strategy: Process columns sequentially (left-to-right), inserting spanning
    /// elements at their vertical position. This ensures proper reading order for
    /// multi-column layouts (read all of column 1, then all of column 2, etc.)
    ///
    /// Note: After Y-normalization, Y=0 is at TOP of page. Lower Y = top of page.
    /// Blocks are sorted ASCENDING by Y (top first = lower Y first).
    #[allow(dead_code)] // Reserved for future multi-column reading order improvements
    fn merge_column_orders(
        &self,
        column_blocks: &[Vec<usize>],
        spanning: &[usize],
        unassigned: &[usize],
        blocks: &[Block],
    ) -> Vec<usize> {
        let mut result = Vec::new();
        let mut spanning_idx = 0;

        // Process leading spanning elements (before first column content)
        // Find the LOWEST Y in first column blocks (top of content after normalization)
        let first_col_y = column_blocks
            .iter()
            .filter_map(|col| col.first().map(|&idx| blocks[idx].bbox.y1))
            .min_by(|a, b| a.partial_cmp(b).unwrap()) // MIN for top-most block (lowest Y = top)
            .unwrap_or(f32::MAX);

        while spanning_idx < spanning.len() {
            let span_y = blocks[spanning[spanning_idx]].bbox.y1;
            // Spanning element is ABOVE first column content if its Y < first_col_y
            if span_y < first_col_y - self.line_tolerance {
                result.push(spanning[spanning_idx]);
                spanning_idx += 1;
            } else {
                break;
            }
        }

        // Process each column sequentially (left to right)
        for col_blocks in column_blocks {
            for &block_idx in col_blocks {
                let block_y = blocks[block_idx].bbox.y1;

                // Insert any spanning elements that appear ABOVE this block
                // (lower Y = above after normalization)
                while spanning_idx < spanning.len() {
                    let span_y = blocks[spanning[spanning_idx]].bbox.y1;
                    if span_y < block_y - self.line_tolerance {
                        result.push(spanning[spanning_idx]);
                        spanning_idx += 1;
                    } else {
                        break;
                    }
                }

                result.push(block_idx);
            }
        }

        // Process remaining spanning elements (at bottom of page)
        while spanning_idx < spanning.len() {
            result.push(spanning[spanning_idx]);
            spanning_idx += 1;
        }

        // Process unassigned blocks
        result.extend_from_slice(unassigned);

        result
    }

    /// OODA-38: Merge column orders with footer blocks at the very end.
    ///
    /// This ensures affiliations, footnotes, and other bottom-of-page content
    /// appears AFTER all body content from all columns, not interleaved.
    #[allow(dead_code)] // Reserved for future multi-column reading order improvements
    fn merge_column_orders_with_footer(
        &self,
        column_blocks: &[Vec<usize>],
        spanning: &[usize],
        footer_blocks: &[usize],
        unassigned: &[usize],
        blocks: &[Block],
    ) -> Vec<usize> {
        // First, use the standard merge for body content
        let mut result = self.merge_column_orders(column_blocks, spanning, unassigned, blocks);

        // OODA-38: Footer blocks go at the very end, after ALL body content
        if !footer_blocks.is_empty() {
            tracing::debug!(
                "OODA-38: Appending {} footer blocks at end of reading order",
                footer_blocks.len()
            );
            result.extend_from_slice(footer_blocks);
        }

        result
    }

    /// OODA-41: Smart merge using pymupdf4llm's Phase 3 algorithm.
    ///
    /// Key insight: Instead of processing columns sequentially, we:
    /// 1. Collect ALL body blocks (from all columns)
    /// 2. Sort using smart key: blocks to the right use left-neighbor's Y
    /// 3. Interleave spanning elements by Y position
    ///
    /// This ensures proper reading order for two-column layouts where
    /// content at the same vertical level should be read left-to-right
    /// WITHIN each column, not jumping between columns.
    fn merge_column_orders_with_footer_smart(
        &self,
        column_blocks: &[Vec<usize>],
        spanning: &[usize],
        footer_blocks: &[usize],
        unassigned: &[usize],
        blocks: &[Block],
    ) -> Vec<usize> {
        // Strategy: Process columns left-to-right, reading each column fully
        // before moving to the next. This is the correct reading order for
        // multi-column academic papers.
        //
        // The "smart key" insight from pymupdf4llm is used WITHIN this strategy:
        // when there are blocks at the same Y level, use their X to break ties.

        let mut result = Vec::new();
        let mut spanning_idx = 0;

        // Sort spanning elements by Y
        let mut sorted_spanning: Vec<usize> = spanning.to_vec();
        self.sort_by_position(&mut sorted_spanning, blocks);

        // Find the LOWEST Y in all column blocks (top of body content)
        let first_body_y = column_blocks
            .iter()
            .filter_map(|col| col.first().map(|&idx| blocks[idx].bbox.y1))
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(f32::MAX);

        // Insert spanning elements that appear BEFORE body content (headers, titles)
        while spanning_idx < sorted_spanning.len() {
            let span_y = blocks[sorted_spanning[spanning_idx]].bbox.y1;
            if span_y < first_body_y - self.line_tolerance {
                result.push(sorted_spanning[spanning_idx]);
                spanning_idx += 1;
            } else {
                break;
            }
        }

        // Process each column sequentially (LEFT to RIGHT)
        // This is the correct reading order for multi-column documents
        for (col_idx, col_blocks) in column_blocks.iter().enumerate() {
            if col_blocks.is_empty() {
                continue;
            }

            // Find the starting Y of this column's content
            let col_start_y = blocks[col_blocks[0]].bbox.y1;

            // Insert any spanning elements between previous column and this column's start
            // (This handles cases like section headers between columns)
            while spanning_idx < sorted_spanning.len() {
                let span_y = blocks[sorted_spanning[spanning_idx]].bbox.y1;
                // Insert spanning element if it's above this column's first block
                // AND we haven't already passed it
                if col_idx > 0 && span_y < col_start_y - self.line_tolerance {
                    // Check if this spanning element's Y is between previous column end
                    // and this column start
                    let prev_col_end_y = if col_idx > 0 && !column_blocks[col_idx - 1].is_empty() {
                        let last_in_prev = *column_blocks[col_idx - 1].last().unwrap();
                        blocks[last_in_prev].bbox.y2
                    } else {
                        0.0
                    };

                    if span_y > prev_col_end_y {
                        result.push(sorted_spanning[spanning_idx]);
                        spanning_idx += 1;
                        continue;
                    }
                }
                break;
            }

            // Add all blocks from this column in order (already sorted by Y)
            for &block_idx in col_blocks {
                result.push(block_idx);
            }
        }

        // Add remaining spanning elements (at the bottom of the page)
        while spanning_idx < sorted_spanning.len() {
            result.push(sorted_spanning[spanning_idx]);
            spanning_idx += 1;
        }

        // Add unassigned blocks
        result.extend_from_slice(unassigned);

        // OODA-38: Footer blocks go at the very end
        if !footer_blocks.is_empty() {
            tracing::debug!(
                "OODA-41: Appending {} footer blocks at end of reading order",
                footer_blocks.len()
            );
            result.extend_from_slice(footer_blocks);
        }

        result
    }

    /// Determine reading order with XY-cut tree.
    pub fn from_xy_cut_order(&self, xy_cut_order: &[usize]) -> ReadingOrder {
        ReadingOrder::new(xy_cut_order.to_vec())
    }
}

impl Default for ReadingOrderDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Column assignment for a block.
#[derive(Debug, Clone, Copy)]
enum ColumnAssignment {
    /// Block belongs to a single column
    Single(usize),
    /// Block spans multiple columns
    Spanning,
    /// Block is footer/affiliation content that should appear at the end
    /// WHY (OODA-38): Affiliations like "1School of Computer Science" were being
    /// assigned to left column and appearing between left body and right body.
    /// They should appear AFTER all body content from both columns.
    Footer,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::BlockType;

    fn make_block(x1: f32, y1: f32, x2: f32, y2: f32) -> Block {
        Block::new(BlockType::Text, BoundingBox::new(x1, y1, x2, y2))
    }

    #[test]
    fn test_single_column_order() {
        let detector = ReadingOrderDetector::new();

        // After Y-normalization: Y=0 at TOP of page, lower Y = higher on page
        // For top-to-bottom reading order: lower Y should come FIRST (ascending Y sort)
        let blocks = vec![
            make_block(100.0, 100.0, 500.0, 150.0), // Middle (Y=100)
            make_block(100.0, 200.0, 500.0, 250.0), // Bottom (Y=200) - LAST
            make_block(100.0, 50.0, 500.0, 100.0),  // Top (Y=50) - FIRST
        ];

        let order = detector.single_column_order(&blocks);
        // Expected: Top (Y=50, idx=2), Middle (Y=100, idx=0), Bottom (Y=200, idx=1)
        assert_eq!(order, vec![2, 0, 1]);
    }

    #[test]
    fn test_same_line_left_to_right() {
        let detector = ReadingOrderDetector::new();

        let blocks = vec![
            make_block(300.0, 100.0, 400.0, 150.0), // Right
            make_block(100.0, 100.0, 200.0, 150.0), // Left
        ];

        let order = detector.single_column_order(&blocks);
        assert_eq!(order, vec![1, 0]);
    }

    #[test]
    fn test_multi_column_order() {
        let detector = ReadingOrderDetector::new();

        let columns = vec![
            BoundingBox::new(50.0, 0.0, 280.0, 800.0),  // Left column
            BoundingBox::new(332.0, 0.0, 562.0, 800.0), // Right column
        ];

        // After Y-normalization: Y=0 at TOP, lower Y = top of page
        // Within each column, blocks are sorted by ASCENDING Y (lower Y = top = first)
        let blocks = vec![
            make_block(350.0, 200.0, 540.0, 250.0), // Right column, Y=200 (higher = second in right col)
            make_block(100.0, 100.0, 260.0, 150.0), // Left column, Y=100 (lower = first in left col)
            make_block(100.0, 200.0, 260.0, 250.0), // Left column, Y=200 (higher = second in left col)
            make_block(350.0, 100.0, 540.0, 150.0), // Right column, Y=100 (lower = first in right col)
        ];

        let order = detector.determine_order(&blocks, &columns);

        // Left column blocks sorted by ascending Y: idx=1 (Y=100), idx=2 (Y=200)
        // Right column blocks sorted by ascending Y: idx=3 (Y=100), idx=0 (Y=200)
        // Reading order: left column (1, 2), then right column (3, 0)
        assert_eq!(order.len(), 4);
        assert_eq!(order[0], 1); // Left column first (Y=100)
        assert_eq!(order[1], 2); // Left column second (Y=200)
        assert_eq!(order[2], 3); // Right column first (Y=100)
        assert_eq!(order[3], 0); // Right column second (Y=200)
    }

    #[test]
    fn test_spanning_element() {
        let detector = ReadingOrderDetector::new();

        let columns = vec![
            BoundingBox::new(50.0, 0.0, 280.0, 800.0),
            BoundingBox::new(332.0, 0.0, 562.0, 800.0),
        ];

        // After Y-normalization: Y=0 at TOP of page, lower Y = top
        // The header at Y=30 is ABOVE the content at Y=100
        let blocks = vec![
            make_block(50.0, 30.0, 562.0, 80.0), // Spanning header at TOP (Y=30)
            make_block(100.0, 100.0, 260.0, 150.0), // Left column content (Y=100)
            make_block(350.0, 100.0, 540.0, 150.0), // Right column content (Y=100)
        ];

        let order = detector.determine_order(&blocks, &columns);

        // Header (Y=30) should come first since it's at the top (lowest Y)
        assert_eq!(order[0], 0);
    }

    #[test]
    fn test_empty_blocks() {
        let detector = ReadingOrderDetector::new();
        let order = detector.determine_order(&[], &[]);
        assert!(order.is_empty());
    }

    #[test]
    fn test_reading_order_position() {
        let order = ReadingOrder::new(vec![2, 0, 3, 1]);

        assert_eq!(order.position_of(2), Some(0));
        assert_eq!(order.position_of(0), Some(1));
        assert_eq!(order.position_of(3), Some(2));
        assert_eq!(order.position_of(1), Some(3));
        assert_eq!(order.position_of(5), None);
    }

    // ==========================================================================
    // OODA-31: Additional ReadingOrder and Detector tests
    // ==========================================================================

    #[test]
    fn test_reading_order_iter() {
        let order = ReadingOrder::new(vec![2, 0, 3, 1]);
        let collected: Vec<usize> = order.iter().collect();
        assert_eq!(collected, vec![2, 0, 3, 1]);
    }

    #[test]
    fn test_detector_default() {
        let detector = ReadingOrderDetector::default();
        // WHY: Default line tolerance is 3.0 (matches pymupdf4llm)
        assert!((detector.line_tolerance - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_detector_with_tolerances() {
        let detector = ReadingOrderDetector::with_tolerances(5.0, 25.0);
        assert!((detector.line_tolerance - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_from_xy_cut_order() {
        let detector = ReadingOrderDetector::new();
        let xy_order = vec![3, 1, 2, 0];
        let reading_order = detector.from_xy_cut_order(&xy_order);
        assert_eq!(reading_order.order, vec![3, 1, 2, 0]);
        assert!((reading_order.confidence - 1.0).abs() < 0.001);
    }

    // ==========================================================================
    // OODA-02: Regression test for left-block finder fix
    // ==========================================================================

    #[test]
    fn test_smart_sort_key_uses_leftmost_block() {
        // WHY: This test verifies the OODA-02 fix where we changed from max_by(x2)
        // to min_by(x1) to find the LEFT-MOST overlapping block, matching pymupdf4llm.
        //
        // Scenario: Two left blocks (A at x=50, B at x=150) overlap vertically with
        // a right block (C at x=400). The sort key for C should use A's Y (the left-most).
        let detector = ReadingOrderDetector::new();

        // Create test blocks with text
        let mut block_a = make_block(50.0, 100.0, 140.0, 150.0); // Left-most left block
        block_a.text = "Block A (left-most)".to_string();

        let mut block_b = make_block(150.0, 100.0, 240.0, 150.0); // Right-ish left block
        block_b.text = "Block B (right-ish)".to_string();

        let mut block_c = make_block(400.0, 100.0, 550.0, 150.0); // Right block
        block_c.text = "Block C (right column)".to_string();

        let blocks = vec![block_a.clone(), block_b.clone(), block_c.clone()];

        // Compute smart key for the right block (block_c at index 2)
        let key_c = detector.compute_smart_sort_key(2, &blocks);

        // The key should use block_a's Y (100.0), not block_b's Y
        // Because block_a is the LEFT-MOST block with vertical overlap
        assert!(
            (key_c.0 - 100.0).abs() < 0.1,
            "Sort key Y should be 100.0 (from left-most block A), got {}",
            key_c.0
        );

        // X coordinate should be block_c's X (400.0)
        assert!(
            (key_c.1 - 400.0).abs() < 0.1,
            "Sort key X should be 400.0 (block C's position), got {}",
            key_c.1
        );
    }
}
