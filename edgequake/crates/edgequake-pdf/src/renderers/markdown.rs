//! Markdown renderer for document output.

use crate::schema::{Block, BlockType, Document, Page, TextSpan};
use crate::Result;

use super::Renderer;

/// Markdown rendering style options.
#[derive(Debug, Clone)]
pub struct MarkdownStyle {
    /// Include page breaks (as horizontal rules)
    pub page_breaks: bool,
    /// Include page numbers as comments
    pub page_numbers: bool,
    /// Maximum heading level (1-6)
    pub max_heading_level: u8,
    /// Use ATX-style headers (# vs underline)
    pub atx_headers: bool,
    /// Indent code blocks with fences
    pub fenced_code: bool,
    /// Language hint for code blocks
    pub default_code_language: Option<String>,
    /// Include block IDs as HTML comments
    pub include_block_ids: bool,
    /// Normalize line breaks
    pub normalize_line_breaks: bool,
}

impl Default for MarkdownStyle {
    fn default() -> Self {
        Self {
            page_breaks: true,
            page_numbers: true,
            max_heading_level: 6,
            atx_headers: true,
            fenced_code: true,
            default_code_language: None,
            include_block_ids: false,
            normalize_line_breaks: true,
        }
    }
}

impl MarkdownStyle {
    /// Create a minimal style (just content, no extras).
    pub fn minimal() -> Self {
        Self {
            page_breaks: false,
            page_numbers: false,
            include_block_ids: false,
            ..Default::default()
        }
    }

    /// Create a verbose style (with all annotations).
    pub fn verbose() -> Self {
        Self {
            page_breaks: true,
            page_numbers: true,
            include_block_ids: true,
            ..Default::default()
        }
    }
}

/// Markdown renderer.
pub struct MarkdownRenderer {
    style: MarkdownStyle,
}

impl MarkdownRenderer {
    /// Create a new Markdown renderer.
    pub fn new() -> Self {
        Self {
            style: MarkdownStyle::default(),
        }
    }

    /// Create with custom style.
    pub fn with_style(style: MarkdownStyle) -> Self {
        Self { style }
    }

    /// Render a page to Markdown.
    fn render_page(&self, page: &Page, output: &mut String) {
        if self.style.page_numbers {
            output.push_str(&format!("## Page {}\n\n", page.number));
        }

        for block in &page.blocks {
            self.render_block(block, output);
        }
    }

    /// Render a block to Markdown.
    fn render_block(&self, block: &Block, output: &mut String) {
        if self.style.include_block_ids {
            output.push_str(&format!("<!-- {} -->\n", block.id));
        }

        match block.block_type {
            BlockType::SectionHeader => {
                self.render_header(block, output);
            }
            BlockType::Text | BlockType::Paragraph | BlockType::TextInlineMath => {
                self.render_text(block, output);
            }
            BlockType::ListItem => {
                self.render_list_item(block, output);
            }
            BlockType::Code => {
                self.render_code(block, output);
            }
            BlockType::Equation => {
                self.render_equation(block, output);
            }
            BlockType::Table => {
                self.render_table(block, output);
            }
            BlockType::Figure | BlockType::Picture => {
                self.render_figure(block, output);
            }
            BlockType::Caption => {
                self.render_caption(block, output);
            }
            BlockType::Footnote => {
                self.render_footnote(block, output);
            }
            BlockType::PageHeader | BlockType::PageFooter => {
                // Skip page headers/footers by default
                if self.style.include_block_ids {
                    output.push_str(&format!("<!-- {} skipped -->\n", block.block_type));
                }
            }
            _ => {
                // Default: render as text
                self.render_text(block, output);
            }
        }
    }

    /// Render a header.
    fn render_header(&self, block: &Block, output: &mut String) {
        let level = block.level.unwrap_or(2).min(self.style.max_heading_level);
        let text = if !block.spans.is_empty() {
            self.render_spans(&block.spans)
        } else {
            self.clean_text(&block.text)
        };

        if self.style.atx_headers {
            let prefix = "#".repeat(level as usize);
            output.push_str(&format!("{} {}\n\n", prefix, text));
        } else {
            output.push_str(&text);
            output.push('\n');
            let underline = if level == 1 { '=' } else { '-' };
            output.push_str(&underline.to_string().repeat(text.len().min(40)));
            output.push_str("\n\n");
        }
    }

    /// Render text paragraph.
    fn render_text(&self, block: &Block, output: &mut String) {
        let text = if !block.spans.is_empty() {
            self.render_spans(&block.spans)
        } else {
            self.clean_text(&block.text)
        };

        if !text.is_empty() {
            output.push_str(&text);
            output.push_str("\n\n");
        }
    }

    /// Render a list item.
    fn render_list_item(&self, block: &Block, output: &mut String) {
        let text = if !block.spans.is_empty() {
            self.render_spans(&block.spans)
        } else {
            self.clean_text(&block.text)
        };

        // Handle indentation for nested lists
        if let Some(indent) = block.metadata.get("indent").and_then(|v| v.as_f64()) {
            // Assume 72.0 is base margin, every 10.0 points is one level of indentation
            // (Using 10.0 instead of 20.0 to be more sensitive to small indents)
            let level = ((indent - 72.0).max(0.0) / 10.0).floor() as usize;
            for _ in 0..level {
                output.push_str("  ");
            }
        }

        // Check if already has bullet/number prefix
        let trimmed = text.trim();
        let needs_prefix = !trimmed.starts_with("- ")
            && !trimmed.starts_with("* ")
            && !trimmed.starts_with("• ")
            && !trimmed
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false);

        if needs_prefix {
            output.push_str("- ");
        }
        output.push_str(&text);
        output.push('\n');
    }

    /// Render structured spans with formatting.
    fn render_spans(&self, spans: &[TextSpan]) -> String {
        let mut result = String::new();
        for span in spans {
            let content = &span.text;
            if content.is_empty() {
                continue;
            }

            let is_bold = span.style.weight.map(|w| w >= 600).unwrap_or(false);
            let is_italic = span.style.italic;
            let is_code = span.style.looks_like_code();

            if is_code {
                result.push_str(&format!("`{}`", content));
            } else {
                let trimmed = content.trim();
                if trimmed.is_empty() {
                    result.push_str(content);
                    continue;
                }

                let leading_space = content.starts_with(' ');
                let trailing_space = content.ends_with(' ');

                let mut styled = trimmed.to_string();
                if is_bold && is_italic {
                    styled = format!("***{}***", styled);
                } else if is_bold {
                    styled = format!("**{}**", styled);
                } else if is_italic {
                    styled = format!("*{}*", styled);
                }

                if leading_space {
                    result.push(' ');
                }
                result.push_str(&styled);
                if trailing_space {
                    result.push(' ');
                }
            }
        }
        result
    }

    /// Render a code block.
    fn render_code(&self, block: &Block, output: &mut String) {
        let text = &block.text;

        if self.style.fenced_code {
            let lang = self.style.default_code_language.as_deref().unwrap_or("");
            output.push_str(&format!("```{}\n{}\n```\n\n", lang, text));
        } else {
            // Indent with 4 spaces
            for line in text.lines() {
                output.push_str("    ");
                output.push_str(line);
                output.push('\n');
            }
            output.push('\n');
        }
    }

    /// Render an equation.
    fn render_equation(&self, block: &Block, output: &mut String) {
        let text = self.clean_text(&block.text);
        output.push_str(&format!("$$\n{}\n$$\n\n", text));
    }

    /// Render a table.
    fn render_table(&self, block: &Block, output: &mut String) {
        // If we have children (table cells), render as proper table
        if !block.children.is_empty() {
            self.render_table_from_children(block, output);
        } else {
            // Plain text table
            let text = self.clean_text(&block.text);
            output.push_str(&text);
            output.push_str("\n\n");
        }
    }

    /// Render table from child cells.
    fn render_table_from_children(&self, block: &Block, output: &mut String) {
        // Group children by row based on Y position
        let mut rows: Vec<Vec<&Block>> = Vec::new();
        let mut current_row: Vec<&Block> = Vec::new();
        let mut current_y: Option<f32> = None;

        for child in &block.children {
            let y = child.bbox.y1;
            if let Some(prev_y) = current_y {
                if (y - prev_y).abs() > 5.0 {
                    if !current_row.is_empty() {
                        // Sort row by X position
                        current_row.sort_by(|a, b| a.bbox.x1.partial_cmp(&b.bbox.x1).unwrap());
                        rows.push(current_row);
                    }
                    current_row = Vec::new();
                }
            }
            current_row.push(child);
            current_y = Some(y);
        }

        if !current_row.is_empty() {
            current_row.sort_by(|a, b| a.bbox.x1.partial_cmp(&b.bbox.x1).unwrap());
            rows.push(current_row);
        }

        // Render as Markdown table
        if rows.is_empty() {
            return;
        }

        // Header row
        let header = &rows[0];
        output.push('|');
        for cell in header {
            output.push_str(&format!(" {} |", self.clean_text(&cell.text)));
        }
        output.push('\n');

        // Separator
        output.push('|');
        for _ in header {
            output.push_str(" --- |");
        }
        output.push('\n');

        // Data rows
        for row in rows.iter().skip(1) {
            output.push('|');
            for cell in row {
                output.push_str(&format!(" {} |", self.clean_text(&cell.text)));
            }
            output.push('\n');
        }

        output.push('\n');
    }

    /// Render a figure/image.
    fn render_figure(&self, block: &Block, output: &mut String) {
        let alt_text = if block.text.is_empty() {
            "Figure"
        } else {
            &block.text
        };

        // If we have an image path in metadata
        if let Some(path) = block.metadata.get("image_path") {
            if let Some(path_str) = path.as_str() {
                output.push_str(&format!("![{}]({})\n\n", alt_text, path_str));
                return;
            }
        }

        // Placeholder
        output.push_str(&format!("![{}]()\n\n", alt_text));
    }

    /// Render a caption.
    fn render_caption(&self, block: &Block, output: &mut String) {
        let text = self.clean_text(&block.text);
        output.push_str(&format!("*{}*\n\n", text));
    }

    /// Render a footnote.
    fn render_footnote(&self, block: &Block, output: &mut String) {
        let text = self.clean_text(&block.text);

        // Try to extract footnote number
        if let Some(num) = block.metadata.get("footnote_num") {
            if let Some(n) = num.as_u64() {
                output.push_str(&format!("[^{}]: {}\n", n, text));
                return;
            }
        }

        // Fallback: just italic text
        output.push_str(&format!("*{}*\n\n", text));
    }

    /// Clean text for Markdown output.
    fn clean_text(&self, text: &str) -> String {
        let mut result = text.to_string();

        if self.style.normalize_line_breaks {
            // Collapse multiple newlines
            while result.contains("\n\n\n") {
                result = result.replace("\n\n\n", "\n\n");
            }

            // Normalize line endings
            result = result.replace("\r\n", "\n").replace('\r', "\n");
        }

        result.trim().to_string()
    }
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer for MarkdownRenderer {
    fn render(&self, document: &Document) -> Result<String> {
        let mut output = String::new();

        // Add document title if available
        if let Some(title) = &document.metadata.title {
            output.push_str(&format!("# {}\n\n", title));
        }

        // Render each page
        for (i, page) in document.pages.iter().enumerate() {
            if i > 0 && self.style.page_breaks {
                output.push_str("\n---\n\n");
            }

            self.render_page(page, &mut output);
        }

        // Trim trailing whitespace
        Ok(output.trim().to_string())
    }

    fn extension(&self) -> &str {
        "md"
    }

    fn mime_type(&self) -> &str {
        "text/markdown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::BoundingBox;

    fn create_test_document() -> Document {
        let mut doc = Document::new();
        doc.metadata.title = Some("Test Document".to_string());

        let mut page = Page::new(1, 612.0, 792.0);

        page.add_block(Block::header(
            "Introduction",
            1,
            BoundingBox::new(72.0, 72.0, 540.0, 100.0),
        ));

        page.add_block(Block::text(
            "This is a paragraph of text.",
            BoundingBox::new(72.0, 120.0, 540.0, 150.0),
        ));

        page.add_block(Block::code(
            "fn main() {\n    println!(\"Hello\");\n}",
            BoundingBox::new(72.0, 170.0, 540.0, 220.0),
        ));

        doc.add_page(page);
        doc
    }

    #[test]
    fn test_markdown_rendering() {
        let renderer = MarkdownRenderer::new();
        let doc = create_test_document();
        let result = renderer.render(&doc).unwrap();

        assert!(result.contains("# Test Document"));
        assert!(result.contains("# Introduction"));
        assert!(result.contains("This is a paragraph"));
        assert!(result.contains("```"));
        assert!(result.contains("fn main()"));
    }

    #[test]
    fn test_markdown_style_minimal() {
        let style = MarkdownStyle::minimal();
        let renderer = MarkdownRenderer::with_style(style);
        let doc = create_test_document();
        let result = renderer.render(&doc).unwrap();

        assert!(!result.contains("<!--"));
        assert!(!result.contains("---"));
    }

    #[test]
    fn test_markdown_style_verbose() {
        let style = MarkdownStyle::verbose();
        let renderer = MarkdownRenderer::with_style(style);
        let doc = create_test_document();
        let result = renderer.render(&doc).unwrap();

        assert!(result.contains("## Page 1"));
        assert!(result.contains("<!-- block_"));
    }

    #[test]
    fn test_list_item_rendering() {
        let renderer = MarkdownRenderer::new();

        let mut doc = Document::new();
        let mut page = Page::new(1, 612.0, 792.0);

        page.add_block(Block::list_item(
            "First item",
            BoundingBox::new(72.0, 100.0, 540.0, 120.0),
        ));
        page.add_block(Block::list_item(
            "- Second item",
            BoundingBox::new(72.0, 130.0, 540.0, 150.0),
        ));

        doc.add_page(page);
        let result = renderer.render(&doc).unwrap();

        assert!(result.contains("- First item"));
        assert!(result.contains("- Second item"));
    }

    #[test]
    fn test_clean_text() {
        let renderer = MarkdownRenderer::new();

        let cleaned = renderer.clean_text("Hello\n\n\n\nWorld");
        assert_eq!(cleaned, "Hello\n\nWorld");

        let trimmed = renderer.clean_text("  spaced  ");
        assert_eq!(trimmed, "spaced");
    }
}
