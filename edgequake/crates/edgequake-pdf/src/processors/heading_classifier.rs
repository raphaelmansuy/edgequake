//! Heading classification module using geometric and semantic features.
//!
//! Implements first-principles heading detection based on font size,
//! text properties, and content analysis.

use crate::schema::Block;

/// Classifies blocks as headings based on geometric and semantic properties.
///
/// **Single Responsibility:** Heading identification and level determination.
///
/// **First Principles:**
/// - Headings are geometrically distinct (larger font)
/// - Headings are short (< 100 chars typically)
/// - Headings don't end with periods (statements do)
/// - Headings contain mixed case (not all-caps like headers)
pub struct HeadingClassifier {
    /// Minimum font size ratio to consider as heading (body_size * threshold)
    min_ratio_threshold: f32,
    /// Maximum heading text length
    max_heading_length: usize,
    /// Minimum percentage of spans that must be large font
    large_font_percentage: f32,
}

impl HeadingClassifier {
    /// Create classifier with default thresholds.
    ///
    /// **Defaults:**
    /// - 1.2x body size minimum (20% larger than body text)
    /// - 100 char maximum (headings are concise)
    /// - 80% spans must be large font (consistency check)
    pub fn new() -> Self {
        Self {
            min_ratio_threshold: 1.2,
            max_heading_length: 100,
            large_font_percentage: 0.8,
        }
    }

    /// Classify a block as heading or not.
    ///
    /// **Returns:** (is_heading, level)
    /// - is_heading: true if block is a heading
    /// - level: heading level (2-6) based on size ratio
    ///
    /// **Algorithm:**
    /// 1. Check font size consistency across spans
    /// 2. Calculate size ratio vs body text
    /// 3. Validate text properties (length, punctuation, case)
    /// 4. Determine level from size ratio
    pub fn classify(&self, block: &Block, body_font_size: f32) -> (bool, u8) {
        if block.spans.is_empty() {
            return (false, 0);
        }

        // Step 1: Analyze font sizes
        let font_stats = self.analyze_font_sizes(block, body_font_size);

        if !self.has_consistent_large_font(&font_stats) {
            return (false, 0);
        }

        // Step 2: Validate text properties
        let text = block.text.trim();
        if !self.is_valid_heading_text(text) {
            return (false, 0);
        }

        // Step 3: Check if any span is bold
        let is_bold = block
            .spans
            .iter()
            .any(|s| s.style.weight.map(|w| w >= 600).unwrap_or(false));

        // Step 4: Determine level from size ratio and boldness
        let level = self.calculate_level(font_stats.max_size, body_font_size, is_bold);

        (true, level)
    }

    /// Analyze font sizes in block spans.
    fn analyze_font_sizes(&self, block: &Block, body_size: f32) -> FontStats {
        let mut stats = FontStats::default();

        for span in &block.spans {
            if let Some(size) = span.style.size {
                stats.total_count += 1;

                if size > body_size * self.min_ratio_threshold {
                    stats.large_count += 1;
                    stats.max_size = stats.max_size.max(size);
                }
            }
        }

        stats
    }

    /// Check if block has consistent large font across spans.
    ///
    /// **Principle:** Headings are consistently styled (not mixed fonts)
    fn has_consistent_large_font(&self, stats: &FontStats) -> bool {
        if stats.total_count == 0 {
            return false;
        }

        let large_ratio = stats.large_count as f32 / stats.total_count as f32;
        large_ratio > self.large_font_percentage
    }

    /// Validate text has heading properties.
    ///
    /// **Checks:**
    /// - Non-empty
    /// - Not too long (headings are concise)
    /// - No trailing period (headings aren't sentences)
    /// - Has lowercase chars (not all-caps like page headers)
    fn is_valid_heading_text(&self, text: &str) -> bool {
        if text.is_empty() || text.len() > self.max_heading_length {
            return false;
        }

        if text.ends_with('.') {
            return false;
        }

        // Must have some lowercase (filters "RUNNING HEADER" style text)
        text.chars().any(|c| c.is_lowercase())
    }

    /// Calculate heading level from size ratio.
    ///
    /// **Mapping (aligned with StyleDetectionProcessor):**
    /// - >= 1.5x body size → H1 (very large, document title)
    /// - >= 1.3x → H2 (large, main sections)
    /// - >= 1.2x → H3 (moderate, subsections)
    /// - >= 1.1x → H4 (slightly large, minor sections)
    /// - >= 1.05x → H5 (small, sub-subsections)
    /// - < 1.05x → H6 (smallest, paragraph-level headings)
    ///
    /// **WHY these thresholds?**
    /// - Aligned with StyleDetectionProcessor ratios
    /// - H1: ratio > 1.5 (main title)
    /// - H2: ratio > 1.2 (numbered sections)
    /// - H3: ratio > 1.1 (subsections)
    /// - H4-H6: Smaller size distinctions for detailed document structure
    fn calculate_level(&self, max_size: f32, body_size: f32, is_bold: bool) -> u8 {
        let ratio = max_size / body_size;

        if ratio >= 1.5 {
            1
        } else if ratio >= 1.3 {
            2
        } else if ratio >= 1.2 {
            3
        } else if ratio >= 1.1 {
            4
        } else if ratio >= 1.05 {
            5
        } else if is_bold {
            // Bold text with body-sized font is typically H4-H6
            4
        } else {
            6
        }
    }
}

impl Default for HeadingClassifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Font statistics for a block.
#[derive(Default)]
struct FontStats {
    /// Total number of spans analyzed
    total_count: usize,
    /// Number of spans with large font
    large_count: usize,
    /// Maximum font size found
    max_size: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_calculation() {
        let classifier = HeadingClassifier::new();

        // Very large (>= 1.5x) = H1
        assert_eq!(classifier.calculate_level(18.0, 12.0, false), 1); // 1.5x

        // Large (>= 1.3x) = H2
        assert_eq!(classifier.calculate_level(15.6, 12.0, false), 2); // 1.3x

        // Moderate (>= 1.2x) = H3
        assert_eq!(classifier.calculate_level(14.5, 12.0, false), 3); // 1.208x - safely above 1.2

        // Slightly large (>= 1.1x) = H4
        assert_eq!(classifier.calculate_level(14.0, 12.0, false), 4); // 1.17x

        // Small (>= 1.05x) = H5
        assert_eq!(classifier.calculate_level(13.0, 12.0, false), 5); // 1.083x

        // Smallest (< 1.05x) = H6
        assert_eq!(classifier.calculate_level(12.5, 12.0, false), 6); // 1.042x

        // Bold text with body-sized font = H4
        assert_eq!(classifier.calculate_level(12.0, 12.0, true), 4); // Bold text
    }

    #[test]
    fn test_heading_text_validation() {
        let classifier = HeadingClassifier::new();

        assert!(classifier.is_valid_heading_text("Introduction"));
        assert!(classifier.is_valid_heading_text("3.2 Methods"));

        assert!(!classifier.is_valid_heading_text("This is a sentence."));
        assert!(!classifier.is_valid_heading_text("RUNNING HEADER"));
        assert!(!classifier.is_valid_heading_text(""));
    }
}
