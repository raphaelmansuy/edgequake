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
use crate::layout::hyphenation::resolve_hyphenation;
use crate::layout::list_hierarchy::compute_list_levels;
use crate::renderers::pua_filter::filter_pua;

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
    /// OODA-10: Include page number in page separators.
    /// When true, renders `-----` followed by `Page N` instead of plain `---`.
    pub page_separators: bool,
}

impl Default for MarkdownConfig {
    fn default() -> Self {
        Self {
            block_spacing: true,
            preserve_styles: true,
            fenced_code: true,
            max_heading_level: 6,
            // OODA-50: Default off for SOTA quality - gold standards don't use separators
            page_separators: false,
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
    /// OODA-03: Computes list hierarchy levels before rendering for proper indentation.
    /// OODA-32: Skips empty/whitespace-only blocks to prevent spurious blank lines.
    pub fn render(&self, blocks: &[Block]) -> String {
        let mut output = String::new();
        let mut last_page = 0;

        // OODA-03: Compute list hierarchy levels from x0 coordinates
        let list_levels = compute_list_levels(blocks);

        for (i, block) in blocks.iter().enumerate() {
            // OODA-32: Skip blocks that are entirely whitespace
            // WHY: Empty blocks from PDF metadata or spacing artifacts create
            // spurious blank lines in output. Filter them early.
            if block.lines.is_empty() || block.text().trim().is_empty() {
                continue;
            }
            // Add page separator if page changed
            if block.page_num != last_page && i > 0 {
                if self.config.page_separators {
                    // OODA-10: Include page number in separator
                    output.push_str(&format!("\n-----\n\nPage {}\n\n", block.page_num + 1));
                }
                // OODA-50: When separators off, just let normal block spacing handle it
                last_page = block.page_num;
            }

            // Render this block (pass list level if applicable)
            let block_text = self.render_block(block, list_levels.get(&i).copied());
            output.push_str(&block_text);

            // Add spacing between blocks
            // OODA-24: Use single newline between consecutive list items (tight list)
            // OODA-29: Also use tight spacing for consecutive code blocks
            // OODA-64: Always use \n\n between paragraphs (match pymupdf4llm behavior)
            // WHY: pymupdf4llm treats each text block as a separate paragraph.
            // Merging via is_continuation() reduced paragraph count by 2x vs gold,
            // destroying ROA (0.273 on Apple-Sandbox).
            if self.config.block_spacing && i < blocks.len() - 1 {
                let next_block = &blocks[i + 1];
                let is_list_continuation = block.block_type == BlockType::ListItem
                    && next_block.block_type == BlockType::ListItem;
                let is_code_continuation =
                    block.block_type == BlockType::Code && next_block.block_type == BlockType::Code;
                if is_list_continuation || is_code_continuation {
                    output.push('\n');
                } else {
                    output.push_str("\n\n");
                }
            }
        }

        // OODA-21: Post-render cleanup
        clean_markdown_output(&mut output);

        output
    }

    fn render_block(&self, block: &Block, list_level: Option<u8>) -> String {
        match block.block_type {
            BlockType::Header(level) => self.render_header(block, level),
            BlockType::Code => self.render_code(block),
            BlockType::ListItem => self.render_list_item(block, list_level.unwrap_or(0)),
            BlockType::Table => self.render_table(block),
            BlockType::Footnote => self.render_footnote(block),
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
            // OODA-22: Detect language from code content for fenced blocks
            let lang = detect_code_language(&code);
            format!("```{}\n{}\n```", lang, code)
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

    /// OODA-03: Render list item with proper indentation based on hierarchy level.
    /// Level 0 = no indent, level 1 = 2 spaces, level 2 = 4 spaces, etc.
    fn render_list_item(&self, block: &Block, level: u8) -> String {
        let indent = "  ".repeat(level as usize);
        let mut lines_iter = block.lines.iter();

        if let Some(first_line) = lines_iter.next() {
            let first_text = self.render_line_styled(first_line);
            let normalized = normalize_bullet(&first_text);

            let continuation: String = lines_iter
                .map(|l| format!("{}  {}", indent, self.render_line_styled(l)))
                .collect::<Vec<_>>()
                .join("\n");

            if continuation.is_empty() {
                format!("{}{}", indent, normalized)
            } else {
                format!("{}{}\n{}", indent, normalized, continuation)
            }
        } else {
            String::new()
        }
    }

    fn render_table(&self, block: &Block) -> String {
        // KNOWN LIMITATION: Proper table rendering not implemented
        // WHY: Requires cell boundary detection which is complex:
        // - May need PDF line/rect detection for borders
        // - Cell content alignment detection
        // - Table structure inference from spatial relationships
        // WORKAROUND: Tables are rendered as paragraphs (text content preserved)
        // FUTURE: Use backend/lattice.rs for table structure detection
        self.render_paragraph(block)
    }

    /// OODA-08: Render footnote as blockquote.
    /// WHY: pymupdf4llm renders footnotes as blockquotes with `> ` prefix,
    /// which visually separates them from body text while preserving content.
    fn render_footnote(&self, block: &Block) -> String {
        let text = self.render_lines_inline(&block.lines);
        text.lines()
            .map(|line| format!("> {}", line))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn render_paragraph(&self, block: &Block) -> String {
        // OODA-65: Join lines with \n\n to match pymupdf4llm line-level paragraph granularity.
        // WHY: pymupdf4llm treats each PDF line as a separate text block, producing \n\n
        // between every line. Our grouper merges adjacent lines into blocks, producing
        // fewer paragraphs (963 vs 1620 for Apple-Sandbox). Splitting at line boundaries
        // matches the gold standard's paragraph granularity, improving ROA.
        // OODA-66: Tried smart sentence-ending heuristic but it was worse overall
        // because single-column golds have \n\n between non-sentence-ending lines too.
        let rendered: Vec<String> = block
            .lines
            .iter()
            .map(|l| self.render_line_styled(l))
            .collect();
        let resolved = resolve_hyphenation(&rendered);
        resolved.join("\n\n")
    }

    /// Render multiple lines joined with newlines (preserve soft breaks).
    /// OODA-05: Applies hyphenation resolution before joining.
    /// OODA-55: Join with newline instead of space to preserve soft line breaks.
    /// WHY: Gold standards (pymupdf4llm) preserve soft line breaks at column width.
    /// Joining with spaces creates mega-lines (2000+ chars) that hurt readability
    /// and ROA paragraph matching. Newlines preserve the natural column structure.
    fn render_lines_inline(&self, lines: &[Line]) -> String {
        let rendered: Vec<String> = lines.iter().map(|l| self.render_line_styled(l)).collect();
        let resolved = resolve_hyphenation(&rendered);
        resolved.join("\n")
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

        // OODA-04: Compute dominant font size for superscript detection
        let ref_font_size = line.dominant_font_size();
        // OODA-19: Compute reference y1 for subscript detection
        let ref_y1 = line.y1;

        if line.spans.len() == 1 {
            let span = &line.spans[0];
            let text = filter_pua(&span.text);
            if text.trim().is_empty() {
                return text;
            }
            let style = get_style_type_with_ref(span, ref_font_size, ref_y1);
            return apply_style(&text, style);
        }

        // Group consecutive spans with same style, including spaces within groups
        let mut groups: Vec<(String, StyleType)> = Vec::new();
        let mut current_text = String::new();
        let mut current_style = get_style_type_with_ref(&line.spans[0], ref_font_size, ref_y1);

        for (i, span) in line.spans.iter().enumerate() {
            // OODA-02: Filter PUA characters from each span
            let span_text = filter_pua(&span.text);
            if span_text.is_empty() {
                continue; // Skip spans that are entirely PUA
            }

            // Determine if we need a space before this span
            let needs_space = if i > 0 {
                let prev = &line.spans[i - 1];
                let gap = span.x0 - prev.x1;
                let avg_size = (prev.font_size + span.font_size) / 2.0;

                let starts_with_hyphen = span.text.starts_with('-')
                    || span.text.starts_with('–')
                    || span.text.starts_with('—');
                let ends_with_hyphen = prev.text.ends_with('-')
                    || prev.text.ends_with('–')
                    || prev.text.ends_with('—');

                // OODA-57: Style-aware space threshold.
                // WHY: Spans are separated either by:
                //   (a) PDF space character → gap = visual space width → NEED space
                //   (b) Font/style change → gap = kerning → NO space needed
                //
                // When font/style is the SAME between spans, the break was caused
                // by a space character in chars_to_spans(). Use LOW threshold (0.08)
                // to catch compressed justified-text spaces.
                //
                // When font/style DIFFERS, the break was from a style change.
                // Use HIGH threshold (0.20) to avoid mid-word splits at style
                // boundaries (e.g., italic→regular within "temporal").
                let same_style = prev.font_name == span.font_name
                    && prev.font_is_bold == span.font_is_bold
                    && prev.font_is_italic == span.font_is_italic;

                let prev_ends_alpha = prev
                    .text
                    .chars()
                    .last()
                    .map(|c| c.is_alphabetic())
                    .unwrap_or(false);
                let cur_starts_alpha = span
                    .text
                    .chars()
                    .next()
                    .map(|c| c.is_alphabetic())
                    .unwrap_or(false);

                let space_threshold = if same_style {
                    // Same font/style: break was from a space char or gap
                    avg_size * 0.10
                } else if prev_ends_alpha && cur_starts_alpha {
                    // Different style, alpha-alpha: likely mid-word style change
                    avg_size * 0.20
                } else {
                    // Different style, involves punctuation/digits
                    avg_size * 0.10
                };

                gap > space_threshold && !starts_with_hyphen && !ends_with_hyphen
            } else {
                false
            };

            let span_style = get_style_type_with_ref(span, ref_font_size, ref_y1);

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

            current_text.push_str(&span_text);
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

    /// Render a line without style markers (plain text).
    /// OODA-02: Applies PUA filtering to prevent garbage symbols.
    /// OODA-20: Use spatial gap detection for spacing (same as styled rendering),
    /// instead of always joining with " " which creates extra spaces in code blocks.
    fn render_line_plain(&self, line: &Line) -> String {
        if line.spans.is_empty() {
            return String::new();
        }

        let mut result = String::new();
        for (i, span) in line.spans.iter().enumerate() {
            let text = filter_pua(&span.text);
            if text.is_empty() {
                continue;
            }

            // Determine if we need a space before this span
            if i > 0 {
                let prev = &line.spans[i - 1];
                let gap = span.x0 - prev.x1;
                let avg_size = (prev.font_size + span.font_size) / 2.0;
                // OODA-57: Style-aware space threshold (same logic as styled renderer)
                let same_style = prev.font_name == span.font_name
                    && prev.font_is_bold == span.font_is_bold
                    && prev.font_is_italic == span.font_is_italic;
                let prev_ends_alpha = prev
                    .text
                    .chars()
                    .last()
                    .map(|c| c.is_alphabetic())
                    .unwrap_or(false);
                let cur_starts_alpha = span
                    .text
                    .chars()
                    .next()
                    .map(|c| c.is_alphabetic())
                    .unwrap_or(false);

                let space_threshold = if same_style {
                    avg_size * 0.10
                } else if prev_ends_alpha && cur_starts_alpha {
                    avg_size * 0.20
                } else {
                    avg_size * 0.10
                };
                if gap > space_threshold {
                    result.push(' ');
                }
            }

            result.push_str(&text);
        }
        result
    }
}

/// Style types for span grouping.
/// OODA-04: Added Superscript for footnote markers
/// OODA-19: Added Subscript for chemical/math notation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StyleType {
    Plain,
    Bold,
    Italic,
    BoldItalic,
    Code,
    Superscript,
    Subscript,
}

/// Get the style type of a span.
/// OODA-04: Accepts reference_font_size for superscript detection.
/// OODA-19: Also accepts ref_y1 for subscript detection.
/// Distinguishes super/subscript by vertical position within the line.
fn get_style_type_with_ref(
    span: &super::pymupdf_structs::Span,
    reference_font_size: f32,
    ref_y1: f32,
) -> StyleType {
    // Check for small text that could be super/subscript
    let is_small = reference_font_size > 0.0
        && span.font_size / reference_font_size < 0.7
        && span.text.chars().count() < 5;

    if is_small {
        // OODA-19: Distinguish by position - subscripts touch the baseline (y1 near ref_y1)
        let near_baseline = (span.y1 - ref_y1).abs() < reference_font_size * 0.15;
        if near_baseline {
            return StyleType::Subscript;
        }
        // Otherwise it's superscript (floats above baseline)
        return StyleType::Superscript;
    }
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
/// OODA-04: Added Superscript rendering as [text] for footnote markers
/// OODA-19: Added Subscript rendering as ~text~ for chemical/math notation
fn apply_style(text: &str, style: StyleType) -> String {
    if text.trim().is_empty() {
        return text.to_string();
    }

    // Preserve leading and trailing whitespace
    let leading_space = text.len() - text.trim_start().len();
    let trailing_space = text.len() - text.trim_end().len();
    let trimmed = text.trim();

    let styled = match style {
        StyleType::Superscript => format!("[{}]", trimmed),
        StyleType::Subscript => format!("~{}~", trimmed),
        StyleType::BoldItalic => format!("**_{}_**", trimmed),
        StyleType::Bold => format!("**{}**", trimmed),
        StyleType::Italic => format!("_{}_", trimmed),
        StyleType::Code if !trimmed.starts_with('`') => format!("`{}`", trimmed),
        _ => trimmed.to_string(),
    };

    // Re-add whitespace
    let leading: String = " ".repeat(leading_space.min(1));
    let trailing: String = " ".repeat(trailing_space.min(1));
    format!("{}{}{}", leading, styled, trailing)
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// OODA-27: Check if the next block is a continuation of the current paragraph.
/// OODA-35: Enhanced with markdown-marker stripping and closing bracket/paren handling.
/// OODA-44: Also merge when current text ends with a comma or conjunction word.
/// A paragraph continues if:
/// - Current text does NOT end with sentence-ending punctuation (.!?:)
/// - Next block starts with a lowercase letter
/// - OR current text ends with comma/conjunction and next starts with letter
#[allow(dead_code)]
fn is_continuation(current_text: &str, next_block: &Block) -> bool {
    let trimmed = current_text.trim_end();
    if trimmed.is_empty() {
        return false;
    }

    // OODA-35: Strip trailing markdown markers to see the actual last text character
    // WHY: Lines like "**bold text**" end with `*` which isn't punctuation,
    // but the actual content ends with "text" which should allow continuation.
    let stripped = trimmed
        .trim_end_matches('*')
        .trim_end_matches('_')
        .trim_end_matches('`')
        .trim_end_matches('~')
        .trim_end_matches(']');

    let effective_end = if stripped.is_empty() {
        trimmed
    } else {
        stripped
    };

    // Check current text doesn't end with sentence-terminal punctuation
    let last_char = effective_end.chars().last().unwrap_or('.');
    if matches!(last_char, '.' | '!' | '?' | ':' | ';') {
        return false;
    }
    // OODA-35: Don't merge after closing parens/brackets (often end of citations)
    if matches!(last_char, ')' | ']') {
        return false;
    }

    // Check next block starts with lowercase
    let next_text = next_block
        .lines
        .first()
        .map(|l| l.text())
        .unwrap_or_default();
    let next_trimmed = next_text.trim_start();

    if let Some(first_char) = next_trimmed.chars().next() {
        if first_char.is_lowercase() {
            return true;
        }

        // OODA-44: If current line ends with comma, merge even with uppercase next
        // WHY: Enumerations like "Smith, Jones," / "Anderson and Brown" should merge
        if last_char == ',' && first_char.is_alphabetic() {
            return true;
        }
    }

    false
}

/// OODA-59: Split reference entries at [N] boundaries.
/// WHY: PDF reference sections often produce continuous text blocks where
/// multiple references are merged. The gold standard has each reference
/// as its own paragraph. This function inserts blank lines before [N]
/// patterns that start a new reference entry within a paragraph.
fn split_reference_entries(text: &mut String) {
    use std::fmt::Write;

    let lines: Vec<&str> = text.lines().collect();
    if lines.len() < 3 {
        return;
    }

    let mut result = String::with_capacity(text.len() + text.len() / 20);
    let mut in_references = false;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Detect when we enter a references section
        if !in_references {
            // OODA-59: Robust reference header detection.
            // WHY: Papers use varied formats: "**References**", "## **References**",
            // "**7.** **References**", "REFERENCES". Strip markdown and check content.
            let stripped = trimmed
                .replace("**", "")
                .replace("##", "")
                .replace('#', "")
                .trim()
                .to_lowercase();
            // Strip leading numbered prefix like "7. " or "7 "
            let content = stripped
                .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.')
                .trim();
            if content == "references" || content == "bibliography" {
                in_references = true;
            }
        }

        if in_references && i > 0 && is_reference_start(trimmed) {
            // Check if previous line was NOT blank or a reference header
            let prev = lines[i - 1].trim();
            if !prev.is_empty() {
                // Insert blank line before this reference entry
                let _ = writeln!(result);
            }
        }

        let _ = writeln!(result, "{}", line);
    }

    *text = result;
}

/// Check if a line starts a reference entry like "[1]", "[23]", etc.
fn is_reference_start(trimmed: &str) -> bool {
    if !trimmed.starts_with('[') {
        return false;
    }
    // Match [N] where N is one or more digits
    let rest = &trimmed[1..];
    let num_end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(0);
    if num_end == 0 {
        return false;
    }
    rest[num_end..].starts_with(']')
}

/// OODA-21: Clean up rendered markdown output.
/// - Trim trailing whitespace from each line
/// - Collapse 3+ consecutive blank lines to 2 (one empty line)
/// - OODA-52: Filter isolated CJK characters between ASCII text
/// - OODA-59: Split reference entries at [N] boundaries
/// - Trim trailing newlines from final output
fn clean_markdown_output(output: &mut String) {
    // OODA-52: Remove isolated CJK characters that appear between ASCII text.
    // WHY: PDFs with figure overlays often produce CJK characters interspersed
    // with English text (e.g., "graceful" → "g你ra的cef落ul") due to overlapping
    // text layers. Filter these by removing single CJK chars surrounded by ASCII.
    filter_isolated_cjk(output);

    // OODA-59: Split reference entries at [N] boundaries.
    // WHY: References like "[1] Author..." "[2] Author..." are often grouped
    // into one block by the grouper, producing a single mega-paragraph.
    // The gold standard separates each reference as its own paragraph.
    // Inserting blank lines at [N] boundaries fixes ROA for reference sections.
    split_reference_entries(output);

    // Trim trailing whitespace from each line and collapse excessive blank lines
    let lines: Vec<&str> = output.lines().collect();
    let mut cleaned = String::with_capacity(output.len());
    let mut blank_count = 0;

    for line in &lines {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            blank_count += 1;
            // Allow at most 2 consecutive blank lines (1 visual separator)
            if blank_count <= 2 {
                cleaned.push('\n');
            }
        } else {
            blank_count = 0;
            cleaned.push_str(trimmed);
            cleaned.push('\n');
        }
    }

    // Trim trailing newlines, then add exactly one
    let trimmed_end = cleaned.trim_end_matches('\n');
    *output = trimmed_end.to_string();
    output.push('\n');
}

/// OODA-52: Remove isolated CJK characters from text.
/// Targets single CJK chars (or short runs of 1-2) that appear between ASCII characters.
/// WHY: PDF figure overlays produce garbage like "g你ra的cef落ul" from overlapping text layers.
fn filter_isolated_cjk(text: &mut String) {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() < 3 {
        return;
    }

    let mut result = String::with_capacity(text.len());
    let mut i = 0;

    while i < chars.len() {
        if is_cjk_char(chars[i]) {
            // Check if this is an isolated CJK char (surrounded by non-CJK on both sides)
            let prev_is_ascii = i > 0 && !is_cjk_char(chars[i - 1]) && chars[i - 1] != '\n';
            let mut cjk_run_end = i + 1;
            while cjk_run_end < chars.len() && is_cjk_char(chars[cjk_run_end]) {
                cjk_run_end += 1;
            }
            let cjk_run_len = cjk_run_end - i;
            let next_is_ascii = cjk_run_end < chars.len()
                && !is_cjk_char(chars[cjk_run_end])
                && chars[cjk_run_end] != '\n';

            // Only filter short CJK runs (1-2 chars) between ASCII text
            if prev_is_ascii && next_is_ascii && cjk_run_len <= 2 {
                // Skip the isolated CJK chars
                i = cjk_run_end;
                continue;
            }
        }
        result.push(chars[i]);
        i += 1;
    }

    *text = result;
}

/// Check if a character is in a CJK Unicode block.
fn is_cjk_char(c: char) -> bool {
    let cp = c as u32;
    matches!(
        cp,
        0x4E00..=0x9FFF       // CJK Unified Ideographs
        | 0x3400..=0x4DBF     // CJK Unified Ideographs Extension A
        | 0x2E80..=0x2EFF     // CJK Radicals Supplement
        | 0x2F00..=0x2FDF     // Kangxi Radicals
        | 0x3000..=0x303F     // CJK Symbols and Punctuation
        | 0xF900..=0xFAFF     // CJK Compatibility Ideographs
    )
}

/// OODA-22: Detect programming language from code block content.
/// OODA-39: Extended with YAML, TOML, Markdown, LaTeX, and Go patterns.
/// Returns language identifier for fenced code blocks or "" if unknown.
fn detect_code_language(code: &str) -> &'static str {
    let trimmed = code.trim();

    // Shell/bash patterns
    if trimmed.starts_with("$ ")
        || trimmed.starts_with("# !/bin")
        || trimmed.starts_with("#!/bin")
        || trimmed.contains("apt-get ")
        || trimmed.contains("sudo ")
        || trimmed.contains("pip install")
    {
        return "bash";
    }

    // Python patterns
    if trimmed.contains("def ") && trimmed.contains(":")
        || trimmed.contains("import ") && (trimmed.contains("from ") || !trimmed.contains('{'))
        || trimmed.starts_with("class ") && trimmed.contains(":")
        || trimmed.contains("print(")
        || trimmed.contains("if __name__")
    {
        return "python";
    }

    // Rust patterns
    if trimmed.contains("fn ") && trimmed.contains("->")
        || trimmed.contains("let mut ")
        || trimmed.contains("impl ") && trimmed.contains('{')
        || trimmed.contains("pub fn ")
        || trimmed.contains("use std::")
    {
        return "rust";
    }

    // Go patterns
    if trimmed.contains("func ") && trimmed.contains('{')
        || trimmed.contains("package main")
        || trimmed.contains("fmt.Println")
        || trimmed.contains(":= ")
    {
        return "go";
    }

    // OODA-45: TypeScript patterns (BEFORE JavaScript to distinguish typed code)
    if trimmed.contains(": string")
        || trimmed.contains(": number")
        || trimmed.contains(": boolean")
        || trimmed.contains("interface ") && trimmed.contains('{')
        || trimmed.contains("type ") && trimmed.contains(" = {")
    {
        return "typescript";
    }

    // JavaScript patterns
    if trimmed.contains("const ") && trimmed.contains(" = ")
        || trimmed.contains("function ") && trimmed.contains("(")
        || trimmed.contains("console.log")
        || trimmed.contains("=> {")
    {
        return "javascript";
    }

    // Java/C# patterns
    if (trimmed.contains("public static") || trimmed.contains("private ")) && trimmed.contains('{')
    {
        return "java";
    }

    // C/C++ patterns
    if trimmed.contains("#include")
        || (trimmed.contains("int main") && trimmed.contains('{'))
        || trimmed.contains("printf(")
    {
        return "c";
    }

    // SQL patterns
    if trimmed.to_uppercase().starts_with("SELECT ")
        || trimmed.to_uppercase().starts_with("INSERT ")
        || trimmed.to_uppercase().starts_with("CREATE TABLE")
    {
        return "sql";
    }

    // JSON patterns
    if ((trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']')))
        && (trimmed.contains("\":") || trimmed.contains("\": "))
    {
        return "json";
    }

    // OODA-39: YAML patterns
    if (trimmed.contains(": ") && trimmed.contains('\n') && !trimmed.contains('{'))
        || trimmed.starts_with("---\n")
        || trimmed.starts_with("apiVersion:")
    {
        return "yaml";
    }

    // OODA-39: TOML patterns
    if trimmed.starts_with("[package]")
        || trimmed.starts_with("[dependencies]")
        || trimmed.starts_with("[tool.")
    {
        return "toml";
    }

    // OODA-39: LaTeX patterns
    if trimmed.starts_with("\\documentclass")
        || trimmed.starts_with("\\begin{")
        || trimmed.contains("\\usepackage")
    {
        return "latex";
    }

    // XML/HTML patterns
    if trimmed.starts_with("<?xml")
        || trimmed.starts_with("<!DOCTYPE")
        || trimmed.starts_with("<html")
    {
        return "xml";
    }

    // OODA-45: R language patterns
    if trimmed.contains("<- ") && (trimmed.contains("function(") || trimmed.contains("c("))
        || trimmed.contains("library(") && trimmed.contains(')')
        || trimmed.contains("ggplot(")
    {
        return "r";
    }

    ""
}

/// Normalize bullet characters to standard Markdown bullets.
/// OODA-39: Also normalize dash-based bullets (en dash, em dash) to standard "- ".
fn normalize_bullet(text: &str) -> String {
    let trimmed = text.trim_start();

    // Common bullet characters to normalize
    // OODA-14: Comprehensive bullet normalization from pymupdf4llm
    // Includes Unicode geometric shapes (U+25A0-25FF), common symbols
    const BULLETS: &[char] = &[
        '•', '●', '○', '◦', '▪', '▫', '–', '—', '∙', '·', '‣', '⁃', '◆', '◇', '►', '▸', '★', '☆',
        '■', '□', '▶', '‐', '‑', '‒', '―', '†', '‡', '※', '¶', '\u{F0A7}', // PUA bullet
        '\u{F0B7}', // PUA bullet
    ];

    for &bullet in BULLETS {
        if let Some(rest) = trimmed.strip_prefix(bullet) {
            return format!("- {}", rest.trim_start());
        }
    }

    // Already standard bullet
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        return text.to_string();
    }

    // OODA-38: Normalize parenthesized list items to standard numbered form
    // "(1) text" → "1. text", "(a) text" → "a. text"
    if trimmed.starts_with('(') {
        if let Some(close) = trimmed.find(')') {
            if close > 1 && close < 6 {
                let inner = &trimmed[1..close];
                let rest = trimmed[close + 1..].trim_start();
                let is_valid = inner.chars().all(|c| c.is_ascii_digit())
                    || (inner.len() == 1 && inner.chars().all(|c| c.is_ascii_lowercase()))
                    || inner.chars().all(|c| matches!(c, 'i' | 'v' | 'x'));
                if is_valid && !rest.is_empty() {
                    return format!("{}. {}", inner, rest);
                }
            }
        }
    }

    // OODA-40: Normalize numbered list separators to standard "N. " form
    // "2) text" → "2. text", "3: text" → "3. text"
    {
        let mut chars = trimmed.chars().peekable();
        let mut digits = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() {
                digits.push(c);
                chars.next();
            } else {
                break;
            }
        }
        if !digits.is_empty() {
            match chars.next() {
                Some(')') | Some(':') => {
                    let rest_str: String = chars.collect();
                    let rest_trimmed = rest_str.trim_start();
                    if !rest_trimmed.is_empty() {
                        return format!("{}. {}", digits, rest_trimmed);
                    }
                }
                _ => {}
            }
        }
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
        // OODA-14: Extended Unicode bullets
        assert_eq!(normalize_bullet("◆ Diamond"), "- Diamond");
        assert_eq!(normalize_bullet("► Arrow"), "- Arrow");
        assert_eq!(normalize_bullet("■ Square"), "- Square");
        assert_eq!(normalize_bullet("‣ Triangular"), "- Triangular");
        assert_eq!(normalize_bullet("† Dagger"), "- Dagger");
        // OODA-38: Parenthesized list items
        assert_eq!(normalize_bullet("(1) First item"), "1. First item");
        assert_eq!(normalize_bullet("(a) Sub-item"), "a. Sub-item");
        assert_eq!(normalize_bullet("(ii) Roman"), "ii. Roman");
        // OODA-40: Numbered list separator normalization
        assert_eq!(normalize_bullet("2) Second item"), "2. Second item");
        assert_eq!(normalize_bullet("3: Third item"), "3. Third item");
        assert_eq!(normalize_bullet("10) Tenth item"), "10. Tenth item");
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

    /// OODA-04: Test superscript rendering as [text] bracket notation
    #[test]
    fn test_render_superscript() {
        let renderer = MarkdownRenderer::new();

        // Create a line with normal text and a superscript footnote marker
        let normal_span = Span {
            text: "reference".to_string(),
            x0: 0.0,
            y0: 0.0,
            x1: 80.0,
            y1: 12.0,
            font_size: 12.0,
            font_name: Some("Arial".to_string()),
            page_num: 0,
            font_is_bold: Some(false),
            font_is_italic: Some(false),
            font_is_monospace: Some(false),
        };
        let superscript_span = Span {
            text: "1".to_string(),
            x0: 80.0,
            y0: 4.0, // Higher position
            x1: 86.0,
            y1: 10.0,
            font_size: 7.0, // Much smaller than 12.0 (< 70%)
            font_name: Some("Arial".to_string()),
            page_num: 0,
            font_is_bold: Some(false),
            font_is_italic: Some(false),
            font_is_monospace: Some(false),
        };

        let line = make_line(vec![normal_span, superscript_span]);
        let block = Block {
            lines: vec![line],
            x0: 0.0,
            y0: 0.0,
            x1: 86.0,
            y1: 12.0,
            page_num: 0,
            block_type: BlockType::Paragraph,
        };

        let md = renderer.render(&[block]);
        assert!(
            md.contains("[1]"),
            "Superscript should render as [1], got: {}",
            md
        );
        assert!(
            md.contains("reference"),
            "Normal text should be preserved: {}",
            md
        );
    }

    /// OODA-19: Test subscript rendering as ~text~ notation
    #[test]
    fn test_render_subscript() {
        let renderer = MarkdownRenderer::new();

        // "H" normal span + "2" subscript span (small font, at bottom of line)
        let normal_span = Span {
            text: "H".to_string(),
            x0: 0.0,
            y0: 0.0,
            x1: 10.0,
            y1: 12.0,
            font_size: 12.0,
            font_name: Some("Arial".to_string()),
            page_num: 0,
            font_is_bold: Some(false),
            font_is_italic: Some(false),
            font_is_monospace: Some(false),
        };
        let subscript_span = Span {
            text: "2".to_string(),
            x0: 10.0,
            y0: 4.0,
            x1: 15.0,
            y1: 12.0,       // y1 at bottom of line (subscript sits at baseline)
            font_size: 7.0, // Much smaller than 12.0 (< 70%)
            font_name: Some("Arial".to_string()),
            page_num: 0,
            font_is_bold: Some(false),
            font_is_italic: Some(false),
            font_is_monospace: Some(false),
        };
        let normal_span2 = Span {
            text: "O".to_string(),
            x0: 15.0,
            y0: 0.0,
            x1: 25.0,
            y1: 12.0,
            font_size: 12.0,
            font_name: Some("Arial".to_string()),
            page_num: 0,
            font_is_bold: Some(false),
            font_is_italic: Some(false),
            font_is_monospace: Some(false),
        };

        let line = make_line(vec![normal_span, subscript_span, normal_span2]);
        let block = Block {
            lines: vec![line],
            x0: 0.0,
            y0: 0.0,
            x1: 25.0,
            y1: 12.0,
            page_num: 0,
            block_type: BlockType::Paragraph,
        };

        let md = renderer.render(&[block]);
        assert!(
            md.contains("~2~"),
            "Subscript should render as ~2~, got: {}",
            md
        );
        assert!(md.contains("H"), "Normal text should be preserved: {}", md);
    }

    /// OODA-08: Test footnote rendering as blockquote
    #[test]
    fn test_render_footnote() {
        let renderer = MarkdownRenderer::new();

        let block = Block {
            lines: vec![make_line(vec![make_span(
                "1 Author affiliation.",
                "Arial",
                8.0,
            )])],
            x0: 50.0,
            y0: 40.0,
            x1: 400.0,
            y1: 48.0,
            page_num: 0,
            block_type: BlockType::Footnote,
        };

        let md = renderer.render(&[block]);
        assert!(
            md.contains("> "),
            "Footnote should be rendered as blockquote: {}",
            md
        );
        assert!(
            md.contains("Author affiliation"),
            "Footnote content should be preserved: {}",
            md
        );
    }

    /// OODA-10: Test page separators with page numbers
    #[test]
    fn test_page_separators() {
        // OODA-50: page_separators now defaults to false, explicitly enable
        let config = MarkdownConfig {
            page_separators: true,
            ..Default::default()
        };
        let renderer = MarkdownRenderer::with_config(config);

        let blocks = vec![
            Block {
                lines: vec![make_line(vec![make_span("First section", "Arial", 12.0)])],
                x0: 0.0,
                y0: 0.0,
                x1: 200.0,
                y1: 12.0,
                page_num: 0,
                block_type: BlockType::Paragraph,
            },
            Block {
                lines: vec![make_line(vec![make_span("Second section", "Arial", 12.0)])],
                x0: 0.0,
                y0: 0.0,
                x1: 200.0,
                y1: 12.0,
                page_num: 1,
                block_type: BlockType::Paragraph,
            },
        ];

        let md = renderer.render(&blocks);
        assert!(
            md.contains("Page 2"),
            "Should contain page number separator: {}",
            md
        );
        assert!(
            md.contains("-----"),
            "Should contain horizontal rule: {}",
            md
        );
    }

    /// OODA-10: Test page separators disabled
    #[test]
    fn test_page_separators_disabled() {
        let config = MarkdownConfig {
            page_separators: false,
            ..Default::default()
        };
        let renderer = MarkdownRenderer::with_config(config);

        let blocks = vec![
            Block {
                lines: vec![make_line(vec![make_span("First section", "Arial", 12.0)])],
                x0: 0.0,
                y0: 0.0,
                x1: 200.0,
                y1: 12.0,
                page_num: 0,
                block_type: BlockType::Paragraph,
            },
            Block {
                lines: vec![make_line(vec![make_span("Second section", "Arial", 12.0)])],
                x0: 0.0,
                y0: 0.0,
                x1: 200.0,
                y1: 12.0,
                page_num: 1,
                block_type: BlockType::Paragraph,
            },
        ];

        let md = renderer.render(&blocks);
        assert!(
            !md.contains("-----"),
            "Should NOT contain enhanced separator when disabled: {}",
            md
        );
        // OODA-50: When separators off, no --- either, just normal block spacing
        assert!(
            !md.contains("---"),
            "Should NOT have any separator when disabled: {}",
            md
        );
        assert!(
            md.contains("First section") && md.contains("Second section"),
            "Should contain both sections: {}",
            md
        );
    }

    /// OODA-52: Test CJK filtering
    #[test]
    fn test_filter_isolated_cjk() {
        // Isolated CJK between ASCII should be removed
        let mut text = "g\u{4F60}ra\u{7684}cef\u{843D}ul".to_string();
        filter_isolated_cjk(&mut text);
        assert_eq!(
            text, "graceful",
            "Should remove isolated CJK chars: {}",
            text
        );

        // Pure CJK text should be preserved
        let mut chinese = "\u{4F60}\u{597D}\u{4E16}\u{754C}".to_string();
        filter_isolated_cjk(&mut chinese);
        assert_eq!(
            chinese, "\u{4F60}\u{597D}\u{4E16}\u{754C}",
            "Should preserve CJK runs"
        );

        // CJK at start/end of line should be preserved
        let mut edge = "\u{4F60}hello".to_string();
        filter_isolated_cjk(&mut edge);
        // Single CJK at start with no prev ASCII -> preserved
        assert!(edge.contains('\u{4F60}'), "CJK at start should be kept");

        // Short text should not crash
        let mut short = "ab".to_string();
        filter_isolated_cjk(&mut short);
        assert_eq!(short, "ab");
    }

    /// OODA-22: Test code language detection
    #[test]
    fn test_detect_code_language() {
        assert_eq!(
            detect_code_language("def hello():\n    print('hi')"),
            "python"
        );
        assert_eq!(detect_code_language("$ pip install torch"), "bash");
        assert_eq!(detect_code_language("fn main() -> Result<()> {\n}"), "rust");
        assert_eq!(detect_code_language("console.log('hello')"), "javascript");
        assert_eq!(detect_code_language("#include <stdio.h>"), "c");
        assert_eq!(detect_code_language("SELECT * FROM users"), "sql");
        assert_eq!(detect_code_language("some random text"), "");
        // OODA-39: New language detections
        assert_eq!(
            detect_code_language("func main() {\n  fmt.Println(\"hi\")\n}"),
            "go"
        );
        assert_eq!(detect_code_language("x := 42"), "go");
        assert_eq!(detect_code_language("[package]\nname = \"foo\""), "toml");
        assert_eq!(detect_code_language("\\documentclass{article}"), "latex");
        assert_eq!(detect_code_language("\\begin{equation}"), "latex");
        // OODA-45: R and TypeScript detection
        assert_eq!(detect_code_language("x <- function(a) { a + 1 }"), "r");
        assert_eq!(detect_code_language("library(ggplot2)"), "r");
        assert_eq!(
            detect_code_language("interface User {\n  name: string\n}"),
            "typescript"
        );
        assert_eq!(detect_code_language("const x: number = 42"), "typescript");
    }
}
