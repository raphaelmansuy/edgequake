//! Reading order detection for document blocks.
//!
//! This module provides algorithms for determining the correct reading order
//! of blocks on a page, handling single-column, multi-column, and complex layouts.

use crate::schema::{Block, BoundingBox};

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
        Self {
            line_tolerance: 5.0,
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

        tracing::info!(
            "READING-ORDER: blocks={} columns={}",
            blocks.len(),
            columns.len()
        );

        if columns.is_empty() || columns.len() == 1 {
            // Single column: simple top-to-bottom, left-to-right
            tracing::info!("READING-ORDER: using single_column_order");
            return self.single_column_order(blocks);
        }

        // Multi-column layout: process column by column
        tracing::info!("READING-ORDER: using multi_column_order");
        self.multi_column_order(blocks, columns)
    }

    /// Determine reading order for single-column layout.
    fn single_column_order(&self, blocks: &[Block]) -> Vec<usize> {
        let mut indices: Vec<usize> = (0..blocks.len()).collect();

        // Sort by Y position (top to bottom), then X (left to right)
        // In PDF coordinates, Y=0 is at BOTTOM of page, so higher Y = top.
        // For top-to-bottom reading order, we need DESCENDING Y (higher Y first).
        indices.sort_by(|&a, &b| {
            let bbox_a = &blocks[a].bbox;
            let bbox_b = &blocks[b].bbox;

            // Group by approximate Y position (same line)
            let y_a = (bbox_a.y1 / self.line_tolerance).floor();
            let y_b = (bbox_b.y1 / self.line_tolerance).floor();

            if y_a != y_b {
                // DESCENDING Y for top-to-bottom (higher Y = top of page = first)
                y_b.partial_cmp(&y_a).unwrap()
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
                        tracing::info!(
                            "ASSIGN: block {} '{}...' x1={:.0} -> column {}",
                            idx,
                            &block.text[..block.text.len().min(30)],
                            block.bbox.x1,
                            col
                        );
                    }
                    column_blocks[col].push(idx);
                }
                Some(ColumnAssignment::Spanning) => {
                    spanning_blocks.push(idx);
                }
                None => {
                    unassigned.push(idx);
                }
            }
        }

        tracing::info!(
            "MULTI-COL: col0={} blocks, col1={} blocks, spanning={}, unassigned={}",
            column_blocks.first().map(|v| v.len()).unwrap_or(0),
            column_blocks.get(1).map(|v| v.len()).unwrap_or(0),
            spanning_blocks.len(),
            unassigned.len()
        );

        // Sort blocks within each column
        for col_blocks in &mut column_blocks {
            self.sort_by_position(col_blocks, blocks);
        }

        // Sort spanning blocks by Y position
        self.sort_by_position(&mut spanning_blocks, blocks);

        // Merge columns respecting spanning elements
        self.merge_column_orders(&column_blocks, &spanning_blocks, &unassigned, blocks)
    }

    /// Assign a block to a column.
    fn assign_to_column(&self, block: &Block, columns: &[BoundingBox]) -> Option<ColumnAssignment> {
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
    /// In PDF coordinates, Y=0 is at BOTTOM, so we sort DESCENDING Y for top-to-bottom.
    fn sort_by_position(&self, indices: &mut [usize], blocks: &[Block]) {
        indices.sort_by(|&a, &b| {
            let bbox_a = &blocks[a].bbox;
            let bbox_b = &blocks[b].bbox;

            // DESCENDING Y (higher Y = top of page = comes first)
            bbox_b
                .y1
                .partial_cmp(&bbox_a.y1)
                .unwrap()
                .then_with(|| bbox_a.x1.partial_cmp(&bbox_b.x1).unwrap())
        });
    }

    /// Merge column orders with spanning elements.
    ///
    /// Strategy: Process columns sequentially (left-to-right), inserting spanning
    /// elements at their vertical position. This ensures proper reading order for
    /// multi-column layouts (read all of column 1, then all of column 2, etc.)
    ///
    /// Note: In PDF coordinates, Y=0 is at BOTTOM. Higher Y = top of page.
    /// Blocks are sorted DESCENDING by Y (top first = higher Y first).
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
        // Find the HIGHEST Y in first column blocks (top of content)
        let first_col_y = column_blocks
            .iter()
            .filter_map(|col| col.first().map(|&idx| blocks[idx].bbox.y1))
            .max_by(|a, b| a.partial_cmp(b).unwrap()) // MAX for top-most block
            .unwrap_or(f32::MIN);

        while spanning_idx < spanning.len() {
            let span_y = blocks[spanning[spanning_idx]].bbox.y1;
            // Spanning element is ABOVE first column content if its Y > first_col_y
            if span_y > first_col_y + self.line_tolerance {
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
                // (higher Y = above in PDF coordinates)
                while spanning_idx < spanning.len() {
                    let span_y = blocks[spanning[spanning_idx]].bbox.y1;
                    if span_y > block_y + self.line_tolerance {
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

        // In PDF coordinates: Y=0 is at BOTTOM, higher Y = higher on page
        // For top-to-bottom reading order: higher Y should come FIRST
        let blocks = vec![
            make_block(100.0, 200.0, 500.0, 250.0), // Middle (Y=200)
            make_block(100.0, 100.0, 500.0, 150.0), // Bottom (Y=100) - LAST
            make_block(100.0, 300.0, 500.0, 350.0), // Top (Y=300) - FIRST
        ];

        let order = detector.single_column_order(&blocks);
        // Expected: Top (Y=300, idx=2), Middle (Y=200, idx=0), Bottom (Y=100, idx=1)
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

        // In PDF coordinates: Y=0 at BOTTOM, higher Y = higher on page
        // Within each column, blocks are sorted by DESCENDING Y (higher Y = top = first)
        let blocks = vec![
            make_block(350.0, 200.0, 540.0, 250.0), // Right column, Y=200 (lower = second in right col)
            make_block(100.0, 200.0, 260.0, 250.0), // Left column, Y=200 (lower = second in left col)
            make_block(100.0, 100.0, 260.0, 150.0), // Left column, Y=100 (lowest = third in left col)
            make_block(350.0, 300.0, 540.0, 350.0), // Right column, Y=300 (higher = first in right col)
        ];

        let order = detector.determine_order(&blocks, &columns);

        // Left column blocks sorted by descending Y: idx=1 (Y=200), idx=2 (Y=100)
        // Right column blocks sorted by descending Y: idx=3 (Y=300), idx=0 (Y=200)
        // Reading order: left column (1, 2), then right column (3, 0)
        assert_eq!(order.len(), 4);
        assert_eq!(order[0], 1); // Left column first (Y=200)
        assert_eq!(order[1], 2); // Left column second (Y=100)
        assert_eq!(order[2], 3); // Right column first (Y=300)
        assert_eq!(order[3], 0); // Right column second (Y=200)
    }

    #[test]
    fn test_spanning_element() {
        let detector = ReadingOrderDetector::new();

        let columns = vec![
            BoundingBox::new(50.0, 0.0, 280.0, 800.0),
            BoundingBox::new(332.0, 0.0, 562.0, 800.0),
        ];

        // In PDF coordinates: Y=0 at BOTTOM, higher Y = higher on page
        // The header at Y=300 is ABOVE the content at Y=100
        let blocks = vec![
            make_block(50.0, 300.0, 562.0, 330.0), // Spanning header at TOP (Y=300)
            make_block(100.0, 100.0, 260.0, 150.0), // Left column content (Y=100)
            make_block(350.0, 100.0, 540.0, 150.0), // Right column content (Y=100)
        ];

        let order = detector.determine_order(&blocks, &columns);

        // Header (Y=300) should come first since it's at the top
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
}
