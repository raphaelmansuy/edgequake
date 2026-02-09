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

use super::footnote::{is_footnote, FootnoteConfig};
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
    /// OODA-08: Footnote detection configuration.
    pub footnote_config: FootnoteConfig,
}

impl Default for BlockClassifier {
    fn default() -> Self {
        Self {
            // OODA-12: Conservative threshold to match pymupdf4llm gold
            header_ratio: 1.50,
            max_header_lines: 2,
            max_header_chars: 150,
            footnote_config: FootnoteConfig::default(),
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
        self.classify_blocks_with_page(blocks, body_font_size, 0.0);
    }

    /// OODA-08: Classify all blocks with page height for footnote detection.
    /// WHY: Footnotes require page_height to check if block is in the bottom
    /// portion of the page. When page_height > 0, footnote detection is enabled.
    pub fn classify_blocks_with_page(
        &self,
        blocks: &mut [Block],
        body_font_size: f32,
        page_height: f32,
    ) {
        for block in blocks {
            block.block_type = self.classify_block(block, body_font_size, page_height);
        }
    }

    /// Classify a single block.
    ///
    /// Classification priority:
    /// 1. Code (all monospace fonts)
    /// 2. Header (large font, short text)
    /// 3. Header (bold-only text with header patterns) [OODA-06]
    /// 4. ListItem (bullet/numbered prefix)
    /// 5. Footnote (bottom of page, small font, starts with marker) [OODA-08]
    /// 6. Paragraph (default)
    pub fn classify_block(
        &self,
        block: &Block,
        body_font_size: f32,
        page_height: f32,
    ) -> BlockType {
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

        // Get first line text for pattern matching
        let first_text = block.lines.first().map(|l| l.text()).unwrap_or_default();
        let trimmed = first_text.trim();

        // Check for header based on font size (OODA-12)
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
            // OODA-33: Finer-grained heading level based on size ratio.
            // - 2.5x+: Document title = h1 (#)
            // - 2.0x:  Major heading  = h1 (#)
            // - 1.7x:  Section        = h2 (##)
            // - 1.5x:  Subsection     = h3 (###)
            let level = if ratio >= 2.0 {
                1 // Very large = #
            } else if ratio >= 1.7 {
                2 // Large = ##
            } else {
                3 // Medium-large = ###
            };
            return BlockType::Header(level);
        }

        // OODA-06: Bold-only header detection.
        // WHY: Many academic PDFs (IEEE, NeurIPS, ICML) have section headers
        // at the same font size as body text, distinguished only by bold weight.
        // Combined with pattern matching for safety (reduce false positives).
        //
        // Criteria:
        // 1. ALL spans are bold
        // 2. Short text (< 80 chars, <= 2 lines)
        // 3. Matches header pattern OR is short enough to be a title
        // 4. NOT a list item
        if block.lines.len() <= self.max_header_lines
            && total_chars < 80
            && !is_bullet_item(trimmed)
            && !is_numbered_list_item(trimmed)
        {
            let all_bold = block
                .lines
                .iter()
                .all(|line| line.spans.iter().all(|span| span.is_bold()));

            if all_bold
                && (is_abstract_header(trimmed)
                    || is_roman_numeral_header(trimmed)
                    || is_numeric_section_header(trimmed)
                    || is_numeric_subsection_header(trimmed)
                    || is_numeric_sub_subsection_header(trimmed)
                    || is_letter_subsection_header(trimmed)
                    || is_all_caps_header(trimmed))
            {
                // Determine level based on pattern
                let level = if is_roman_numeral_header(trimmed)
                    || is_numeric_section_header(trimmed)
                    || is_abstract_header(trimmed)
                {
                    2 // Major section = ##
                } else if is_numeric_subsection_header(trimmed)
                    || is_letter_subsection_header(trimmed)
                {
                    3 // Subsection = ###
                } else if is_numeric_sub_subsection_header(trimmed) {
                    4 // OODA-23: Sub-subsection = ####
                } else {
                    2 // Default for all-caps headers
                };
                return BlockType::Header(level);
            }
        }

        // Check for list item
        if let Some(first_line) = block.lines.first() {
            let text = first_line.text();
            let trimmed = text.trim_start();
            if is_bullet_item(trimmed) || is_numbered_list_item(trimmed) {
                return BlockType::ListItem;
            }
        }

        // OODA-08: Check for footnote (bottom of page, small font, starts with marker)
        // WHY: Footnotes are common in academic PDFs and should be rendered as blockquotes
        // to visually separate them from body text.
        if page_height > 0.0
            && body_font_size > 0.0
            && is_footnote(block, page_height, body_font_size, &self.footnote_config)
        {
            return BlockType::Footnote;
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
/// Patterns: "1. ", "2) ", "3: ", "(1) ", "(a) "
///
/// OODA-10: Excludes section header patterns (X.Y.) which look similar.
/// OODA-37: Added parenthesized patterns "(1)", "(a)", "(i)".
pub fn is_numbered_list_item(text: &str) -> bool {
    let trimmed = text.trim_start();

    // OODA-37: Check for parenthesized list items: "(1) ", "(a) ", "(i) "
    if trimmed.starts_with('(') {
        if let Some(close_pos) = trimmed.find(')') {
            if close_pos > 1 && close_pos < 6 {
                let inner = &trimmed[1..close_pos];
                let is_valid = inner.chars().all(|c| c.is_ascii_digit())
                    || (inner.len() == 1 && inner.chars().all(|c| c.is_ascii_lowercase()))
                    || inner.chars().all(|c| matches!(c, 'i' | 'v' | 'x'));
                if is_valid {
                    // Must have space or text after closing paren
                    let after = &trimmed[close_pos + 1..];
                    if after.starts_with(' ') || after.starts_with('\t') {
                        return true;
                    }
                }
            }
        }
    }

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
            // OODA-23: Check for sub-subsection (X.Y.Z)
            Some(c) if c.is_ascii_digit() => false, // Handled by is_numeric_sub_subsection_header
            _ => false,
        },
        Some(' ') => chars.next().map(|c| c.is_uppercase()).unwrap_or(false),
        _ => false,
    }
}

/// OODA-23: Check if text starts with a numeric sub-subsection pattern.
///
/// Patterns: "2.1.1 Training Details", "3.2.3. Evaluation"
///
/// WHY: Papers with deep structure use X.Y.Z numbering for sub-subsections.
pub fn is_numeric_sub_subsection_header(text: &str) -> bool {
    if text.len() < 8 {
        return false;
    }

    let mut chars = text.chars().peekable();

    // First number (section)
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

    // Second number (subsection)
    let mut has_second = false;
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            has_second = true;
            chars.next();
        } else {
            break;
        }
    }
    if !has_second || chars.next() != Some('.') {
        return false;
    }

    // Third number (sub-subsection)
    let mut has_third = false;
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            has_third = true;
            chars.next();
        } else {
            break;
        }
    }
    if !has_third {
        return false;
    }

    // Must be followed by ". " or " "
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

/// OODA-06: Check if text is an all-caps header.
///
/// Patterns: "REFERENCES", "ACKNOWLEDGMENTS", "CONCLUSION", "INTRODUCTION"
///
/// Criteria:
/// - At least 3 alpha characters (excludes "I.", "II.")
/// - At least 60% uppercase among alphabetic characters
/// - No more than 50 chars (excludes full article titles)
pub fn is_all_caps_header(text: &str) -> bool {
    let trimmed = text.trim();
    let alpha_count = trimmed.chars().filter(|c| c.is_alphabetic()).count();
    if alpha_count < 3 || trimmed.len() > 50 {
        return false;
    }
    let upper_count = trimmed.chars().filter(|c| c.is_uppercase()).count();
    (upper_count as f32 / alpha_count as f32) >= 0.6
}

/// OODA-31: Check if text starts with a figure/table caption pattern.
///
/// Patterns: "Figure 1:", "Fig. 2.", "Table 3:", "Tab. 1.", "Figure S1:"
///
/// WHY: Captions in academic PDFs should be rendered differently from body text.
/// They typically start with Figure/Table followed by a number and separator.
pub fn is_caption(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.len() < 6 {
        return false;
    }
    let lower = trimmed.to_lowercase();
    // Full prefix patterns
    let prefixes = [
        "figure ", "fig. ", "fig ", "table ", "tab. ", "tab ", "scheme ", "chart ", "graph ",
        "plate ", "listing ",
    ];
    for prefix in &prefixes {
        if lower.starts_with(prefix) {
            let rest = &trimmed[prefix.len()..];
            // Must be followed by a digit (or 's' for supplementary: "S1")
            if let Some(first) = rest.chars().next() {
                if first.is_ascii_digit() || first == 'S' || first == 's' {
                    return true;
                }
            }
        }
    }
    false
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
        // OODA-37: Parenthesized list items
        assert!(is_numbered_list_item("(1) First item"));
        assert!(is_numbered_list_item("(a) Sub-item"));
        assert!(is_numbered_list_item("(ii) Roman numeral"));

        // Section headers should NOT match
        assert!(!is_numbered_list_item("2.1. Subsection"));
        assert!(!is_numbered_list_item("3.2 Architecture"));
        // Parenthesized non-list
        assert!(!is_numbered_list_item("(see above)"));
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
            classifier.classify_block(&header_block, body_size, 0.0),
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
            classifier.classify_block(&list_block, body_size, 0.0),
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
                classifier.classify_block(&make_heading_block(20.0, "Major Title"), body_size, 0.0),
                BlockType::Header(1)
            ),
            "20pt on 10pt body (2.0x) should be H1"
        );

        // H2: ratio >= 1.7, < 2.0 (18pt / 10pt = 1.8)
        assert!(
            matches!(
                classifier.classify_block(
                    &make_heading_block(18.0, "Section Heading"),
                    body_size,
                    0.0
                ),
                BlockType::Header(2)
            ),
            "18pt on 10pt body (1.8x) should be H2"
        );

        // H3: ratio >= 1.5, < 1.7 (16pt / 10pt = 1.6)
        // OODA-33: Mid-range headers now map to H3 for finer granularity
        assert!(
            matches!(
                classifier.classify_block(&make_heading_block(16.0, "Subsection"), body_size, 0.0),
                BlockType::Header(3)
            ),
            "16pt on 10pt body (1.6x) should be H3 (OODA-33 finer levels)"
        );

        // Paragraph: ratio < 1.5 (10pt / 10pt = 1.0)
        assert!(
            matches!(
                classifier.classify_block(
                    &make_heading_block(10.0, "Regular paragraph text"),
                    body_size,
                    0.0
                ),
                BlockType::Paragraph
            ),
            "10pt on 10pt body (1.0x) should be Paragraph"
        );

        // Edge case: exactly 1.5x threshold (15pt / 10pt = 1.5)
        // Should NOT be header because we need > 1.5 (header_ratio default)
        assert!(
            matches!(
                classifier.classify_block(
                    &make_heading_block(15.0, "Edge case text"),
                    body_size,
                    0.0
                ),
                BlockType::Paragraph
            ),
            "15pt on 10pt body (1.5x) should be Paragraph (at threshold)"
        );
    }

    /// OODA-15: Test subsection pattern detection functions.
    /// WHY: Validates IEEE-style (A. B. C.) and ICML-style (1. 2.) and X.Y patterns.
    #[test]
    fn test_subsection_patterns() {
        // Letter subsection (IEEE-style): "A. Background"
        assert!(
            is_letter_subsection_header("A. Background"),
            "A. Background should match letter subsection"
        );
        assert!(
            is_letter_subsection_header("B. Policy Representations"),
            "B. Policy should match"
        );
        assert!(
            is_letter_subsection_header("Z. Final Section"),
            "Z. Final should match"
        );

        // Invalid letter subsections
        assert!(
            !is_letter_subsection_header("A.NoSpace"),
            "No space after period"
        );
        assert!(
            !is_letter_subsection_header("AB. Too Long"),
            "Multiple letters"
        );
        assert!(
            !is_letter_subsection_header("1. Not a letter"),
            "Digit not letter"
        );

        // Numeric section (ICML-style): "1. INTRODUCTION"
        assert!(
            is_numeric_section_header("1. INTRODUCTION"),
            "1. INTRO should match"
        );
        assert!(
            is_numeric_section_header("2. METHODS"),
            "2. METHODS should match"
        );

        // Invalid numeric sections
        assert!(
            !is_numeric_section_header("1.1. Subsection"),
            "X.Y is not a section"
        );
        assert!(
            !is_numeric_section_header("1. lowercase text"),
            "Must have uppercase"
        );
        assert!(
            !is_numeric_section_header("2 METHODS"),
            "Missing period after digit"
        );

        // Numeric subsection: "2.1. Agentic Training"
        assert!(
            is_numeric_subsection_header("2.1. Agentic Training"),
            "2.1. should match"
        );
        assert!(
            is_numeric_subsection_header("3.2 Architecture"),
            "3.2 should match"
        );

        // Invalid numeric subsections
        assert!(
            !is_numeric_subsection_header("2. Main section"),
            "Single number is section"
        );
        assert!(
            !is_numeric_subsection_header("Not a subsection"),
            "No pattern"
        );
    }

    /// OODA-06: Test all-caps header detection.
    #[test]
    fn test_all_caps_header() {
        assert!(is_all_caps_header("REFERENCES"));
        assert!(is_all_caps_header("ACKNOWLEDGMENTS"));
        assert!(is_all_caps_header("CONCLUSION"));
        assert!(is_all_caps_header("I. INTRODUCTION")); // Roman + caps

        // Not all-caps headers
        assert!(!is_all_caps_header("Normal text here"));
        assert!(!is_all_caps_header("AB")); // Too short (< 3 alpha)
        assert!(!is_all_caps_header("")); // Empty
    }

    /// OODA-06: Test bold-only header classification for academic papers.
    #[test]
    fn test_bold_header_classification() {
        let classifier = BlockClassifier::new();
        let body_size = 10.0;

        // Bold academic section header "I. INTRODUCTION" at body font size
        let bold_section = Block::from_line(Line {
            spans: vec![Span {
                text: "I. INTRODUCTION".to_string(),
                x0: 10.0,
                y0: 100.0,
                x1: 150.0,
                y1: 110.0,
                font_size: 10.0, // Same as body
                font_name: Some("Times-Bold".to_string()),
                page_num: 0,
                font_is_bold: Some(true),
                font_is_italic: Some(false),
                font_is_monospace: Some(false),
            }],
            x0: 10.0,
            y0: 100.0,
            x1: 150.0,
            y1: 110.0,
            page_num: 0,
        });

        assert!(
            matches!(
                classifier.classify_block(&bold_section, body_size, 0.0),
                BlockType::Header(2)
            ),
            "Bold 'I. INTRODUCTION' should be H2"
        );

        // Bold "Abstract" header
        let bold_abstract = Block::from_line(Line {
            spans: vec![Span {
                text: "Abstract".to_string(),
                x0: 10.0,
                y0: 100.0,
                x1: 80.0,
                y1: 110.0,
                font_size: 10.0,
                font_name: Some("Arial-Bold".to_string()),
                page_num: 0,
                font_is_bold: Some(true),
                font_is_italic: Some(false),
                font_is_monospace: Some(false),
            }],
            x0: 10.0,
            y0: 100.0,
            x1: 80.0,
            y1: 110.0,
            page_num: 0,
        });

        assert!(
            matches!(
                classifier.classify_block(&bold_abstract, body_size, 0.0),
                BlockType::Header(2)
            ),
            "Bold 'Abstract' should be H2"
        );

        // Non-bold text should NOT be classified as header
        let non_bold = Block::from_line(Line {
            spans: vec![Span {
                text: "I. INTRODUCTION".to_string(),
                x0: 10.0,
                y0: 100.0,
                x1: 150.0,
                y1: 110.0,
                font_size: 10.0,
                font_name: Some("Times".to_string()),
                page_num: 0,
                font_is_bold: Some(false),
                font_is_italic: Some(false),
                font_is_monospace: Some(false),
            }],
            x0: 10.0,
            y0: 100.0,
            x1: 150.0,
            y1: 110.0,
            page_num: 0,
        });

        assert!(
            matches!(
                classifier.classify_block(&non_bold, body_size, 0.0),
                BlockType::Paragraph
            ),
            "Non-bold 'I. INTRODUCTION' should remain Paragraph"
        );
    }

    /// OODA-08: Test footnote classification
    #[test]
    fn test_classify_footnote() {
        let classifier = BlockClassifier::new();
        let body_size = 10.0;
        let page_height = 792.0; // US Letter

        // Footnote: small font, bottom of page, starts with marker
        let footnote_block = Block {
            lines: vec![Line::from_span(crate::layout::pymupdf_structs::Span {
                text: "1 Author affiliation and correspondence.".to_string(),
                x0: 50.0,
                y0: 40.0,
                x1: 400.0,
                y1: 48.0,
                font_size: 8.0, // 80% of body = footnote size
                font_name: Some("Arial".to_string()),
                page_num: 0,
                font_is_bold: Some(false),
                font_is_italic: Some(false),
                font_is_monospace: Some(false),
            })],
            x0: 50.0,
            y0: 40.0,
            x1: 400.0,
            y1: 48.0,
            page_num: 0,
            block_type: BlockType::Paragraph,
        };

        assert!(
            matches!(
                classifier.classify_block(&footnote_block, body_size, page_height),
                BlockType::Footnote
            ),
            "Block at bottom of page with small font and marker should be Footnote"
        );

        // Same block but without page_height = no footnote detection
        assert!(
            matches!(
                classifier.classify_block(&footnote_block, body_size, 0.0),
                BlockType::Paragraph
            ),
            "Without page_height, should fall back to Paragraph"
        );
    }

    /// OODA-23: Test sub-subsection header detection
    #[test]
    fn test_sub_subsection_header() {
        assert!(is_numeric_sub_subsection_header("2.1.1 Training Details"));
        assert!(is_numeric_sub_subsection_header(
            "3.2.3. Evaluation Metrics"
        ));
        assert!(is_numeric_sub_subsection_header("1.1.1. Background"));

        assert!(!is_numeric_sub_subsection_header("2.1 Subsection")); // Only two levels
        assert!(!is_numeric_sub_subsection_header("1. Section")); // Only one level
        assert!(!is_numeric_sub_subsection_header("short")); // Too short
    }

    /// OODA-31: Test caption detection
    #[test]
    fn test_caption_detection() {
        assert!(is_caption("Figure 1: Overview of the architecture."));
        assert!(is_caption("Fig. 2. Results of our experiment."));
        assert!(is_caption("Table 3: Performance comparison."));
        assert!(is_caption("Tab. 1. Summary statistics."));
        assert!(is_caption("Figure S1: Supplementary analysis."));
        assert!(is_caption("Listing 1: Code example."));
        assert!(is_caption("Scheme 2: Reaction pathway."));

        assert!(!is_caption("The figure shows results."));
        assert!(!is_caption("Figure out the solution."));
        assert!(!is_caption("short"));
        assert!(!is_caption(""));
    }
}
