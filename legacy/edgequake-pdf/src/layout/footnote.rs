//! Footnote detection for PDF blocks.
//!
//! Footnotes in academic PDFs are typically:
//! - At the bottom of the page (low y-coordinate)
//! - Smaller font size than body text
//! - Start with a superscript number or symbol
//! - May be separated from body text by a horizontal line
//!
//! ## Detection Algorithm
//!
//! ```text
//! 1. Check if block is in the bottom portion of the page
//! 2. Check if font size is smaller than body text
//! 3. Check if text starts with a footnote marker (number, *, †)
//! ```

use super::pymupdf_structs::Block;

/// Configuration for footnote detection.
#[derive(Debug, Clone)]
pub struct FootnoteConfig {
    /// Portion of page height from bottom to search for footnotes.
    /// Default: 0.25 (bottom 25% of page)
    pub bottom_portion: f32,
    /// Maximum font size ratio (footnote/body) for a block to be a footnote.
    /// Default: 0.85 (footnote font typically 85% of body or smaller)
    pub max_font_ratio: f32,
}

impl Default for FootnoteConfig {
    fn default() -> Self {
        Self {
            bottom_portion: 0.25,
            max_font_ratio: 0.85,
        }
    }
}

/// Check if a block looks like a footnote.
///
/// Returns true if:
/// 1. Block is in the bottom portion of the page
/// 2. Font size is smaller than body text
/// 3. Text starts with a footnote marker
pub fn is_footnote(
    block: &Block,
    page_height: f32,
    body_font_size: f32,
    config: &FootnoteConfig,
) -> bool {
    if block.lines.is_empty() || page_height <= 0.0 || body_font_size <= 0.0 {
        return false;
    }

    // Check position: must be in bottom portion of page
    let bottom_threshold = page_height * config.bottom_portion;
    if block.y1 > bottom_threshold {
        return false; // Too high on the page
    }

    // Check font size: must be smaller than body
    let block_font_size = block
        .lines
        .iter()
        .map(|l| l.dominant_font_size())
        .fold(0.0_f32, f32::max);

    let ratio = block_font_size / body_font_size;
    if ratio > config.max_font_ratio {
        return false; // Font too large for a footnote
    }

    // Check for footnote marker at start of text
    let text = block.text();
    let trimmed = text.trim();
    starts_with_footnote_marker(trimmed)
}

/// Check if text starts with a footnote marker.
///
/// Common patterns:
/// - Superscript numbers: "1 text", "¹text", "²text"
/// - Symbols: "*text", "†text", "‡text"
/// - Bracketed: "[1] text"
/// - OODA-28: Multi-digit numbers: "12 text", "23. text"
/// - OODA-28: Letter markers: "a text", "b) note"
/// - OODA-43: Symbol sequences: "**text", "††text", "‡‡text"
pub fn starts_with_footnote_marker(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }

    let first = text.chars().next().unwrap();

    // Superscript digits (Unicode)
    if matches!(
        first,
        '¹' | '²' | '³' | '⁴' | '⁵' | '⁶' | '⁷' | '⁸' | '⁹' | '⁰'
    ) {
        return true;
    }

    // Footnote symbols (single or repeated sequences)
    if matches!(first, '*' | '†' | '‡' | '§' | '¶' | '‖') {
        // OODA-43: Accept symbol followed by more symbols or space/text
        let rest = &text[first.len_utf8()..];
        // Accept repeated symbols (**, ††, ‡‡) or symbol followed by space/text
        if rest.is_empty() || rest.starts_with(first) || !rest.trim().is_empty() {
            return true;
        }
    }

    // Bracketed number: "[1]", "[2]", "[12]"
    if first == '[' {
        let end = text.find(']');
        if let Some(pos) = end {
            let inner = &text[1..pos];
            if inner.chars().all(|c| c.is_ascii_digit()) && !inner.is_empty() {
                return true;
            }
        }
    }

    // OODA-28: Digit(s) followed by space, period, or parenthesis: "1 ", "12.", "3)"
    if first.is_ascii_digit() {
        let mut chars = text.chars().skip(1);
        // Consume additional digits for multi-digit markers
        let mut next_ch = None;
        for ch in chars.by_ref() {
            if ch.is_ascii_digit() {
                continue;
            }
            next_ch = Some(ch);
            break;
        }
        if let Some(sep) = next_ch {
            if matches!(sep, ' ' | '.' | ')') {
                return true;
            }
        }
    }

    // OODA-28: Single lowercase letter followed by separator: "a ", "b)", "c."
    if first.is_ascii_lowercase() && text.len() > 1 {
        let second = text.chars().nth(1).unwrap_or(' ');
        if matches!(second, ')' | '.' | ' ') {
            // Only if the rest looks like footnote text (short prefix)
            let prefix_end = if matches!(second, ')' | '.') { 2 } else { 1 };
            let rest = text[prefix_end..].trim_start();
            // Must have text after the marker (not just a letter at start of paragraph)
            if !rest.is_empty() && rest.len() > 3 {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::pymupdf_structs::{BlockType, Line, Span};

    fn make_footnote_block(text: &str, font_size: f32, y0: f32, y1: f32) -> Block {
        Block {
            lines: vec![Line::from_span(Span {
                text: text.to_string(),
                x0: 50.0,
                y0,
                x1: 400.0,
                y1,
                font_size,
                font_name: None,
                page_num: 0,
                font_is_bold: None,
                font_is_italic: None,
                font_is_monospace: None,
            })],
            x0: 50.0,
            y0,
            x1: 400.0,
            y1,
            page_num: 0,
            block_type: BlockType::Paragraph,
        }
    }

    #[test]
    fn test_footnote_marker_detection() {
        assert!(starts_with_footnote_marker("¹Corresponding author."));
        assert!(starts_with_footnote_marker("†Equal contribution."));
        assert!(starts_with_footnote_marker("*Work done during internship."));
        assert!(starts_with_footnote_marker("[1] Source reference."));
        assert!(starts_with_footnote_marker("1 This is a footnote."));
        assert!(starts_with_footnote_marker("2. Second footnote."));
        // OODA-28: Multi-digit markers
        assert!(starts_with_footnote_marker("12 Multi-digit footnote."));
        assert!(starts_with_footnote_marker("23. Another footnote."));
        // OODA-28: Letter markers
        assert!(starts_with_footnote_marker("a) Author affiliation info."));
        assert!(starts_with_footnote_marker("b Equal contribution."));
        // OODA-43: Symbol sequences
        assert!(starts_with_footnote_marker("** Double asterisk note."));
        assert!(starts_with_footnote_marker("†† Double dagger note."));
        assert!(starts_with_footnote_marker("§ Section symbol note."));

        assert!(!starts_with_footnote_marker("Normal paragraph text."));
        assert!(!starts_with_footnote_marker("The experiment showed..."));
        assert!(!starts_with_footnote_marker(""));
    }

    #[test]
    fn test_is_footnote_basic() {
        let config = FootnoteConfig::default();
        let page_height = 792.0; // Letter
        let body_font_size = 10.0;

        // Footnote: bottom of page, small font, starts with marker
        let footnote = make_footnote_block("1 Author affiliation.", 8.0, 40.0, 48.0);
        assert!(is_footnote(&footnote, page_height, body_font_size, &config));

        // Not a footnote: body text in middle of page
        let body = make_footnote_block("1 Regular text.", 10.0, 400.0, 410.0);
        assert!(!is_footnote(&body, page_height, body_font_size, &config));
    }

    #[test]
    fn test_footnote_too_large_font() {
        let config = FootnoteConfig::default();
        // Large font at bottom - not a footnote
        let block = make_footnote_block("1 Big header", 12.0, 40.0, 52.0);
        assert!(!is_footnote(&block, 792.0, 10.0, &config));
    }

    #[test]
    fn test_footnote_no_marker() {
        let config = FootnoteConfig::default();
        // Small text at bottom but no marker
        let block = make_footnote_block("Copyright notice", 7.0, 20.0, 27.0);
        assert!(!is_footnote(&block, 792.0, 10.0, &config));
    }
}
