//! Layout detection and analysis module.
//!
//! This module provides algorithms for detecting document layout:
//! - XY-cut algorithm for recursive document segmentation
//! - Column detection for multi-column layouts
//! - Geometric clustering for spatial analysis (DBSCAN)
//! - Reading order determination
//! - Margin detection
//! - Block classification (header, list, code, paragraph)
//! - PyMuPDF4LLM-inspired text structures (Span, Line, Block)
//!
//! ## Module Organization (OODA-45 SRP)
//!
//! ```text
//! layout/
//! ├── mod.rs              ← This file: public exports
//! ├── block_classifier.rs ← Block type detection (header/list/code)
//! ├── column_detector.rs  ← Multi-column layout detection
//! ├── reading_order.rs    ← Reading order algorithms
//! ├── pymupdf_grouper.rs  ← Core text grouping (chars→spans→lines→blocks)
//! ├── pymupdf_renderer.rs ← Markdown rendering
//! ├── pymupdf_structs.rs  ← Data structures (Span, Line, Block)
//! ├── geometric.rs        ← DBSCAN clustering utilities
//! └── xy_cut.rs           ← XY-cut segmentation algorithm
//! ```

mod block_classifier;
mod column_detector;
pub mod footnote;
mod geometric;
pub mod hyphenation;
pub mod list_hierarchy;
pub mod page_filter;
mod pymupdf_grouper;
mod pymupdf_renderer;
pub mod pymupdf_structs; // OODA-43: Made public for pdfium_backend imports
pub mod quality_metrics; // OODA-47: Quality metrics (CLF, SPS, ROA, NR)
mod reading_order;
mod xy_cut;

// OODA-45: Export block classification functions for DRY compliance
pub use block_classifier::{
    is_all_caps_header, is_bullet_item, is_caption, is_numbered_list_item, BlockClassifier,
};
pub use column_detector::{ColumnDetector, ColumnLayout};
pub use geometric::{dbscan_1d, Cluster, Column, GeometricClusterer};
pub use pymupdf_grouper::{GroupingParams, TextGrouper};
pub use pymupdf_renderer::{MarkdownConfig, MarkdownRenderer};
pub use pymupdf_structs::{Block as TextBlock, BlockType, Line, Span};
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
/// @implements FEAT0502
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

        // OODA-22: Calculate actual confidence instead of hardcoded 0.9
        let confidence =
            self.calculate_confidence(blocks.len(), &columns, &reading_order, &regions);

        LayoutAnalysis {
            columns,
            regions,
            reading_order,
            margins,
            confidence,
        }
    }

    /// Calculate layout analysis confidence score.
    ///
    /// WHY: Confidence indicates how reliable the layout analysis is.
    /// A low confidence suggests the extraction may have issues.
    ///
    /// Factors (weighted average):
    /// - **Reading order coverage (50%)**: All blocks should be in reading order
    /// - **Column detection (30%)**: Single column = confident, multi = slightly less
    /// - **Region quality (20%)**: XY-cut should produce reasonable region count
    fn calculate_confidence(
        &self,
        block_count: usize,
        columns: &[BoundingBox],
        reading_order: &[usize],
        regions: &[LayoutRegion],
    ) -> f32 {
        // 1. Reading order coverage: all blocks should appear in reading order
        let order_coverage = if block_count == 0 {
            1.0
        } else {
            (reading_order.len() as f32 / block_count as f32).min(1.0)
        };

        // 2. Column detection confidence
        // Single column is always confident (1.0)
        // Multi-column has slight uncertainty (0.95) due to boundary detection
        let column_confidence = if columns.is_empty() || columns.len() == 1 {
            1.0
        } else {
            0.95
        };

        // 3. Region quality from XY-cut
        // - No regions: 0.8 (likely issue with XY-cut)
        // - Over-fragmented (>2x blocks): 0.7 (too many splits)
        // - Reasonable: 1.0
        let region_confidence = if regions.is_empty() {
            0.8
        } else if block_count > 0 && regions.len() > block_count * 2 {
            0.7
        } else {
            1.0
        };

        // Weighted average (reading order is most critical)
        (order_coverage * 0.5 + column_confidence * 0.3 + region_confidence * 0.2).clamp(0.0, 1.0)
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

    // OODA-22: Test confidence calculation

    #[test]
    fn test_confidence_calculation_perfect() {
        let analyzer = LayoutAnalyzer::new();
        // Perfect case: all blocks in reading order, single column, reasonable regions
        let block_count = 5;
        let columns = vec![]; // Single column
        let reading_order = vec![0, 1, 2, 3, 4]; // All 5 blocks
        let regions = vec![
            LayoutRegion::new(
                BoundingBox::new(0.0, 0.0, 100.0, 100.0),
                RegionType::TextBody,
            ),
            LayoutRegion::new(
                BoundingBox::new(0.0, 100.0, 100.0, 200.0),
                RegionType::TextBody,
            ),
        ];

        let confidence =
            analyzer.calculate_confidence(block_count, &columns, &reading_order, &regions);
        assert!(
            confidence >= 0.95,
            "Perfect case should have high confidence"
        );
    }

    #[test]
    fn test_confidence_calculation_missing_blocks() {
        let analyzer = LayoutAnalyzer::new();
        // Missing blocks: only 3 of 5 in reading order
        let block_count = 5;
        let columns = vec![];
        let reading_order = vec![0, 1, 2]; // Only 3 of 5
        let regions = vec![];

        let confidence =
            analyzer.calculate_confidence(block_count, &columns, &reading_order, &regions);
        // order_coverage = 0.6, column = 1.0, region = 0.8
        // weighted = 0.6*0.5 + 1.0*0.3 + 0.8*0.2 = 0.3 + 0.3 + 0.16 = 0.76
        assert!(
            confidence < 0.8,
            "Missing blocks should reduce confidence: {}",
            confidence
        );
    }

    #[test]
    fn test_confidence_calculation_empty() {
        let analyzer = LayoutAnalyzer::new();
        // Empty case: no blocks
        let confidence = analyzer.calculate_confidence(0, &[], &[], &[]);
        // order = 1.0, column = 1.0, region = 0.8 (empty is penalized slightly)
        // weighted = 0.5 + 0.3 + 0.16 = 0.96
        assert!(
            confidence >= 0.95,
            "Empty blocks should have high confidence: {}",
            confidence
        );
    }
}
