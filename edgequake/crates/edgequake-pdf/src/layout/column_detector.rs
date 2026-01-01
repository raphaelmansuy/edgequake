//! Column detection for multi-column document layouts.

use crate::schema::BoundingBox;

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
#[derive(Debug, Clone)]
pub struct ColumnDetector {
    /// Minimum gap width to consider as column separator
    min_gap_width: f32,
    /// Minimum column width
    min_column_width: f32,
    /// Minimum vertical overlap ratio for items in same column
    min_overlap_ratio: f32,
    /// Histogram bin size for projection analysis
    bin_size: f32,
}

impl ColumnDetector {
    /// Create a new column detector with default settings.
    pub fn new() -> Self {
        Self {
            min_gap_width: 15.0,     // ~0.2 inch
            min_column_width: 100.0, // ~1.4 inch
            min_overlap_ratio: 0.5,
            bin_size: 5.0,
        }
    }

    /// Create with custom gap width.
    pub fn with_min_gap(mut self, gap: f32) -> Self {
        self.min_gap_width = gap;
        self
    }

    /// Detect columns from a list of bounding boxes.
    pub fn detect(&self, items: &[BoundingBox], page_width: f32) -> Vec<BoundingBox> {
        if items.is_empty() {
            return Vec::new();
        }

        // Filter out items that are too wide (likely headers or spanning elements)
        let filtered_items: Vec<BoundingBox> = items
            .iter()
            .filter(|bbox| bbox.width() < page_width * 0.8)
            .cloned()
            .collect();

        let items_to_use = if filtered_items.is_empty() {
            items
        } else {
            &filtered_items
        };

        // Build horizontal projection histogram
        let histogram = self.build_projection_histogram(items_to_use, page_width);

        // Find valleys (gaps) in the histogram
        let gaps = self.find_gaps(&histogram, page_width);

        // Convert gaps to columns
        self.gaps_to_columns(&gaps, items, page_width)
    }

    /// Build a histogram of horizontal projection.
    fn build_projection_histogram(&self, items: &[BoundingBox], page_width: f32) -> Vec<u32> {
        let num_bins = (page_width / self.bin_size).ceil() as usize;
        let mut histogram = vec![0u32; num_bins];

        for bbox in items {
            let start_bin = (bbox.x1 / self.bin_size).floor() as usize;
            let end_bin = ((bbox.x2 / self.bin_size).ceil() as usize).min(num_bins);

            for bin in start_bin..end_bin {
                if bin < num_bins {
                    histogram[bin] += 1;
                }
            }
        }

        histogram
    }

    /// Find gaps in the projection histogram.
    fn find_gaps(&self, histogram: &[u32], page_width: f32) -> Vec<(f32, f32)> {
        let min_gap_bins = (self.min_gap_width / self.bin_size).ceil() as usize;
        let mut gaps = Vec::new();
        let mut gap_start: Option<usize> = None;

        // Use a threshold based on average density
        let avg_count = histogram.iter().sum::<u32>() as f32 / histogram.len() as f32;
        let threshold = (avg_count * 0.1).max(0.0) as u32;

        for (i, &count) in histogram.iter().enumerate() {
            if count <= threshold {
                if gap_start.is_none() {
                    gap_start = Some(i);
                }
            } else if let Some(start) = gap_start {
                let gap_length = i - start;
                if gap_length >= min_gap_bins {
                    // Convert bin indices to coordinates
                    let x1 = start as f32 * self.bin_size;
                    let x2 = i as f32 * self.bin_size;

                    // Don't include margins as gaps
                    if x1 > self.min_column_width * 0.5
                        && x2 < page_width - self.min_column_width * 0.5
                    {
                        gaps.push((x1, x2));
                    }
                }
                gap_start = None;
            }
        }

        gaps
    }

    /// Convert gaps to column bounding boxes.
    fn gaps_to_columns(
        &self,
        gaps: &[(f32, f32)],
        items: &[BoundingBox],
        page_width: f32,
    ) -> Vec<BoundingBox> {
        if gaps.is_empty() {
            // Single column - compute from content
            return self.compute_single_column(items, page_width);
        }

        // Compute page height from items
        let page_height = items.iter().map(|b| b.y2).fold(0.0f32, |a, b| a.max(b));
        let page_top = items.iter().map(|b| b.y1).fold(f32::MAX, |a, b| a.min(b));

        let mut columns = Vec::new();
        let mut prev_x = 0.0;

        for (gap_start, gap_end) in gaps {
            // Column from previous boundary to gap start
            if *gap_start - prev_x >= self.min_column_width {
                columns.push(BoundingBox::new(prev_x, page_top, *gap_start, page_height));
            }
            prev_x = *gap_end;
        }

        // Final column
        if page_width - prev_x >= self.min_column_width {
            columns.push(BoundingBox::new(prev_x, page_top, page_width, page_height));
        }

        columns
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
        assert_eq!(columns.len(), 2, "Expected 2 columns, got {:?}", columns);

        // First column should be on left
        assert!(columns[0].x2 < columns[1].x1);
    }

    #[test]
    fn test_column_layout() {
        let detector = ColumnDetector::new();

        let items = vec![
            make_bbox(50.0, 50.0, 250.0, 100.0),
            make_bbox(350.0, 50.0, 550.0, 100.0),
        ];

        let layout = detector.analyze(&items, 612.0);

        assert!(layout.is_multi_column());
        assert_eq!(layout.count(), 2);
        assert!(layout.gutter_width > 0.0);
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
