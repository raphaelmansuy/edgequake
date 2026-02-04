//! Layout structures for pymupdf4llm-inspired text extraction.
//!
//! This module implements the core text hierarchy used by pymupdf4llm:
//!
//! ```text
//! RawChar → Span → Line → Block → Page
//! ```
//!
//! ## Key Concepts
//!
//! - **Span**: Contiguous characters with same font style (name, size, flags)
//! - **Line**: Spans on same baseline (within vertical tolerance)
//! - **Block**: Lines in same column/region
//! - **Page**: All blocks on a page, with layout metadata
//!
//! ## Algorithm Overview
//!
//! 1. Group consecutive `RawChar`s with same style → `Span`
//! 2. Group `Span`s on same baseline (±tolerance) → `Line`
//! 3. Detect columns using gap analysis
//! 4. Group `Line`s in same column → `Block`
//! 5. Render `Block`s as Markdown with proper reading order

use crate::backend::elements::RawChar;

/// A span is a contiguous sequence of characters with the same font style.
///
/// This corresponds to PyMuPDF's "span" in the DICT extraction format.
#[derive(Debug, Clone)]
pub struct Span {
    /// The text content of this span
    pub text: String,
    /// Bounding box: left edge
    pub x0: f32,
    /// Bounding box: bottom edge (PDF coordinates, origin at bottom-left)
    pub y0: f32,
    /// Bounding box: right edge
    pub x1: f32,
    /// Bounding box: top edge
    pub y1: f32,
    /// Font size in points
    pub font_size: f32,
    /// Font name (e.g., "Arial-Bold", "Times-Italic")
    pub font_name: Option<String>,
    /// Page number (0-indexed)
    pub page_num: usize,
}

impl Span {
    /// Create an empty span at the given position.
    pub fn new(page_num: usize) -> Self {
        Self {
            text: String::new(),
            x0: f32::MAX,
            y0: f32::MAX,
            x1: f32::MIN,
            y1: f32::MIN,
            font_size: 0.0,
            font_name: None,
            page_num,
        }
    }

    /// Check if a character can be appended to this span (same style, same word).
    ///
    /// Returns false if:
    /// - Different page
    /// - Different font style (name, size)
    /// - Large horizontal gap (word boundary)
    /// - Vertical misalignment
    pub fn can_append(&self, ch: &RawChar) -> bool {
        if self.text.is_empty() {
            return true;
        }

        // Same page
        if self.page_num != ch.page_num {
            return false;
        }

        // Same font size (within tolerance)
        if (self.font_size - ch.font_size).abs() > 0.5 {
            return false;
        }

        // Same font name
        if self.font_name != ch.font_name {
            return false;
        }

        // Vertically aligned (baseline within tolerance)
        let y_tolerance = self.font_size * 0.3;
        if (self.y0 - ch.y0).abs() > y_tolerance {
            return false;
        }

        // Check horizontal gap for word boundary detection
        // A space is typically ~0.25-0.33 of font size
        let space_width = self.font_size * 0.25;
        let gap = ch.x0 - self.x1;

        // If gap is larger than a space, it's a word boundary → new span
        if gap > space_width {
            return false;
        }

        // If gap is negative (overlapping or backwards), reject
        // unless it's minor overlap from kerning
        let avg_char_width = (self.x1 - self.x0) / self.text.len().max(1) as f32;
        if gap < -avg_char_width * 0.3 {
            return false;
        }

        true
    }

    /// Append a character to this span.
    pub fn append(&mut self, ch: &RawChar) {
        if self.text.is_empty() {
            self.font_size = ch.font_size;
            self.font_name = ch.font_name.clone();
        }

        self.text.push(ch.char);
        self.x0 = self.x0.min(ch.x0);
        self.y0 = self.y0.min(ch.y0);
        self.x1 = self.x1.max(ch.x1);
        self.y1 = self.y1.max(ch.y1);
    }

    /// Width of the span in points.
    pub fn width(&self) -> f32 {
        self.x1 - self.x0
    }

    /// Height of the span in points.
    pub fn height(&self) -> f32 {
        self.y1 - self.y0
    }

    /// Check if this span is bold based on font name.
    pub fn is_bold(&self) -> bool {
        self.font_name
            .as_ref()
            .map(|n| {
                let lower = n.to_lowercase();
                lower.contains("bold") || lower.contains("black") || lower.contains("heavy")
            })
            .unwrap_or(false)
    }

    /// Check if this span is italic based on font name.
    pub fn is_italic(&self) -> bool {
        self.font_name
            .as_ref()
            .map(|n| {
                let lower = n.to_lowercase();
                lower.contains("italic") || lower.contains("oblique")
            })
            .unwrap_or(false)
    }

    /// Check if this span uses a monospace font.
    pub fn is_monospace(&self) -> bool {
        self.font_name
            .as_ref()
            .map(|n| {
                let lower = n.to_lowercase();
                lower.contains("mono")
                    || lower.contains("courier")
                    || lower.contains("consolas")
                    || lower.contains("menlo")
                    || lower.contains("source code")
            })
            .unwrap_or(false)
    }
}

/// A line is a sequence of spans on the same baseline.
#[derive(Debug, Clone)]
pub struct Line {
    /// Spans in this line, sorted left-to-right
    pub spans: Vec<Span>,
    /// Bounding box: left edge
    pub x0: f32,
    /// Bounding box: bottom edge
    pub y0: f32,
    /// Bounding box: right edge
    pub x1: f32,
    /// Bounding box: top edge
    pub y1: f32,
    /// Page number (0-indexed)
    pub page_num: usize,
}

impl Line {
    /// Create a new line from a single span.
    pub fn from_span(span: Span) -> Self {
        Self {
            x0: span.x0,
            y0: span.y0,
            x1: span.x1,
            y1: span.y1,
            page_num: span.page_num,
            spans: vec![span],
        }
    }

    /// Check if a span belongs on this line (same baseline).
    ///
    /// Returns false if:
    /// - Span is on a different page
    /// - Vertical alignment differs by more than tolerance
    /// - There's a large horizontal gap (column gutter detection)
    pub fn can_add_span(&self, span: &Span, tolerance: f32) -> bool {
        if self.page_num != span.page_num {
            return false;
        }

        // Compare baseline (bottom of character)
        // Also allow top alignment for consistency
        let vertically_aligned = (self.y0 - span.y0).abs() <= tolerance 
            || (self.y1 - span.y1).abs() <= tolerance;
        
        if !vertically_aligned {
            return false;
        }

        // Check for column gutter - large horizontal gap suggests different column
        // A typical column gutter in academic papers is 15-30pt
        // We use 50pt as a generous threshold
        let column_gap_threshold = 50.0;
        
        // Check if this span would be far from the current line extent
        let gap_from_line = if span.x0 > self.x1 {
            span.x0 - self.x1  // span is to the right
        } else if self.x0 > span.x1 {
            self.x0 - span.x1  // span is to the left
        } else {
            0.0  // overlapping
        };
        
        // If the gap is larger than a typical column gutter, treat as different line
        gap_from_line < column_gap_threshold
    }

    /// Add a span to this line.
    pub fn add_span(&mut self, span: Span) {
        self.x0 = self.x0.min(span.x0);
        self.y0 = self.y0.min(span.y0);
        self.x1 = self.x1.max(span.x1);
        self.y1 = self.y1.max(span.y1);
        self.spans.push(span);
    }

    /// Sort spans left-to-right.
    pub fn sort_spans(&mut self) {
        self.spans.sort_by(|a, b| a.x0.partial_cmp(&b.x0).unwrap());
    }

    /// Get the full text of this line with appropriate spacing between spans.
    ///
    /// Uses gap analysis to determine if a space is needed between spans.
    /// Avoids adding spaces before hyphens/dashes to preserve hyphenated words.
    pub fn text(&self) -> String {
        if self.spans.is_empty() {
            return String::new();
        }

        if self.spans.len() == 1 {
            return self.spans[0].text.clone();
        }

        let mut result = String::new();
        for (i, span) in self.spans.iter().enumerate() {
            if i > 0 {
                // Check gap between this span and previous
                let prev = &self.spans[i - 1];
                let gap = span.x0 - prev.x1;

                // Only add space if there's a significant gap
                // Use the average font size for threshold
                let avg_size = (prev.font_size + span.font_size) / 2.0;
                let space_threshold = avg_size * 0.15; // ~15% of font size

                // Don't add space if current span starts with hyphen/dash
                // This preserves hyphenated words like "Qwen2.5-7B-Instruct"
                let starts_with_hyphen = span.text.starts_with('-') 
                    || span.text.starts_with('–')  // en-dash
                    || span.text.starts_with('—'); // em-dash

                // Don't add space if previous span ends with hyphen
                let ends_with_hyphen = prev.text.ends_with('-')
                    || prev.text.ends_with('–')
                    || prev.text.ends_with('—');

                if gap > space_threshold && !starts_with_hyphen && !ends_with_hyphen {
                    result.push(' ');
                }
            }
            result.push_str(&span.text);
        }
        result
    }

    /// Width of the line in points.
    pub fn width(&self) -> f32 {
        self.x1 - self.x0
    }

    /// Height of the line in points.
    pub fn height(&self) -> f32 {
        self.y1 - self.y0
    }

    /// Get the dominant font size in this line.
    pub fn dominant_font_size(&self) -> f32 {
        if self.spans.is_empty() {
            return 0.0;
        }

        // Weight by text length
        let mut total_weight = 0.0;
        let mut weighted_size = 0.0;
        for span in &self.spans {
            let weight = span.text.len() as f32;
            weighted_size += span.font_size * weight;
            total_weight += weight;
        }

        if total_weight > 0.0 {
            weighted_size / total_weight
        } else {
            self.spans[0].font_size
        }
    }
}

/// A block is a collection of lines in the same column/region.
#[derive(Debug, Clone)]
pub struct Block {
    /// Lines in this block, sorted top-to-bottom
    pub lines: Vec<Line>,
    /// Bounding box: left edge
    pub x0: f32,
    /// Bounding box: bottom edge  
    pub y0: f32,
    /// Bounding box: right edge
    pub x1: f32,
    /// Bounding box: top edge
    pub y1: f32,
    /// Page number (0-indexed)
    pub page_num: usize,
    /// Block type for markdown rendering
    pub block_type: BlockType,
}

/// Type of content block for markdown rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    /// Regular paragraph text
    Paragraph,
    /// Header (h1-h6)
    Header(u8),
    /// Code block (monospace)
    Code,
    /// List item (bullet or numbered)
    ListItem,
    /// Table content
    Table,
}

impl Block {
    /// Create a new block from a single line.
    pub fn from_line(line: Line) -> Self {
        Self {
            x0: line.x0,
            y0: line.y0,
            x1: line.x1,
            y1: line.y1,
            page_num: line.page_num,
            lines: vec![line],
            block_type: BlockType::Paragraph,
        }
    }

    /// Check if a line belongs to this block.
    ///
    /// Uses horizontal overlap and vertical proximity.
    pub fn can_add_line(&self, line: &Line, line_gap_tolerance: f32) -> bool {
        if self.page_num != line.page_num {
            return false;
        }

        // Check vertical proximity (line should be below current block)
        // Note: PDF y-coordinates increase upward, so y0 of new line should be less
        let vertical_gap = self.y0 - line.y1;
        if vertical_gap < 0.0 || vertical_gap > line_gap_tolerance {
            return false;
        }

        // Check horizontal overlap
        let overlap_start = self.x0.max(line.x0);
        let overlap_end = self.x1.min(line.x1);
        let overlap = overlap_end - overlap_start;

        // Require significant horizontal overlap (at least 50% of narrower element)
        let min_width = self.width().min(line.width());
        overlap >= min_width * 0.5
    }

    /// Add a line to this block.
    pub fn add_line(&mut self, line: Line) {
        self.x0 = self.x0.min(line.x0);
        self.y0 = self.y0.min(line.y0);
        self.x1 = self.x1.max(line.x1);
        self.y1 = self.y1.max(line.y1);
        self.lines.push(line);
    }

    /// Sort lines top-to-bottom (decreasing y in PDF coordinates).
    pub fn sort_lines(&mut self) {
        // In PDF, larger y = higher on page, so sort descending by y1
        self.lines.sort_by(|a, b| b.y1.partial_cmp(&a.y1).unwrap());
    }

    /// Width of the block in points.
    pub fn width(&self) -> f32 {
        self.x1 - self.x0
    }

    /// Height of the block in points.
    pub fn height(&self) -> f32 {
        self.y1 - self.y0
    }

    /// Get the full text of this block.
    pub fn text(&self) -> String {
        self.lines
            .iter()
            .map(|l| l.text())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_append() {
        let mut span = Span::new(0);
        let ch1 = RawChar {
            char: 'H',
            x0: 10.0,
            y0: 100.0,
            x1: 18.0,
            y1: 112.0,
            font_size: 12.0,
            font_name: Some("Arial".to_string()),
            page_num: 0,
        };
        let ch2 = RawChar {
            char: 'i',
            x0: 18.0,
            y0: 100.0,
            x1: 22.0,
            y1: 112.0,
            font_size: 12.0,
            font_name: Some("Arial".to_string()),
            page_num: 0,
        };

        assert!(span.can_append(&ch1));
        span.append(&ch1);
        assert!(span.can_append(&ch2));
        span.append(&ch2);

        assert_eq!(span.text, "Hi");
        assert_eq!(span.x0, 10.0);
        assert_eq!(span.x1, 22.0);
    }

    #[test]
    fn test_span_style_detection() {
        let bold_span = Span {
            text: "Bold".to_string(),
            x0: 0.0,
            y0: 0.0,
            x1: 50.0,
            y1: 12.0,
            font_size: 12.0,
            font_name: Some("Arial-Bold".to_string()),
            page_num: 0,
        };

        let italic_span = Span {
            text: "Italic".to_string(),
            x0: 0.0,
            y0: 0.0,
            x1: 50.0,
            y1: 12.0,
            font_size: 12.0,
            font_name: Some("Arial-Italic".to_string()),
            page_num: 0,
        };

        let mono_span = Span {
            text: "code".to_string(),
            x0: 0.0,
            y0: 0.0,
            x1: 50.0,
            y1: 12.0,
            font_size: 12.0,
            font_name: Some("Courier".to_string()),
            page_num: 0,
        };

        assert!(bold_span.is_bold());
        assert!(!bold_span.is_italic());

        assert!(italic_span.is_italic());
        assert!(!italic_span.is_bold());

        assert!(mono_span.is_monospace());
    }

    #[test]
    fn test_line_from_spans() {
        let span1 = Span {
            text: "Hello".to_string(),
            x0: 10.0,
            y0: 100.0,
            x1: 50.0,
            y1: 112.0,
            font_size: 12.0,
            font_name: Some("Arial".to_string()),
            page_num: 0,
        };

        let span2 = Span {
            text: "World".to_string(),
            x0: 55.0,
            y0: 100.0,
            x1: 95.0,
            y1: 112.0,
            font_size: 12.0,
            font_name: Some("Arial".to_string()),
            page_num: 0,
        };

        let mut line = Line::from_span(span1);
        assert!(line.can_add_span(&span2, 3.0));
        line.add_span(span2);
        line.sort_spans();

        assert_eq!(line.text(), "Hello World");
        assert_eq!(line.spans.len(), 2);
    }

    #[test]
    fn test_block_from_lines() {
        let line1 = Line {
            spans: vec![Span {
                text: "First line".to_string(),
                x0: 10.0,
                y0: 100.0,
                x1: 100.0,
                y1: 112.0,
                font_size: 12.0,
                font_name: None,
                page_num: 0,
            }],
            x0: 10.0,
            y0: 100.0,
            x1: 100.0,
            y1: 112.0,
            page_num: 0,
        };

        let line2 = Line {
            spans: vec![Span {
                text: "Second line".to_string(),
                x0: 10.0,
                y0: 85.0,
                x1: 110.0,
                y1: 97.0,
                font_size: 12.0,
                font_name: None,
                page_num: 0,
            }],
            x0: 10.0,
            y0: 85.0,
            x1: 110.0,
            y1: 97.0,
            page_num: 0,
        };

        let mut block = Block::from_line(line1);
        // Line gap is 100 - 97 = 3 points
        assert!(block.can_add_line(&line2, 5.0));
        block.add_line(line2);
        block.sort_lines();

        assert_eq!(block.lines.len(), 2);
        assert!(block.text().contains("First line"));
        assert!(block.text().contains("Second line"));
    }
}
