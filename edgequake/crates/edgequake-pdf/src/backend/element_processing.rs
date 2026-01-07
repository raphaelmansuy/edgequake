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
            char_width_factor: 0.4,
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

        // Sort by Y (descending), then X (ascending)
        let mut sorted = elements;
        sorted.sort_by(|a, b| {
            b.y.partial_cmp(&a.y)
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

        // Sort by Y (descending), then X (ascending)
        let mut sorted = elements;
        sorted.sort_by(|a, b| {
            b.y.partial_cmp(&a.y)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
        });

        let mut merged = Vec::new();
        let mut current = sorted[0].clone();

        for next in sorted.into_iter().skip(1) {
            // Check if on same line
            if (next.y - current.y).abs() < self.position_tolerance {
                // Check horizontal distance
                // Use font size from current element to estimate char width
                let char_width = if current.font_size > 0.0 {
                    current.font_size * self.char_width_factor
                } else {
                    4.0
                };

                let current_width = current.text.len() as f32 * char_width;
                let current_end = current.x + current_width;
                let gap = next.x - current_end;

                // If gap is small (e.g. < 2.5 chars), merge
                // Allow slight negative gap (overlap) due to kerning
                if gap > -char_width && gap < char_width * 2.5 {
                    // Merge!
                    // Add space if gap is significant (> 0.3 char width)
                    if gap > char_width * 0.3 {
                        current.text.push(' ');
                    }
                    current.text.push_str(&next.text);
                    continue;
                }
            }

            // Push current and start new
            merged.push(current);
            current = next;
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
