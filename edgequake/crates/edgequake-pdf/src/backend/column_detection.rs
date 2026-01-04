//! Column detection for PDF layouts.
//!
//! This module handles detecting multi-column layouts using vertical projection histograms.
//! The XY-Cut algorithm with adaptive thresholds is used to find column boundaries.
//!
//! **WHY projection histogram approach:**
//! - Academic papers commonly use two-column layout with a "gutter" in the middle
//! - Vertical projection histograms count text elements per horizontal bin
//! - A gap in the histogram (near page center) indicates the column separator
//! - This is more robust than x-position clustering for varying column widths

use super::elements::TextElement;
use tracing::info;

/// Column detection engine using vertical projection histograms
pub struct ColumnDetector {
    /// Bin size for projection histogram (in points)
    bin_size: f32,
    /// Minimum gap width in bins to be considered a column separator
    min_gap_bins: usize,
}

impl ColumnDetector {
    pub fn new() -> Self {
        Self {
            bin_size: 5.0,   // 5pt bins for fine granularity
            min_gap_bins: 4, // Minimum 4 bins (20pt) for column separator
        }
    }

    /// Detect if page has two-column layout using projection histogram.
    /// Returns Some(column_boundary_x) if two-column layout detected, None otherwise.
    pub fn detect_columns(&self, elements: &[TextElement], page_width: f32) -> Option<f32> {
        if elements.len() < 10 {
            debug!("Too few elements ({}) for column detection", elements.len());
            return None;
        }

        debug!(
            "Column detection: {} elements, page_width={:.1}",
            elements.len(),
            page_width
        );

        // Use projection histogram approach from spec_algo.md
        let proj = self.compute_vertical_projection(elements, page_width);

        // Find gaps - minimum gap of 4 bins (20pt) for column separator
        let gaps = self.find_projection_gaps(&proj);
        debug!("Projection gaps found: {:?}", gaps);

        // Look for a gap near the center of the page (column boundary)
        // In academic papers, the gutter is typically around 45-55% of page width
        let center = page_width / 2.0;
        let center_range = page_width * 0.15; // ±15% from center

        let center_gap = gaps
            .iter()
            .find(|&&gap| (gap - center).abs() < center_range);

        if let Some(&boundary) = center_gap {
            // Verify with element distribution
            // The gap position is the START of the gap (whitespace column).
            // Text in the right column starts AFTER the gap, not at the gap.
            // Use asymmetric thresholds: elements ending before gap = left, elements starting after gap = right
            // For now, use boundary as the rough separation point with wider margin

            // Find elements that are clearly in left column (well before boundary)
            // and elements that are in right column (at or after boundary)
            // A gap at X means content is sparse there - left column ends before X, right column starts at or after X
            let left_count = elements.iter().filter(|e| e.x < boundary).count();
            let right_count = elements.iter().filter(|e| e.x >= boundary).count();

            // Both columns should have significant content and be somewhat balanced
            let balance = if left_count > right_count {
                right_count as f32 / left_count as f32
            } else {
                left_count as f32 / right_count as f32
            };

            debug!(
                "Projection gap at X={:.1}: left={}, right={}, balance={:.2}",
                boundary, left_count, right_count, balance
            );

            if left_count >= 5 && right_count >= 5 && balance > 0.25 {
                debug!(
                    "Detected TWO-COLUMN layout with boundary at {:.1}",
                    boundary
                );
                return Some(boundary);
            } else {
                debug!(
                    "Projection gap rejected: left={}, right={}, balance={:.2} (need >=5 each, balance>0.25)",
                    left_count, right_count, balance
                );
            }
        }

        // If global check failed, try checking only the bottom portion of the page
        // This handles pages with full-width headers/abstracts but two-column body
        // Use adaptive threshold based on content distribution instead of fixed 75%
        let max_y = elements
            .iter()
            .map(|e| e.y)
            .fold(f32::NEG_INFINITY, f32::max);
        let min_y = elements.iter().map(|e| e.y).fold(f32::INFINITY, f32::min);
        let page_height_content = max_y - min_y;

        // Only try this if we have enough vertical content
        if page_height_content > 200.0 {
            // Calculate adaptive threshold based on content density
            // Use 20th percentile of y-coordinates to find natural content boundary
            let mut y_coords: Vec<f32> = elements.iter().map(|e| e.y).collect();
            y_coords.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let percentile_idx = (y_coords.len() as f32 * 0.20) as usize;
            let threshold_y = y_coords
                .get(percentile_idx)
                .copied()
                .unwrap_or(max_y - page_height_content * 0.25);
            let bottom_elements: Vec<TextElement> = elements
                .iter()
                .filter(|e| e.y < threshold_y)
                .cloned()
                .collect();

            if bottom_elements.len() > 20 {
                let proj_bottom = self.compute_vertical_projection(&bottom_elements, page_width);
                let gaps_bottom = self.find_projection_gaps(&proj_bottom);

                let center_gap_bottom = gaps_bottom
                    .iter()
                    .find(|&&gap| (gap - center).abs() < center_range);

                if let Some(&boundary) = center_gap_bottom {
                    // Verify with bottom element distribution
                    let left_count = bottom_elements
                        .iter()
                        .filter(|e| e.x < boundary - 10.0)
                        .count();
                    let right_count = bottom_elements
                        .iter()
                        .filter(|e| e.x > boundary + 10.0)
                        .count();

                    let balance = if left_count > right_count {
                        right_count as f32 / left_count as f32
                    } else {
                        left_count as f32 / right_count as f32
                    };

                    debug!(
                        "Bottom-only Projection gap at X={:.1}: left={}, right={}, balance={:.2}",
                        boundary, left_count, right_count, balance
                    );

                    if left_count >= 5 && right_count >= 5 && balance > 0.25 {
                        debug!(
                            "Detected TWO-COLUMN layout (bottom-only) with boundary at {:.1}",
                            boundary
                        );
                        return Some(boundary);
                    } else {
                        debug!(
                            "Bottom-only gap rejected: left={}, right={}, balance={:.2}",
                            left_count, right_count, balance
                        );
                    }
                }
            }
        }

        // Fallback: simple zone-based detection for papers with unusual layout
        let column_boundary = page_width * 0.49;
        let left_zone_end = page_width * 0.45;
        let right_zone_start = page_width * 0.50;

        let mut left_starts = 0;
        let mut right_starts = 0;

        for elem in elements {
            if elem.x < left_zone_end {
                left_starts += 1;
            } else if elem.x > right_zone_start {
                right_starts += 1;
            }
        }

        let balance_ratio = if left_starts > 0 && right_starts > 0 {
            let (min_col, max_col) = if left_starts < right_starts {
                (left_starts, right_starts)
            } else {
                (right_starts, left_starts)
            };
            min_col as f32 / max_col as f32
        } else {
            0.0
        };

        debug!(
            "Column fallback detection: left_starts={}, right_starts={}, balance={:.2}",
            left_starts, right_starts, balance_ratio
        );

        // Two-column layout if:
        // 1. Most elements start clearly in left or right zones
        // 2. Both columns have significant content
        // 3. Columns are somewhat balanced
        if left_starts >= 5 && right_starts >= 5 && balance_ratio > 0.3 {
            info!(
                "Detected TWO-COLUMN layout with boundary at {:.1}",
                column_boundary
            );
            Some(column_boundary)
        } else {
            info!(
                "Detected SINGLE-COLUMN layout (left_starts={}, right_starts={}, balance={:.2})",
                left_starts, right_starts, balance_ratio
            );
            None
        }
    }

    /// Compute vertical projection histogram from text elements.
    /// Returns a vector where each bin contains the density (count) of elements.
    fn compute_vertical_projection(&self, elements: &[TextElement], page_width: f32) -> Vec<usize> {
        let num_bins = (page_width / self.bin_size).ceil() as usize;
        let mut proj = vec![0; num_bins];

        for elem in elements {
            // Count each element's contribution to bins it spans
            let start_bin = (elem.x / self.bin_size) as usize;
            let end_bin = ((elem.x + 20.0) / self.bin_size) as usize; // Approximate text width
            for bin in proj
                .iter_mut()
                .take((end_bin + 1).min(num_bins))
                .skip(start_bin)
            {
                *bin += 1;
            }
        }
        proj
    }

    /// Find gaps (valleys) in the projection histogram.
    /// Returns midpoint positions of significant gaps.
    fn find_projection_gaps(&self, proj: &[usize]) -> Vec<f32> {
        let mut gaps = Vec::new();
        let mut low_start: Option<usize> = None;

        // Calculate adaptive threshold based on content distribution
        // This is a first-principles approach that adapts to document density
        let total: usize = proj.iter().sum();
        let _avg_density = if proj.is_empty() {
            0
        } else {
            total / proj.len()
        };

        // Use 20th percentile instead of fixed 20% of average
        // This adapts to skewed distributions better
        let mut sorted_proj = proj.to_vec();
        sorted_proj.sort();
        let percentile_idx = (sorted_proj.len() as f32 * 0.20) as usize;
        let low_threshold = sorted_proj.get(percentile_idx).copied().unwrap_or(0);

        for (i, &count) in proj.iter().enumerate() {
            if count <= low_threshold {
                // Low density region
                if low_start.is_none() {
                    low_start = Some(i);
                }
            } else if let Some(start) = low_start {
                // End of low density region
                let gap_width = i - start;
                if gap_width >= self.min_gap_bins {
                    // Significant gap found - record midpoint
                    let midpoint = ((start + i) as f32 / 2.0) * self.bin_size;
                    gaps.push(midpoint);
                    debug!("Found gap at X={:.1} (width={} bins)", midpoint, gap_width);
                }
                low_start = None;
            }
        }
        gaps
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

    fn make_element(x: f32, y: f32, text: &str) -> TextElement {
        TextElement {
            text: text.to_string(),
            x,
            y,
            font_name: "test".to_string(),
            font_size: 12.0,
            is_bold: false,
            is_italic: false,
        }
    }

    #[test]
    fn test_column_detector_default() {
        let detector = ColumnDetector::default();
        assert_eq!(detector.bin_size, 5.0);
        assert_eq!(detector.min_gap_bins, 4);
    }

    #[test]
    fn test_detect_single_column() {
        let detector = ColumnDetector::new();
        // All elements in the center - single column layout
        let elements: Vec<TextElement> = (0..20)
            .map(|i| make_element(200.0, i as f32 * 15.0, "text"))
            .collect();

        let result = detector.detect_columns(&elements, 600.0);
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_two_columns() {
        let detector = ColumnDetector::new();
        // Left column elements (x = 50-200)
        let mut elements: Vec<TextElement> = (0..15)
            .map(|i| make_element(100.0, i as f32 * 20.0, "left"))
            .collect();
        // Right column elements (x = 350-500)
        elements.extend((0..15).map(|i| make_element(400.0, i as f32 * 20.0, "right")));

        let result = detector.detect_columns(&elements, 600.0);
        assert!(result.is_some());
        let boundary = result.unwrap();
        // Boundary should be near center (around 250-350)
        assert!(boundary > 200.0 && boundary < 400.0);
    }

    #[test]
    fn test_detect_columns_insufficient_elements() {
        let detector = ColumnDetector::new();
        let elements = vec![
            make_element(50.0, 10.0, "a"),
            make_element(400.0, 10.0, "b"),
        ];

        // Need at least 10 elements
        let result = detector.detect_columns(&elements, 600.0);
        assert!(result.is_none());
    }

    #[test]
    fn test_vertical_projection_histogram() {
        let detector = ColumnDetector::new();
        let elements = vec![
            make_element(10.0, 10.0, "a"),
            make_element(10.0, 20.0, "b"),
            make_element(10.0, 30.0, "c"),
        ];

        let proj = detector.compute_vertical_projection(&elements, 100.0);
        // First few bins should have counts, rest should be zero
        assert!(proj[0] > 0 || proj[1] > 0 || proj[2] > 0);
    }

    #[test]
    fn test_find_projection_gaps() {
        let detector = ColumnDetector::new();
        // Simulate a projection with a clear gap in the middle
        let proj = vec![5, 5, 5, 5, 0, 0, 0, 0, 0, 5, 5, 5, 5];

        let gaps = detector.find_projection_gaps(&proj);
        // Should detect the gap
        assert!(!gaps.is_empty());
    }

    #[test]
    fn test_no_gaps_uniform_distribution() {
        let detector = ColumnDetector::new();
        // Uniform distribution - no gaps
        let proj = vec![5; 20];

        let gaps = detector.find_projection_gaps(&proj);
        assert!(gaps.is_empty());
    }

    #[test]
    fn test_detect_columns_imbalanced() {
        let detector = ColumnDetector::new();
        // Very imbalanced - most content in left column
        let mut elements: Vec<TextElement> = (0..30)
            .map(|i| make_element(100.0, i as f32 * 15.0, "left"))
            .collect();
        // Only 2 elements in right (below balance threshold)
        elements.push(make_element(400.0, 10.0, "right"));
        elements.push(make_element(400.0, 25.0, "right"));

        let result = detector.detect_columns(&elements, 600.0);
        // Should reject due to imbalance (balance < 0.25)
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_columns_wide_page() {
        let detector = ColumnDetector::new();
        // Wide page with two clear columns
        let mut elements: Vec<TextElement> = (0..10)
            .map(|i| make_element(100.0, i as f32 * 20.0, "left"))
            .collect();
        elements.extend((0..10).map(|i| make_element(700.0, i as f32 * 20.0, "right")));

        let result = detector.detect_columns(&elements, 1000.0);
        assert!(result.is_some());
    }
}
