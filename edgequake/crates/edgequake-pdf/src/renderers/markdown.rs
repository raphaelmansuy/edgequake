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

        for (i, block) in page.blocks.iter().enumerate() {
            self.render_block(block, output);

            // Add extra newline after list items if the next block is not a list item
            if block.block_type == BlockType::ListItem {
                let next_is_list = page
                    .blocks
                    .get(i + 1)
                    .map(|b| b.block_type == BlockType::ListItem)
                    .unwrap_or(false);
                if !next_is_list {
                    output.push('\n');
                }
            }
        }
    }

    /// Render a block to Markdown.
    fn render_block(&self, block: &Block, output: &mut String) {
        if block.block_type == BlockType::Table {
            tracing::info!(
                "render_block: Found Table block with text_len={}, text='{}'",
                block.text.len(),
                block.text
            );
        }

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
            self.render_spans_styled(&block.spans, true, false)
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
        // WHY: After HyphenContinuation and BlockMerge, spans may be stale.
        // We check if spans text matches block.text. If they don't match, use block.text.
        // This prevents rendering stale span text from before hyphenation fixes.
        let span_text: String = block.spans.iter().map(|s| s.text.as_str()).collect();
        let spans_valid = if block.spans.is_empty() {
            false
        } else {
            // Spans are valid if they contain the same content as block.text
            let normalized_span = span_text.split_whitespace().collect::<Vec<_>>().join(" ");
            let normalized_text = block.text.split_whitespace().collect::<Vec<_>>().join(" ");
            normalized_span == normalized_text || normalized_text.starts_with(&normalized_span)
        };

        let text = if spans_valid {
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
        // Handle indentation for nested lists
        let level = if let Some(lvl) = block.metadata.get("level").and_then(|v| v.as_i64()) {
            // WHY as_i64: JSON stores i32 from ListDetectionProcessor as Number
            // as_u64 would fail on negative values
            tracing::debug!("  Rendering list item with level={}", lvl);
            lvl.max(0) as usize
        } else if let Some(indent) = block.metadata.get("indent").and_then(|v| v.as_f64()) {
            // Fallback to old logic if level not present
            let lvl = ((indent - 72.0).max(0.0) / 20.0).floor() as usize;
            tracing::debug!(
                "  Rendering list item with indent={:.1} -> level={}",
                indent,
                lvl
            );
            lvl
        } else {
            tracing::debug!("  Rendering list item with no level/indent metadata");
            0
        };

        // WHY subtract 1: level=1 is the base level (no indent), level=2 gets one indent
        let adjusted_level = level.saturating_sub(1);
        let len_before_indent = output.len();
        for _i in 0..adjusted_level {
            output.push_str("  "); // Two ASCII spaces (0x20 0x20)
        }
        let len_after_indent = output.len();
        if adjusted_level > 0 {
            // Show the actual bytes we added
            let added_bytes: Vec<u8> = output[len_before_indent..len_after_indent]
                .bytes()
                .collect();
            tracing::debug!(
                "  Indentation: before={} after={} bytes={:02x?}",
                len_before_indent,
                len_after_indent,
                added_bytes
            );
        }

        // Use raw text for pattern matching to avoid bold markers interfering
        // WHY: render_spans may wrap bullet chars in **bold** from PDF font info
        let raw_text = block.text.trim();

        // Trace the start of the list item in output buffer
        if raw_text.contains("Child") {
            tracing::debug!(
                "  At child item: output len before prefix = {}",
                output.len()
            );
        }

        // Check for various bullet/number patterns in RAW text
        let has_dash =
            raw_text.starts_with("- ") || raw_text.starts_with("– ") || raw_text.starts_with("— ");
        let has_asterisk = raw_text.starts_with("* ");
        let has_bullet =
            raw_text.starts_with("• ") || raw_text.starts_with("◦ ") || raw_text.starts_with("▪ ");
        let has_number = raw_text
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false);

        // Get content after the bullet/prefix for rendering with spans
        let content_start = if has_bullet || has_dash || has_asterisk {
            // Find first space after bullet and skip to content
            raw_text.find(' ').map(|i| i + 1).unwrap_or(0)
        } else if has_number {
            // OODA-14 FIX: For numbered lists like "1. " or "1.Text" (no space)
            // Find the ". " or ") " or just "." or ")" and skip
            // WHY: PDF text extraction may not preserve spaces after list markers
            raw_text
                .find(". ")
                .map(|i| i + 2)
                .or_else(|| raw_text.find(") ").map(|i| i + 2))
                .or_else(|| {
                    // No space after marker - find the first "." or ")" followed by a letter
                    for (i, c) in raw_text.chars().enumerate() {
                        if (c == '.' || c == ')') && i > 0 {
                            // Check if next char is a letter (no space)
                            if let Some(next) = raw_text.chars().nth(i + 1) {
                                if next.is_alphabetic() {
                                    return Some(i + 1); // Skip past the "." or ")"
                                }
                            }
                        }
                    }
                    None
                })
                .unwrap_or(0)
        } else {
            0
        };

        // Render content with formatting (from spans if available)
        let content = if content_start > 0 && !block.spans.is_empty() {
            // Get the text content after the prefix
            let after_prefix = &raw_text[content_start..];
            // For simplicity, use clean text since spans may not align well with prefix removal
            self.clean_text(after_prefix)
        } else if !block.spans.is_empty() {
            self.render_spans(&block.spans)
        } else {
            self.clean_text(raw_text)
        };

        // Output the normalized list item
        if has_bullet || has_dash {
            let before = output.len();
            output.push_str("- ");
            output.push_str(&content);
            let after = output.len();
            if content.contains("Child") {
                tracing::debug!(
                    "  Output for child: added {} bytes, content='{}', raw_text='{}'",
                    after - before,
                    content,
                    raw_text
                );
            }
        } else if has_number {
            // OODA-14 FIX: Normalize numbered list output to "N. content" format
            // WHY: Markdown requires space after the period for proper list rendering
            // Extract just the number(s) from the prefix
            let number: String = raw_text.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !number.is_empty() {
                output.push_str(&number);
                output.push_str(". ");
                output.push_str(&content);
            } else {
                // Fallback: use original prefix
                let prefix: String = raw_text.chars().take(content_start).collect();
                output.push_str(&prefix);
                output.push_str(&content);
            }
        } else if has_asterisk {
            output.push_str(&content);
        } else {
            // No prefix, add dash
            output.push_str("- ");
            output.push_str(&content);
        }
        output.push('\n');
    }

    /// Render structured spans with formatting.
    fn render_spans(&self, spans: &[TextSpan]) -> String {
        self.render_spans_styled(spans, false, false)
    }

    /// Render structured spans with optional style skipping.
    fn render_spans_styled(
        &self,
        spans: &[TextSpan],
        skip_bold: bool,
        skip_italic: bool,
    ) -> String {
        let mut result = String::new();
        for span in spans {
            let content = &span.text;
            if content.is_empty() {
                continue;
            }

            let is_bold = span.style.weight.map(|w| w >= 600).unwrap_or(false) && !skip_bold;
            let is_italic = span.style.italic && !skip_italic;
            let is_code = span.style.looks_like_code();
            let is_superscript = span.style.superscript;
            let is_subscript = span.style.subscript;

            if is_code {
                // WHY preserve leading/trailing space? In "the `print()` function",
                // the space before `print()` must be outside the backticks.
                let leading_space = content.starts_with(' ');
                let trailing_space = content.ends_with(' ');
                let trimmed = content.trim();
                if leading_space {
                    result.push(' ');
                }
                result.push_str(&format!("`{}`", trimmed));
                if trailing_space {
                    result.push(' ');
                }
            } else {
                let trimmed = content.trim();
                if trimmed.is_empty() {
                    result.push_str(content);
                    continue;
                }

                let leading_space = content.starts_with(' ');
                let trailing_space = content.ends_with(' ');

                let mut styled = trimmed.to_string();

                if is_superscript {
                    styled = format!("^{}^", styled);
                } else if is_subscript {
                    styled = format!("~{}~", styled);
                }

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
        tracing::debug!(
            "render_table called: has_children={}, text_len={}",
            !block.children.is_empty(),
            block.text.len()
        );

        // If we have children (table cells), render as proper table
        if !block.children.is_empty() {
            self.render_table_from_children(block, output);
        } else {
            // WHY: Lattice-generated tables have pre-formatted markdown syntax in block.text.
            // We must NOT call clean_text() which would escape pipe characters!
            // Simply render the markdown table syntax directly.
            tracing::debug!("Rendering lattice table with {} chars", block.text.len());
            output.push_str(&block.text);
            if !block.text.ends_with('\n') {
                output.push('\n');
            }
            output.push('\n');
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
                if (y - prev_y).abs() > 10.0 {
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

        // FIRST PRINCIPLES: Escape leading pipe characters that might be misinterpreted as table syntax.
        // Lines starting with | that are NOT followed by another | (i.e., not table rows)
        // should have the | escaped to prevent markdown table rendering.
        // Examples to escape: "|Y ∩ *Y*ˆ ∗|", "| symbol tables..."
        // Examples to preserve: "| col1 | col2 |" (actual table rows)
        result = self.escape_non_table_pipes(&result);

        result.trim().to_string()
    }

    /// Escape leading pipe characters that are not part of markdown tables.
    /// A markdown table row must have: |col1|col2| or | col1 | col2 |
    /// A single leading | followed by text (no second |) is NOT a table.
    fn escape_non_table_pipes(&self, text: &str) -> String {
        let mut result = String::with_capacity(text.len());

        for line in text.lines() {
            let trimmed = line.trim();

            // Check if line starts with | but is NOT a valid table row
            if trimmed.starts_with('|') {
                // A valid table row should have at least 2 pipe characters
                // (e.g., "| cell |" has pipes at start and end)
                let pipe_count = trimmed.chars().filter(|&c| c == '|').count();

                // Also check for separator row pattern: |---|---|
                let is_separator = trimmed
                    .chars()
                    .all(|c| c == '|' || c == '-' || c == ':' || c.is_whitespace());

                // If only 1 pipe, or the line doesn't have proper table structure, escape it
                if pipe_count < 2 && !is_separator {
                    // Escape the leading pipe
                    result.push_str(&line.replacen("|", r"\|", 1));
                    result.push('\n');
                    continue;
                }
            }

            result.push_str(line);
            result.push('\n');
        }

        // Remove trailing newline if original didn't have it
        if !text.ends_with('\n') && result.ends_with('\n') {
            result.pop();
        }

        result
    }

    /// Normalize excessive whitespace in final output.
    /// Removes double spaces while preserving:
    /// - Code blocks
    /// - Tables
    /// - Leading indentation (for nested lists)
    fn normalize_excessive_whitespace(&self, text: &str) -> String {
        let mut result = String::with_capacity(text.len());
        let mut in_code_block = false;

        for line in text.lines() {
            // Detect code block boundaries
            if line.trim_start().starts_with("```") {
                in_code_block = !in_code_block;
                result.push_str(line);
                result.push('\n');
                continue;
            }

            // Don't normalize inside code blocks or table rows
            if in_code_block || line.trim_start().starts_with('|') {
                result.push_str(line);
                result.push('\n');
                continue;
            }

            // Preserve leading whitespace (indentation), normalize rest of line
            let leading_spaces = line.len() - line.trim_start().len();
            let leading_indent = &line[..leading_spaces];
            let rest_of_line = &line[leading_spaces..];

            // Add preserved leading indent
            result.push_str(leading_indent);

            // Normalize double spaces only in the non-indent portion
            let mut prev_char = '\0';
            for ch in rest_of_line.chars() {
                if ch == ' ' && prev_char == ' ' {
                    // Skip consecutive spaces in content (not indent)
                    continue;
                }
                result.push(ch);
                prev_char = ch;
            }
            result.push('\n');
        }

        result
    }

    /// Clean up malformed markdown-like artifacts from PDF extraction.
    /// These often come from figure/table annotations, checkboxes, or bullet points.
    fn cleanup_markdown_artifacts(&self, text: &str) -> String {
        use regex::Regex;
        let mut result = text.to_string();

        // Remove patterns like "*[]*.*", "*-*", "*.*", "*[]**.*"
        // These are garbled representations of bullets/checkboxes
        let artifact_patterns = [
            (r"\*\[\]\*\*\.\*", " "), // *[]**.*
            (r"\*\[\]\*", " "),       // *[]*
            (r" \*-\*\s*", " "),      // *-*  (space before)
            (r"\*\.\*\s*", " "),      // *.*
            (r" - \*-\*", " "),       // - *-*
            (r"\n\*\.\*\s*", "\n"),   // *.* at start of line
        ];

        for (pattern, replacement) in artifact_patterns {
            if let Ok(re) = Regex::new(pattern) {
                result = re.replace_all(&result, replacement).to_string();
            }
        }

        result
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

        // Add document title if available and not already present as the first block
        if let Some(title) = &document.metadata.title {
            let first_block_text = document
                .pages
                .first()
                .and_then(|p| p.blocks.first())
                .map(|b| b.text.trim());

            if first_block_text != Some(title.trim()) {
                output.push_str(&format!("# {}\n\n", title));
            }
        }

        // Render each page
        for (i, page) in document.pages.iter().enumerate() {
            if i > 0 && self.style.page_breaks {
                output.push_str("\n---\n\n");
            }

            self.render_page(page, &mut output);
        }

        // Final normalization: remove excessive whitespace
        // This catches any double-spaces that slipped through span/block processing
        let output = output.trim().to_string();
        let output = self.normalize_excessive_whitespace(&output);
        let output = self.cleanup_markdown_artifacts(&output);

        Ok(output)
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

    // Additional markdown renderer tests for Phase 4.1

    #[test]
    fn test_default_style() {
        let style = MarkdownStyle::default();
        assert!(style.page_breaks);
        assert!(style.page_numbers);
        assert_eq!(style.max_heading_level, 6);
        assert!(style.atx_headers);
        assert!(style.fenced_code);
        assert!(!style.include_block_ids);
    }

    #[test]
    fn test_renderer_extension() {
        let renderer = MarkdownRenderer::new();
        assert_eq!(renderer.extension(), "md");
    }

    #[test]
    fn test_renderer_mime_type() {
        let renderer = MarkdownRenderer::new();
        assert_eq!(renderer.mime_type(), "text/markdown");
    }

    #[test]
    fn test_empty_document() {
        let renderer = MarkdownRenderer::new();
        let doc = Document::new();
        let result = renderer.render(&doc).unwrap();
        // Empty doc should produce empty or minimal output
        assert!(result.is_empty() || result.trim().is_empty());
    }

    #[test]
    fn test_document_with_title() {
        let renderer = MarkdownRenderer::new();
        let mut doc = Document::new();
        doc.metadata.title = Some("My Title".to_string());
        let result = renderer.render(&doc).unwrap();
        assert!(result.contains("# My Title"));
    }

    #[test]
    fn test_code_block_rendering() {
        let renderer = MarkdownRenderer::new();

        let mut doc = Document::new();
        let mut page = Page::new(1, 612.0, 792.0);
        page.add_block(Block::code(
            "let x = 42;",
            BoundingBox::new(72.0, 100.0, 540.0, 120.0),
        ));
        doc.add_page(page);

        let result = renderer.render(&doc).unwrap();
        assert!(result.contains("```"));
        assert!(result.contains("let x = 42;"));
    }

    #[test]
    fn test_table_rendering() {
        let renderer = MarkdownRenderer::new();

        let mut doc = Document::new();
        let mut page = Page::new(1, 612.0, 792.0);

        let mut table_block = Block::new(
            BlockType::Table,
            BoundingBox::new(72.0, 100.0, 540.0, 200.0),
        );
        table_block.text = "A\tB\tC\n1\t2\t3".to_string();
        page.add_block(table_block);
        doc.add_page(page);

        let result = renderer.render(&doc).unwrap();
        // Should contain the table content in some form
        assert!(result.contains("A") && result.contains("B") && result.contains("C"));
    }

    #[test]
    fn test_multiple_pages() {
        let renderer = MarkdownRenderer::new();

        let mut doc = Document::new();
        let mut page1 = Page::new(1, 612.0, 792.0);
        page1.add_block(Block::text("Page one content", BoundingBox::default()));
        doc.add_page(page1);

        let mut page2 = Page::new(2, 612.0, 792.0);
        page2.add_block(Block::text("Page two content", BoundingBox::default()));
        doc.add_page(page2);

        let result = renderer.render(&doc).unwrap();
        assert!(result.contains("Page 1"));
        assert!(result.contains("Page 2"));
        assert!(result.contains("Page one content"));
        assert!(result.contains("Page two content"));
    }

    #[test]
    fn test_heading_levels() {
        let renderer = MarkdownRenderer::new();

        let mut doc = Document::new();
        let mut page = Page::new(1, 612.0, 792.0);
        page.add_block(Block::header("H1", 1, BoundingBox::default()));
        page.add_block(Block::header("H2", 2, BoundingBox::default()));
        page.add_block(Block::header("H3", 3, BoundingBox::default()));
        doc.add_page(page);

        let result = renderer.render(&doc).unwrap();
        assert!(result.contains("# H1"));
        assert!(result.contains("## H2") || result.contains("### H2")); // Depends on page number header
        assert!(result.contains("### H3") || result.contains("#### H3"));
    }

    #[test]
    fn test_nested_list_items() {
        let renderer = MarkdownRenderer::new();

        let mut doc = Document::new();
        let mut page = Page::new(1, 612.0, 792.0);

        let mut item1 = Block::list_item("Item 1", BoundingBox::default());
        item1
            .metadata
            .insert("level".to_string(), serde_json::json!(0));
        page.add_block(item1);

        let mut item2 = Block::list_item("Nested item", BoundingBox::default());
        item2
            .metadata
            .insert("level".to_string(), serde_json::json!(1));
        page.add_block(item2);

        doc.add_page(page);

        let result = renderer.render(&doc).unwrap();
        assert!(result.contains("Item 1"));
        assert!(result.contains("Nested item"));
    }

    #[test]
    fn test_max_heading_level() {
        let style = MarkdownStyle {
            max_heading_level: 3,
            ..Default::default()
        };
        let renderer = MarkdownRenderer::with_style(style);

        let mut doc = Document::new();
        let mut page = Page::new(1, 612.0, 792.0);
        page.add_block(Block::header("Deep heading", 6, BoundingBox::default()));
        doc.add_page(page);

        let result = renderer.render(&doc).unwrap();
        // Level 6 should be clamped to max 3
        assert!(result.contains("###"));
        assert!(!result.contains("######"));
    }
}
