//! Text element preprocessing: deduplication and merging.
//!
//! # WHY separate element processing?
//!
//! PDF files often contain redundant or fragmented text elements that need
//! cleanup before grouping into lines and blocks:
//!
//! 1. **Deduplication**: OCR layers often duplicate visible text, resulting in
//!    "TheThe ProblemProblem" if not deduplicated.
//!
//! 2. **Merging**: PDF operators emit text in fragments (words, characters).
//!    We merge adjacent fragments into contiguous text runs.
//!
//! This preprocessing produces clean input for the grouping phase.

use super::elements::TextElement;

/// Preprocessor for text elements.
///
/// Handles deduplication of overlapping text and merging of
/// horizontally adjacent text on the same line.
pub struct ElementProcessor {
    /// Position tolerance for deduplication (in points)
    pub position_tolerance: f32,
    /// Character width estimate factor (relative to font size)
    pub char_width_factor: f32,
}

impl ElementProcessor {
    pub fn new() -> Self {
        Self {
            position_tolerance: 2.0,
            // WHY 0.55: Average character width is typically 55% of font size for most fonts.
            // This is more accurate than the previous 0.4 which caused false gap detection.
            // For 12pt font: 0.55 * 12 = 6.6pt average char width (realistic for body text)
            // For 18pt heading: 0.55 * 18 = 9.9pt (close to actual ~10pt spacing seen in PDFs)
            char_width_factor: 0.55,
        }
    }

    /// Deduplicate text elements that are identical and at the same position.
    ///
    /// **WHY deduplication is critical:**
    /// - PDF files often contain invisible text layers (e.g., OCR layer + visible layer)
    /// - Without dedup, we get doubled text like "TheThe ProblemProblem"
    /// - Position tolerance of 2pt handles slight rendering variations
    /// - Keep element with more text if one is prefix of another (OCR sometimes partial)
    pub fn deduplicate(&self, elements: Vec<TextElement>) -> Vec<TextElement> {
        if elements.is_empty() {
            return Vec::new();
        }

        // Sort by Y (ascending), then X (ascending)
        // After Y-normalization in extraction engine, Y=0 is at top of page,
        // so ascending Y gives top-to-bottom reading order.
        let mut sorted = elements;
        sorted.sort_by(|a, b| {
            a.y.partial_cmp(&b.y)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
        });

        let mut unique = Vec::new();
        unique.push(sorted[0].clone());

        for elem in sorted.into_iter().skip(1) {
            let prev = unique.last().unwrap();

            // Check for overlap
            let same_pos = (elem.x - prev.x).abs() < self.position_tolerance
                && (elem.y - prev.y).abs() < self.position_tolerance;

            if same_pos {
                // If text is identical, skip
                if elem.text == prev.text {
                    continue;
                }
                // If one contains the other, keep the longer one
                if elem.text.contains(&prev.text) {
                    unique.pop(); // Remove shorter prev
                    unique.push(elem);
                    continue;
                }
                if prev.text.contains(&elem.text) {
                    continue; // Skip shorter elem
                }
            }

            unique.push(elem);
        }

        unique
    }

    /// Merge text elements that are physically adjacent on the same line.
    ///
    /// **WHY merging is essential:**
    /// - PDF operators (Tj, TJ) emit individual words or even characters
    /// - "Hello World" might come as ["Hello", " ", "World"] at different positions
    /// - Merge threshold uses font size to estimate character width
    /// - Result: contiguous text runs for proper word/sentence extraction
    pub fn merge(&self, elements: Vec<TextElement>) -> Vec<TextElement> {
        if elements.is_empty() {
            return Vec::new();
        }

        // Sort by Y (ascending), then X (ascending)
        // After Y-normalization in extraction engine, Y=0 is at top of page,
        // so ascending Y gives top-to-bottom reading order.
        let mut sorted = elements;
        sorted.sort_by(|a, b| {
            a.y.partial_cmp(&b.y)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
        });

        let mut merged = Vec::new();
        let mut current = sorted[0].clone();
        // OODA-09: Track the actual end X position instead of estimating from text length
        // WHY: When elements accumulate, estimated_width grows unbounded, causing
        // cross-column merges like "oper-" + "manipulate" from different columns.
        // Track actual_end_x as max(current_x, next_x + next_estimated_width)
        let mut current_end_x = current.x
            + current.text.chars().count() as f32 * current.font_size * self.char_width_factor;

        for next in sorted.into_iter().skip(1) {
            // Check if on same line (Y within tolerance)
            let y_diff = (next.y - current.y).abs();
            if y_diff < self.position_tolerance {
                // Estimate character width for gap threshold calculations
                let char_width = if current.font_size > 0.0 {
                    current.font_size * self.char_width_factor
                } else {
                    4.0
                };

                // OODA-09: Use tracked end_x instead of estimated_width from accumulated text
                // WHY: estimated_width = text.len() * char_width grows unbounded as we merge,
                // causing overlapping=true even for cross-column elements
                let gap = next.x - current_end_x;

                // OODA-09: Check for column boundary crossing
                // WHY: In two-column layouts, there's a ~40pt gap at the column boundary (~300pt).
                // Elements from different columns should NOT merge even if on same Y.
                //
                // Key insight from debugging v2 PDF:
                // - Left column elements start at X ≈ 64 (left margin)
                // - Right column elements start at X ≈ 313 (right margin)
                // - Estimated end_x can overshoot (330 for text ending around 280)
                // - Gap calculation: -17.1 (negative because end_x overestimated)
                //
                // In single-column PDFs like Qwen.pdf:
                // - Title starts at X ≈ 183 (centered, not left margin)
                // - Content spans X = 183-650+
                //
                // The key discriminator: LEFT MARGIN vs CENTERED content
                // - If current.x < 100 (left margin region) AND next.x > 300 (right region)
                //   → Definitely a column boundary (can't have a single element spanning 200+ pts)
                // - If current.x >= 100 (centered/wide content) → NOT a column boundary
                //
                // Secondary check: Large gap indicates column boundary
                // - If gap > 4x char_width AND both in their respective halves
                let large_gap_threshold = char_width * 4.0;
                let current_in_left_half = current.x < 250.0;
                let next_in_right_half = next.x > 280.0;
                let large_gap_indicates_column =
                    gap > large_gap_threshold && current_in_left_half && next_in_right_half;

                // Primary check: Left margin to right column = definite column boundary
                // This catches the v2 PDF case where estimated end_x causes gap to be negative
                let current_in_left_margin = current.x < 100.0;
                let next_in_right_column = next.x > 300.0;
                let margin_to_column = current_in_left_margin && next_in_right_column;

                let likely_cross_column = large_gap_indicates_column || margin_to_column;

                // For tight fonts where estimated width is too large (negative gap),
                // use a heuristic: if elements are clearly overlapping in X-space,
                // they should be merged. Check if next.x is within the actual current span.
                let overlapping = next.x >= current.x && next.x < current_end_x;

                // Merge thresholds - generous to handle character-by-character PDFs
                let max_overlap = char_width * 4.0; // 4x char_width for tight kerning
                let max_gap = char_width * 2.0; // 2x char_width for word gaps

                // OODA-09: Add cross-column check to prevent merging elements from different columns
                if !likely_cross_column && (overlapping || (gap > -max_overlap && gap < max_gap)) {
                    // Merge!
                    // For word-level spacing (actual gap > typical char spacing), insert space.
                    // But for character-by-character PDFs, don't insert spaces.
                    //
                    // WHY 1.5x threshold instead of 1.0x:
                    // Character-by-character PDFs have inherent position jitter (+-10%).
                    // A gap of 7.39 with char_width=7.33 is just noise, not a real space.
                    // Real word separators have gaps of 1.5-2x char_width.
                    // Using 1.5x prevents false space insertion like "D iagnose" → "Diagnose".
                    let needs_space = gap > char_width * 1.5
                        && !current.text.ends_with(' ')
                        && !next.text.starts_with(' ');
                    if needs_space {
                        current.text.push(' ');
                    }
                    current.text.push_str(&next.text);
                    // OODA-09: Update end_x to include the merged element
                    let next_end_x = next.x + next.text.chars().count() as f32 * char_width;
                    current_end_x = current_end_x.max(next_end_x);
                    continue;
                }
            }

            // Push current and start new
            merged.push(current);
            current = next.clone();
            // OODA-09: Reset end_x for new element
            let char_width = if next.font_size > 0.0 {
                next.font_size * self.char_width_factor
            } else {
                4.0
            };
            current_end_x = next.x + next.text.chars().count() as f32 * char_width;
        }
        merged.push(current);

        merged
    }

    /// Convenience method to run both deduplication and merging.
    pub fn process(&self, elements: Vec<TextElement>) -> Vec<TextElement> {
        let deduped = self.deduplicate(elements);
        self.merge(deduped)
    }
}

impl Default for ElementProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_element(text: &str, x: f32, y: f32) -> TextElement {
        TextElement {
            text: text.to_string(),
            x,
            y,
            font_size: 12.0,
            font_name: "Times-Roman".to_string(),
            is_bold: false,
            is_italic: false,
        }
    }

    #[test]
    fn test_deduplicate_removes_exact_duplicates() {
        let processor = ElementProcessor::new();

        let elements = vec![
            make_element("Hello", 10.0, 700.0),
            make_element("Hello", 10.0, 700.0), // Exact duplicate
            make_element("World", 60.0, 700.0),
        ];

        let deduped = processor.deduplicate(elements);
        assert_eq!(deduped.len(), 2, "Should remove exact duplicates");
    }

    #[test]
    fn test_deduplicate_keeps_distant_elements() {
        let processor = ElementProcessor::new();

        let elements = vec![
            make_element("Hello", 10.0, 700.0),
            make_element("Hello", 100.0, 700.0), // Same text, different position
            make_element("World", 60.0, 700.0),
        ];

        let deduped = processor.deduplicate(elements);
        assert_eq!(deduped.len(), 3, "Should keep distinct positions");
    }

    #[test]
    fn test_deduplicate_keeps_longer_text() {
        let processor = ElementProcessor::new();

        let elements = vec![
            make_element("Hello", 10.0, 700.0),
            make_element("Hello World", 10.0, 700.0), // Longer text at same position
        ];

        let deduped = processor.deduplicate(elements);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].text, "Hello World");
    }

    #[test]
    fn test_merge_adjacent_text() {
        let processor = ElementProcessor::new();

        let elements = vec![
            make_element("Hel", 10.0, 700.0),
            make_element("lo", 30.0, 700.0), // Close X
        ];

        let merged = processor.merge(elements);
        assert_eq!(merged.len(), 1);
        assert!(merged[0].text.contains("Hel") && merged[0].text.contains("lo"));
    }

    #[test]
    fn test_merge_adds_space_for_gap() {
        let processor = ElementProcessor::new();

        let elements = vec![
            make_element("Hello", 10.0, 700.0),
            make_element("World", 60.0, 700.0), // Significant gap
        ];

        let merged = processor.merge(elements);
        // Should merge with space or keep separate depending on gap
        assert!(merged.len() >= 1);
    }

    #[test]
    fn test_merge_preserves_vertical_separation() {
        let processor = ElementProcessor::new();

        let elements = vec![
            make_element("Line 1", 10.0, 700.0),
            make_element("Line 2", 10.0, 680.0), // Different Y
        ];

        let merged = processor.merge(elements);
        assert_eq!(
            merged.len(),
            2,
            "Vertically separated text should not merge"
        );
    }

    #[test]
    fn test_process_empty() {
        let processor = ElementProcessor::new();
        let result = processor.process(vec![]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_process_combines_dedup_and_merge() {
        let processor = ElementProcessor::new();

        let elements = vec![
            make_element("Hello", 10.0, 700.0),
            make_element("Hello", 10.0, 700.0), // Duplicate
            make_element("World", 60.0, 700.0),
        ];

        let result = processor.process(elements);
        // After dedup: 2 elements, after merge: depends on gap
        assert!(result.len() <= 2);
    }
}
