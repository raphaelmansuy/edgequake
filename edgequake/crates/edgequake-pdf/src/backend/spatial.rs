//! Spatial indexing utilities using R-tree for O(n log n) queries.
//!
//! # WHY Spatial Indexing
//!
//! PDF extraction often requires finding nearby elements:
//! - Line intersection detection for table identification
//! - Nearby text deduplication
//! - Block overlap detection
//!
//! Naive O(n²) all-pairs comparison becomes slow on pages with 1000+ elements.
//! R-tree spatial indexing reduces this to O(n log n) by only checking
//! elements whose bounding boxes actually overlap the query region.
//!
//! # Example
//!
//! ```rust,ignore
//! use edgequake_pdf::backend::spatial::{LineSpatialIndex, LineRect};
//!
//! let lines = vec![line1, line2, line3];
//! let index = LineSpatialIndex::new(&lines);
//!
//! // Find lines near a query region
//! let nearby = index.query_region(10.0, 10.0, 100.0, 100.0);
//! ```

use rstar::{RTree, RTreeObject, AABB};

/// A line segment with R-tree envelope for spatial indexing.
///
/// WHY: R-tree requires objects to implement RTreeObject trait.
/// We wrap line data with its bounding box for efficient spatial queries.
#[derive(Debug, Clone)]
pub struct LineRect {
    /// Index into the original line array
    pub idx: usize,
    /// Line start X
    pub x1: f32,
    /// Line start Y
    pub y1: f32,
    /// Line end X
    pub x2: f32,
    /// Line end Y
    pub y2: f32,
}

impl LineRect {
    /// Create a new line rectangle from coordinates.
    pub fn new(idx: usize, x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self {
            idx,
            x1,
            y1,
            x2,
            y2,
        }
    }

    /// Expand the bounding box by a tolerance (for intersection queries).
    pub fn expanded(&self, tolerance: f32) -> AABB<[f32; 2]> {
        let min_x = self.x1.min(self.x2) - tolerance;
        let max_x = self.x1.max(self.x2) + tolerance;
        let min_y = self.y1.min(self.y2) - tolerance;
        let max_y = self.y1.max(self.y2) + tolerance;
        AABB::from_corners([min_x, min_y], [max_x, max_y])
    }
}

impl RTreeObject for LineRect {
    type Envelope = AABB<[f32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        let min_x = self.x1.min(self.x2);
        let max_x = self.x1.max(self.x2);
        let min_y = self.y1.min(self.y2);
        let max_y = self.y1.max(self.y2);
        AABB::from_corners([min_x, min_y], [max_x, max_y])
    }
}

/// Spatial index for line segments.
///
/// WHY: Finding intersecting lines is O(n²) with brute force.
/// R-tree reduces this to O(n log n) by only examining lines
/// whose bounding boxes overlap the query region.
pub struct LineSpatialIndex {
    tree: RTree<LineRect>,
}

impl LineSpatialIndex {
    /// Build a spatial index from line segments.
    ///
    /// WHY: bulk_load is O(n log n) and creates a well-balanced tree.
    pub fn new(lines: Vec<LineRect>) -> Self {
        Self {
            tree: RTree::bulk_load(lines),
        }
    }

    /// Query lines whose bounding boxes **intersect** the given region.
    ///
    /// WHY: `locate_in_envelope_intersecting` finds overlapping boxes (not containment).
    /// Returns indices of matching lines.
    pub fn query_region(&self, x1: f32, y1: f32, x2: f32, y2: f32) -> Vec<usize> {
        let query_box = AABB::from_corners([x1.min(x2), y1.min(y2)], [x1.max(x2), y1.max(y2)]);
        self.tree
            .locate_in_envelope_intersecting(&query_box)
            .map(|line| line.idx)
            .collect()
    }

    /// Query lines whose bounding boxes intersect another line's expanded bbox.
    ///
    /// WHY: For intersection detection, we expand the query region by tolerance
    /// to catch near-intersections caused by floating point imprecision.
    /// Uses `locate_in_envelope_intersecting` for overlap detection (not containment).
    pub fn query_near_line(&self, line: &LineRect, tolerance: f32) -> Vec<usize> {
        let query_box = line.expanded(tolerance);
        self.tree
            .locate_in_envelope_intersecting(&query_box)
            .filter(|other| other.idx != line.idx) // Exclude self
            .map(|other| other.idx)
            .collect()
    }

    /// Get all lines in the index.
    pub fn all_lines(&self) -> impl Iterator<Item = &LineRect> {
        self.tree.iter()
    }

    /// Number of lines in the index.
    pub fn len(&self) -> usize {
        self.tree.size()
    }

    /// Check if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.tree.size() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_rect_envelope() {
        let line = LineRect::new(0, 10.0, 20.0, 30.0, 40.0);
        let env = line.envelope();
        let lower = env.lower();
        let upper = env.upper();
        assert_eq!(lower[0], 10.0);
        assert_eq!(lower[1], 20.0);
        assert_eq!(upper[0], 30.0);
        assert_eq!(upper[1], 40.0);
    }

    #[test]
    fn test_line_rect_reversed_coords() {
        // Line with x2 < x1 and y2 < y1
        let line = LineRect::new(0, 30.0, 40.0, 10.0, 20.0);
        let env = line.envelope();
        let lower = env.lower();
        let upper = env.upper();
        assert_eq!(lower[0], 10.0); // min(30, 10)
        assert_eq!(lower[1], 20.0); // min(40, 20)
        assert_eq!(upper[0], 30.0);
        assert_eq!(upper[1], 40.0);
    }

    #[test]
    fn test_spatial_index_query() {
        let lines = vec![
            LineRect::new(0, 0.0, 0.0, 100.0, 0.0), // Horizontal at y=0
            LineRect::new(1, 0.0, 100.0, 100.0, 100.0), // Horizontal at y=100
            LineRect::new(2, 50.0, 0.0, 50.0, 100.0), // Vertical at x=50
        ];
        let index = LineSpatialIndex::new(lines);

        // Query region that overlaps all lines
        let all = index.query_region(0.0, 0.0, 100.0, 100.0);
        assert_eq!(all.len(), 3);

        // Query region that only overlaps bottom horizontal
        let bottom = index.query_region(0.0, -5.0, 100.0, 5.0);
        assert!(bottom.contains(&0));
        assert!(!bottom.contains(&1));
    }

    #[test]
    fn test_query_near_line() {
        let lines = vec![
            LineRect::new(0, 0.0, 0.0, 100.0, 0.0),   // Horizontal at y=0
            LineRect::new(1, 50.0, 0.0, 50.0, 100.0), // Vertical crossing horizontal
            LineRect::new(2, 0.0, 200.0, 100.0, 200.0), // Far horizontal at y=200
        ];
        let index = LineSpatialIndex::new(lines.clone());

        // The horizontal line (0,0)-(100,0) has bbox (-5,-5) to (105,5) when expanded
        // The vertical line (50,0)-(50,100) has bbox (50,0) to (50,100)
        // These bboxes overlap: x=50 is in 0-100, and y ranges (−5,5) ∩ (0,100) = (0,5) ≠ ∅
        let near = index.query_near_line(&lines[0], 5.0);

        assert!(
            near.contains(&1),
            "Vertical line should be near horizontal line"
        );
        assert!(!near.contains(&2), "Far horizontal should NOT be near");
    }

    #[test]
    fn test_empty_index() {
        let index = LineSpatialIndex::new(vec![]);
        assert!(index.is_empty());
        assert_eq!(index.len(), 0);
        assert!(index.query_region(0.0, 0.0, 100.0, 100.0).is_empty());
    }

    // Additional spatial tests for Phase 4.1

    #[test]
    fn test_single_line_index() {
        let lines = vec![LineRect::new(0, 10.0, 10.0, 50.0, 10.0)];
        let index = LineSpatialIndex::new(lines);

        assert_eq!(index.len(), 1);
        assert!(!index.is_empty());
    }

    #[test]
    fn test_query_no_matches() {
        let lines = vec![
            LineRect::new(0, 0.0, 0.0, 10.0, 0.0),
            LineRect::new(1, 0.0, 10.0, 10.0, 10.0),
        ];
        let index = LineSpatialIndex::new(lines);

        // Query region far from all lines
        let result = index.query_region(1000.0, 1000.0, 2000.0, 2000.0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_line_rect_expanded() {
        let line = LineRect::new(0, 10.0, 20.0, 30.0, 40.0);
        let expanded = line.expanded(5.0);
        let lower = expanded.lower();
        let upper = expanded.upper();

        assert_eq!(lower[0], 5.0); // 10 - 5
        assert_eq!(lower[1], 15.0); // 20 - 5
        assert_eq!(upper[0], 35.0); // 30 + 5
        assert_eq!(upper[1], 45.0); // 40 + 5
    }

    #[test]
    fn test_all_lines_iterator() {
        let lines = vec![
            LineRect::new(0, 0.0, 0.0, 10.0, 0.0),
            LineRect::new(1, 20.0, 20.0, 30.0, 20.0),
        ];
        let index = LineSpatialIndex::new(lines);

        let all: Vec<_> = index.all_lines().collect();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_overlapping_lines_query() {
        // Two overlapping horizontal lines
        let lines = vec![
            LineRect::new(0, 0.0, 10.0, 100.0, 10.0),
            LineRect::new(1, 50.0, 10.0, 150.0, 10.0), // Overlaps 0 at x=50-100
        ];
        let index = LineSpatialIndex::new(lines.clone());

        let near = index.query_near_line(&lines[0], 2.0);
        assert!(
            near.contains(&1),
            "Overlapping lines should be near each other"
        );
    }

    #[test]
    fn test_query_self_excluded() {
        let lines = vec![
            LineRect::new(0, 0.0, 0.0, 100.0, 0.0),
            LineRect::new(1, 0.0, 0.0, 100.0, 0.0), // Same position, different idx
        ];
        let index = LineSpatialIndex::new(lines.clone());

        // When querying near line 0, line 0 should NOT be in results
        let near = index.query_near_line(&lines[0], 5.0);
        assert!(!near.contains(&0), "Self should be excluded from results");
        assert!(
            near.contains(&1),
            "Other overlapping line should be included"
        );
    }

    #[test]
    fn test_vertical_line_envelope() {
        // Vertical line: x1 == x2
        let line = LineRect::new(0, 50.0, 0.0, 50.0, 100.0);
        let env = line.envelope();
        let lower = env.lower();
        let upper = env.upper();

        assert_eq!(lower[0], 50.0);
        assert_eq!(lower[1], 0.0);
        assert_eq!(upper[0], 50.0);
        assert_eq!(upper[1], 100.0);
    }

    #[test]
    fn test_point_line() {
        // Degenerate line: single point
        let line = LineRect::new(0, 50.0, 50.0, 50.0, 50.0);
        let env = line.envelope();

        // Should still have valid envelope (point)
        assert_eq!(env.lower()[0], 50.0);
        assert_eq!(env.upper()[0], 50.0);
    }

    #[test]
    fn test_large_index_performance() {
        // Create 100 lines
        let lines: Vec<LineRect> = (0..100)
            .map(|i| LineRect::new(i, i as f32 * 10.0, 0.0, i as f32 * 10.0 + 100.0, 0.0))
            .collect();

        let index = LineSpatialIndex::new(lines);
        assert_eq!(index.len(), 100);

        // Query should still work efficiently
        let result = index.query_region(0.0, -5.0, 1000.0, 5.0);
        assert!(!result.is_empty());
    }
}
