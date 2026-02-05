//! Block classification and pattern detection.
//!
//! This module provides block type classification (header, list, code, paragraph)
//! based on font size analysis and text patterns.
//!
//! ## Algorithm (OODA-45 SRP Extraction)
//!
//! ```text
//!                    ┌──────────────────────────────────────────┐
//!                    │         CLASSIFICATION PIPELINE          │
//!                    └──────────────────────────────────────────┘
//!                                        │
//!     ┌──────────────────────────────────┼──────────────────────────────────┐
//!     │                                  ▼                                  │
//!     │    Step 1: Check monospace → Code block                            │
//!     │                                  │                                  │
//!     │                                  ▼                                  │
//!     │    Step 2: Check font size ratio → Header (>1.50x body)            │
//!     │            WHY: pymupdf4llm is CONSERVATIVE - only largest fonts   │
//!     │                                  │                                  │
//!     │                                  ▼                                  │
//!     │    Step 3: Check bullet patterns → ListItem                        │
//!     │            WHY: Comprehensive Unicode bullet detection             │
//!     │                                  │                                  │
//!     │                                  ▼                                  │
//!     │    Default: Paragraph                                              │
//!     │                                                                    │
//!     └────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Pattern Functions (from pymupdf4llm)
//!
//! - `is_bullet_item`: Comprehensive Unicode bullet detection
//! - `is_numbered_list_item`: Numeric list patterns (1., 2), 3:)
//! - `is_roman_numeral_header`: IEEE-style sections (I., II.)
//! - `is_numeric_section_header`: ICML/NeurIPS sections (1., 2.)
//!
//! Note: Pattern-based header detection is DISABLED (OODA-10/11) because
//! pymupdf4llm gold standards are very conservative about headers.

use super::pymupdf_structs::{Block, BlockType};

/// Block classifier using font analysis and patterns.
///
/// WHY separate struct: Allows configuration of classification thresholds
/// without coupling to the TextGrouper.
#[derive(Debug, Clone)]
pub struct BlockClassifier {
    /// Font size ratio threshold for header detection.
    /// Default: 1.50 (50% larger than body = header)
    pub header_ratio: f32,
    /// Maximum lines for a header block.
    /// Default: 2 (headers are short)
    pub max_header_lines: usize,
    /// Maximum characters for a header block.
    /// Default: 150
    pub max_header_chars: usize,
}

impl Default for BlockClassifier {
    fn default() -> Self {
        Self {
            // OODA-12: Conservative threshold to match pymupdf4llm gold
            header_ratio: 1.50,
            max_header_lines: 2,
            max_header_chars: 150,
        }
    }
}

impl BlockClassifier {
    /// Create a new classifier with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Classify all blocks using body font size as reference.
    pub fn classify_blocks(&self, blocks: &mut [Block], body_font_size: f32) {
        for block in blocks {
            block.block_type = self.classify_block(block, body_font_size);
        }
    }

    /// Classify a single block.
    ///
    /// Classification priority:
    /// 1. Code (all monospace fonts)
    /// 2. Header (large font, short text)
    /// 3. ListItem (bullet/numbered prefix)
    /// 4. Paragraph (default)
    pub fn classify_block(&self, block: &Block, body_font_size: f32) -> BlockType {
        if block.lines.is_empty() {
            return BlockType::Paragraph;
        }

        // Check for code block (all monospace)
        let all_mono = block
            .lines
            .iter()
            .all(|line| line.spans.iter().all(|span| span.is_monospace()));
        if all_mono {
            return BlockType::Code;
        }

        // Get first line text for pattern matching (used for list detection)
        let first_text = block.lines.first().map(|l| l.text()).unwrap_or_default();
        let trimmed = first_text.trim();

        // OODA-10/11: Pattern-based header detection DISABLED
        // Keep references to suppress dead_code warnings
        let _ = is_roman_numeral_header;
        let _ = is_letter_subsection_header;
        let _ = is_numeric_section_header;
        let _ = is_numeric_subsection_header;
        let _ = is_abstract_header;
        let _ = &trimmed;

        // Check for header based ONLY on font size (OODA-12)
        let dominant_size = block
            .lines
            .iter()
            .map(|l| l.dominant_font_size())
            .fold(0.0_f32, f32::max);

        let total_chars: usize = block.lines.iter().map(|l| l.text().len()).sum();

        // WHY 1.50x: Conservative to match pymupdf4llm gold standards
        if dominant_size > body_font_size * self.header_ratio
            && block.lines.len() <= self.max_header_lines
            && total_chars < self.max_header_chars
        {
            let ratio = dominant_size / body_font_size;
            // WHY (OODA-12): Heading level based on size ratio to body text.
            // - 2.0x: Very large (double body) = major heading (#)
            // - 1.7x: Large (70% bigger) = secondary heading (##)
            // - 1.5x: Medium = default to # (conservative)
            let level = if ratio >= 2.0 {
                1 // Very large = #
            } else if ratio >= 1.7 {
                2 // Large = ##
            } else {
                1 // Title = # (most conservative)
            };
            return BlockType::Header(level);
        }

        // Check for list item
        if let Some(first_line) = block.lines.first() {
            let text = first_line.text();
            let trimmed = text.trim_start();
            if is_bullet_item(trimmed) || is_numbered_list_item(trimmed) {
                return BlockType::ListItem;
            }
        }

        BlockType::Paragraph
    }
}

// =============================================================================
// PATTERN DETECTION FUNCTIONS
// =============================================================================

/// Check if text starts with a bullet character.
///
/// Based on pymupdf4llm's comprehensive BULLETS list (get_text_lines.py).
///
/// WHY: PDFs use many different bullet characters beyond just `•`, `-`, `*`.
/// This includes various Unicode bullet points, dashes, and geometric shapes.
///
/// ## Supported bullets:
/// ```text
/// ASCII:    * - >
/// Latin-1:  ¶ ·
/// Dashes:   ‐ ‑ ‒ – — ―
/// Symbols:  † ‡ • ∙ −
/// Private:  \uF0A7 \uF0B7
/// Shapes:   ■ □ ▪ ▫ ● ○ ◆ ◇ etc. (U+25A0-25FF)
/// ```
pub fn is_bullet_item(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }

    let first_char = text.chars().next().unwrap();

    // WHY match ranges: Geometric shapes block has many bullet variants
    let is_bullet = matches!(
        first_char,
        '\u{2A}'     // * asterisk
        | '\u{2D}'   // - hyphen-minus
        | '\u{3E}'   // > greater-than
        | '\u{6F}'   // o lowercase o
        | '\u{B6}'   // ¶ pilcrow
        | '\u{B7}'   // · middle dot
        | '\u{2010}' // ‐ hyphen
        | '\u{2011}' // ‑ non-breaking hyphen
        | '\u{2012}' // ‒ figure dash
        | '\u{2013}' // – en dash
        | '\u{2014}' // — em dash
        | '\u{2015}' // ― horizontal bar
        | '\u{2020}' // † dagger
        | '\u{2021}' // ‡ double dagger
        | '\u{2022}' // • bullet
        | '\u{2212}' // − minus sign
        | '\u{2219}' // ∙ bullet operator
        | '\u{F0A7}' // private use (common in PDFs)
        | '\u{F0B7}' // private use (common in PDFs)
        | '\u{FFFD}' // replacement character
        | '\u{25A0}'..='\u{25FF}' // geometric shapes
    );

    // Must be followed by whitespace to be a list item
    if is_bullet && text.len() > first_char.len_utf8() {
        let rest = &text[first_char.len_utf8()..];
        return rest.starts_with(' ') || rest.starts_with('\t');
    }

    false
}

/// Check if text starts with a numbered list item pattern.
///
/// Patterns: "1. ", "2) ", "3: "
///
/// OODA-10: Excludes section header patterns (X.Y.) which look similar.
pub fn is_numbered_list_item(text: &str) -> bool {
    let trimmed = text.trim_start();
    let mut chars = trimmed.chars().peekable();

    // Check for digit(s)
    let mut has_digit = false;
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            has_digit = true;
            chars.next();
        } else {
            break;
        }
    }

    if !has_digit {
        return false;
    }

    // Check separator
    match chars.next() {
        Some('.') => {
            // Exclude section headers like "2.1."
            if let Some(&next_c) = chars.peek() {
                if next_c.is_ascii_digit() {
                    return false;
                }
            }
            true
        }
        Some(')') | Some(':') => true,
        _ => false,
    }
}

/// Check if text starts with a Roman numeral section pattern.
///
/// Patterns: "I. INTRODUCTION", "II. RELATED WORKS"
///
/// WHY: IEEE-style papers use Roman numerals (I-X) for major sections.
pub fn is_roman_numeral_header(text: &str) -> bool {
    if text.len() < 4 {
        return false;
    }

    let mut chars = text.chars().peekable();

    // Collect Roman numeral characters
    let mut has_roman = false;
    while let Some(&c) = chars.peek() {
        if c == 'I' || c == 'V' || c == 'X' {
            has_roman = true;
            chars.next();
        } else {
            break;
        }
    }

    if !has_roman {
        return false;
    }

    // Must be followed by ". "
    match (chars.next(), chars.next()) {
        (Some('.'), Some(' ')) => {
            let rest: String = chars.collect();
            let uppercase_count = rest.chars().filter(|c| c.is_uppercase()).count();
            let alpha_count = rest.chars().filter(|c| c.is_alphabetic()).count();
            // WHY (OODA-12): 50% uppercase threshold for all-caps section detection.
            // True all-caps = 100%, but OCR/extraction may have errors.
            // 50% catches "ABSTRACT", "REFERENCES" with some lowercase mixed in.
            alpha_count > 0 && (uppercase_count as f32 / alpha_count as f32) >= 0.5
        }
        _ => false,
    }
}

/// Check if text starts with a letter subsection pattern.
///
/// Patterns: "A. Background", "B. Policy Representations"
///
/// WHY: IEEE-style papers use single letters (A-Z) for subsections.
/// Note: Excludes I, V, X (Roman numerals).
pub fn is_letter_subsection_header(text: &str) -> bool {
    if text.len() < 4 {
        return false;
    }

    let mut chars = text.chars();
    let first = chars.next();
    let second = chars.next();
    let third = chars.next();

    match (first, second, third) {
        (Some(c), Some('.'), Some(' '))
            if c.is_ascii_uppercase() && c != 'I' && c != 'V' && c != 'X' =>
        {
            chars.next().map(|c| c.is_uppercase()).unwrap_or(false)
        }
        _ => false,
    }
}

/// Check if text starts with a numeric section pattern.
///
/// Patterns: "1. Introduction", "2. Related Works"
///
/// WHY: ICML/NeurIPS-style papers use numbers for major sections.
pub fn is_numeric_section_header(text: &str) -> bool {
    if text.len() < 4 || text.len() > 50 {
        return false;
    }

    let mut chars = text.chars().peekable();

    // Check for 1-2 digits
    let mut digit_count = 0;
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            digit_count += 1;
            chars.next();
        } else {
            break;
        }
    }

    if digit_count == 0 || digit_count > 2 {
        return false;
    }

    match (chars.next(), chars.next()) {
        (Some('.'), Some(' ')) => {
            let rest: String = chars.collect();
            if rest.contains(':') {
                return false;
            }
            rest.chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false)
        }
        _ => false,
    }
}

/// Check if text starts with a numeric subsection pattern.
///
/// Patterns: "2.1. Agentic Training", "3.2 Architecture"
///
/// WHY: Many papers use X.Y numbering for subsections.
pub fn is_numeric_subsection_header(text: &str) -> bool {
    if text.len() < 6 {
        return false;
    }

    let mut chars = text.chars().peekable();

    // First number
    let mut has_first = false;
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            has_first = true;
            chars.next();
        } else {
            break;
        }
    }

    if !has_first || chars.next() != Some('.') {
        return false;
    }

    // Second number
    let mut has_second = false;
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            has_second = true;
            chars.next();
        } else {
            break;
        }
    }

    if !has_second {
        return false;
    }

    match chars.next() {
        Some('.') => match chars.next() {
            Some(' ') => chars.next().map(|c| c.is_uppercase()).unwrap_or(false),
            _ => false,
        },
        Some(' ') => chars.next().map(|c| c.is_uppercase()).unwrap_or(false),
        _ => false,
    }
}

/// Check if text is an "Abstract" header.
///
/// Patterns: "Abstract", "ABSTRACT", "Abstract:"
pub fn is_abstract_header(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower == "abstract" || lower == "abstract:" || lower.starts_with("abstract ")
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::pymupdf_structs::{Line, Span};

    #[test]
    fn test_bullet_detection() {
        assert!(is_bullet_item("• Item"));
        assert!(is_bullet_item("- Item"));
        assert!(is_bullet_item("* Item"));
        assert!(is_bullet_item("– Item")); // en dash
        assert!(is_bullet_item("■ Item")); // geometric shape

        assert!(!is_bullet_item("•Item")); // no space
        assert!(!is_bullet_item("Not a bullet"));
        assert!(!is_bullet_item(""));
    }

    #[test]
    fn test_numbered_list_detection() {
        assert!(is_numbered_list_item("1. First"));
        assert!(is_numbered_list_item("23) Item"));
        assert!(is_numbered_list_item("5: Something"));

        // Section headers should NOT match
        assert!(!is_numbered_list_item("2.1. Subsection"));
        assert!(!is_numbered_list_item("3.2 Architecture"));
    }

    #[test]
    fn test_roman_numeral_header() {
        assert!(is_roman_numeral_header("I. INTRODUCTION"));
        assert!(is_roman_numeral_header("II. RELATED WORKS"));
        assert!(is_roman_numeral_header("X. CONCLUSION"));

        assert!(!is_roman_numeral_header("I.NOSPACE"));
        assert!(!is_roman_numeral_header("Not a header"));
    }

    #[test]
    fn test_block_classifier() {
        let classifier = BlockClassifier::new();
        let body_size = 12.0;

        // Header detection (large font)
        let header_block = Block::from_line(Line {
            spans: vec![Span {
                text: "Title".to_string(),
                x0: 10.0,
                y0: 100.0,
                x1: 100.0,
                y1: 130.0,
                font_size: 24.0,
                font_name: Some("Arial-Bold".to_string()),
                page_num: 0,
                font_is_bold: Some(true),
                font_is_italic: None,
                font_is_monospace: None,
            }],
            x0: 10.0,
            y0: 100.0,
            x1: 100.0,
            y1: 130.0,
            page_num: 0,
        });

        assert!(matches!(
            classifier.classify_block(&header_block, body_size),
            BlockType::Header(1)
        ));

        // List item detection
        let list_block = Block::from_line(Line {
            spans: vec![Span {
                text: "• Item one".to_string(),
                x0: 10.0,
                y0: 50.0,
                x1: 100.0,
                y1: 62.0,
                font_size: 12.0,
                font_name: Some("Arial".to_string()),
                page_num: 0,
                font_is_bold: None,
                font_is_italic: None,
                font_is_monospace: None,
            }],
            x0: 10.0,
            y0: 50.0,
            x1: 100.0,
            y1: 62.0,
            page_num: 0,
        });

        assert_eq!(
            classifier.classify_block(&list_block, body_size),
            BlockType::ListItem
        );
    }

    /// OODA-14: Test heading level classification based on font size ratio.
    /// WHY: Validates the 2.0x/1.7x/1.5x thresholds for H1/H2 classification.
    #[test]
    fn test_heading_level_classification() {
        let classifier = BlockClassifier::new();
        let body_size = 10.0;

        // Helper to create a block with given font size
        fn make_heading_block(font_size: f32, text: &str) -> Block {
            Block::from_line(Line {
                spans: vec![Span {
                    text: text.to_string(),
                    x0: 10.0,
                    y0: 100.0,
                    x1: 200.0,
                    y1: 100.0 + font_size,
                    font_size,
                    font_name: Some("Arial".to_string()),
                    page_num: 0,
                    font_is_bold: Some(true),
                    font_is_italic: None,
                    font_is_monospace: None,
                }],
                x0: 10.0,
                y0: 100.0,
                x1: 200.0,
                y1: 100.0 + font_size,
                page_num: 0,
            })
        }

        // H1: ratio >= 2.0 (20pt / 10pt = 2.0)
        assert!(
            matches!(
                classifier.classify_block(&make_heading_block(20.0, "Major Title"), body_size),
                BlockType::Header(1)
            ),
            "20pt on 10pt body (2.0x) should be H1"
        );

        // H2: ratio >= 1.7, < 2.0 (18pt / 10pt = 1.8)
        assert!(
            matches!(
                classifier.classify_block(&make_heading_block(18.0, "Section Heading"), body_size),
                BlockType::Header(2)
            ),
            "18pt on 10pt body (1.8x) should be H2"
        );

        // H1 conservative: ratio >= 1.5, < 1.7 (16pt / 10pt = 1.6)
        // WHY: Conservative approach - mid-range headers default to H1
        assert!(
            matches!(
                classifier.classify_block(&make_heading_block(16.0, "Subsection"), body_size),
                BlockType::Header(1)
            ),
            "16pt on 10pt body (1.6x) should be H1 (conservative)"
        );

        // Paragraph: ratio < 1.5 (10pt / 10pt = 1.0)
        assert!(
            matches!(
                classifier.classify_block(&make_heading_block(10.0, "Regular paragraph text"), body_size),
                BlockType::Paragraph
            ),
            "10pt on 10pt body (1.0x) should be Paragraph"
        );

        // Edge case: exactly 1.5x threshold (15pt / 10pt = 1.5)
        // Should NOT be header because we need > 1.5 (header_ratio default)
        assert!(
            matches!(
                classifier.classify_block(&make_heading_block(15.0, "Edge case text"), body_size),
                BlockType::Paragraph
            ),
            "15pt on 10pt body (1.5x) should be Paragraph (at threshold)"
        );
    }
}
