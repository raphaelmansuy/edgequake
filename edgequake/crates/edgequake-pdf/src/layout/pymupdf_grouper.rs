//! Text grouping algorithms for pymupdf4llm-inspired extraction.
//!
//! This module provides the `TextGrouper` that converts a stream of `RawChar`s
//! into structured `Span`s, `Line`s, and `Block`s.
//!
//! ## Algorithm
//!
//! 1. **Char → Span**: Group consecutive chars with same font style
//! 2. **Span → Line**: Group spans on same baseline (vertical tolerance)
//! 3. **Line → Block**: Group lines in same column with vertical proximity
//!
//! This mirrors the pymupdf4llm approach but implemented in pure Rust.

use super::pymupdf_structs::{Block, BlockType, Line, Span};
use crate::backend::elements::RawChar;

/// Parameters for text grouping.
#[derive(Debug, Clone)]
pub struct GroupingParams {
    /// Vertical tolerance for same-line detection (in points)
    pub line_tolerance: f32,
    /// Maximum gap between lines in same block (in points)
    pub block_gap: f32,
    /// Minimum horizontal overlap for same-column detection (0.0-1.0)
    pub column_overlap: f32,
}

impl Default for GroupingParams {
    fn default() -> Self {
        Self {
            line_tolerance: 3.0,
            block_gap: 20.0,
            column_overlap: 0.5,
        }
    }
}

/// Groups raw characters into spans, lines, and blocks.
pub struct TextGrouper {
    params: GroupingParams,
}

impl TextGrouper {
    /// Create a new text grouper with default parameters.
    pub fn new() -> Self {
        Self {
            params: GroupingParams::default(),
        }
    }

    /// Create a text grouper with custom parameters.
    pub fn with_params(params: GroupingParams) -> Self {
        Self { params }
    }

    /// Group raw characters into spans.
    ///
    /// Characters are grouped when they have:
    /// - Same page
    /// - Same font name
    /// - Similar font size (within 0.5pt)
    /// - Horizontal adjacency (gap < 1.5 * char width)
    /// - Vertical alignment (within font_size * 0.3)
    pub fn chars_to_spans(&self, chars: &[RawChar]) -> Vec<Span> {
        if chars.is_empty() {
            return vec![];
        }

        let mut spans = Vec::new();
        let mut current_span = Span::new(chars[0].page_num);

        for ch in chars {
            // Skip control characters and zero-width chars
            if ch.char.is_control() || ch.x0 >= ch.x1 {
                continue;
            }

            if current_span.can_append(ch) {
                current_span.append(ch);
            } else {
                // Save current span if non-empty
                if !current_span.text.is_empty() {
                    spans.push(current_span);
                }
                // Start new span
                current_span = Span::new(ch.page_num);
                current_span.append(ch);
            }
        }

        // Don't forget the last span
        if !current_span.text.is_empty() {
            spans.push(current_span);
        }

        spans
    }

    /// Group spans into lines based on vertical alignment.
    ///
    /// Spans are grouped on the same line if their baseline (y0) or
    /// top (y1) coordinates are within the tolerance.
    pub fn spans_to_lines(&self, spans: Vec<Span>) -> Vec<Line> {
        if spans.is_empty() {
            return vec![];
        }

        // Sort spans by page, then by y (descending = top first), then by x
        let mut sorted_spans = spans;
        sorted_spans.sort_by(|a, b| {
            a.page_num
                .cmp(&b.page_num)
                .then(b.y1.partial_cmp(&a.y1).unwrap()) // descending y
                .then(a.x0.partial_cmp(&b.x0).unwrap()) // ascending x
        });

        let mut lines = Vec::new();
        let mut current_line = Line::from_span(sorted_spans.remove(0));

        for span in sorted_spans {
            if current_line.can_add_span(&span, self.params.line_tolerance) {
                current_line.add_span(span);
            } else {
                // Finalize current line
                current_line.sort_spans();
                lines.push(current_line);
                // Start new line
                current_line = Line::from_span(span);
            }
        }

        // Don't forget the last line
        current_line.sort_spans();
        lines.push(current_line);

        // Sort lines by page, then top-to-bottom
        lines.sort_by(|a, b| {
            a.page_num
                .cmp(&b.page_num)
                .then(b.y1.partial_cmp(&a.y1).unwrap())
        });

        lines
    }

    /// Group lines into blocks based on column alignment and vertical proximity.
    pub fn lines_to_blocks(&self, lines: Vec<Line>) -> Vec<Block> {
        if lines.is_empty() {
            return vec![];
        }

        let mut blocks: Vec<Block> = Vec::new();

        for line in lines {
            // Try to add to an existing block
            let mut added = false;
            for block in &mut blocks {
                if block.can_add_line(&line, self.params.block_gap) {
                    block.add_line(line.clone());
                    added = true;
                    break;
                }
            }

            if !added {
                blocks.push(Block::from_line(line));
            }
        }

        // Sort lines within each block
        for block in &mut blocks {
            block.sort_lines();
        }

        // Sort blocks by page, then reading order (top-to-bottom, left-to-right)
        blocks.sort_by(|a, b| {
            a.page_num
                .cmp(&b.page_num)
                .then(b.y1.partial_cmp(&a.y1).unwrap()) // top first
                .then(a.x0.partial_cmp(&b.x0).unwrap()) // left first
        });

        blocks
    }

    /// Full pipeline: chars → spans → lines → blocks
    pub fn group(&self, chars: &[RawChar]) -> Vec<Block> {
        let spans = self.chars_to_spans(chars);
        let lines = self.spans_to_lines(spans);
        self.lines_to_blocks(lines)
    }

    /// Detect block types based on content analysis.
    ///
    /// This analyzes:
    /// - Font size relative to body text → headers
    /// - Monospace fonts → code blocks
    /// - Bullet/number prefixes → list items
    pub fn classify_blocks(&self, blocks: &mut [Block], body_font_size: f32) {
        for block in blocks {
            block.block_type = self.classify_block(block, body_font_size);
        }
    }

    fn classify_block(&self, block: &Block, body_font_size: f32) -> BlockType {
        if block.lines.is_empty() {
            return BlockType::Paragraph;
        }

        // Check for code block (all monospace)
        let all_mono = block.lines.iter().all(|line| {
            line.spans.iter().all(|span| span.is_monospace())
        });
        if all_mono {
            return BlockType::Code;
        }

        // Check for header (larger font size, single line usually)
        let dominant_size = block.lines.iter()
            .map(|l| l.dominant_font_size())
            .fold(0.0_f32, |a, b| a.max(b));

        if dominant_size > body_font_size * 1.2 && block.lines.len() <= 2 {
            // Map size ratio to header level
            let ratio = dominant_size / body_font_size;
            let level = if ratio >= 2.0 {
                1
            } else if ratio >= 1.7 {
                2
            } else if ratio >= 1.5 {
                3
            } else if ratio >= 1.3 {
                4
            } else {
                5
            };
            return BlockType::Header(level);
        }

        // Check for list item
        if let Some(first_line) = block.lines.first() {
            let text = first_line.text();
            let trimmed = text.trim_start();
            if trimmed.starts_with("• ")
                || trimmed.starts_with("- ")
                || trimmed.starts_with("* ")
                || is_numbered_list_item(trimmed)
            {
                return BlockType::ListItem;
            }
        }

        BlockType::Paragraph
    }
}

impl Default for TextGrouper {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if text starts with a numbered list item pattern.
fn is_numbered_list_item(text: &str) -> bool {
    let mut chars = text.chars().peekable();

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

    // Check for separator (., ), :)
    match chars.next() {
        Some('.') | Some(')') | Some(':') => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_char(c: char, x0: f32, y0: f32, font_size: f32, page: usize) -> RawChar {
        let width = font_size * 0.6; // Approximate character width
        RawChar {
            char: c,
            x0,
            y0,
            x1: x0 + width,
            y1: y0 + font_size,
            font_size,
            font_name: Some("Arial".to_string()),
            page_num: page,
        }
    }

    #[test]
    fn test_chars_to_spans() {
        let grouper = TextGrouper::new();

        // Create "Hi" on one line
        let chars = vec![
            make_char('H', 10.0, 100.0, 12.0, 0),
            make_char('i', 17.2, 100.0, 12.0, 0),
        ];

        let spans = grouper.chars_to_spans(&chars);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "Hi");
    }

    #[test]
    fn test_spans_to_lines() {
        let grouper = TextGrouper::new();

        // Two spans on same line
        let spans = vec![
            Span {
                text: "Hello".to_string(),
                x0: 10.0,
                y0: 100.0,
                x1: 50.0,
                y1: 112.0,
                font_size: 12.0,
                font_name: Some("Arial".to_string()),
                page_num: 0,
            },
            Span {
                text: "World".to_string(),
                x0: 55.0,
                y0: 100.0,
                x1: 95.0,
                y1: 112.0,
                font_size: 12.0,
                font_name: Some("Arial".to_string()),
                page_num: 0,
            },
        ];

        let lines = grouper.spans_to_lines(spans);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text(), "Hello World");
    }

    #[test]
    fn test_full_pipeline() {
        let grouper = TextGrouper::new();

        // Create two lines of text
        let chars = vec![
            // Line 1: "Hi"
            make_char('H', 10.0, 100.0, 12.0, 0),
            make_char('i', 17.2, 100.0, 12.0, 0),
            // Line 2: "Bye" (lower y = below line 1)
            make_char('B', 10.0, 85.0, 12.0, 0),
            make_char('y', 17.2, 85.0, 12.0, 0),
            make_char('e', 24.4, 85.0, 12.0, 0),
        ];

        let blocks = grouper.group(&chars);

        // Should produce one block with two lines
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].lines.len(), 2);
    }

    #[test]
    fn test_numbered_list_detection() {
        assert!(is_numbered_list_item("1. First item"));
        assert!(is_numbered_list_item("23) Item"));
        assert!(is_numbered_list_item("5: Something"));
        assert!(!is_numbered_list_item("No number here"));
        assert!(!is_numbered_list_item("a. Letter prefix"));
    }

    #[test]
    fn test_block_classification() {
        let mut grouper = TextGrouper::new();
        let body_size = 12.0;

        // Header block (larger font)
        let mut header_block = Block::from_line(Line {
            spans: vec![Span {
                text: "Title".to_string(),
                x0: 10.0,
                y0: 100.0,
                x1: 100.0,
                y1: 130.0,
                font_size: 24.0,
                font_name: Some("Arial-Bold".to_string()),
                page_num: 0,
            }],
            x0: 10.0,
            y0: 100.0,
            x1: 100.0,
            y1: 130.0,
            page_num: 0,
        });

        grouper.classify_blocks(std::slice::from_mut(&mut header_block), body_size);
        assert!(matches!(header_block.block_type, BlockType::Header(1)));

        // List item block
        let mut list_block = Block::from_line(Line {
            spans: vec![Span {
                text: "• Item one".to_string(),
                x0: 10.0,
                y0: 50.0,
                x1: 100.0,
                y1: 62.0,
                font_size: 12.0,
                font_name: Some("Arial".to_string()),
                page_num: 0,
            }],
            x0: 10.0,
            y0: 50.0,
            x1: 100.0,
            y1: 62.0,
            page_num: 0,
        });

        grouper.classify_blocks(std::slice::from_mut(&mut list_block), body_size);
        assert_eq!(list_block.block_type, BlockType::ListItem);
    }
}
