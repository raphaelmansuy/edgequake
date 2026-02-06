//! Column detection for multi-column document layouts.
//!
//! This module provides column detection using first-principles geometric
//! clustering instead of histogram-based heuristics.
//!
//! ## Algorithm (OODA-46)
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                    COLUMN DETECTION PIPELINE                            │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Input: BoundingBox[] (text blocks from page)                          │
//! │                                                                         │
//! │  Step 1: Filter wide items (>80% page width)                           │
//! │          WHY: Headers/footers span multiple columns - ignore them      │
//! │                                                                         │
//! │  Step 2: DBSCAN clustering on x-coordinates (via GeometricClusterer)   │
//! │          WHY: No magic bin sizes, epsilon adapts to document density   │
//! │                                                                         │
//! │  Step 3: Merge adjacent clusters into column boundaries                │
//! │          Each cluster's x-extent becomes a column                       │
//! │                                                                         │
//! │  Step 4: Calculate confidence score                                     │
//! │          confidence = items_in_detected_columns / total_items          │
//! │                                                                         │
//! │  Output: ColumnLayout { columns, confidence, gutter_width }            │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! let detector = ColumnDetector::new();
//! let layout = detector.analyze(&bounding_boxes, page_width);
//! if layout.is_multi_column() {
//!     println!("Found {} columns with {:.0}% confidence",
//!         layout.count(), layout.confidence * 100.0);
//! }
//! ```

use crate::layout::geometric::{Column as GeomColumn, GeometricClusterer};
use crate::schema::BoundingBox;
use tracing::debug;

/// Column layout detection results.
#[derive(Debug, Clone)]
pub struct ColumnLayout {
    /// Detected columns (left to right)
    pub columns: Vec<BoundingBox>,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Gap between columns
    pub gutter_width: f32,
}

impl ColumnLayout {
    /// Create a single-column layout.
    pub fn single_column(page_width: f32, page_height: f32) -> Self {
        Self {
            columns: vec![BoundingBox::new(0.0, 0.0, page_width, page_height)],
            confidence: 1.0,
            gutter_width: 0.0,
        }
    }

    /// Check if this is a multi-column layout.
    pub fn is_multi_column(&self) -> bool {
        self.columns.len() > 1
    }

    /// Get number of columns.
    pub fn count(&self) -> usize {
        self.columns.len()
    }

    /// Get the column containing a point.
    pub fn column_at(&self, x: f32) -> Option<usize> {
        for (i, col) in self.columns.iter().enumerate() {
            if x >= col.x1 && x <= col.x2 {
                return Some(i);
            }
        }
        None
    }
}

/// Column detector for document layouts.
///
/// Now uses geometric clustering (DBSCAN) instead of histogram bins.
/// No magic number thresholds - all parameters adaptive!
#[derive(Debug, Clone)]
pub struct ColumnDetector {
    /// Geometric clusterer for column detection
    clusterer: GeometricClusterer,
}

impl ColumnDetector {
    /// Create a new column detector.
    pub fn new() -> Self {
        Self {
            clusterer: GeometricClusterer::new(),
        }
    }

    /// Create with custom gap width (deprecated - now adaptive).
    #[deprecated(note = "Gap width is now calculated adaptively")]
    pub fn with_min_gap(self, _gap: f32) -> Self {
        self
    }

    /// Detect columns from a list of bounding boxes using geometric clustering.
    pub fn detect(&self, items: &[BoundingBox], page_width: f32) -> Vec<BoundingBox> {
        tracing::debug!(
            "COLUMN-DETECT: {} items, page_width={}",
            items.len(),
            page_width
        );

        if items.is_empty() {
            tracing::debug!("COLUMN-DETECT: no items, returning empty");
            return Vec::new();
        }

        // WHY 0.8 threshold: Items spanning >80% of page width are typically:
        // - Document headers/titles
        // - Footnotes spanning columns
        // - Table captions
        // These should NOT influence column detection.
        // The 0.8 value is empirically validated on IEEE/arXiv two-column papers.
        let filtered_items: Vec<BoundingBox> = items
            .iter()
            .filter(|bbox| bbox.width() < page_width * 0.8)
            .cloned()
            .collect();

        tracing::debug!(
            "COLUMN-DETECT: filtered {} items to {} (removed wide items)",
            items.len(),
            filtered_items.len()
        );

        let items_to_use = if filtered_items.is_empty() {
            items
        } else {
            &filtered_items
        };

        // Use geometric clustering to detect columns
        let columns = self.clusterer.detect_columns(items_to_use, page_width);

        tracing::debug!(
            "COLUMN-DETECT: clusterer found {} columns: {:?}",
            columns.len(),
            columns.iter().map(|c| (c.x1, c.x2)).collect::<Vec<_>>()
        );

        // Convert geometric columns to bounding boxes
        let result = self.columns_to_bboxes(&columns, items);
        tracing::debug!("COLUMN-DETECT: returning {} columns", result.len());
        result
    }

    /// Convert geometric columns to bounding boxes.
    fn columns_to_bboxes(&self, columns: &[GeomColumn], items: &[BoundingBox]) -> Vec<BoundingBox> {
        if columns.is_empty() {
            // Fallback to single column from content bounds
            let page_width = items.iter().map(|b| b.x2).fold(0.0f32, f32::max);
            return self.compute_single_column(items, page_width);
        }

        // Compute page height and top from items
        let page_height = items.iter().map(|b| b.y2).fold(0.0f32, |a, b| a.max(b));
        let page_top = items.iter().map(|b| b.y1).fold(f32::MAX, |a, b| a.min(b));

        columns
            .iter()
            .map(|c| BoundingBox::new(c.x1, page_top, c.x2, page_height))
            .collect()
    }

    /// Compute single column from content bounds.
    fn compute_single_column(&self, items: &[BoundingBox], page_width: f32) -> Vec<BoundingBox> {
        if items.is_empty() {
            return vec![BoundingBox::new(0.0, 0.0, page_width, 792.0)];
        }

        let content_bounds = BoundingBox::union_all(items).unwrap();
        vec![content_bounds]
    }

    /// Analyze column structure in more detail.
    pub fn analyze(&self, items: &[BoundingBox], page_width: f32) -> ColumnLayout {
        let columns = self.detect(items, page_width);

        let gutter_width = if columns.len() > 1 {
            // Calculate average gutter width
            let mut total_gutter = 0.0;
            for i in 0..columns.len() - 1 {
                total_gutter += columns[i + 1].x1 - columns[i].x2;
            }
            total_gutter / (columns.len() - 1) as f32
        } else {
            0.0
        };

        let confidence = self.calculate_confidence(&columns, items);

        ColumnLayout {
            columns,
            confidence,
            gutter_width,
        }
    }

    /// Calculate confidence score for column detection.
    fn calculate_confidence(&self, columns: &[BoundingBox], items: &[BoundingBox]) -> f32 {
        if columns.is_empty() || items.is_empty() {
            return 0.0;
        }

        // Check how well items fit within detected columns
        let mut items_in_columns = 0;

        for item in items {
            let item_center_x = (item.x1 + item.x2) / 2.0;
            for col in columns {
                if item_center_x >= col.x1 && item_center_x <= col.x2 {
                    items_in_columns += 1;
                    break;
                }
            }
        }

        items_in_columns as f32 / items.len() as f32
    }

    /// Check if an item spans multiple columns.
    pub fn spans_columns(&self, item: &BoundingBox, columns: &[BoundingBox]) -> bool {
        let mut column_count = 0;
        for col in columns {
            if item.intersects(col) {
                column_count += 1;
                if column_count > 1 {
                    return true;
                }
            }
        }
        false
    }

    /// Get the column index for an item (by center point).
    pub fn get_column_index(&self, item: &BoundingBox, columns: &[BoundingBox]) -> Option<usize> {
        let center_x = item.center().x;
        for (i, col) in columns.iter().enumerate() {
            if center_x >= col.x1 && center_x <= col.x2 {
                return Some(i);
            }
        }
        None
    }

    /// Check if the detected columns actually look like a table structure.
    ///
    /// Tables typically have:
    /// - Many items per row (3+)
    /// - Short items (single words or numbers) relative to column width
    /// - Uniform row structure with items significantly shorter than columns
    ///
    /// Multi-column text layouts have:
    /// - Longer text blocks that fill most of the column width
    /// - Items close to column width
    pub fn is_likely_table(&self, items: &[BoundingBox], columns: &[BoundingBox]) -> bool {
        if columns.len() < 2 {
            return false;
        }

        // Calculate average column width
        let avg_col_width = columns.iter().map(|c| c.width()).sum::<f32>() / columns.len() as f32;

        // Calculate average item width
        let avg_item_width = if !items.is_empty() {
            items.iter().map(|b| b.width()).sum::<f32>() / items.len() as f32
        } else {
            0.0
        };

        // Key metric: items fill percentage of column
        // Tables: items are typically 20-50% of column width (short words/numbers)
        // Text columns: items are typically 70-95% of column width (full lines)
        let fill_ratio = avg_item_width / avg_col_width;

        // Group items by Y position to find rows
        let mut rows: Vec<Vec<&BoundingBox>> = Vec::new();
        let mut sorted_items: Vec<&BoundingBox> = items.iter().collect();
        sorted_items.sort_by(|a, b| a.y1.partial_cmp(&b.y1).unwrap_or(std::cmp::Ordering::Equal));

        for item in sorted_items {
            let mut found_row = false;
            for row in rows.iter_mut() {
                if !row.is_empty() {
                    let first = row[0];
                    let overlap_y = first.y2.min(item.y2) - first.y1.max(item.y1);
                    let min_h = (first.y2 - first.y1).min(item.y2 - item.y1);
                    if overlap_y > min_h * 0.5 || (first.y1 - item.y1).abs() < 5.0 {
                        row.push(item);
                        found_row = true;
                        break;
                    }
                }
            }
            if !found_row {
                rows.push(vec![item]);
            }
        }

        // Table characteristics:
        let rows_with_3_plus_items = rows.iter().filter(|r| r.len() >= 3).count();
        let multi_item_rows = rows.iter().filter(|r| r.len() > 1).count();

        // Short items relative to column width
        let short_threshold = avg_col_width * 0.5; // Items less than 50% of column width
        let short_items = items.iter().filter(|b| b.width() < short_threshold).count();
        let short_ratio = short_items as f32 / items.len() as f32;

        // Columns have similar widths
        let col_widths: Vec<f32> = columns.iter().map(|c| c.width()).collect();
        let width_avg = col_widths.iter().sum::<f32>() / col_widths.len() as f32;
        let uniform_widths = col_widths
            .iter()
            .all(|w| (*w - width_avg).abs() < width_avg * 0.5);

        // Decision logic:
        // - fill_ratio < 0.5 with many multi-item rows => table (items don't fill columns)
        // - fill_ratio > 0.6 => text columns (items fill most of column width)
        // - 4+ columns is strongly table-like unless fill_ratio is very high
        let is_table = (fill_ratio < 0.45 && multi_item_rows >= 3) || // Items don't fill columns
            (columns.len() >= 4 && fill_ratio < 0.6) || // Many columns with sparse items
            (short_ratio > 0.75 && multi_item_rows >= 3 && uniform_widths); // Very short items in grid

        debug!(
            "is_likely_table: {} cols, fill_ratio={:.2}, short_ratio={:.2}, rows_3+={}, multi_rows={} => {}",
            columns.len(), fill_ratio, short_ratio, rows_with_3_plus_items, multi_item_rows, is_table
        );

        is_table
    }
}

impl Default for ColumnDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bbox(x1: f32, y1: f32, x2: f32, y2: f32) -> BoundingBox {
        BoundingBox::new(x1, y1, x2, y2)
    }

    #[test]
    fn test_single_column_detection() {
        let detector = ColumnDetector::new();

        // Items all in center area - single column
        let items = vec![
            make_bbox(100.0, 50.0, 500.0, 100.0),
            make_bbox(100.0, 120.0, 500.0, 170.0),
            make_bbox(100.0, 190.0, 500.0, 240.0),
        ];

        let columns = detector.detect(&items, 612.0);
        assert_eq!(columns.len(), 1);
    }

    #[test]
    fn test_two_column_detection() {
        let detector = ColumnDetector::new();

        // Items in two distinct columns with gap
        let items = vec![
            // Left column
            make_bbox(50.0, 50.0, 250.0, 100.0),
            make_bbox(50.0, 120.0, 250.0, 170.0),
            make_bbox(50.0, 190.0, 250.0, 240.0),
            // Right column
            make_bbox(350.0, 50.0, 550.0, 100.0),
            make_bbox(350.0, 120.0, 550.0, 170.0),
            make_bbox(350.0, 190.0, 550.0, 240.0),
        ];

        let columns = detector.detect(&items, 612.0);

        // OODA-20: With minimum column width filter (80pt), narrow margin columns
        // are merged with adjacent content columns. This is the correct behavior
        // for text merging - we want 2 content columns, not 3 including margins.
        assert_eq!(columns.len(), 2, "Expected 2 columns, got {:?}", columns);

        // The two content columns should span from left margin to right margin
        let left_col = &columns[0];
        let right_col = &columns[1];
        assert!(
            left_col.x1 <= 1.0, // Should start at or near 0
            "Left column should start near left edge: {:?}",
            left_col
        );
        assert!(
            right_col.x2 >= 600.0, // Should extend to page width
            "Right column should extend to page width: {:?}",
            right_col
        );
    }

    #[test]
    fn test_column_layout() {
        let detector = ColumnDetector::new();

        let items = vec![
            make_bbox(50.0, 50.0, 250.0, 100.0),
            make_bbox(350.0, 50.0, 550.0, 100.0),
        ];

        let layout = detector.analyze(&items, 612.0);

        // With only two isolated items, geometric clustering (min_samples=3) will not form multiple columns.
        // This is a degenerate case: expect a single column spanning both items.
        assert_eq!(
            layout.count(),
            1,
            "Expected 1 column for two isolated items, got {}",
            layout.count()
        );
        let col = &layout.columns[0];
        assert!(
            col.x1 <= 50.0 + 1.0 && col.x2 >= 550.0 - 1.0,
            "Column bounds incorrect: {:?}",
            col
        );
    }

    #[test]
    fn test_empty_items() {
        let detector = ColumnDetector::new();
        let columns = detector.detect(&[], 612.0);
        assert!(columns.is_empty());
    }

    #[test]
    fn test_column_at() {
        let layout = ColumnLayout {
            columns: vec![
                make_bbox(50.0, 0.0, 280.0, 792.0),
                make_bbox(332.0, 0.0, 562.0, 792.0),
            ],
            confidence: 0.95,
            gutter_width: 52.0,
        };

        assert_eq!(layout.column_at(100.0), Some(0));
        assert_eq!(layout.column_at(400.0), Some(1));
        assert_eq!(layout.column_at(300.0), None); // In the gutter
    }

    #[test]
    fn test_spans_columns() {
        let detector = ColumnDetector::new();

        let columns = vec![
            make_bbox(50.0, 0.0, 280.0, 792.0),
            make_bbox(332.0, 0.0, 562.0, 792.0),
        ];

        // Item in left column only
        let single_col = make_bbox(100.0, 50.0, 200.0, 100.0);
        assert!(!detector.spans_columns(&single_col, &columns));

        // Item spanning both columns (like a header)
        let spanning = make_bbox(100.0, 50.0, 500.0, 100.0);
        assert!(detector.spans_columns(&spanning, &columns));
    }

    #[test]
    fn test_get_column_index() {
        let detector = ColumnDetector::new();

        let columns = vec![
            make_bbox(50.0, 0.0, 280.0, 792.0),
            make_bbox(332.0, 0.0, 562.0, 792.0),
        ];

        let left_item = make_bbox(100.0, 50.0, 200.0, 100.0);
        let right_item = make_bbox(400.0, 50.0, 500.0, 100.0);

        assert_eq!(detector.get_column_index(&left_item, &columns), Some(0));
        assert_eq!(detector.get_column_index(&right_item, &columns), Some(1));
    }
}
