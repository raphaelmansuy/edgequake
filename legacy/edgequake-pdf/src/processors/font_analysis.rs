//! Font analysis module for heading detection.
//!
//! Provides first-principles based font size analysis to detect headings
//! geometrically rather than heuristically.

use crate::schema::{Block, BlockType, Document};

/// Analyzes font sizes across a document to establish baseline metrics.
///
/// **Single Responsibility:** Font size statistics collection and analysis.
///
/// **First Principles:**
/// - Body text has consistent font size across a document
/// - Median is more robust than mean for font size detection
/// - Outliers (very large/small fonts) should be filtered
pub struct FontAnalyzer {
    /// Minimum valid font size (filters noise)
    min_size: f32,
    /// Maximum valid font size (filters outliers)
    max_size: f32,
}

impl FontAnalyzer {
    /// Create a new font analyzer with sensible defaults.
    ///
    /// Size range [4.0, 72.0] covers typical document fonts while filtering:
    /// - Superscripts/subscripts (< 4pt)
    /// - Title page decorations (> 72pt)
    pub fn new() -> Self {
        Self {
            min_size: 4.0,
            max_size: 72.0,
        }
    }

    /// Detect the body font size from document.
    ///
    /// **Algorithm:**
    /// 1. Collect all font sizes from text/paragraph blocks
    /// 2. Filter invalid sizes (outside min/max range)
    /// 3. Return median (50th percentile) for robustness
    ///
    /// **Why median over mean?**
    /// - Robust to outliers (large titles don't skew result)
    /// - Represents the "typical" font size
    /// - Works even with varied heading sizes in document
    pub fn detect_body_font_size(&self, document: &Document) -> f32 {
        let mut sizes: Vec<f32> = Vec::new();

        // Collect sizes from text blocks only (not headers/tables)
        for page in &document.pages {
            for block in &page.blocks {
                if self.is_body_text_block(block) {
                    self.collect_span_sizes(block, &mut sizes);
                }
            }
        }

        self.calculate_median(sizes)
    }

    /// Check if block is likely body text (not a header or special block).
    #[inline]
    fn is_body_text_block(&self, block: &Block) -> bool {
        matches!(block.block_type, BlockType::Text | BlockType::Paragraph)
    }

    /// Collect valid font sizes from block spans.
    fn collect_span_sizes(&self, block: &Block, sizes: &mut Vec<f32>) {
        for span in &block.spans {
            if let Some(size) = span.style.size {
                if self.is_valid_size(size) {
                    sizes.push(size);
                }
            }
        }
    }

    /// Validate font size is within reasonable range.
    #[inline]
    fn is_valid_size(&self, size: f32) -> bool {
        size >= self.min_size && size <= self.max_size
    }

    /// Calculate median from collected sizes.
    ///
    /// Returns default of 12.0pt if no sizes collected (empty document).
    fn calculate_median(&self, mut sizes: Vec<f32>) -> f32 {
        if sizes.is_empty() {
            return 12.0; // Standard body text default
        }

        sizes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        sizes[sizes.len() / 2]
    }
}

impl Default for FontAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_size_range() {
        let analyzer = FontAnalyzer::new();
        assert!(!analyzer.is_valid_size(3.0)); // Too small
        assert!(analyzer.is_valid_size(12.0)); // Normal
        assert!(!analyzer.is_valid_size(100.0)); // Too large
    }

    #[test]
    fn test_median_calculation() {
        let analyzer = FontAnalyzer::new();
        assert_eq!(analyzer.calculate_median(vec![10.0, 12.0, 14.0]), 12.0);
        assert_eq!(analyzer.calculate_median(vec![]), 12.0); // Default
    }

    // OODA-19: Additional edge case tests for font analysis

    #[test]
    fn test_median_even_count() {
        let analyzer = FontAnalyzer::new();
        // Even count: median is element at index n/2
        // [10, 11, 12, 13] → index 2 → 12
        assert_eq!(
            analyzer.calculate_median(vec![10.0, 11.0, 12.0, 13.0]),
            12.0
        );
    }

    #[test]
    fn test_median_single_element() {
        let analyzer = FontAnalyzer::new();
        assert_eq!(analyzer.calculate_median(vec![14.0]), 14.0);
    }

    #[test]
    fn test_median_two_elements() {
        let analyzer = FontAnalyzer::new();
        // Two elements: n/2 = 1, so second element after sort
        assert_eq!(analyzer.calculate_median(vec![10.0, 14.0]), 14.0);
    }

    #[test]
    fn test_median_with_outliers() {
        let analyzer = FontAnalyzer::new();
        // WHY median: Outliers don't affect result (unlike mean)
        // [4, 4, 4, 10, 12, 12, 12, 48, 72] → sorted, index 4 → 12
        assert_eq!(
            analyzer.calculate_median(vec![4.0, 48.0, 12.0, 72.0, 4.0, 12.0, 12.0, 4.0, 10.0]),
            12.0
        );
    }

    #[test]
    fn test_valid_size_boundary() {
        let analyzer = FontAnalyzer::new();
        // WHY: Boundary conditions are common sources of off-by-one errors
        assert!(analyzer.is_valid_size(4.0)); // Min valid (inclusive)
        assert!(analyzer.is_valid_size(72.0)); // Max valid (inclusive)
        assert!(!analyzer.is_valid_size(3.9)); // Just below min
        assert!(!analyzer.is_valid_size(72.1)); // Just above max
    }
}
