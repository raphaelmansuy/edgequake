//! Heading classification module using geometric and semantic features.
//!
//! Implements first-principles heading detection based on font size,
//! text properties, and content analysis.

use crate::schema::Block;

/// OODA-IT19: Check if text contains prose indicator patterns that suggest it's
/// a sentence, NOT a heading.
///
/// **First Principles:** Headings are short, declarative labels. Sentences contain
/// articles and copulas (connecting verbs) followed by lowercase words.
///
/// ```text
/// ┌──────────────────────────────────────────────────────────┐
/// │  PROSE INDICATOR DETECTION                                │
/// │                                                           │
/// │  "Introduction"          → No indicators → HEADING OK     │
/// │  "This is the second"    → "is" + "the" (lower) → PROSE  │
/// │  "What We Deliver"       → "We" (uppercase) → HEADING OK │
/// │  "It was a dark night"   → "was" + "a" (lower) → PROSE   │
/// └──────────────────────────────────────────────────────────┘
/// ```
///
/// WHY public: Used by both `HeadingClassifier` and `structure_detection.rs`
/// to prevent prose text from being classified as headings. DRY principle.
pub fn has_prose_indicators(text: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() < 3 {
        return false;
    }
    // Check positions 1..min(5, len) for prose indicators
    // Position 0 is the subject/first word - we check AFTER it
    for i in 1..words.len().min(5) {
        let word_lower = words[i].to_lowercase();
        let is_indicator = matches!(
            word_lower.as_str(),
            "the" | "a" | "an" | "it" | "this" | "that" | "as" | "is" | "are" | "was"
        );
        if is_indicator {
            if let Some(next_word) = words.get(i + 1) {
                let next_starts_lower = next_word
                    .chars()
                    .next()
                    .map(|c| c.is_lowercase())
                    .unwrap_or(false);
                if next_starts_lower {
                    return true;
                }
            }
        }
    }
    false
}

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

        // OODA-12: Level 6 means "not a header" in conservative mode
        // WHY: pymupdf4llm only creates H1 and H2 headers for paper titles and
        // major sections. Anything else should be bold text, not a header.
        if level >= 6 {
            return (false, 0);
        }

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
    /// - OODA-25: No internal sentence boundaries (`. [A-Z]` pattern)
    /// - No commas (prose indicator)
    /// - No figure/table caption patterns
    fn is_valid_heading_text(&self, text: &str) -> bool {
        if text.is_empty() || text.len() > self.max_heading_length {
            return false;
        }

        if text.ends_with('.') {
            return false;
        }

        // OODA-25: Generic sentence boundary detection
        // WHY: Headings don't contain sentence breaks. Pattern `. [A-Z]` indicates
        // a period followed by a new sentence, e.g., "Abstract. This paper..."
        // This is a generic prose detector - no document-specific heuristics needed.
        if self.contains_sentence_boundary(text) {
            return false;
        }

        // Generic prose indicator: commas typically indicate complex sentences
        if text.contains(',') {
            return false;
        }

        // OODA-IT19: Use shared prose indicator detection (DRY).
        // WHY: article/pronoun words after first position indicate sentence
        // continuation, not heading structure. Same logic used by
        // structure_detection.rs to prevent prose text from becoming headings.
        if has_prose_indicators(text) {
            return false;
        }

        // Generic caption pattern: figures and tables are labeled with standard prefixes
        let lower = text.to_lowercase();
        if lower.starts_with("fig.")
            || lower.starts_with("figure")
            || lower.starts_with("table")
            || lower.starts_with("tab.")
        {
            return false;
        }

        // Must have some lowercase (filters all-caps running headers)
        text.chars().any(|c| c.is_lowercase())
    }

    /// Generic sentence boundary detection.
    /// Returns true if text contains patterns like `. [A-Z]` indicating
    /// a sentence break followed by a new sentence.
    fn contains_sentence_boundary(&self, text: &str) -> bool {
        // WHY: Use char_indices() instead of chars() to get proper byte positions.
        // Direct indexing like chars[i] with byte slicing &text[..i] can panic
        // when i is a character index but text contains multi-byte UTF-8 chars like '€' (3 bytes).
        let chars: Vec<(usize, char)> = text.char_indices().collect();
        for i in 0..chars.len().saturating_sub(2) {
            // Pattern: sentence-ending punctuation + space + capital letter
            let (_, char_i) = chars[i];
            let (_, char_i1) = chars[i + 1];
            let is_sentence_end = matches!(char_i, '.' | '?' | '!');
            let is_space = char_i1 == ' ';
            let is_capital = chars
                .get(i + 2)
                .map(|(_, c)| c.is_uppercase())
                .unwrap_or(false);

            if is_sentence_end && is_space && is_capital {
                // Exception: common abbreviations like "Dr. Smith", "Fig. 1", "vs. The"
                // Check if preceding word is a common abbreviation
                // WHY: Use byte position from char_indices to safely slice string
                let byte_pos = chars.get(i + 1).map(|(pos, _)| *pos).unwrap_or(text.len());
                let preceding = &text[..byte_pos];
                if !self.is_abbreviation_context(preceding) {
                    return true;
                }
            }
        }
        false
    }

    /// Check if the text ends with a common abbreviation.
    /// This prevents false positives like "Dr. Smith" or "Fig. 1" being treated as sentence breaks.
    fn is_abbreviation_context(&self, text: &str) -> bool {
        let lower = text.to_lowercase();
        // Common abbreviations that appear before capitalized words
        lower.ends_with("dr.")
            || lower.ends_with("mr.")
            || lower.ends_with("mrs.")
            || lower.ends_with("ms.")
            || lower.ends_with("prof.")
            || lower.ends_with("fig.")
            || lower.ends_with("tab.")
            || lower.ends_with("eq.")
            || lower.ends_with("vs.")
            || lower.ends_with("et al.")
            || lower.ends_with("e.g.")
            || lower.ends_with("i.e.")
    }

    /// Calculate heading level from size ratio.
    ///
    /// OODA-12: Conservative heading detection to match pymupdf4llm gold standards.
    /// pymupdf4llm only creates H1 and H2 headers - subsections are bold text.
    ///
    /// **Mapping (conservative to match gold standards):**
    /// - >= 1.5x body size → H1 (very large, document title only)
    /// - >= 1.4x → H2 (large, main sections like "1. Introduction")
    /// - < 1.4x → NOT a header (return H6 which gets filtered out)
    ///
    /// **WHY these thresholds?**
    /// - Gold file 2900_Goyal_et_al.pymupdf.gold.md has only 10 headers
    /// - 1 H1 (paper title), 9 H2 (major sections)
    /// - Subsections (1.1, 3.1.1, etc.) are NOT headers - they're bold text
    /// - Previous thresholds (1.2x→H3, 1.1x→H4) created 33+ headers
    fn calculate_level(&self, max_size: f32, body_size: f32, is_bold: bool) -> u8 {
        let ratio = max_size / body_size;

        if ratio >= 1.5 {
            1 // Very large = H1 (paper title only)
        } else if ratio >= 1.4 {
            2 // Large = H2 (major sections)
        } else {
            // OODA-12: Return 6 for anything else - will be filtered out
            // Bold text with smaller font ratios should NOT become headers
            // in pymupdf4llm's conservative approach
            let _ = is_bold; // Silence unused warning
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

        // OODA-12: Conservative thresholds matching pymupdf4llm gold standard
        // Only H1 (>=1.5x) and H2 (>=1.4x) are recognized as headers
        // Everything else becomes H6 (filtered out)

        // Very large (>= 1.5x) = H1 (paper title only)
        assert_eq!(classifier.calculate_level(18.0, 12.0, false), 1); // 1.5x

        // Large (>= 1.4x) = H2 (major sections)
        assert_eq!(classifier.calculate_level(16.8, 12.0, false), 2); // 1.4x

        // Below 1.4x = H6 (filtered out - not a header in pymupdf approach)
        assert_eq!(classifier.calculate_level(15.6, 12.0, false), 6); // 1.3x - too small
        assert_eq!(classifier.calculate_level(14.5, 12.0, false), 6); // 1.208x - too small
        assert_eq!(classifier.calculate_level(14.0, 12.0, false), 6); // 1.17x - too small
        assert_eq!(classifier.calculate_level(13.0, 12.0, false), 6); // 1.083x - too small
        assert_eq!(classifier.calculate_level(12.5, 12.0, false), 6); // 1.042x - too small

        // Bold text with body-sized font = H6 (not a header in pymupdf approach)
        assert_eq!(classifier.calculate_level(12.0, 12.0, true), 6);
    }

    #[test]
    fn test_heading_text_validation() {
        let classifier = HeadingClassifier::new();

        assert!(classifier.is_valid_heading_text("Introduction"));
        assert!(classifier.is_valid_heading_text("3.2 Methods"));

        assert!(!classifier.is_valid_heading_text("This is a sentence."));
        assert!(!classifier.is_valid_heading_text("RUNNING HEADER"));
        assert!(!classifier.is_valid_heading_text(""));

        // OODA-25: Test sentence boundary detection
        // "Abstract. This paper..." has `. T` pattern - should NOT be a heading
        assert!(!classifier.is_valid_heading_text("Abstract. This paper reviews the architecture"));
        // But single-word section names should be valid
        assert!(classifier.is_valid_heading_text("Abstract"));
    }

    #[test]
    fn test_sentence_boundary_detection() {
        let classifier = HeadingClassifier::new();

        // Should detect sentence boundaries
        assert!(classifier.contains_sentence_boundary("Hello. World"));
        assert!(classifier.contains_sentence_boundary("Done! Next step"));
        assert!(classifier.contains_sentence_boundary("Ready? Start now"));

        // Should NOT detect (no capital after punctuation)
        assert!(!classifier.contains_sentence_boundary("Hello. world"));
        assert!(!classifier.contains_sentence_boundary("version 1.0"));

        // Abbreviations should NOT be detected as boundaries
        assert!(!classifier.contains_sentence_boundary("Dr. Smith"));
        assert!(!classifier.contains_sentence_boundary("Fig. 1"));
        assert!(!classifier.contains_sentence_boundary("e.g. Example"));
    }

    // =================================================================
    // OODA-IT19: Tests for shared has_prose_indicators() function
    // =================================================================

    #[test]
    fn test_prose_indicators_sentence_patterns() {
        // Clearly prose: articles/copulas + lowercase
        assert!(has_prose_indicators("This is the second"));
        assert!(has_prose_indicators("It was a dark night"));
        assert!(has_prose_indicators("There are many options"));
        assert!(has_prose_indicators("She is the manager"));
    }

    #[test]
    fn test_prose_indicators_heading_patterns() {
        // Real headings: no prose indicators
        assert!(!has_prose_indicators("Introduction"));
        assert!(!has_prose_indicators("Methods and Results"));
        assert!(!has_prose_indicators("What We Deliver"));
        assert!(!has_prose_indicators("Architecture & Governance"));
        assert!(!has_prose_indicators("Executive Summary"));
        assert!(!has_prose_indicators("Next Steps"));
    }

    #[test]
    fn test_prose_indicators_short_text() {
        // Less than 3 words: can't detect prose patterns
        assert!(!has_prose_indicators("Hello"));
        assert!(!has_prose_indicators("Two Words"));
        assert!(!has_prose_indicators(""));
    }

    #[test]
    fn test_prose_indicators_uppercase_after_indicator() {
        // Indicator followed by uppercase: heading, not prose
        // "What Is AI" → "Is" is indicator but "AI" is uppercase → OK
        assert!(!has_prose_indicators("What Is AI"));
        assert!(!has_prose_indicators("This Is Important"));
    }
}
