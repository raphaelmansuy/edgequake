//! Layout detection and analysis module.
//!
//! This module provides algorithms for detecting document layout:
//! - XY-cut algorithm for recursive document segmentation
//! - Column detection for multi-column layouts
//! - Reading order determination
//! - Margin detection

mod column_detector;
mod reading_order;
mod xy_cut;

pub use column_detector::{ColumnDetector, ColumnLayout};
pub use reading_order::{ReadingOrder, ReadingOrderDetector};
pub use xy_cut::{XYCut, XYCutNode, XYCutParams};

use crate::schema::{Block, BoundingBox};

/// Layout analysis results for a page.
#[derive(Debug, Clone)]
pub struct LayoutAnalysis {
    /// Detected columns (if multi-column layout)
    pub columns: Vec<BoundingBox>,
    /// Page regions identified by XY-cut
    pub regions: Vec<LayoutRegion>,
    /// Reading order of blocks
    pub reading_order: Vec<usize>,
    /// Detected page margins
    pub margins: PageMargins,
    /// Layout confidence score
    pub confidence: f32,
}

impl LayoutAnalysis {
    /// Create a new layout analysis.
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
            regions: Vec::new(),
            reading_order: Vec::new(),
            margins: PageMargins::default(),
            confidence: 1.0,
        }
    }

    /// Get number of columns.
    pub fn column_count(&self) -> usize {
        self.columns.len().max(1)
    }

    /// Check if layout is multi-column.
    pub fn is_multi_column(&self) -> bool {
        self.columns.len() > 1
    }
}

impl Default for LayoutAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

/// A region in the document layout.
#[derive(Debug, Clone)]
pub struct LayoutRegion {
    /// Bounding box of the region
    pub bbox: BoundingBox,
    /// Region type
    pub region_type: RegionType,
    /// Child regions (for nested layouts)
    pub children: Vec<LayoutRegion>,
    /// Reading order position
    pub order: usize,
}

impl LayoutRegion {
    /// Create a new region.
    pub fn new(bbox: BoundingBox, region_type: RegionType) -> Self {
        Self {
            bbox,
            region_type,
            children: Vec::new(),
            order: 0,
        }
    }
}

/// Types of layout regions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionType {
    /// Main text area
    TextBody,
    /// Column within text body
    Column,
    /// Header area
    Header,
    /// Footer area
    Footer,
    /// Sidebar
    Sidebar,
    /// Figure/image area
    Figure,
    /// Table area
    Table,
    /// Margin note
    MarginNote,
}

/// Page margins.
#[derive(Debug, Clone, Copy, Default)]
pub struct PageMargins {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl PageMargins {
    /// Create uniform margins.
    pub fn uniform(margin: f32) -> Self {
        Self {
            top: margin,
            right: margin,
            bottom: margin,
            left: margin,
        }
    }

    /// Create margins from content bounds within page.
    pub fn from_content_bounds(page_width: f32, page_height: f32, content: &BoundingBox) -> Self {
        Self {
            top: content.y1,
            left: content.x1,
            right: page_width - content.x2,
            bottom: page_height - content.y2,
        }
    }

    /// Get the content area given page dimensions.
    pub fn content_area(&self, page_width: f32, page_height: f32) -> BoundingBox {
        BoundingBox::new(
            self.left,
            self.top,
            page_width - self.right,
            page_height - self.bottom,
        )
    }
}

/// Layout analyzer for processing page blocks.
pub struct LayoutAnalyzer {
    /// Column detector
    column_detector: ColumnDetector,
    /// Reading order detector
    reading_order_detector: ReadingOrderDetector,
    /// XY-cut parameters
    xy_cut_params: XYCutParams,
}

impl LayoutAnalyzer {
    /// Create a new layout analyzer with default settings.
    pub fn new() -> Self {
        Self {
            column_detector: ColumnDetector::new(),
            reading_order_detector: ReadingOrderDetector::new(),
            xy_cut_params: XYCutParams::default(),
        }
    }

    /// Create with custom XY-cut parameters.
    pub fn with_xy_cut_params(mut self, params: XYCutParams) -> Self {
        self.xy_cut_params = params;
        self
    }

    /// Get a reference to the column detector.
    pub fn column_detector(&self) -> &ColumnDetector {
        &self.column_detector
    }
    /// Analyze layout of blocks on a page.
    pub fn analyze(&self, blocks: &[Block], page_width: f32, page_height: f32) -> LayoutAnalysis {
        if blocks.is_empty() {
            return LayoutAnalysis::default();
        }

        // Get bounding boxes for layout analysis
        let bboxes: Vec<BoundingBox> = blocks.iter().map(|b| b.bbox).collect();

        // Detect margins from content bounds
        let content_bounds = BoundingBox::union_all(&bboxes).unwrap_or_default();
        let margins = PageMargins::from_content_bounds(page_width, page_height, &content_bounds);

        // Detect columns
        let columns = self.column_detector.detect(&bboxes, page_width);

        // Determine reading order
        let reading_order = self
            .reading_order_detector
            .determine_order(blocks, &columns);

        // Run XY-cut for region detection
        let regions = self.detect_regions(&bboxes, page_width, page_height);

        LayoutAnalysis {
            columns,
            regions,
            reading_order,
            margins,
            confidence: 0.9, // TODO: calculate actual confidence
        }
    }

    /// Detect layout regions using XY-cut algorithm.
    fn detect_regions(
        &self,
        bboxes: &[BoundingBox],
        page_width: f32,
        page_height: f32,
    ) -> Vec<LayoutRegion> {
        let page_bbox = BoundingBox::new(0.0, 0.0, page_width, page_height);
        let xy_cut = XYCut::new(self.xy_cut_params.clone());
        let tree = xy_cut.segment(bboxes, &page_bbox);

        // Convert XY-cut tree to layout regions
        self.tree_to_regions(&tree, 0)
    }

    /// Convert XY-cut tree to layout regions.
    fn tree_to_regions(&self, node: &XYCutNode, order: usize) -> Vec<LayoutRegion> {
        let mut regions = Vec::new();
        let mut current_order = order;

        match node {
            XYCutNode::Leaf { bbox, items } => {
                if !items.is_empty() {
                    let mut region = LayoutRegion::new(*bbox, RegionType::TextBody);
                    region.order = current_order;
                    regions.push(region);
                }
            }
            XYCutNode::HorizontalCut { children, .. } | XYCutNode::VerticalCut { children, .. } => {
                for child in children {
                    let child_regions = self.tree_to_regions(child, current_order);
                    current_order += child_regions.len();
                    regions.extend(child_regions);
                }
            }
        }

        regions
    }

    /// Sort blocks by reading order.
    pub fn sort_by_reading_order(&self, blocks: &mut [Block], columns: &[BoundingBox]) {
        let reading_order = self.reading_order_detector.determine_order(blocks, columns);

        // Create position map
        let mut position_map: Vec<(usize, usize)> = reading_order
            .iter()
            .enumerate()
            .map(|(order, &orig)| (orig, order))
            .collect();
        position_map.sort_by_key(|&(orig, _)| orig);

        // Update block positions
        for (orig_idx, (_, new_order)) in position_map.into_iter().enumerate() {
            if orig_idx < blocks.len() {
                blocks[orig_idx].position = new_order;
            }
        }

        // Sort by new position
        blocks.sort_by_key(|b| b.position);
    }
}

impl Default for LayoutAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_analysis_default() {
        let analysis = LayoutAnalysis::default();
        assert_eq!(analysis.column_count(), 1);
        assert!(!analysis.is_multi_column());
    }

    #[test]
    fn test_page_margins() {
        let margins = PageMargins {
            top: 72.0,
            right: 72.0,
            bottom: 72.0,
            left: 72.0,
        };

        let content = margins.content_area(612.0, 792.0);
        assert_eq!(content.x1, 72.0);
        assert_eq!(content.y1, 72.0);
        assert_eq!(content.x2, 540.0);
        assert_eq!(content.y2, 720.0);
    }

    #[test]
    fn test_layout_analyzer_empty_blocks() {
        let analyzer = LayoutAnalyzer::new();
        let analysis = analyzer.analyze(&[], 612.0, 792.0);
        assert!(analysis.columns.is_empty());
        assert!(analysis.reading_order.is_empty());
    }

    #[test]
    fn test_margins_from_content_bounds() {
        let content = BoundingBox::new(50.0, 40.0, 560.0, 750.0);
        let margins = PageMargins::from_content_bounds(612.0, 792.0, &content);

        assert_eq!(margins.left, 50.0);
        assert_eq!(margins.top, 40.0);
        assert_eq!(margins.right, 52.0);
        assert_eq!(margins.bottom, 42.0);
    }
}
