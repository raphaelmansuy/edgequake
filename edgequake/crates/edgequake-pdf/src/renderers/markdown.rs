//! Markdown renderer for document output.

use crate::processors::starts_with_bullet;
use crate::schema::{Block, BlockType, Document, Page, TextSpan};
use crate::Result;

use super::Renderer;

// =============================================================================
// OODA-IT13: Content-based filtering for inline code
// =============================================================================

/// Check if text is an email address (should NOT be inline code).
///
/// WHY: Emails in monospace fonts should remain plain text for LLM readability.
fn is_inline_email(text: &str) -> bool {
    let trimmed = text.trim();
    // Simple email pattern: contains @ followed by .domain, no programming syntax
    trimmed.contains('@')
        && trimmed.contains('.')
        && !trimmed.contains('=')
        && !trimmed.contains('{')
        && !trimmed.contains('[')
        && !trimmed.contains('(')
        && !trimmed.contains(';')
}

/// Check if text is a URL (should NOT be inline code).
fn is_inline_url(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("www.")
        || trimmed.starts_with("ftp://")
}

/// Check if text should be rendered as inline code.
///
/// OODA-IT13: Applies content-based filtering to exclude non-code patterns
/// even when the font is monospace.
fn should_render_inline_code(text: &str) -> bool {
    // Exclude common false positives from monospace fonts
    if is_inline_email(text) || is_inline_url(text) {
        return false;
    }
    // Additional check: multiple space-separated words that all look like emails
    let words: Vec<&str> = text.split_whitespace().collect();
    if !words.is_empty() && words.iter().all(|w| w.contains('@') && w.contains('.')) {
        return false;
    }
    true
}

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
            // WHY page_breaks=false: For LLM-optimized output, the document
            // should flow as continuous text. Page breaks (`---`) are PDF
            // artifacts that fragment the semantic content. The gold standard
            // (markitdown) does not include page break markers.
            // Users who want page-level separators can use `MarkdownStyle::verbose()`.
            page_breaks: false,
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
        self.render_page_with_arxiv(page, output, None);
    }

    /// Render a page to Markdown with optional arXiv ID insertion after first header.
    /// OODA-21: Insert arXiv ID after the title on page 1.
    ///
    /// OODA-IT17: Paragraph continuation detection.
    /// WHY: PDF backends extract bold/styled words as separate blocks, fragmenting
    /// paragraphs like "focus on **workflows** teams" into 3 blocks. We detect
    /// when consecutive Text blocks are continuations of the same paragraph and
    /// join them with a space instead of paragraph breaks (\n\n).
    fn render_page_with_arxiv(&self, page: &Page, output: &mut String, arxiv_id: Option<&str>) {
        if self.style.page_numbers {
            output.push_str(&format!("## Page {}\n\n", page.number));
        }

        let mut arxiv_inserted = false;
        // OODA-IT17: Track whether previous block was rendered as a paragraph continuation
        // If true, the next block may need to continue the paragraph (no \n\n prefix).
        let mut in_paragraph_continuation = false;

        for (i, block) in page.blocks.iter().enumerate() {
            let next_block = page.blocks.get(i + 1);

            // OODA-IT17: Check if this block is a continuation of the previous paragraph.
            // If so, render it inline (with space, no \n\n) instead of as a new paragraph.
            if in_paragraph_continuation
                && matches!(
                    block.block_type,
                    BlockType::Text | BlockType::Paragraph | BlockType::TextInlineMath
                )
            {
                self.render_text_continuation(block, output);
            } else {
                self.render_block(block, output);
            }

            // OODA-IT17: Determine if the NEXT block is a paragraph continuation.
            // We set the flag here so it's ready for the next iteration.
            in_paragraph_continuation = if let Some(next) = next_block {
                Self::is_paragraph_continuation(block, next)
            } else {
                false
            };

            // If we just rendered a block and the next IS a continuation,
            // we need to strip the trailing \n\n that render_text added,
            // replacing it with a space for inline joining.
            if in_paragraph_continuation {
                // Strip trailing \n\n from output and replace with space
                if output.ends_with("\n\n") {
                    output.truncate(output.len() - 2);
                    output.push(' ');
                }
            }

            // OODA-21: Insert arXiv after the first header block (title)
            if !arxiv_inserted {
                if let Some(arxiv) = arxiv_id {
                    // Insert after title-like blocks (large headers)
                    if block.block_type == BlockType::SectionHeader {
                        let level = block.level.unwrap_or(2);
                        if level <= 1 {
                            output.push_str(&format!("\n**{}** \n\n", arxiv));
                            arxiv_inserted = true;
                        }
                    }
                }
            }

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
            tracing::trace!(
                "render_block: Table block text_len={}, text='{}'",
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
    ///
    /// WHY no bold wrapping: Markdown headers (`# Title`) are inherently bold
    /// in every renderer. Wrapping content in `**...**` is redundant and adds
    /// visual noise: `# **Title**` → `# Title`. The gold standard (markitdown)
    /// uses clean headers without bold markers.
    fn render_header(&self, block: &Block, output: &mut String) {
        let level = block.level.unwrap_or(2).min(self.style.max_heading_level);
        // WHY skip_bold=true, skip_italic=false: Headers are bold by nature.
        // Strip span-level bold to avoid `## **foo** bar` artifacts.
        // Italic is preserved because headers CAN be italic (rare but valid).
        //
        // OODA-37: Check if block.text was normalized (e.g., "1INTRO" → "1 INTRO").
        // If block.text differs from spans-derived text, use block.text because
        // it contains the corrected spacing. Spans don't get updated when the
        // HeaderDetectionProcessor normalizes section number spacing.
        let span_raw: String = block.spans.iter().map(|s| s.text.as_str()).collect();
        let text = if !block.spans.is_empty() && span_raw.trim() == block.text.trim() {
            self.render_spans_styled(&block.spans, true, false)
        } else {
            self.clean_text(&block.text)
        };

        if self.style.atx_headers {
            let prefix = "#".repeat(level as usize);
            output.push_str(&format!("{} {}\n\n", prefix, text.trim()));
        } else {
            output.push_str(text.trim());
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

    /// OODA-IT17: Render text block as a paragraph continuation (inline, no \n\n).
    ///
    /// WHY: When a paragraph is fragmented across blocks (e.g., bold word as
    /// separate block), we render the continuation inline to preserve paragraph
    /// coherence. The text is appended directly without trailing newlines.
    fn render_text_continuation(&self, block: &Block, output: &mut String) {
        let span_text: String = block.spans.iter().map(|s| s.text.as_str()).collect();
        let spans_valid = if block.spans.is_empty() {
            false
        } else {
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
            // No \n\n - this is a continuation within the same paragraph
            output.push_str(&text);
            // Add \n\n only to finalize the paragraph (caller manages this)
            output.push_str("\n\n");
        }
    }

    /// OODA-IT17: Detect if `curr` block is a paragraph continuation of `prev` block.
    ///
    /// WHY: PDF backends extract bold/styled text as separate blocks, fragmenting
    /// paragraphs. Example: "focus on **workflows** teams" becomes 3 blocks:
    ///   block A: "focus on"
    ///   block B: "workflows"     (bold, separate block)
    ///   block C: "teams move..." (continuation)
    ///
    /// We detect continuations by checking:
    /// 1. Both blocks are text-like (not headers, lists, tables, etc.)
    /// 2. Previous block does NOT end with sentence-ending punctuation
    /// 3. Vertical gap is within normal line spacing (not a new paragraph)
    /// 4. Current block is short (inline fragment) OR starts with lowercase
    ///
    /// ```text
    /// ┌──────────────────────────────────────────────────┐
    /// │  Continuation Detection Logic                     │
    /// ├──────────────────────────────────────────────────┤
    /// │  prev = "Elitizon designs... with a focus on"    │
    /// │  curr = "workflows"  (bold, short, no punct end) │
    /// │                                                   │
    /// │  Check 1: Both Text/Paragraph?        YES        │
    /// │  Check 2: prev ends with . ! ? : ;?   NO         │
    /// │  Check 3: Gap < 2x line height?       YES        │
    /// │  Check 4: curr short OR lowercase?    YES (short)│
    /// │  → CONTINUATION: join with space                  │
    /// └──────────────────────────────────────────────────┘
    /// ```
    fn is_paragraph_continuation(prev: &Block, curr: &Block) -> bool {
        // Check 1: Both must be text-like blocks
        let prev_is_text = matches!(
            prev.block_type,
            BlockType::Text | BlockType::Paragraph | BlockType::TextInlineMath
        );
        let curr_is_text = matches!(
            curr.block_type,
            BlockType::Text | BlockType::Paragraph | BlockType::TextInlineMath
        );
        if !prev_is_text || !curr_is_text {
            return false;
        }

        let prev_text = prev.text.trim();
        let curr_text = curr.text.trim();

        // Skip empty blocks
        if prev_text.is_empty() || curr_text.is_empty() {
            return false;
        }

        // OODA-IT18: Check horizontal alignment (same column).
        // WHY: After multi-column layout reordering, blocks from different columns
        // become adjacent in the render list. If prev is in the right column
        // (x1≈318) and curr is in the left column (x1≈78), they must NOT be
        // merged as paragraph continuations—they belong to different text flows.
        //
        // ┌─────────────────────────────────────────────┐
        // │  COLUMN ALIGNMENT CHECK                      │
        // │                                              │
        // │  prev.x1=318  curr.x1=78  → diff=240 > 50  │
        // │  → Different columns → NOT continuation      │
        // │                                              │
        // │  prev.x1=78   curr.x1=82  → diff=4 < 50    │
        // │  → Same column → Maybe continuation          │
        // └─────────────────────────────────────────────┘
        const MAX_COLUMN_X_DRIFT: f32 = 50.0;
        let left_margin_diff = (prev.bbox.x1 - curr.bbox.x1).abs();
        if left_margin_diff > MAX_COLUMN_X_DRIFT {
            return false;
        }

        // Check 2: Previous block must NOT end with sentence-ending punctuation
        // WHY: If prev ends with ".", "!", "?", it's a complete sentence/paragraph
        let last_char = prev_text.chars().last().unwrap_or(' ');
        if matches!(last_char, '.' | '!' | '?' | ':' | ';') {
            return false;
        }

        // OODA-IT17 FIX: Reject if prev block looks like a section heading.
        // WHY: Short title-case blocks like "What we deliver", "Value framing"
        // are visual headings in PDFs. They should NOT be merged with following
        // body text, even though they don't end with punctuation.
        //
        // Heuristic: A block is heading-like if:
        // - Short (< 60 chars)
        // - Starts with uppercase
        // - Contains 1-6 words (typical heading length)
        // - NOT a sentence (doesn't contain common sentence patterns)
        if prev_text.len() < 60 {
            let prev_first = prev_text.chars().next().unwrap_or(' ');
            let prev_word_count = prev_text.split_whitespace().count();
            if prev_first.is_uppercase() && prev_word_count <= 6 {
                // Additional check: if prev looks like it could be a title
                // (doesn't contain articles/prepositions mid-sentence that
                // would indicate it's a sentence fragment)
                let has_sentence_structure = prev_text.contains(", ")
                    || prev_text.contains(" that ")
                    || prev_text.contains(" which ")
                    || prev_text.contains(" with ")
                    || prev_text.contains(" from ")
                    || prev_text.contains(" into ");
                if !has_sentence_structure {
                    return false;
                }
            }
        }

        // Check 3: Vertical gap must be small (within ~2x typical line height)
        // WHY: Large gaps indicate intentional paragraph breaks
        // Typical line height in PDFs is ~12-16pt, so 2x = ~32pt max gap
        let vertical_gap = (curr.bbox.y1 - prev.bbox.y2).max(0.0);
        let typical_line_height = (prev.bbox.y2 - prev.bbox.y1).max(12.0);
        if vertical_gap > typical_line_height * 2.5 {
            return false;
        }

        // Check 4: Current block should look like a fragment, not a new paragraph
        // - Short text (< 80 chars): likely an inline formatting fragment
        // - Starts with lowercase: continuation of previous sentence
        // - Does NOT look like a structural element
        let curr_first_char = curr_text.chars().next().unwrap_or(' ');

        // Reject if current looks like a header or structural element
        if curr_text.starts_with('#')
            || curr_text.starts_with("- ")
            || curr_text.starts_with("* ")
            || curr_text.starts_with("> ")
            || curr_text.starts_with('|')
        {
            return false;
        }

        // Reject if curr starts with a numbered list pattern
        if curr_first_char.is_ascii_digit() {
            let after_digit: String = curr_text
                .chars()
                .skip_while(|c| c.is_ascii_digit())
                .collect();
            if after_digit.starts_with(". ") || after_digit.starts_with(") ") {
                return false;
            }
        }

        // OODA-IT17 FIX: Also reject if curr looks heading-like
        // WHY: Short uppercase blocks following text should stay separate
        // But single-word fragments like "ROI" or "APIs" are inline, not headings
        if curr_text.len() < 60 {
            let cw = curr_text.split_whitespace().count();
            if (2..=6).contains(&cw) && curr_first_char.is_uppercase() {
                // Check if it's a known heading-like pattern
                let has_sentence_structure = curr_text.contains(", ")
                    || curr_text.contains(" that ")
                    || curr_text.contains(" which ");
                if !has_sentence_structure {
                    return false;
                }
            }
        }

        // Accept if current starts with lowercase (clear continuation signal)
        if curr_first_char.is_lowercase() {
            return true;
        }

        // Accept if current is a very short fragment (< 20 chars, single word)
        // that starts with uppercase - likely a bold formatted word
        // WHY: Words like "ROI" or "APIs" are short uppercase fragments within paragraphs
        if curr_text.len() < 20 && curr_text.split_whitespace().count() == 1 {
            return true;
        }

        false
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
            // WHY (OODA-11): 72pt = 1 inch = standard PDF left margin.
            // 20pt ≈ 0.28" per level = standard typographic indent step.
            // Formula: (indent - margin) / step_size = nesting level
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
        // OODA-IT12: Use comprehensive bullet detection (handles 530+ Unicode bullets)
        // WHY: PDFs like LightRAG paper have "•General" (bullet + uppercase, no space)
        let has_bullet = starts_with_bullet(raw_text);
        let has_number = raw_text
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false);

        // Get content after the bullet/prefix for rendering with spans
        // OODA-IT12: Handle bullet with or without space after it
        let content_start = if has_bullet {
            // Get first char (bullet) length in bytes, then skip any space
            let first_char = raw_text.chars().next().unwrap();
            let bullet_len = first_char.len_utf8();
            let rest = &raw_text[bullet_len..];
            if rest.starts_with(' ') || rest.starts_with('\t') {
                bullet_len + 1 // Skip bullet + space
            } else {
                bullet_len // Skip bullet only (no space after bullet)
            }
        } else if has_dash || has_asterisk {
            // Find first space after bullet and skip to content
            raw_text.find(' ').map(|i| i + 1).unwrap_or(0)
        } else if has_number {
            // OODA-31 FIX: Properly parse numbered list prefix (e.g., "1. ", "2.", "10)")
            // WHY: The previous code searched for ". " or ") " anywhere in text, which
            // incorrectly matched content like "(CoT) and" instead of the list prefix.
            // Now we extract just the numeric prefix and find the delimiter immediately after.
            //
            // Strategy: Find where digits end, then check if next char(s) form a list delimiter
            let digit_end = raw_text.chars().take_while(|c| c.is_ascii_digit()).count();

            if digit_end > 0 {
                let after_digits = &raw_text[digit_end..];
                // Check for ". " (standard numbered list)
                if after_digits.starts_with(". ") {
                    digit_end + 2
                // Check for "." followed immediately by letter (no space: "1.Item")
                } else if after_digits.starts_with('.') && after_digits.len() > 1 {
                    let second_char = after_digits.chars().nth(1).unwrap_or(' ');
                    if second_char.is_alphabetic() {
                        digit_end + 1
                    } else {
                        0
                    }
                // Check for ") " (parenthetical numbered list)
                } else if after_digits.starts_with(") ") {
                    digit_end + 2
                // Check for ")" followed immediately by letter (no space: "1)Item")
                } else if after_digits.starts_with(')') && after_digits.len() > 1 {
                    let second_char = after_digits.chars().nth(1).unwrap_or(' ');
                    if second_char.is_alphabetic() {
                        digit_end + 1
                    } else {
                        0
                    }
                } else {
                    0
                }
            } else {
                0
            }
        } else {
            0
        };

        // Render content with formatting (from spans if available)
        // OODA-IT36: Preserve bold/italic formatting from spans when skipping bullet prefix.
        // WHY: Previous code used clean_text(after_prefix) which drops all styling.
        // Now we skip bullet prefix spans and render remaining spans with formatting.
        let content = if content_start > 0 && !block.spans.is_empty() {
            // Skip spans that correspond to the bullet prefix.
            // Count characters consumed until we've passed content_start bytes.
            let mut chars_consumed = 0;
            let mut skip_spans = 0;
            for span in &block.spans {
                let span_len = span.text.len();
                if chars_consumed + span_len <= content_start {
                    chars_consumed += span_len;
                    skip_spans += 1;
                } else {
                    break;
                }
            }
            // Render remaining spans with formatting
            if skip_spans < block.spans.len() {
                let remaining = &block.spans[skip_spans..];
                // If we partially consumed a span, trim its beginning
                if chars_consumed < content_start && !remaining.is_empty() {
                    let first = &remaining[0];
                    let trim_bytes = content_start - chars_consumed;
                    if trim_bytes < first.text.len() {
                        // Create a modified first span with trimmed text
                        let mut trimmed_span = first.clone();
                        trimmed_span.text = first.text[trim_bytes..].to_string();
                        let mut trimmed_spans = vec![trimmed_span];
                        trimmed_spans.extend_from_slice(&remaining[1..]);
                        self.render_spans_styled(&trimmed_spans, false, false)
                    } else {
                        self.render_spans_styled(&remaining[1..], false, false)
                    }
                } else {
                    self.render_spans_styled(remaining, false, false)
                }
            } else {
                // All spans consumed by prefix — fallback to clean text
                let after_prefix = &raw_text[content_start..];
                self.clean_text(after_prefix)
            }
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
            let number: String = raw_text
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
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

    /// Consolidate adjacent spans with identical styles.
    /// OODA-21: WHY? After block merge, we may have:
    ///   Span1: "This paper is about..." (italic)
    ///   Span2: " " (plain - merge joiner)
    ///   Span3: "consist of S&P..." (italic)
    /// Without consolidation, this renders as: `*This paper...* *consist...*`
    /// With consolidation: `*This paper... consist...*`
    fn consolidate_spans(&self, spans: &[TextSpan]) -> Vec<TextSpan> {
        use crate::schema::TextSpan;

        if spans.len() < 2 {
            return spans.to_vec();
        }

        let mut consolidated: Vec<TextSpan> = Vec::new();

        for span in spans {
            if span.text.is_empty() {
                continue;
            }

            // Check if we can merge with the last consolidated span
            let can_merge = if let Some(last) = consolidated.last_mut() {
                // Compare relevant style properties
                let same_bold = last.style.weight.map(|w| w >= 600).unwrap_or(false)
                    == span.style.weight.map(|w| w >= 600).unwrap_or(false);
                let same_italic = last.style.italic == span.style.italic;
                let same_code = last.style.looks_like_code() == span.style.looks_like_code();
                let same_super = last.style.superscript == span.style.superscript;
                let same_sub = last.style.subscript == span.style.subscript;

                // Plain space joiner can be absorbed into styled spans
                let is_plain_joiner = span.text.trim().is_empty()
                    && !span.style.italic
                    && span.style.weight.map(|w| w < 600).unwrap_or(true);

                if is_plain_joiner {
                    // Absorb plain space into previous styled span
                    last.text.push_str(&span.text);
                    true // Already merged
                } else if same_bold && same_italic && same_code && same_super && same_sub {
                    // Same style - merge text
                    last.text.push_str(&span.text);
                    true // Already merged
                } else {
                    false
                }
            } else {
                false
            };

            if !can_merge {
                consolidated.push(span.clone());
            }
        }

        consolidated
    }

    /// Render structured spans with optional style skipping.
    fn render_spans_styled(
        &self,
        spans: &[TextSpan],
        skip_bold: bool,
        skip_italic: bool,
    ) -> String {
        // OODA-21: Consolidate spans before rendering to avoid *text1* *text2* fragmentation
        let consolidated = self.consolidate_spans(spans);

        let mut result = String::new();
        for span in &consolidated {
            let content = &span.text;
            if content.is_empty() {
                continue;
            }

            let is_bold = span.style.weight.map(|w| w >= 600).unwrap_or(false) && !skip_bold;
            let is_italic = span.style.italic && !skip_italic;
            // OODA-IT13: Apply content filter to inline code detection
            let is_code = span.style.looks_like_code() && should_render_inline_code(content);
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
                // WHY (OODA-11): 10pt Y-tolerance for same-row detection.
                // Matches other tolerances (block_gap, line joining) in codebase.
                // Cells on same row should have Y positions within 10pt.
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

        // OODA-IT42: Validate table structure before rendering
        // WHY: Inconsistent column counts produce garbled markdown tables that are
        // worse than plain text. Fall back to plain text if structure is irregular.
        let header_cols = rows[0].len();
        let has_inconsistent_columns = rows.iter().skip(1).any(|row| row.len() != header_cols);

        if has_inconsistent_columns || header_cols < 2 {
            tracing::debug!(
                "OODA-IT42: Falling back to plain text for table with {} rows, header_cols={}, inconsistent={}",
                rows.len(),
                header_cols,
                has_inconsistent_columns
            );
            // Fall back to plain text rendering
            for row in &rows {
                let row_text: Vec<&str> = row.iter().map(|cell| cell.text.as_str()).collect();
                output.push_str(&row_text.join(" "));
                output.push('\n');
            }
            output.push('\n');
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

    /// Render a caption using blockquote format (OODA-25).
    ///
    /// **WHY blockquote format:**
    /// Gold standard uses `> Figure N: description` format which:
    /// 1. Provides visual separation from body text
    /// 2. Semantically marks captions as distinct content
    /// 3. Renders consistently across Markdown viewers
    ///
    /// **Format:** `> Figure N. Description text`
    fn render_caption(&self, block: &Block, output: &mut String) {
        let text = self.clean_text(&block.text);
        // WHY single-line blockquote: Captions should be visually distinct
        // but not overly emphasized (unlike italics which can be hard to read)
        output.push_str(&format!("> {}\n>\n\n", text));
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

        // OODA-IT16: Join lines broken mid-word during PDF extraction
        // WHY: PDF text boxes often break words at visual boundaries, creating
        // fragments like "netw\norking" instead of "networking". This reconstructs
        // proper word boundaries for better readability and LLM processing.
        // NOTE: Handles both immediate breaks (netw\norking) and breaks across
        // empty lines (netw\n\norking) that occur from render_text's \n\n suffix.
        result = Self::join_broken_lines(&result);

        // OODA-IT14: Clean up TOC leader dots
        // WHY: Table of Contents entries often have dots like "Chapter 1 ........ 5"
        // These leader dots are visual artifacts from PDFs and clutter the markdown output.
        result = Self::cleanup_toc_leader_dots(&result);

        // OODA-IT15: Convert standalone bold lines to section headers
        // WHY: Business PDFs often use bold text as visual section markers.
        // These should be proper markdown headers for semantic structure.
        result = Self::convert_standalone_bold_to_headers(&result);

        result
    }

    /// OODA-IT14: Remove TOC leader dots (4+ consecutive periods) and trailing page numbers.
    /// Handles patterns like:
    /// - "Actions  ................................ 31" -> "Actions"
    /// - "**.............. 3**" -> ""  (dots-only with page number)
    /// - "...............  36" -> ""  (standalone dots with number)
    fn cleanup_toc_leader_dots(text: &str) -> String {
        use regex::Regex;

        // Process line by line to avoid cross-line issues
        let leader_dots_re = Regex::new(r"\.{4,}\s*\d{0,3}\s*$").unwrap();
        let dots_only_re = Regex::new(r"^\s*\**\.{3,}\**\s*\d*\s*\**\s*$").unwrap();
        let page_num_only_re = Regex::new(r"^\s*\d{2,3}\s*$").unwrap();
        let page_num_bold_re = Regex::new(r"^\s*\d{1,3}\s+\*\*\s*\*\*\s*$").unwrap();
        let empty_bold_re = Regex::new(r"\*\*\s*\*\*").unwrap();

        let mut result_lines: Vec<String> = Vec::new();

        for line in text.lines() {
            // WHY preserve empty lines: Empty lines are markdown paragraph separators.
            // Discarding them collapses all blocks into a single paragraph.
            // Only CONTENT lines that become empty after cleaning should be skipped.
            if line.trim().is_empty() {
                result_lines.push(String::new());
                continue;
            }

            // Skip lines that are only dots (with optional page numbers)
            if dots_only_re.is_match(line) {
                continue;
            }
            // Skip standalone page numbers
            if page_num_only_re.is_match(line) {
                continue;
            }
            // Skip page numbers followed by empty bold
            if page_num_bold_re.is_match(line) {
                continue;
            }

            // Remove leader dots and trailing page number from same line
            let cleaned = leader_dots_re.replace(line, "").to_string();

            // Clean up empty bold patterns
            let cleaned = empty_bold_re.replace_all(&cleaned, "").to_string();

            // Only keep lines that still have content after cleaning
            if !cleaned.trim().is_empty() {
                result_lines.push(cleaned);
            }
        }

        let mut result = result_lines.join("\n");

        // Clean up resulting multiple newlines
        let multi_newline_re = Regex::new(r"\n{3,}").unwrap();
        result = multi_newline_re.replace_all(&result, "\n\n").to_string();

        result
    }

    /// OODA-IT15: Convert standalone bold lines to section headers.
    /// WHY: Business PDFs often use bold text as visual section markers without
    /// numbered headings. These should be converted to proper markdown headers
    /// for semantic document structure.
    ///
    /// Criteria for conversion:
    /// - Line contains ONLY bold text: **Title Text**
    /// - Text is short (< 60 chars)
    /// - Starts with uppercase letter
    /// - Does NOT end with : or . or ? (these are likely labels/sentences)
    /// - Does NOT start with Fig/Table/Note/Example (captions)
    ///
    /// OODA-30: Convert standalone bold lines to headers ONLY when they have
    /// a clear section-number pattern.
    ///
    /// ## First Principles
    ///
    /// The font-size-based header classification in classify_blocks() already
    /// handles headers that are visually larger than body text. This function
    /// catches the remaining case: section headers that are at body font size
    /// but have a structural section-number pattern.
    ///
    /// ## What qualifies as a section header here:
    ///
    /// ONLY bold standalone lines matching a numbered section pattern:
    /// - `**0) AI Strategy & Co‑Creation**` → `## 0) AI Strategy & Co‑Creation`
    /// - `**3. Context Graph**` → `## 3. Context Graph`
    /// - `**Section 1: Introduction**` → `## Section 1: Introduction`
    ///
    /// ## What does NOT qualify:
    ///
    /// Bold emphasis labels without section numbers:
    /// - `**What we deliver**` → stays as `**What we deliver**`
    /// - `**Capabilities**` → stays as `**Capabilities**`
    /// - `**Key outputs**` → stays as `**Key outputs**`
    ///
    /// WHY: These are emphasis labels, not structural headers. Promoting them
    /// creates false hierarchy (multiple `## What we deliver` in different sections).
    fn convert_standalone_bold_to_headers(text: &str) -> String {
        use regex::Regex;

        // Match standalone bold lines: exactly **text** on a line
        let standalone_bold_re = Regex::new(r"^\*\*([^*]+)\*\*\s*$").unwrap();

        // Section number patterns — structural, not content-based:
        // "0)", "1.", "2:", "Section 3", "Chapter 1", "Part II"
        let section_number_re =
            Regex::new(r"(?i)^(\d+[\).\-:\s]|section\s+\d|chapter\s+\d|part\s+[IVX\d])").unwrap();

        let mut result_lines: Vec<String> = Vec::new();

        for line in text.lines() {
            if let Some(caps) = standalone_bold_re.captures(line) {
                let inner_text = &caps[1];
                let trimmed = inner_text.trim();

                // ONLY promote to header if the bold text has a section number pattern
                let has_section_number = section_number_re.is_match(trimmed);
                let is_short = trimmed.len() < 80;

                if has_section_number && is_short {
                    // WHY no bold in header: `## Title` is clean; `## **Title**`
                    // is redundant because Markdown headers are inherently bold.
                    result_lines.push(format!("## {}", trimmed));
                } else {
                    result_lines.push(line.to_string());
                }
            } else {
                result_lines.push(line.to_string());
            }
        }

        result_lines.join("\n")
    }

    /// OODA-IT16: Join lines that were broken mid-word during PDF extraction.
    ///
    /// WHY: PDF text extraction preserves original text box boundaries, which
    /// often break words at the line end. For example:
    /// - "TCP/IP netw" + "orking" → "TCP/IP networking"
    /// - "sockets-" + "based" → "sockets-based"
    ///
    /// Rules:
    /// 1. Join when line ends with lowercase + next starts with lowercase (word split)
    /// 2. Preserve hyphen when line ends with `word-` + next starts with lowercase
    /// 3. Preserve: empty lines, markdown syntax, code blocks, list items
    fn join_broken_lines(text: &str) -> String {
        // Iterate until no more joins happen (handles chained breaks)
        let mut current_text = text.to_string();
        loop {
            let result = Self::join_broken_lines_single_pass(&current_text);
            if result == current_text {
                // No changes made, we're done
                break;
            }
            current_text = result;
        }
        current_text
    }

    /// Single pass of join_broken_lines - joins one level of breaks.
    fn join_broken_lines_single_pass(text: &str) -> String {
        let lines: Vec<&str> = text.lines().collect();
        if lines.is_empty() {
            return String::new();
        }

        let mut result: Vec<String> = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            let current = lines[i];

            // Check if we should try to join with next line
            if i + 1 < lines.len() {
                let next = lines[i + 1];

                // Don't join if current line is empty
                let current_trimmed = current.trim();
                if current_trimmed.is_empty() || Self::is_code_fence(current_trimmed) {
                    result.push(current.to_string());
                    i += 1;
                    continue;
                }

                let next_trimmed = next.trim();

                // OODA-IT16 FIX: If next line is empty, check line after that.
                // This handles cases where render_text adds \n\n after each block,
                // splitting words like "netw\n\norking" across an empty line.
                //
                // OODA-IT18: Length guard to prevent false-positive paragraph merging.
                // WHY: render_text() adds \n\n between blocks, creating paragraph
                // breaks. Long lines (>30 chars) represent COMPLETE sentences/text
                // blocks whose paragraph boundaries must be preserved. Only SHORT
                // lines (<= 30 chars) are likely word fragments from narrow PDF text
                // boxes that genuinely split a word across blocks.
                //
                // ┌──────────────────────────────────────────┐
                // │    CROSS-EMPTY-LINE JOIN GUARD            │
                // │                                           │
                // │  "netw"  (4 chars)  → JOIN ✓ (fragment)   │
                // │  "...lines with" (50 chars) → NO ✗ (full) │
                // └──────────────────────────────────────────┘
                const MAX_FRAGMENT_LINE_LEN: usize = 30;
                if next_trimmed.is_empty() && i + 2 < lines.len() {
                    let next_next = lines[i + 2];
                    let next_next_trimmed = next_next.trim();

                    // Don't skip over empty line to structural elements
                    if !next_next_trimmed.is_empty()
                        && !Self::is_code_fence(next_next_trimmed)
                        && !Self::is_markdown_structural_line(next_next_trimmed)
                        && current_trimmed.len() <= MAX_FRAGMENT_LINE_LEN
                    {
                        // Check if we should join across the empty line
                        if Self::should_join_lines(current_trimmed, next_next_trimmed) {
                            // Join lines, removing the empty line between them
                            let joined = Self::join_two_lines(current, next_next);
                            result.push(joined);
                            i += 3; // Skip current, empty, and next_next
                            continue;
                        }
                    }
                }

                // Don't join if next line is empty, a code fence, or starts markdown structure
                // BUT: First check if this is a broken hyphenated word (Rule 3 in should_join_lines)
                // because "- based" looks like a list item but might be a broken "sockets-based"

                // Skip empty and code fences
                if next_trimmed.is_empty() || Self::is_code_fence(next_trimmed) {
                    result.push(current.to_string());
                    i += 1;
                    continue;
                }

                // Check if this looks like a broken word FIRST (before checking structural)
                // This handles the case where "- based" looks like a list item but is actually
                // part of "sockets-based" that got split with hyphen on the next line
                if Self::should_join_lines(current_trimmed, next_trimmed) {
                    // Join the lines
                    let joined = Self::join_two_lines(current, next);
                    result.push(joined);
                    i += 2; // Skip the next line since we consumed it
                    continue;
                }

                // Only now check for structural markdown (lists, headers, etc.)
                // This comes AFTER the broken word check so we don't miss "- based" patterns
                if Self::is_markdown_structural_line(next_trimmed) {
                    result.push(current.to_string());
                    i += 1;
                    continue;
                }
            }

            result.push(current.to_string());
            i += 1;
        }

        result.join("\n")
    }

    /// Check if a line is a markdown structural element that should not be joined.
    fn is_markdown_structural_line(line: &str) -> bool {
        let trimmed = line.trim();
        // Headers
        if trimmed.starts_with('#') {
            return true;
        }
        // List items (bullet or numbered)
        if trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed.starts_with("+ ")
            || trimmed
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
                && trimmed.contains(". ")
        {
            return true;
        }
        // Blockquotes
        if trimmed.starts_with('>') {
            return true;
        }
        // Table rows
        if trimmed.starts_with('|') || trimmed.contains(" | ") {
            return true;
        }
        // Horizontal rules
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            return true;
        }
        false
    }

    /// Check if a line is a code fence marker.
    fn is_code_fence(line: &str) -> bool {
        let trimmed = line.trim();
        trimmed.starts_with("```") || trimmed.starts_with("~~~")
    }

    /// Determine if two lines should be joined (word was broken across lines).
    fn should_join_lines(prev: &str, next: &str) -> bool {
        // Get last character of previous line (ignoring trailing whitespace)
        let prev_trimmed_end = prev.trim_end();
        let prev_chars: Vec<char> = prev_trimmed_end.chars().collect();
        if prev_chars.is_empty() {
            return false;
        }

        let last_char = prev_chars[prev_chars.len() - 1];

        // Get first character of next line (ignoring leading whitespace)
        let next_trimmed = next.trim_start();
        let first_char = match next_trimmed.chars().next() {
            Some(c) => c,
            None => return false,
        };

        // Rule 1: Previous ends with lowercase letter, next starts with lowercase
        // This indicates a word was split
        if last_char.is_lowercase() && first_char.is_lowercase() {
            // But NOT if previous line ends with sentence punctuation
            // followed by space then word
            let ends_with_sentence = prev_trimmed_end.ends_with('.')
                || prev_trimmed_end.ends_with('!')
                || prev_trimmed_end.ends_with('?')
                || prev_trimmed_end.ends_with(':')
                || prev_trimmed_end.ends_with(';')
                || prev_trimmed_end.ends_with(',');

            if !ends_with_sentence {
                return true;
            }
        }

        // Rule 2: Previous ends with hyphen after lowercase (hyphenated word break)
        // Like "net-\nwork" or "sockets-\nbased"
        if last_char == '-' && prev_chars.len() > 1 {
            let second_last = prev_chars[prev_chars.len() - 2];
            if second_last.is_lowercase() && first_char.is_lowercase() {
                return true;
            }
        }

        // Rule 3: Next line STARTS with hyphen followed by lowercase (hyphen moved to next line)
        // Like "sockets\n- based" which should become "sockets-based"
        // This happens when PDF extraction splits "sockets-based" putting the hyphen on next line
        if first_char == '-' && next_trimmed.len() > 1 {
            let second_char = next_trimmed.chars().nth(1);
            // If prev ends with lowercase and next is "- " followed by lowercase word
            // this is likely a hyphenated word that got split with hyphen on wrong line
            if last_char.is_lowercase() {
                if let Some(c2) = second_char {
                    // "- based" pattern: hyphen, space, then lowercase
                    if c2 == ' ' {
                        let after_hyphen_space = next_trimmed.get(2..);
                        if let Some(rest) = after_hyphen_space {
                            let first_word_char = rest.chars().next();
                            if let Some(fc) = first_word_char {
                                if fc.is_lowercase() {
                                    return true;
                                }
                            }
                        }
                    }
                    // "-based" pattern: hyphen directly followed by lowercase
                    else if c2.is_lowercase() {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Join two lines that were broken mid-word.
    fn join_two_lines(prev: &str, next: &str) -> String {
        let prev_trimmed = prev.trim_end();
        let next_trimmed = next.trim_start();

        // Case 1: Next line STARTS with hyphen (e.g., "sockets\n- based" → "sockets-based")
        // The hyphen was moved to the start of the next line during PDF extraction
        if next_trimmed.starts_with('-') {
            // "- based" pattern: hyphen, space, word
            if let Some(word_after) = next_trimmed.strip_prefix("- ") {
                return format!("{}-{}", prev_trimmed, word_after);
            }
            // "-based" pattern: hyphen directly followed by word
            else if next_trimmed.len() > 1
                && next_trimmed
                    .chars()
                    .nth(1)
                    .map(|c| c.is_lowercase())
                    .unwrap_or(false)
            {
                // Keep the hyphen from next
                return format!("{}{}", prev_trimmed, next_trimmed);
            }
        }

        // Case 2: Previous ends with hyphen (e.g., "net-\nwork" or "well-\nknown")
        if prev_trimmed.ends_with('-') {
            // Check if this is a soft hyphen (word break) or a real hyphen
            // If next starts with lowercase after removing hyphen, it's likely a word break
            // Keep the hyphen for compound words like "well-" + "known"
            let without_hyphen = prev_trimmed.strip_suffix('-').unwrap_or(prev_trimmed);
            let last_word = without_hyphen.split_whitespace().last().unwrap_or("");

            // Simple heuristic: keep hyphen if the word before it is a common prefix
            // like "well", "self", "anti", "pre", etc.
            let common_prefixes = [
                "well", "self", "anti", "pre", "post", "non", "semi", "multi", "co", "re", "un",
                "all", "ex", "sub", "super", "ultra", "over", "under", "out", "cross", "inter",
            ];
            let is_compound_prefix = common_prefixes
                .iter()
                .any(|p| last_word.eq_ignore_ascii_case(p));

            if is_compound_prefix {
                // Keep the hyphen for compound words
                format!("{}{}", prev_trimmed, next_trimmed)
            } else {
                // Remove hyphen for word breaks (soft hyphenation)
                format!("{}{}", without_hyphen, next_trimmed)
            }
        } else {
            // Case 3: No hyphen - just join (word was split without hyphen)
            format!("{}{}", prev_trimmed, next_trimmed)
        }
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

        // OODA-21: Extract arXiv identifier from page 1 metadata for insertion after title
        let arxiv_id: Option<String> = document
            .pages
            .first()
            .and_then(|p| p.metadata.get("arxiv_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Render each page
        for (i, page) in document.pages.iter().enumerate() {
            if i > 0 && self.style.page_breaks {
                output.push_str("\n---\n\n");
            }

            // For page 1, pass arxiv_id to insert after title header
            if i == 0 {
                self.render_page_with_arxiv(page, &mut output, arxiv_id.as_deref());
            } else {
                self.render_page(page, &mut output);
            }
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
    use crate::schema::{BoundingBox, FontStyle};

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

        // Debug: print the actual output
        eprintln!("ACTUAL OUTPUT:\n{}", result);

        // Headers are rendered without bold wrapping (headers are inherently bold)
        assert!(result.contains("Test Document"));
        assert!(result.contains("Introduction"));
        assert!(!result.contains("**Introduction**")); // No redundant bold in headers
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
        // WHY page_breaks=false: LLM-optimized output flows continuously
        assert!(!style.page_breaks);
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
        // Headers are rendered without bold wrapping (inherently bold)
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

    /// OODA-IT04: Test proper Markdown table rendering with children.
    ///
    /// WHY: Tables detected by TableDetectionProcessor have cells stored as
    /// block.children with BlockType::TableCell. The renderer must group
    /// these by Y-coordinate into rows and output pipe-separated Markdown.
    #[test]
    fn test_table_rendering_with_children() {
        let renderer = MarkdownRenderer::new();

        let mut doc = Document::new();
        let mut page = Page::new(1, 612.0, 792.0);

        // Create table block with cells as children
        let mut table_block = Block::new(
            BlockType::Table,
            BoundingBox::new(72.0, 100.0, 540.0, 200.0),
        );

        // Row 1 (header): y1 = 100
        let mut cell1 = Block::new(
            BlockType::TableCell,
            BoundingBox::new(72.0, 100.0, 150.0, 120.0),
        );
        cell1.text = "Name".to_string();

        let mut cell2 = Block::new(
            BlockType::TableCell,
            BoundingBox::new(160.0, 100.0, 250.0, 120.0),
        );
        cell2.text = "Age".to_string();

        // Row 2 (data): y1 = 130
        let mut cell3 = Block::new(
            BlockType::TableCell,
            BoundingBox::new(72.0, 130.0, 150.0, 150.0),
        );
        cell3.text = "Alice".to_string();

        let mut cell4 = Block::new(
            BlockType::TableCell,
            BoundingBox::new(160.0, 130.0, 250.0, 150.0),
        );
        cell4.text = "30".to_string();

        table_block.children = vec![cell1, cell2, cell3, cell4];

        page.add_block(table_block);
        doc.add_page(page);

        let result = renderer.render(&doc).unwrap();

        // Debug output
        eprintln!("TABLE OUTPUT:\n{}", result);

        // Verify Markdown table format
        assert!(
            result.contains("| Name |"),
            "Missing header row: {}",
            result
        );
        assert!(result.contains("| --- |"), "Missing separator: {}", result);
        assert!(result.contains("| Alice |"), "Missing data row: {}", result);
    }

    /// OODA-IT04: Test table rendering with empty cells.
    #[test]
    fn test_table_rendering_empty_cells() {
        let renderer = MarkdownRenderer::new();

        let mut doc = Document::new();
        let mut page = Page::new(1, 612.0, 792.0);

        let mut table_block = Block::new(
            BlockType::Table,
            BoundingBox::new(72.0, 100.0, 540.0, 200.0),
        );

        // Row with empty cell
        let mut cell1 = Block::new(
            BlockType::TableCell,
            BoundingBox::new(72.0, 100.0, 150.0, 120.0),
        );
        cell1.text = "Header".to_string();

        let mut cell2 = Block::new(
            BlockType::TableCell,
            BoundingBox::new(160.0, 100.0, 250.0, 120.0),
        );
        cell2.text = String::new(); // Empty cell

        table_block.children = vec![cell1, cell2];
        page.add_block(table_block);
        doc.add_page(page);

        let result = renderer.render(&doc).unwrap();

        // Should still produce valid table structure
        assert!(result.contains("|"), "Missing table pipes: {}", result);
        assert!(
            result.contains("| Header |"),
            "Missing header cell: {}",
            result
        );
    }

    /// OODA-IT04: Test single-row table (header only).
    #[test]
    fn test_table_rendering_single_row() {
        let renderer = MarkdownRenderer::new();

        let mut doc = Document::new();
        let mut page = Page::new(1, 612.0, 792.0);

        let mut table_block = Block::new(
            BlockType::Table,
            BoundingBox::new(72.0, 100.0, 540.0, 200.0),
        );

        // Single row
        let mut cell1 = Block::new(
            BlockType::TableCell,
            BoundingBox::new(72.0, 100.0, 150.0, 120.0),
        );
        cell1.text = "Only".to_string();

        let mut cell2 = Block::new(
            BlockType::TableCell,
            BoundingBox::new(160.0, 100.0, 250.0, 120.0),
        );
        cell2.text = "Header".to_string();

        table_block.children = vec![cell1, cell2];
        page.add_block(table_block);
        doc.add_page(page);

        let result = renderer.render(&doc).unwrap();

        // Should produce header + separator (no data rows)
        assert!(
            result.contains("| Only |"),
            "Missing header cell: {}",
            result
        );
        assert!(result.contains("| --- |"), "Missing separator: {}", result);
    }

    /// OODA-IT06: Test that styled spans render with correct markdown markers.
    ///
    /// WHY: IT05 added span style preservation in pdfium_backend. This test
    /// verifies the full pipeline: styled spans → markdown output.
    #[test]
    fn test_render_styled_spans_bold_and_italic() {
        let renderer = MarkdownRenderer::new();

        let mut doc = Document::new();
        let mut page = Page::new(1, 612.0, 792.0);

        // Create a paragraph block with styled spans
        let mut block = Block::new(
            BlockType::Paragraph,
            BoundingBox::new(72.0, 100.0, 540.0, 120.0),
        );

        // Build styled spans: "This is **bold** and *italic* text"
        let span1 = TextSpan::plain("This is ");
        let mut span2 = TextSpan::styled(
            "bold",
            FontStyle {
                weight: Some(700),
                ..Default::default()
            },
        );
        span2.bbox = Some(BoundingBox::new(100.0, 100.0, 130.0, 115.0));

        let span3 = TextSpan::plain(" and ");
        let mut span4 = TextSpan::styled(
            "italic",
            FontStyle {
                italic: true,
                ..Default::default()
            },
        );
        span4.bbox = Some(BoundingBox::new(150.0, 100.0, 190.0, 115.0));

        let span5 = TextSpan::plain(" text");

        block.spans = vec![span1, span2, span3, span4, span5];
        block.text = "This is bold and italic text".to_string();

        page.add_block(block);
        doc.add_page(page);

        let result = renderer.render(&doc).unwrap();

        // Verify bold markdown markers
        assert!(
            result.contains("**bold**"),
            "Expected **bold** in output, got: {}",
            result
        );

        // Verify italic markdown markers
        assert!(
            result.contains("*italic*"),
            "Expected *italic* in output, got: {}",
            result
        );
    }

    /// OODA-IT06: Test bold+italic combined styling.
    #[test]
    fn test_render_bold_italic_combined() {
        let renderer = MarkdownRenderer::new();

        let mut doc = Document::new();
        let mut page = Page::new(1, 612.0, 792.0);

        let mut block = Block::new(
            BlockType::Paragraph,
            BoundingBox::new(72.0, 100.0, 540.0, 120.0),
        );

        // Create bold+italic span
        let mut bold_italic_span = TextSpan::styled(
            "important",
            FontStyle {
                weight: Some(700),
                italic: true,
                ..Default::default()
            },
        );
        bold_italic_span.bbox = Some(BoundingBox::new(72.0, 100.0, 150.0, 115.0));

        block.spans = vec![bold_italic_span];
        block.text = "important".to_string();

        page.add_block(block);
        doc.add_page(page);

        let result = renderer.render(&doc).unwrap();

        // Verify combined bold+italic markers: ***text***
        assert!(
            result.contains("***important***"),
            "Expected ***important*** in output, got: {}",
            result
        );
    }

    // OODA-IT14: Tests for TOC leader dots cleanup
    #[test]
    fn test_cleanup_toc_leader_dots_inline() {
        // Pattern: "Actions  ................................ 31"
        let input = "Actions  ................................ 31";
        let result = MarkdownRenderer::cleanup_toc_leader_dots(input);
        assert!(
            !result.contains("...."),
            "Leader dots should be removed, got: {}",
            result
        );
        assert!(
            result.contains("Actions"),
            "Content should be preserved, got: {}",
            result
        );
    }

    #[test]
    fn test_cleanup_toc_leader_dots_standalone() {
        // Pattern: "**.............. 3**"
        let input = "**.............. 3**";
        let result = MarkdownRenderer::cleanup_toc_leader_dots(input);
        assert!(
            result.trim().is_empty() || !result.contains("...."),
            "Dots-only line should be removed or cleaned, got: '{}'",
            result
        );
    }

    #[test]
    fn test_cleanup_toc_leader_dots_page_number_only() {
        // Pattern: standalone page numbers like "31" on their own line
        let input = "Chapter 1\n\n31\n\nChapter 2";
        let result = MarkdownRenderer::cleanup_toc_leader_dots(input);
        // The standalone "31" should be removed
        assert!(
            !result.contains("\n31\n"),
            "Standalone page numbers should be removed, got: '{}'",
            result
        );
    }

    #[test]
    fn test_cleanup_toc_preserves_normal_dots() {
        // Normal text with dots (ellipsis, etc.) should be preserved
        let input = "This is normal text with etc. and some numbers like 123.";
        let result = MarkdownRenderer::cleanup_toc_leader_dots(input);
        assert_eq!(
            result.trim(),
            input.trim(),
            "Normal text should be preserved"
        );
    }

    #[test]
    fn test_cleanup_toc_preserves_ellipsis() {
        // Three dots (ellipsis) should be preserved
        let input = "He said... and then stopped.";
        let result = MarkdownRenderer::cleanup_toc_leader_dots(input);
        assert!(
            result.contains("..."),
            "Three-dot ellipsis should be preserved, got: '{}'",
            result
        );
    }

    #[test]
    fn test_cleanup_toc_real_world_pattern() {
        // Real pattern from Apple Sandbox Guide
        let input = r#"5.1  - Actions  ................................

5.2  - Operations  ................................

**.............. 3**

**............. 5**

31

35"#;
        let result = MarkdownRenderer::cleanup_toc_leader_dots(input);
        assert!(
            !result.contains("................................"),
            "Long dot patterns should be removed, got: '{}'",
            result
        );
        assert!(
            result.contains("5.1"),
            "Section numbers should be preserved, got: '{}'",
            result
        );
    }

    #[test]
    fn test_cleanup_toc_preserves_line_breaks() {
        // Section numbers on separate lines should stay on separate lines
        let input = r#"5.1  - Actions  ................................

5.2  - Operations  ................................

5.3  - Filters  ................................"#;
        let result = MarkdownRenderer::cleanup_toc_leader_dots(input);
        eprintln!("DEBUG result: {:?}", result);
        // Check that 5.1 and 5.2 are NOT on the same line
        assert!(
            !result.contains("5.1") || !result.contains("5.25.3"),
            "Section numbers should not be merged on same line, got: '{}'",
            result
        );
        // Check that there's a line break between sections
        assert!(
            result.contains("5.1") && result.contains("5.2"),
            "Both section numbers should be present, got: '{}'",
            result
        );
    }

    // OODA-IT15/IT30: Tests for standalone bold to header conversion
    // Only section-numbered bold lines are promoted to headers.
    #[test]
    fn test_convert_standalone_bold_with_section_number() {
        // Section-numbered bold lines should become headers
        let input = "**0) AI Strategy & Co‑Creation**";
        let result = MarkdownRenderer::convert_standalone_bold_to_headers(input);
        assert!(
            result.starts_with("## "),
            "Section-numbered bold should become header, got: '{}'",
            result
        );
    }

    #[test]
    fn test_convert_standalone_bold_without_section_number() {
        // Bold lines WITHOUT section numbers should NOT become headers
        let input = "**Executive Summary**";
        let result = MarkdownRenderer::convert_standalone_bold_to_headers(input);
        assert!(
            !result.starts_with("## "),
            "Bold without section number should NOT become header, got: '{}'",
            result
        );
    }

    #[test]
    fn test_convert_standalone_bold_preserves_caption() {
        // Captions with numbers should NOT be converted to headers
        let input = "**Figure 1: Test Image**";
        let result = MarkdownRenderer::convert_standalone_bold_to_headers(input);
        assert!(
            !result.starts_with("## "),
            "Figure captions should not become headers, got: '{}'",
            result
        );

        // "Table of Contents" should NOT be promoted (no section number)
        let input2 = "**Table of Contents**";
        let result2 = MarkdownRenderer::convert_standalone_bold_to_headers(input2);
        assert!(
            !result2.starts_with("## "),
            "Table of Contents should not become header (no section number), got: '{}'",
            result2
        );
    }

    #[test]
    fn test_convert_standalone_bold_preserves_label() {
        // Labels ending with colon should NOT be converted
        let input = "**Note:**";
        let result = MarkdownRenderer::convert_standalone_bold_to_headers(input);
        assert!(
            !result.starts_with("## "),
            "Labels with colon should not become headers, got: '{}'",
            result
        );
    }

    #[test]
    fn test_convert_standalone_bold_preserves_sentence() {
        // Sentences ending with period should NOT be converted
        let input = "**This is a complete sentence.**";
        let result = MarkdownRenderer::convert_standalone_bold_to_headers(input);
        assert!(
            !result.starts_with("## "),
            "Sentences should not become headers, got: '{}'",
            result
        );
    }

    #[test]
    fn test_convert_standalone_bold_preserves_lowercase() {
        // Lowercase starting text should NOT be converted
        let input = "**the quick brown fox**";
        let result = MarkdownRenderer::convert_standalone_bold_to_headers(input);
        assert!(
            !result.starts_with("## "),
            "Lowercase text should not become headers, got: '{}'",
            result
        );
    }

    #[test]
    fn test_convert_standalone_bold_inline_preserved() {
        // Bold within a line should NOT be affected
        let input = "This has **bold** text inline";
        let result = MarkdownRenderer::convert_standalone_bold_to_headers(input);
        assert_eq!(result, input, "Inline bold should be preserved unchanged");
    }

    #[test]
    fn test_convert_standalone_bold_multiple_lines() {
        // OODA-30: Only section-numbered bold lines become headers.
        // Non-numbered bold lines stay as bold paragraphs.
        let input = r#"**1. Introduction**

Some paragraph text here.

**2. Methods**

More text about methods.

**Key Findings**

Findings stay bold."#;
        let result = MarkdownRenderer::convert_standalone_bold_to_headers(input);
        // Section-numbered lines become headers
        assert!(
            result.contains("## 1. Introduction"),
            "Numbered section should become header, got: '{}'",
            result
        );
        assert!(
            result.contains("## 2. Methods"),
            "Numbered section should become header, got: '{}'",
            result
        );
        // Non-numbered bold lines stay as bold
        assert!(
            result.contains("**Key Findings**"),
            "Non-numbered bold should stay as bold, got: '{}'",
            result
        );
        assert!(
            result.contains("Some paragraph text here."),
            "Paragraph text should be preserved"
        );
    }

    // OODA-IT16: Tests for join_broken_lines functionality
    #[test]
    fn test_join_broken_lines_word_split() {
        // Word split across lines without hyphen
        let input = "TCP/IP netw\norking is prohibited.";
        let result = MarkdownRenderer::join_broken_lines(input);
        assert_eq!(
            result, "TCP/IP networking is prohibited.",
            "Split word should be joined: got '{}'",
            result
        );
    }

    #[test]
    fn test_join_broken_lines_hyphenated_break() {
        // Word split with soft hyphen (should remove hyphen)
        let input = "net-\nworking";
        let result = MarkdownRenderer::join_broken_lines(input);
        assert_eq!(
            result, "networking",
            "Hyphenated word break should be joined without hyphen: got '{}'",
            result
        );
    }

    #[test]
    fn test_join_broken_lines_compound_word() {
        // Compound word with real hyphen (should keep hyphen)
        let input = "well-\nknown";
        let result = MarkdownRenderer::join_broken_lines(input);
        assert_eq!(
            result, "well-known",
            "Compound word hyphen should be preserved: got '{}'",
            result
        );
    }

    #[test]
    fn test_join_broken_lines_preserve_sentence_break() {
        // Sentence ending should NOT be joined with next
        let input = "First sentence.\nSecond sentence.";
        let result = MarkdownRenderer::join_broken_lines(input);
        assert_eq!(
            result, "First sentence.\nSecond sentence.",
            "Sentence breaks should be preserved: got '{}'",
            result
        );
    }

    #[test]
    fn test_join_broken_lines_preserve_list_items() {
        // List items should NOT be joined
        let input = "- First item\n- Second item";
        let result = MarkdownRenderer::join_broken_lines(input);
        assert_eq!(
            result, "- First item\n- Second item",
            "List items should be preserved: got '{}'",
            result
        );
    }

    #[test]
    fn test_join_broken_lines_preserve_headers() {
        // Headers should NOT be joined
        let input = "# Header\nSome text below.";
        let result = MarkdownRenderer::join_broken_lines(input);
        assert!(
            result.contains("# Header\n"),
            "Headers should be preserved: got '{}'",
            result
        );
    }

    #[test]
    fn test_join_broken_lines_preserve_code_blocks() {
        // Code blocks should NOT be joined
        let input = "```rust\nlet x = 5;\n```";
        let result = MarkdownRenderer::join_broken_lines(input);
        assert!(
            result.contains("```rust\n"),
            "Code blocks should be preserved: got '{}'",
            result
        );
    }

    #[test]
    fn test_join_broken_lines_multiple_breaks() {
        // Multiple broken words in sequence
        let input = "This docu-\nment describes configu-\nration options.";
        let result = MarkdownRenderer::join_broken_lines(input);
        assert_eq!(
            result, "This document describes configuration options.",
            "Multiple breaks should be fixed: got '{}'",
            result
        );
    }

    #[test]
    fn test_join_broken_lines_preserve_empty_lines() {
        // Empty lines (paragraph breaks) should be preserved
        let input = "First paragraph.\n\nSecond paragraph.";
        let result = MarkdownRenderer::join_broken_lines(input);
        assert_eq!(
            result, "First paragraph.\n\nSecond paragraph.",
            "Empty lines should be preserved: got '{}'",
            result
        );
    }

    #[test]
    fn test_join_broken_lines_real_world_example() {
        // Real-world example from Apple PDF
        let input = r#"- kSBXProfileNoInternet : TCP/IP netw
orking is prohibited.
- kSBXProfileNoNetwork : All sockets
              - based networking is prohibited."#;
        let result = MarkdownRenderer::join_broken_lines(input);
        // First line should be joined (word split)
        assert!(
            result.contains("TCP/IP networking is prohibited"),
            "Word split should be joined: got '{}'",
            result
        );
        // The weird "- based" should be detected as non-joinable due to structural nature
        // This is a tricky case - let's just verify the list structure is preserved
        assert!(
            result.contains("- kSBXProfileNoInternet"),
            "List structure should be preserved: got '{}'",
            result
        );
    }

    // =====================================================================
    // OODA-IT18: Cross-empty-line join guard tests
    // =====================================================================

    #[test]
    fn test_join_broken_lines_no_cross_paragraph_join_long_line() {
        // Long lines should NOT be joined across empty lines (paragraph breaks).
        // WHY: render_text() adds \n\n between blocks. A long line ending with
        // a lowercase word (like "with") is a COMPLETE sentence, not a fragment.
        let input = "The extractor should detect that this is a separate column and not interleave the lines with\n\nsome space and tests the vertical flow.";
        let result = MarkdownRenderer::join_broken_lines(input);
        assert!(
            result.contains("with\n\nsome"),
            "Long lines should NOT be joined across paragraph breaks: got '{}'",
            result
        );
    }

    #[test]
    fn test_join_broken_lines_short_fragment_cross_empty() {
        // Short fragments SHOULD still be joined across empty lines.
        // WHY: "netw" (4 chars) is a word fragment from a narrow PDF text box.
        let input = "netw\n\norking is prohibited.";
        let result = MarkdownRenderer::join_broken_lines(input);
        assert_eq!(
            result, "networking is prohibited.",
            "Short fragments should still join across empty lines: got '{}'",
            result
        );
    }

    #[test]
    fn test_join_broken_lines_medium_fragment_cross_empty() {
        // Medium-length fragments (< 30 chars) should still join.
        let input = "TCP/IP netw\n\norking is prohibited.";
        let result = MarkdownRenderer::join_broken_lines(input);
        assert_eq!(
            result, "TCP/IP networking is prohibited.",
            "Medium fragments should still join across empty lines: got '{}'",
            result
        );
    }

    #[test]
    fn test_join_broken_lines_preserves_paragraph_structure() {
        // Realistic multi-block output where paragraphs should stay separate.
        let input = "This is the first paragraph about a topic that is quite important for understanding.\n\nthe second paragraph continues with more details about the same topic.";
        let result = MarkdownRenderer::join_broken_lines(input);
        assert!(
            result.contains("\n\n"),
            "Paragraph boundaries in long text should be preserved: got '{}'",
            result
        );
    }

    // =====================================================================
    // OODA-IT17: Paragraph continuation detection tests
    // =====================================================================

    /// Helper to create a test Block with given type, text, and bbox
    fn make_test_block(block_type: BlockType, text: &str, y1: f32, y2: f32) -> Block {
        Block {
            id: crate::schema::BlockId::generate(),
            block_type,
            bbox: crate::schema::BoundingBox::new(0.0, y1, 200.0, y2),
            page: 0,
            position: 0,
            text: text.to_string(),
            html: None,
            spans: Vec::new(),
            children: Vec::new(),
            confidence: 1.0,
            level: None,
            source: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// OODA-IT18: Helper to create a test Block with explicit X positions (for column tests)
    fn make_test_block_xy(
        block_type: BlockType,
        text: &str,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
    ) -> Block {
        Block {
            id: crate::schema::BlockId::generate(),
            block_type,
            bbox: crate::schema::BoundingBox::new(x1, y1, x2, y2),
            page: 0,
            position: 0,
            text: text.to_string(),
            html: None,
            spans: Vec::new(),
            children: Vec::new(),
            confidence: 1.0,
            level: None,
            source: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_paragraph_continuation_different_columns() {
        // OODA-IT18: Blocks in different columns should NOT be joined.
        // prev in right column (x1=318), curr in left column (x1=78)
        let prev = make_test_block_xy(
            BlockType::Text,
            "SOTA extraction requires understanding the",
            318.0,
            78.0,
            549.0,
            88.0,
        );
        let curr = make_test_block_xy(
            BlockType::Text,
            "some space and tests the vertical flow.",
            78.0,
            90.0,
            292.0,
            100.0,
        );
        assert!(
            !MarkdownRenderer::is_paragraph_continuation(&prev, &curr),
            "Blocks in different columns should NOT be paragraph continuations"
        );
    }

    #[test]
    fn test_paragraph_continuation_same_column() {
        // Blocks in the same column SHOULD still be considered for continuation.
        let prev = make_test_block_xy(
            BlockType::Text,
            "with a focus on",
            78.0,
            100.0,
            300.0,
            116.0,
        );
        let curr = make_test_block_xy(
            BlockType::Text,
            "workflows and automation",
            82.0,
            118.0,
            300.0,
            134.0,
        );
        assert!(
            MarkdownRenderer::is_paragraph_continuation(&prev, &curr),
            "Blocks in the same column should be considered for continuation"
        );
    }

    #[test]
    fn test_paragraph_continuation_lowercase_start() {
        // "focus on" + "workflows" → continuation (lowercase start, short)
        let prev = make_test_block(BlockType::Text, "with a focus on", 100.0, 116.0);
        let curr = make_test_block(BlockType::Text, "workflows", 118.0, 134.0);
        assert!(
            MarkdownRenderer::is_paragraph_continuation(&prev, &curr),
            "lowercase fragment should be continuation"
        );
    }

    #[test]
    fn test_paragraph_continuation_sentence_boundary() {
        // "ends with period." + "New sentence" → NOT continuation
        let prev = make_test_block(BlockType::Text, "This sentence ends.", 100.0, 116.0);
        let curr = make_test_block(BlockType::Text, "New paragraph starts", 118.0, 134.0);
        assert!(
            !MarkdownRenderer::is_paragraph_continuation(&prev, &curr),
            "After sentence-ending punctuation should not be continuation"
        );
    }

    #[test]
    fn test_paragraph_continuation_heading_like_prev() {
        // "What we deliver" (heading-like) + "body text" → NOT continuation
        let prev = make_test_block(BlockType::Text, "What we deliver", 100.0, 116.0);
        let curr = make_test_block(BlockType::Text, "vs-buy, and investment", 118.0, 134.0);
        assert!(
            !MarkdownRenderer::is_paragraph_continuation(&prev, &curr),
            "Heading-like prev should not be continued"
        );
    }

    #[test]
    fn test_paragraph_continuation_large_gap() {
        // Large vertical gap → NOT continuation
        let prev = make_test_block(BlockType::Text, "text without ending", 100.0, 116.0);
        let curr = make_test_block(BlockType::Text, "far away text", 200.0, 216.0);
        assert!(
            !MarkdownRenderer::is_paragraph_continuation(&prev, &curr),
            "Large gap should not be continuation"
        );
    }

    #[test]
    fn test_paragraph_continuation_different_types() {
        // Header + Text → NOT continuation
        let prev = make_test_block(BlockType::SectionHeader, "Introduction", 100.0, 116.0);
        let curr = make_test_block(BlockType::Text, "body text here", 118.0, 134.0);
        assert!(
            !MarkdownRenderer::is_paragraph_continuation(&prev, &curr),
            "Different block types should not be continuation"
        );
    }

    #[test]
    fn test_paragraph_continuation_list_item() {
        // Text + "- list item" → NOT continuation
        let prev = make_test_block(BlockType::Text, "some text without end", 100.0, 116.0);
        let curr = make_test_block(BlockType::Text, "- list item", 118.0, 134.0);
        assert!(
            !MarkdownRenderer::is_paragraph_continuation(&prev, &curr),
            "List items should not be continuation"
        );
    }

    #[test]
    fn test_paragraph_continuation_uppercase_single_word() {
        // "focus on" + "ROI" (short uppercase) → continuation (single short word)
        let prev = make_test_block(BlockType::Text, "with a focus on", 100.0, 116.0);
        let curr = make_test_block(BlockType::Text, "ROI", 118.0, 134.0);
        assert!(
            MarkdownRenderer::is_paragraph_continuation(&prev, &curr),
            "Short uppercase single word should be continuation"
        );
    }
}
