//! Markdown rendering from structured text blocks.
//!
//! This module converts structured `Block`s into Markdown format,
//! handling:
//! - Headers with proper # prefixes
//! - Bold/italic text from font styles
//! - Code blocks with monospace detection
//! - Lists (bullet and numbered)
//! - Paragraph separation

use super::pymupdf_structs::{Block, BlockType, Line};

/// Markdown renderer configuration.
#[derive(Debug, Clone)]
pub struct MarkdownConfig {
    /// Insert blank lines between blocks
    pub block_spacing: bool,
    /// Preserve bold/italic styling
    pub preserve_styles: bool,
    /// Render code blocks with fences
    pub fenced_code: bool,
    /// Maximum heading level (1-6)
    pub max_heading_level: u8,
}

impl Default for MarkdownConfig {
    fn default() -> Self {
        Self {
            block_spacing: true,
            preserve_styles: true,
            fenced_code: true,
            max_heading_level: 6,
        }
    }
}

/// Renders structured blocks to Markdown text.
pub struct MarkdownRenderer {
    config: MarkdownConfig,
}

impl MarkdownRenderer {
    /// Create a new renderer with default config.
    pub fn new() -> Self {
        Self {
            config: MarkdownConfig::default(),
        }
    }

    /// Create a renderer with custom config.
    pub fn with_config(config: MarkdownConfig) -> Self {
        Self { config }
    }

    /// Render blocks to Markdown string.
    pub fn render(&self, blocks: &[Block]) -> String {
        let mut output = String::new();
        let mut last_page = 0;

        for (i, block) in blocks.iter().enumerate() {
            // Add page separator if page changed
            if block.page_num != last_page && i > 0 {
                output.push_str("\n---\n\n");
                last_page = block.page_num;
            }

            // Render this block
            let block_text = self.render_block(block);
            output.push_str(&block_text);

            // Add spacing between blocks
            if self.config.block_spacing && i < blocks.len() - 1 {
                output.push_str("\n\n");
            }
        }

        output
    }

    fn render_block(&self, block: &Block) -> String {
        match block.block_type {
            BlockType::Header(level) => self.render_header(block, level),
            BlockType::Code => self.render_code(block),
            BlockType::ListItem => self.render_list_item(block),
            BlockType::Table => self.render_table(block),
            BlockType::Paragraph => self.render_paragraph(block),
        }
    }

    fn render_header(&self, block: &Block, level: u8) -> String {
        let level = level.min(self.config.max_heading_level);
        let prefix = "#".repeat(level as usize);
        // OODA-12: Join header lines with space, not newline
        // WHY: Headers like paper titles may wrap across lines in PDF but should
        // render as a single line in Markdown: "### **Title Part 1** **Part 2**"
        // OODA-12: pymupdf4llm wraps header content in bold: ## **1. Introduction**
        let text = block
            .lines
            .iter()
            .map(|l| self.render_line_plain(l))
            .collect::<Vec<_>>()
            .join(" ");
        let trimmed = text.trim();
        // Wrap header text in bold to match pymupdf4llm gold format
        format!("{} **{}**", prefix, trimmed)
    }

    fn render_code(&self, block: &Block) -> String {
        if self.config.fenced_code {
            let code = block
                .lines
                .iter()
                .map(|l| self.render_line_plain(l))
                .collect::<Vec<_>>()
                .join("\n");
            format!("```\n{}\n```", code)
        } else {
            // Indent with 4 spaces
            block
                .lines
                .iter()
                .map(|l| format!("    {}", self.render_line_plain(l)))
                .collect::<Vec<_>>()
                .join("\n")
        }
    }

    fn render_list_item(&self, block: &Block) -> String {
        // First line should have the bullet/number
        // Subsequent lines are continuation
        let mut lines_iter = block.lines.iter();

        if let Some(first_line) = lines_iter.next() {
            let first_text = self.render_line_styled(first_line);

            // Check if we need to normalize the bullet
            let normalized = normalize_bullet(&first_text);

            let continuation: String = lines_iter
                .map(|l| format!("  {}", self.render_line_styled(l)))
                .collect::<Vec<_>>()
                .join("\n");

            if continuation.is_empty() {
                normalized
            } else {
                format!("{}\n{}", normalized, continuation)
            }
        } else {
            String::new()
        }
    }

    fn render_table(&self, block: &Block) -> String {
        // For now, just render as paragraph
        // TODO: Implement proper table detection and rendering
        self.render_paragraph(block)
    }

    fn render_paragraph(&self, block: &Block) -> String {
        self.render_lines_inline(&block.lines)
    }

    /// Render multiple lines joined by newlines (preserving PDF line breaks).
    /// WHY: pymupdf4llm preserves line breaks within paragraphs for:
    /// 1. Visual structure matching the original PDF layout
    /// 2. Proper hyphenation handling (words broken across lines)
    /// 3. Better ROUGE-L alignment when comparing with gold standards
    fn render_lines_inline(&self, lines: &[Line]) -> String {
        lines
            .iter()
            .map(|l| self.render_line_styled(l))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// OODA-10: Render multiple lines as plain text (no bold/italic).
    /// Used for headers where the ## markers already provide emphasis.
    #[allow(dead_code)] // Reserved for future header rendering refactoring
    fn render_lines_plain(&self, lines: &[Line]) -> String {
        lines
            .iter()
            .map(|l| self.render_line_plain(l))
            .collect::<Vec<_>>()
            .join(" ") // Join header lines with space, not newline
    }

    /// Render a line with style markers (bold, italic).
    ///
    /// This method applies style markers while respecting actual spacing
    /// between spans to avoid fragmenting words or adding extra spaces.
    /// It also merges consecutive spans with the same style to avoid
    /// creating invalid markdown like `*word**another*`.
    fn render_line_styled(&self, line: &Line) -> String {
        if !self.config.preserve_styles {
            return self.render_line_plain(line);
        }

        if line.spans.is_empty() {
            return String::new();
        }

        if line.spans.len() == 1 {
            let span = &line.spans[0];
            let text = &span.text;
            if text.trim().is_empty() {
                return text.clone();
            }
            return self.style_text(text, span);
        }

        // Group consecutive spans with same style, including spaces within groups
        let mut groups: Vec<(String, StyleType)> = Vec::new();
        let mut current_text = String::new();
        let mut current_style = get_style_type(&line.spans[0]);

        for (i, span) in line.spans.iter().enumerate() {
            // Determine if we need a space before this span
            let needs_space = if i > 0 {
                let prev = &line.spans[i - 1];
                let gap = span.x0 - prev.x1;
                let avg_size = (prev.font_size + span.font_size) / 2.0;
                let space_threshold = avg_size * 0.15;

                let starts_with_hyphen = span.text.starts_with('-')
                    || span.text.starts_with('–')
                    || span.text.starts_with('—');
                let ends_with_hyphen = prev.text.ends_with('-')
                    || prev.text.ends_with('–')
                    || prev.text.ends_with('—');

                gap > space_threshold && !starts_with_hyphen && !ends_with_hyphen
            } else {
                false
            };

            let span_style = get_style_type(span);

            // Only flush when style actually changes
            if span_style != current_style {
                if !current_text.is_empty() {
                    groups.push((current_text.clone(), current_style));
                    current_text.clear();
                }
                // Add space to the NEW group if needed
                if needs_space {
                    current_text.push(' ');
                }
                current_style = span_style;
            } else if needs_space {
                // Same style, just add space within the group
                current_text.push(' ');
            }

            current_text.push_str(&span.text);
        }

        // Don't forget the last group
        if !current_text.is_empty() {
            groups.push((current_text, current_style));
        }

        // Render each group with appropriate style
        groups
            .into_iter()
            .map(|(text, style)| apply_style(&text, style))
            .collect::<String>()
    }

    /// Apply style markers (bold/italic) to text based on span properties.
    fn style_text(&self, text: &str, span: &super::pymupdf_structs::Span) -> String {
        apply_style(text, get_style_type(span))
    }

    /// Render a line without style markers (plain text).
    fn render_line_plain(&self, line: &Line) -> String {
        line.spans
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Style types for span grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StyleType {
    Plain,
    Bold,
    Italic,
    BoldItalic,
    Code,
}

/// Get the style type of a span.
fn get_style_type(span: &super::pymupdf_structs::Span) -> StyleType {
    if span.is_bold() && span.is_italic() {
        StyleType::BoldItalic
    } else if span.is_bold() {
        StyleType::Bold
    } else if span.is_italic() {
        StyleType::Italic
    } else if span.is_monospace() {
        StyleType::Code
    } else {
        StyleType::Plain
    }
}

/// Apply style markers to text.
/// OODA-12: Preserve leading/trailing spaces outside style markers
/// WHY: pymupdf4llm produces `_italic_ **bold**` not `*italic***bold**`
/// OODA-12: Use underscores for italic to match gold standard format
fn apply_style(text: &str, style: StyleType) -> String {
    if text.trim().is_empty() {
        return text.to_string();
    }

    // Preserve leading and trailing whitespace
    let leading_space = text.len() - text.trim_start().len();
    let trailing_space = text.len() - text.trim_end().len();
    let trimmed = text.trim();

    let styled = match style {
        StyleType::BoldItalic => format!("**_{}_**", trimmed),
        StyleType::Bold => format!("**{}**", trimmed),
        StyleType::Italic => format!("_{}_", trimmed), // Use underscores for italic
        StyleType::Code if !trimmed.starts_with('`') => format!("`{}`", trimmed),
        _ => trimmed.to_string(),
    };

    // Re-add whitespace
    let leading: String = " ".repeat(leading_space.min(1)); // Cap at 1 space
    let trailing: String = " ".repeat(trailing_space.min(1));
    format!("{}{}{}", leading, styled, trailing)
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Normalize bullet characters to standard Markdown bullets.
fn normalize_bullet(text: &str) -> String {
    let trimmed = text.trim_start();

    // Common bullet characters to normalize
    const BULLETS: &[char] = &['•', '●', '○', '◦', '▪', '▫', '–', '—'];

    for &bullet in BULLETS {
        if let Some(rest) = trimmed.strip_prefix(bullet) {
            return format!("- {}", rest.trim_start());
        }
    }

    // Already standard bullet
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        return text.to_string();
    }

    // Numbered list - keep as is
    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::Span;

    fn make_span(text: &str, font_name: &str, font_size: f32) -> Span {
        Span {
            text: text.to_string(),
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: font_size,
            font_size,
            font_name: Some(font_name.to_string()),
            page_num: 0,
            font_is_bold: None,
            font_is_italic: None,
            font_is_monospace: None,
        }
    }

    fn make_line(spans: Vec<Span>) -> Line {
        let (x0, y0, x1, y1) = spans.iter().fold(
            (f32::MAX, f32::MAX, f32::MIN, f32::MIN),
            |(x0, y0, x1, y1), s| (x0.min(s.x0), y0.min(s.y0), x1.max(s.x1), y1.max(s.y1)),
        );
        Line {
            spans,
            x0,
            y0,
            x1,
            y1,
            page_num: 0,
        }
    }

    #[test]
    fn test_render_header() {
        let renderer = MarkdownRenderer::new();

        let block = Block {
            lines: vec![make_line(vec![make_span(
                "Introduction",
                "Arial-Bold",
                24.0,
            )])],
            x0: 0.0,
            y0: 0.0,
            x1: 200.0,
            y1: 24.0,
            page_num: 0,
            block_type: BlockType::Header(1),
        };

        let md = renderer.render(&[block]);
        assert!(md.contains("# "));
        assert!(md.contains("Introduction"));
    }

    #[test]
    fn test_render_bold_italic() {
        let renderer = MarkdownRenderer::new();

        let block = Block {
            lines: vec![make_line(vec![
                make_span("Normal", "Arial", 12.0),
                make_span("bold", "Arial-Bold", 12.0),
                make_span("italic", "Arial-Italic", 12.0),
            ])],
            x0: 0.0,
            y0: 0.0,
            x1: 200.0,
            y1: 12.0,
            page_num: 0,
            block_type: BlockType::Paragraph,
        };

        let md = renderer.render(&[block]);
        assert!(md.contains("**bold**"), "Missing bold: {}", md);
        // Accept either *italic* or _italic_ - both are valid markdown
        assert!(
            md.contains("*italic*") || md.contains("_italic_"),
            "Missing italic: {}",
            md
        );
    }

    #[test]
    fn test_render_code_block() {
        let renderer = MarkdownRenderer::new();

        let block = Block {
            lines: vec![
                make_line(vec![make_span("fn main() {", "Courier", 12.0)]),
                make_line(vec![make_span("    println!(\"Hello\");", "Courier", 12.0)]),
                make_line(vec![make_span("}", "Courier", 12.0)]),
            ],
            x0: 0.0,
            y0: 0.0,
            x1: 200.0,
            y1: 36.0,
            page_num: 0,
            block_type: BlockType::Code,
        };

        let md = renderer.render(&[block]);
        assert!(md.contains("```"), "Missing code fence: {}", md);
        assert!(md.contains("fn main()"), "Missing code content: {}", md);
    }

    #[test]
    fn test_normalize_bullet() {
        assert_eq!(normalize_bullet("• Item one"), "- Item one");
        assert_eq!(normalize_bullet("● Item two"), "- Item two");
        assert_eq!(normalize_bullet("- Already normal"), "- Already normal");
        assert_eq!(normalize_bullet("1. Numbered"), "1. Numbered");
    }

    #[test]
    fn test_render_list() {
        let renderer = MarkdownRenderer::new();

        let block = Block {
            lines: vec![make_line(vec![make_span("• First item", "Arial", 12.0)])],
            x0: 0.0,
            y0: 0.0,
            x1: 200.0,
            y1: 12.0,
            page_num: 0,
            block_type: BlockType::ListItem,
        };

        let md = renderer.render(&[block]);
        assert!(
            md.contains("- First item"),
            "Missing normalized bullet: {}",
            md
        );
    }
}
