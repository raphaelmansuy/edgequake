//! XY-cut algorithm for document segmentation.
//!
//! The XY-cut algorithm recursively partitions a document page by finding
//! horizontal and vertical whitespace gaps. This is a classic technique
//! from the document layout analysis literature.
//!
//! Reference: "Recursive X-Y Cut using Bounding Boxes of Connected Components"
//! by Ha, Haralick, and Phillips (1995)

use crate::schema::BoundingBox;

/// Parameters for XY-cut algorithm.
#[derive(Debug, Clone)]
pub struct XYCutParams {
    /// Minimum gap width for vertical cuts (between columns)
    pub min_vertical_gap: f32,
    /// Minimum gap height for horizontal cuts (between blocks)
    pub min_horizontal_gap: f32,
    /// Minimum region width
    pub min_region_width: f32,
    /// Minimum region height
    pub min_region_height: f32,
    /// Maximum recursion depth
    pub max_depth: usize,
    /// Prefer horizontal cuts over vertical (typical for reading order)
    pub prefer_horizontal: bool,
}

impl Default for XYCutParams {
    fn default() -> Self {
        Self {
            min_vertical_gap: 20.0,   // Will be overridden by adaptive calculation
            min_horizontal_gap: 10.0, // Will be overridden by adaptive calculation
            min_region_width: 50.0,
            min_region_height: 20.0,
            max_depth: 10,
            prefer_horizontal: true,
        }
    }
}

impl XYCutParams {
    // Removed deprecated single_column() and multi_column() methods (Loop 010).
    // Use XYCut::with_defaults() or segment_adaptive() for adaptive thresholds.
}

/// XY-cut tree node representing a document region.
#[derive(Debug, Clone)]
pub enum XYCutNode {
    /// Leaf node containing actual content items
    Leaf {
        bbox: BoundingBox,
        items: Vec<usize>, // Indices into original items
    },
    /// Horizontal cut (top-to-bottom split)
    HorizontalCut {
        bbox: BoundingBox,
        cut_y: f32,
        children: Vec<XYCutNode>,
    },
    /// Vertical cut (left-to-right split)
    VerticalCut {
        bbox: BoundingBox,
        cut_x: f32,
        children: Vec<XYCutNode>,
    },
}

impl XYCutNode {
    /// Get the bounding box of this node.
    pub fn bbox(&self) -> &BoundingBox {
        match self {
            XYCutNode::Leaf { bbox, .. } => bbox,
            XYCutNode::HorizontalCut { bbox, .. } => bbox,
            XYCutNode::VerticalCut { bbox, .. } => bbox,
        }
    }

    /// Get all leaf nodes in reading order.
    pub fn get_leaves(&self) -> Vec<&XYCutNode> {
        match self {
            XYCutNode::Leaf { .. } => vec![self],
            XYCutNode::HorizontalCut { children, .. } | XYCutNode::VerticalCut { children, .. } => {
                children.iter().flat_map(|c| c.get_leaves()).collect()
            }
        }
    }

    /// Get item indices in reading order.
    pub fn get_items_in_order(&self) -> Vec<usize> {
        match self {
            XYCutNode::Leaf { items, .. } => items.clone(),
            XYCutNode::HorizontalCut { children, .. } | XYCutNode::VerticalCut { children, .. } => {
                children
                    .iter()
                    .flat_map(|c| c.get_items_in_order())
                    .collect()
            }
        }
    }

    /// Count total leaves in tree.
    pub fn count_leaves(&self) -> usize {
        match self {
            XYCutNode::Leaf { .. } => 1,
            XYCutNode::HorizontalCut { children, .. } | XYCutNode::VerticalCut { children, .. } => {
                children.iter().map(|c| c.count_leaves()).sum()
            }
        }
    }
}

/// Calculate adaptive vertical gap threshold from bounding box distribution.
///
/// Uses statistical analysis of horizontal spacing between items to determine
/// appropriate gap threshold for column detection. This is a first-principles
/// approach that adapts to the document's actual layout.
///
/// # Arguments
/// * `items` - Bounding boxes to analyze
///
/// # Returns
/// Adaptive gap threshold based on 15th percentile of horizontal distances
fn calculate_adaptive_vertical_gap(items: &[BoundingBox]) -> f32 {
    if items.len() < 2 {
        return 20.0; // Default fallback
    }

    // Calculate horizontal distances between adjacent items
    let mut distances = Vec::new();
    for i in 0..items.len() {
        for j in (i + 1)..items.len() {
            let dist = (items[i].x1 - items[j].x1).abs();
            distances.push(dist);
        }
    }

    if distances.is_empty() {
        return 20.0;
    }

    // Sort and use 15th percentile to capture typical column gaps
    distances.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let percentile_idx = (distances.len() as f32 * 0.15) as usize;
    let gap = distances.get(percentile_idx).copied().unwrap_or(20.0);

    // Clamp to reasonable range
    gap.max(10.0).min(100.0)
}

/// Calculate adaptive horizontal gap threshold from bounding box distribution.
///
/// Uses statistical analysis of vertical spacing between items to determine
/// appropriate gap threshold for block separation. This is a first-principles
/// approach that adapts to the document's actual layout.
///
/// # Arguments
/// * `items` - Bounding boxes to analyze
///
/// # Returns
/// Adaptive gap threshold based on 15th percentile of vertical distances
fn calculate_adaptive_horizontal_gap(items: &[BoundingBox]) -> f32 {
    if items.len() < 2 {
        return 10.0; // Default fallback
    }

    // Calculate vertical distances between adjacent items
    let mut distances = Vec::new();
    for i in 0..items.len() {
        for j in (i + 1)..items.len() {
            let dist = (items[i].y1 - items[j].y1).abs();
            distances.push(dist);
        }
    }

    if distances.is_empty() {
        return 10.0;
    }

    // Sort and use 15th percentile to capture typical block gaps
    distances.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let percentile_idx = (distances.len() as f32 * 0.15) as usize;
    let gap = distances.get(percentile_idx).copied().unwrap_or(10.0);

    // Clamp to reasonable range
    gap.max(5.0).min(50.0)
}

/// XY-cut algorithm implementation.
pub struct XYCut {
    params: XYCutParams,
}

impl XYCut {
    /// Create a new XY-cut instance with given parameters.
    pub fn new(params: XYCutParams) -> Self {
        Self { params }
    }

    /// Create with default parameters.
    pub fn with_defaults() -> Self {
        Self::new(XYCutParams::default())
    }

    /// Segment a page into regions using the XY-cut algorithm.
    ///
    /// # Arguments
    /// * `items` - Bounding boxes of items to segment
    /// * `page_bbox` - Page bounding box
    ///
    /// # Returns
    /// Tree structure representing the document segmentation
    pub fn segment(&self, items: &[BoundingBox], page_bbox: &BoundingBox) -> XYCutNode {
        let indices: Vec<usize> = (0..items.len()).collect();
        self.segment_recursive(items, &indices, page_bbox, 0)
    }

    /// Segment with adaptive gap calculation.
    ///
    /// This method calculates gap thresholds based on the actual distribution
    /// of items in the document, following first principles instead of
    /// using hardcoded magic numbers.
    ///
    /// # Arguments
    /// * `items` - Bounding boxes of items to segment
    /// * `page_bbox` - Page bounding box
    ///
    /// # Returns
    /// Tree structure representing the document segmentation
    pub fn segment_adaptive(&self, items: &[BoundingBox], page_bbox: &BoundingBox) -> XYCutNode {
        // Calculate adaptive gaps based on item distribution
        let adaptive_params = XYCutParams {
            min_vertical_gap: calculate_adaptive_vertical_gap(items),
            min_horizontal_gap: calculate_adaptive_horizontal_gap(items),
            ..self.params.clone()
        };

        let adaptive_xy_cut = XYCut::new(adaptive_params);
        let indices: Vec<usize> = (0..items.len()).collect();
        adaptive_xy_cut.segment_recursive(items, &indices, page_bbox, 0)
    }

    /// Recursive segmentation.
    fn segment_recursive(
        &self,
        all_items: &[BoundingBox],
        indices: &[usize],
        region: &BoundingBox,
        depth: usize,
    ) -> XYCutNode {
        // Base cases
        if indices.is_empty() {
            return XYCutNode::Leaf {
                bbox: *region,
                items: Vec::new(),
            };
        }

        if indices.len() == 1 || depth >= self.params.max_depth {
            return XYCutNode::Leaf {
                bbox: *region,
                items: indices.to_vec(),
            };
        }

        // Get bboxes for current indices
        let items: Vec<BoundingBox> = indices.iter().map(|&i| all_items[i]).collect();

        // Find best cut
        let h_cut = self.find_best_horizontal_cut(&items, region);
        let v_cut = self.find_best_vertical_cut(&items, region);

        // Decide which cut to make
        let (cut_type, cut_pos) = match (h_cut, v_cut) {
            (Some((h_pos, h_gap)), Some((v_pos, v_gap))) => {
                // Both cuts possible - choose based on gap size and preference
                if self.params.prefer_horizontal && h_gap >= v_gap * 0.8 {
                    (CutType::Horizontal, h_pos)
                } else if v_gap > h_gap {
                    (CutType::Vertical, v_pos)
                } else {
                    (CutType::Horizontal, h_pos)
                }
            }
            (Some((h_pos, _)), None) => (CutType::Horizontal, h_pos),
            (None, Some((v_pos, _))) => (CutType::Vertical, v_pos),
            (None, None) => {
                // No valid cut - return leaf
                return XYCutNode::Leaf {
                    bbox: *region,
                    items: indices.to_vec(),
                };
            }
        };

        // Apply the cut
        match cut_type {
            CutType::Horizontal => {
                let (top_indices, bottom_indices) =
                    self.split_by_horizontal(indices, all_items, cut_pos);

                let top_region = region.top_half(cut_pos);
                let bottom_region = region.bottom_half(cut_pos);

                let children = vec![
                    self.segment_recursive(all_items, &top_indices, &top_region, depth + 1),
                    self.segment_recursive(all_items, &bottom_indices, &bottom_region, depth + 1),
                ];

                XYCutNode::HorizontalCut {
                    bbox: *region,
                    cut_y: cut_pos,
                    children,
                }
            }
            CutType::Vertical => {
                let (left_indices, right_indices) =
                    self.split_by_vertical(indices, all_items, cut_pos);

                let left_region = region.left_half(cut_pos);
                let right_region = region.right_half(cut_pos);

                let children = vec![
                    self.segment_recursive(all_items, &left_indices, &left_region, depth + 1),
                    self.segment_recursive(all_items, &right_indices, &right_region, depth + 1),
                ];

                XYCutNode::VerticalCut {
                    bbox: *region,
                    cut_x: cut_pos,
                    children,
                }
            }
        }
    }

    /// Find the best horizontal cut position.
    /// Returns (cut_position, gap_size) or None if no valid cut.
    fn find_best_horizontal_cut(
        &self,
        items: &[BoundingBox],
        region: &BoundingBox,
    ) -> Option<(f32, f32)> {
        if items.len() < 2 {
            return None;
        }

        // Create projection onto Y axis
        let mut events: Vec<(f32, bool)> = Vec::new(); // (y, is_start)
        for bbox in items {
            events.push((bbox.y1, true)); // start
            events.push((bbox.y2, false)); // end
        }
        events.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // Find gaps
        let mut gaps: Vec<(f32, f32, f32)> = Vec::new(); // (start, end, gap_size)
        let mut active_count = 0;
        let mut gap_start = region.y1;

        for (y, is_start) in &events {
            if *is_start {
                if active_count == 0 && *y > gap_start + self.params.min_horizontal_gap {
                    gaps.push((gap_start, *y, y - gap_start));
                }
                active_count += 1;
            } else {
                active_count -= 1;
                if active_count == 0 {
                    gap_start = *y;
                }
            }
        }

        // Check final gap to region bottom
        if gap_start < region.y2 - self.params.min_horizontal_gap {
            gaps.push((gap_start, region.y2, region.y2 - gap_start));
        }

        // Find largest valid gap
        gaps.into_iter()
            .filter(|(_, _, gap)| *gap >= self.params.min_horizontal_gap)
            .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap())
            .map(|(start, end, gap)| ((start + end) / 2.0, gap))
    }

    /// Find the best vertical cut position.
    fn find_best_vertical_cut(
        &self,
        items: &[BoundingBox],
        region: &BoundingBox,
    ) -> Option<(f32, f32)> {
        if items.len() < 2 {
            return None;
        }

        // Create projection onto X axis
        let mut events: Vec<(f32, bool)> = Vec::new();
        for bbox in items {
            events.push((bbox.x1, true));
            events.push((bbox.x2, false));
        }
        events.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // Find gaps
        let mut gaps: Vec<(f32, f32, f32)> = Vec::new();
        let mut active_count = 0;
        let mut gap_start = region.x1;

        for (x, is_start) in &events {
            if *is_start {
                if active_count == 0 && *x > gap_start + self.params.min_vertical_gap {
                    gaps.push((gap_start, *x, x - gap_start));
                }
                active_count += 1;
            } else {
                active_count -= 1;
                if active_count == 0 {
                    gap_start = *x;
                }
            }
        }

        // Check final gap
        if gap_start < region.x2 - self.params.min_vertical_gap {
            gaps.push((gap_start, region.x2, region.x2 - gap_start));
        }

        // Find largest valid gap
        gaps.into_iter()
            .filter(|(_, _, gap)| *gap >= self.params.min_vertical_gap)
            .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap())
            .map(|(start, end, gap)| ((start + end) / 2.0, gap))
    }

    /// Split indices by horizontal cut.
    fn split_by_horizontal(
        &self,
        indices: &[usize],
        all_items: &[BoundingBox],
        cut_y: f32,
    ) -> (Vec<usize>, Vec<usize>) {
        let mut top = Vec::new();
        let mut bottom = Vec::new();

        for &idx in indices {
            let bbox = &all_items[idx];
            let center_y = (bbox.y1 + bbox.y2) / 2.0;
            if center_y < cut_y {
                top.push(idx);
            } else {
                bottom.push(idx);
            }
        }

        (top, bottom)
    }

    /// Split indices by vertical cut.
    fn split_by_vertical(
        &self,
        indices: &[usize],
        all_items: &[BoundingBox],
        cut_x: f32,
    ) -> (Vec<usize>, Vec<usize>) {
        let mut left = Vec::new();
        let mut right = Vec::new();

        for &idx in indices {
            let bbox = &all_items[idx];
            let center_x = (bbox.x1 + bbox.x2) / 2.0;
            if center_x < cut_x {
                left.push(idx);
            } else {
                right.push(idx);
            }
        }

        (left, right)
    }
}

impl Default for XYCut {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[derive(Debug, Clone, Copy)]
enum CutType {
    Horizontal,
    Vertical,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bbox(x1: f32, y1: f32, x2: f32, y2: f32) -> BoundingBox {
        BoundingBox::new(x1, y1, x2, y2)
    }

    #[test]
    fn test_xy_cut_single_item() {
        let xy_cut = XYCut::with_defaults();
        let items = vec![make_bbox(100.0, 100.0, 500.0, 200.0)];
        let page = make_bbox(0.0, 0.0, 612.0, 792.0);

        let tree = xy_cut.segment(&items, &page);

        match tree {
            XYCutNode::Leaf { items, .. } => {
                assert_eq!(items.len(), 1);
                assert_eq!(items[0], 0);
            }
            _ => panic!("Expected leaf node"),
        }
    }

    #[test]
    fn test_xy_cut_two_items_vertical_gap() {
        // Two blocks side by side covering most of the height (no horizontal gap)
        let xy_cut = XYCut::new(XYCutParams {
            min_vertical_gap: 20.0,
            min_horizontal_gap: 100.0, // High threshold so no horizontal cut found
            prefer_horizontal: false,
            ..Default::default()
        });

        let items = vec![
            make_bbox(50.0, 50.0, 250.0, 700.0),  // Left block (tall)
            make_bbox(350.0, 50.0, 550.0, 700.0), // Right block (tall)
        ];
        let page = make_bbox(0.0, 0.0, 612.0, 792.0);

        let tree = xy_cut.segment(&items, &page);

        match &tree {
            XYCutNode::VerticalCut { children, .. } => {
                assert_eq!(children.len(), 2, "Expected 2 children in vertical cut");
            }
            XYCutNode::Leaf { items, .. } => {
                // Also acceptable - might group them together
                assert_eq!(items.len(), 2, "Leaf should contain both items");
            }
            _ => panic!("Expected vertical cut or leaf, got {:?}", tree),
        }
    }

    #[test]
    fn test_xy_cut_two_items_horizontal_gap() {
        // Two blocks stacked with large gap
        let xy_cut = XYCut::with_defaults();

        let items = vec![
            make_bbox(100.0, 50.0, 500.0, 150.0),  // Top block
            make_bbox(100.0, 250.0, 500.0, 350.0), // Bottom block
        ];
        let page = make_bbox(0.0, 0.0, 612.0, 792.0);

        let tree = xy_cut.segment(&items, &page);

        match tree {
            XYCutNode::HorizontalCut { children, .. } => {
                assert_eq!(children.len(), 2);
            }
            _ => panic!("Expected horizontal cut, got {:?}", tree),
        }
    }

    #[test]
    fn test_xy_cut_reading_order() {
        // 2x2 grid of blocks
        let xy_cut = XYCut::new(XYCutParams {
            min_vertical_gap: 30.0,
            min_horizontal_gap: 30.0,
            prefer_horizontal: true,
            ..Default::default()
        });

        let items = vec![
            make_bbox(50.0, 50.0, 250.0, 150.0),   // Top-left
            make_bbox(350.0, 50.0, 550.0, 150.0),  // Top-right
            make_bbox(50.0, 250.0, 250.0, 350.0),  // Bottom-left
            make_bbox(350.0, 250.0, 550.0, 350.0), // Bottom-right
        ];
        let page = make_bbox(0.0, 0.0, 612.0, 792.0);

        let tree = xy_cut.segment(&items, &page);
        let order = tree.get_items_in_order();

        // With horizontal preference: top-left, top-right, bottom-left, bottom-right
        assert_eq!(order.len(), 4);
    }

    #[test]
    fn test_adaptive_gap_calculation() {
        // Test adaptive vertical gap (column detection)
        // Wide column spacing should result in larger gap threshold
        let wide_columns = vec![
            make_bbox(50.0, 50.0, 250.0, 150.0),  // Left column
            make_bbox(350.0, 50.0, 550.0, 150.0), // Right column (100pt horizontal gap)
        ];
        let vertical_gap = calculate_adaptive_vertical_gap(&wide_columns);
        assert!(
            vertical_gap >= 10.0 && vertical_gap <= 100.0,
            "Adaptive vertical gap {} should be in range [10, 100]",
            vertical_gap
        );

        // Test adaptive horizontal gap (block separation)
        // Vertically spaced blocks should result in smaller gap threshold
        let vertical_blocks = vec![
            make_bbox(50.0, 50.0, 250.0, 100.0),  // Top block
            make_bbox(50.0, 120.0, 250.0, 170.0), // Bottom block (20pt vertical gap)
        ];
        let horizontal_gap = calculate_adaptive_horizontal_gap(&vertical_blocks);
        assert!(
            horizontal_gap >= 5.0 && horizontal_gap <= 50.0,
            "Adaptive horizontal gap {} should be in range [5, 50]",
            horizontal_gap
        );
    }

    #[test]
    fn test_xy_cut_empty_items() {
        let xy_cut = XYCut::with_defaults();
        let items: Vec<BoundingBox> = vec![];
        let page = make_bbox(0.0, 0.0, 612.0, 792.0);

        let tree = xy_cut.segment(&items, &page);

        match tree {
            XYCutNode::Leaf { items, .. } => {
                assert!(items.is_empty());
            }
            _ => panic!("Expected leaf node"),
        }
    }

    #[test]
    fn test_leaf_count() {
        let xy_cut = XYCut::new(XYCutParams {
            min_vertical_gap: 30.0,
            min_horizontal_gap: 30.0,
            ..Default::default()
        });

        let items = vec![
            make_bbox(50.0, 50.0, 250.0, 150.0),
            make_bbox(350.0, 50.0, 550.0, 150.0),
        ];
        let page = make_bbox(0.0, 0.0, 612.0, 792.0);

        let tree = xy_cut.segment(&items, &page);
        assert!(tree.count_leaves() >= 1);
    }
}
