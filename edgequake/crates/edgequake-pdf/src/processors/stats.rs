//! Document statistics for adaptive threshold calculation.
//!
//! This module provides First Principles-based statistical analysis of PDF documents
//! to derive adaptive thresholds instead of using magic numbers.

use crate::schema::{Block, Document};

/// Statistical properties of a PDF document used to derive adaptive thresholds.
///
/// All thresholds are calculated from actual document properties rather than
/// hardcoded magic numbers, ensuring the processor adapts to:
/// - Different font sizes (8pt to 24pt)
/// - Different page sizes (Letter, A4, presentations)
/// - Different layouts (single-column, multi-column, tables)
#[derive(Debug, Clone)]
pub struct DocumentStats {
    /// Median font size across all blocks (most common body text size).
    pub body_font_size: f32,

    /// Median vertical gap between consecutive lines in the same block.
    pub typical_line_spacing: f32,

    /// Adaptive X-coordinate tolerance for column alignment (10th percentile of nearest-neighbor distances).
    pub column_alignment_tolerance: f32,

    /// Most common page width in the document.
    pub page_width: f32,

    /// Most common page height in the document.
    pub page_height: f32,
}

impl DocumentStats {
    /// Calculate statistics from a document.
    ///
    /// Uses robust statistical methods (median, percentiles) to avoid outlier sensitivity.
    pub fn from_document(doc: &Document) -> Self {
        let body_font_size = Self::calculate_body_font_size(doc);
        let typical_line_spacing = Self::calculate_line_spacing(doc, body_font_size);
        let column_alignment_tolerance = Self::calculate_alignment_tolerance(doc);
        let (page_width, page_height) = Self::most_common_page_size(doc);

        Self {
            body_font_size,
            typical_line_spacing,
            column_alignment_tolerance,
            page_width,
            page_height,
        }
    }

    /// Calculate median font size across all text spans.
    ///
    /// Uses median instead of mean to be robust against headers/footnotes.
    fn calculate_body_font_size(doc: &Document) -> f32 {
        let mut sizes: Vec<f32> = doc
            .pages
            .iter()
            .flat_map(|p| &p.blocks)
            .flat_map(|b| &b.spans)
            .filter_map(|s| s.style.size)
            .collect();

        if sizes.is_empty() {
            return 10.0; // Default for empty documents
        }

        sizes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        Self::percentile(&sizes, 0.5)
    }

    /// Calculate typical intra-paragraph line spacing (gap between wrapped lines).
    ///
    /// **WHY 1.5x body_font_size filter?** In typical typesetting:
    ///   - Intra-paragraph gaps: ~1.2-1.4x font size (single line spacing)
    ///   - Inter-paragraph gaps: ~2.0-3.0x font size (paragraph break)
    ///
    /// Using 1.5x excludes paragraph breaks, focusing only on wrapped line gaps.
    ///
    /// **WHY 30th percentile?** Intra-paragraph gaps cluster at the low end.
    /// The 30th percentile captures the typical line gap while ignoring outliers.
    ///
    /// **WHY cap at 1.5x body_font_size?** Even if document has mostly paragraph
    /// breaks, we want a reasonable intra-line threshold for merge decisions.
    fn calculate_line_spacing(doc: &Document, body_font_size: f32) -> f32 {
        let mut gaps: Vec<f32> = Vec::new();

        for page in &doc.pages {
            let blocks: Vec<&Block> = page.blocks.iter().collect();
            for window in blocks.windows(2) {
                let gap = (window[0].bbox.y1 - window[1].bbox.y2).abs();
                // Filter: only intra-paragraph gaps (< 1.5x body size)
                // WHY: Inter-paragraph gaps are ~2-3x body size, we want line gaps only
                if gap > 0.0 && gap < body_font_size * 1.5 {
                    gaps.push(gap);
                }
            }
        }

        if gaps.is_empty() {
            // Default leading factor 1.2 (tight single spacing)
            return body_font_size * 1.2;
        }

        gaps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        // WHY 30th percentile: We want the low-end typical gap, not median
        let calculated = Self::percentile(&gaps, 0.3);
        // Cap at 1.5x body font size to ensure paragraph breaks aren't merged
        calculated.min(body_font_size * 1.5)
    }

    /// Calculate X-coordinate alignment tolerance using nearest-neighbor analysis.
    ///
    /// Uses 10th percentile of nearest-neighbor distances to find natural alignment clusters.
    /// Similar to DBSCAN epsilon calculation.
    ///
    /// **WHY minimum 2.0?** PDF rendering can have sub-point variations (kerning, font metrics).
    /// A tolerance below 2pt risks splitting blocks that visually align.
    fn calculate_alignment_tolerance(doc: &Document) -> f32 {
        let mut x_coords: Vec<f32> = doc
            .pages
            .iter()
            .flat_map(|p| &p.blocks)
            .map(|b| b.bbox.x1)
            .collect();

        if x_coords.len() < 2 {
            return 20.0; // Default for sparse documents
        }

        x_coords.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Calculate nearest-neighbor distances
        let mut nearest_dists: Vec<f32> = Vec::new();
        for i in 1..x_coords.len() {
            let dist = x_coords[i] - x_coords[i - 1];
            if dist > 0.1 {
                // Filter duplicates
                nearest_dists.push(dist);
            }
        }

        if nearest_dists.is_empty() {
            return 20.0;
        }

        nearest_dists.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        // 10th percentile: tight enough for alignment, loose enough for minor variations
        let calculated = Self::percentile(&nearest_dists, 0.1);
        // Ensure minimum tolerance of 2.0 points to account for PDF rendering variations
        calculated.max(2.0)
    }

    /// Find most common page size in document.
    ///
    /// Handles mixed-size documents by using majority vote.
    fn most_common_page_size(doc: &Document) -> (f32, f32) {
        use std::collections::HashMap;
        let mut size_counts: HashMap<(i32, i32), usize> = HashMap::new();

        for page in &doc.pages {
            let key = (page.width as i32, page.height as i32);
            *size_counts.entry(key).or_insert(0) += 1;
        }

        size_counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|((w, h), _)| (*w as f32, *h as f32))
            .unwrap_or((612.0, 792.0)) // Letter size default (8.5" x 11")
    }

    /// Calculate percentile from sorted array.
    fn percentile(sorted: &[f32], p: f32) -> f32 {
        let idx = ((sorted.len() - 1) as f32 * p).max(0.0) as usize;
        sorted[idx]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_document_defaults() {
        let doc = Document::new();
        let stats = DocumentStats::from_document(&doc);
        // Should return sensible defaults
        assert_eq!(stats.body_font_size, 10.0);
        // WHY 12.0: Changed from 1.4x to 1.2x for tighter line spacing detection
        assert_eq!(stats.typical_line_spacing, 12.0); // 10.0 * 1.2
        assert_eq!(stats.column_alignment_tolerance, 20.0);
        assert_eq!(stats.page_width, 612.0); // Letter
        assert_eq!(stats.page_height, 792.0);
    }

    #[test]
    fn test_percentile_calculation() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(DocumentStats::percentile(&data, 0.0), 1.0); // Min
        assert_eq!(DocumentStats::percentile(&data, 0.5), 3.0); // Median
        assert_eq!(DocumentStats::percentile(&data, 1.0), 5.0); // Max
    }

    // OODA-20: Additional percentile edge case tests

    #[test]
    fn test_percentile_single_element() {
        let data = vec![42.0];
        // WHY: Any percentile of single element should return that element
        assert_eq!(DocumentStats::percentile(&data, 0.0), 42.0);
        assert_eq!(DocumentStats::percentile(&data, 0.5), 42.0);
        assert_eq!(DocumentStats::percentile(&data, 1.0), 42.0);
    }

    #[test]
    fn test_percentile_two_elements() {
        let data = vec![10.0, 20.0];
        // p=0.0 → index 0 (10.0)
        // p=1.0 → index 1 (20.0)
        assert_eq!(DocumentStats::percentile(&data, 0.0), 10.0);
        assert_eq!(DocumentStats::percentile(&data, 1.0), 20.0);
    }

    #[test]
    fn test_percentile_interpolation() {
        // Array [0, 1, 2, 3, 4, 5, 6, 7, 8, 9] (10 elements)
        // WHY test these percentiles: They match the ones used in the module
        // - 0.1 (10th) used for alignment tolerance
        // - 0.3 (30th) used for line spacing
        // - 0.5 (50th) used for body font size (median)
        let data: Vec<f32> = (0..10).map(|i| i as f32).collect();
        // p=0.1 → index 0.9 → 0 (floor) → 0.0
        assert_eq!(DocumentStats::percentile(&data, 0.1), 0.0);
        // p=0.3 → index 2.7 → 2 (floor) → 2.0
        assert_eq!(DocumentStats::percentile(&data, 0.3), 2.0);
        // p=0.5 → index 4.5 → 4 (floor) → 4.0
        assert_eq!(DocumentStats::percentile(&data, 0.5), 4.0);
        // p=0.9 → index 8.1 → 8 (floor) → 8.0
        assert_eq!(DocumentStats::percentile(&data, 0.9), 8.0);
    }

    #[test]
    fn test_percentile_large_array() {
        // 100 elements: 1.0 to 100.0
        // WHY: Test with larger dataset to verify scaling
        let data: Vec<f32> = (1..=100).map(|i| i as f32).collect();
        // p=0.10 → index 9.9 → 9 → 10.0 (value at index 9)
        assert_eq!(DocumentStats::percentile(&data, 0.10), 10.0);
        // p=0.50 → index 49.5 → 49 → 50.0
        assert_eq!(DocumentStats::percentile(&data, 0.50), 50.0);
        // p=0.90 → index 89.1 → 89 → 90.0
        assert_eq!(DocumentStats::percentile(&data, 0.90), 90.0);
    }

    // More comprehensive tests would require creating a full Block structure
    // which is complex. The integration tests in tests/ directory will verify
    // the full functionality of adaptive thresholds.
}
