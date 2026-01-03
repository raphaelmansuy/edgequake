//! Document structure detection processors.
//!
//! **Single Responsibility:** Detecting semantic structure elements.
//!
//! This module contains processors for recognizing document structure:
//! - `HeaderDetectionProcessor`: Section headers (H1-H6) from font size and patterns
//! - `CaptionDetectionProcessor`: Figure/table captions from text patterns
//! - `ListDetectionProcessor`: Bullet and numbered lists
//! - `CodeBlockDetectionProcessor`: Code blocks from monospace fonts
//!
//! **First Principles:**
//! - Structure detection uses font metrics, not hardcoded keywords
//! - Headers are distinguished by font size ratio to body text
//! - Lists have consistent indentation and bullet patterns
//! - Code blocks use monospace fonts consistently

use crate::schema::{Block, BlockType, Document};
use crate::Result;
use regex::Regex;

use super::Processor;

// =============================================================================
// HeaderDetectionProcessor
// =============================================================================

/// Detects section headers using font size ratios and numbering patterns.
///
/// **Detection Hierarchy:**
/// 1. Numbered sections ("1. Introduction", "3.2 Methods")
/// 2. Font size ratio to body text
/// 3. Position-aware heuristics (first page title detection)
///
/// **WHY font ratios, not keywords:**
/// Academic papers vary in section naming. Font metrics are universal.
pub struct HeaderDetectionProcessor {}

impl HeaderDetectionProcessor {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for HeaderDetectionProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for HeaderDetectionProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        // 1. Calculate font size statistics to find body text size
        let mut size_counts = std::collections::HashMap::new();
        for page in &document.pages {
            for block in &page.blocks {
                if let Some(span) = block.spans.first() {
                    // Quantize to 0.1pt precision
                    let size = (span.style.size.unwrap_or(10.0) * 10.0).round() as i32;
                    *size_counts.entry(size).or_insert(0) += block.text.len();
                }
            }
        }

        // Body size = most common (by character count)
        let body_size_int = size_counts
            .iter()
            .max_by_key(|&(_, count)| count)
            .map(|(s, _)| *s)
            .unwrap_or(100);
        let body_size = body_size_int as f32 / 10.0;

        // 2. Compile heading detection patterns
        // Subsection: "1.1", "2.3.4" → H3+
        let subsection_heading = Regex::new(r"^\d+\.\d+(?:\.\d+)*\.?\s+[A-Z]").unwrap();
        // Single number: "1." or "2." → H2 (needs additional validation)
        let single_number_heading = Regex::new(r"^\d+\.?\s+[A-Z]").unwrap();

        for page in &mut document.pages {
            for block in &mut page.blocks {
                if !matches!(block.block_type, BlockType::Text | BlockType::SectionHeader) {
                    continue;
                }

                let text = block.text.trim();

                // Position-aware length threshold
                let is_first_page = page.number == 1;
                let block_y = block.bbox.y1;
                let page_height = page.height;
                let is_top_of_page = block_y > (page_height - 200.0);

                let font_size = block
                    .spans
                    .first()
                    .and_then(|s| s.style.size)
                    .unwrap_or(10.0);
                let is_large_font = font_size > body_size * 1.4;

                // Allow longer text for document titles (first page, top, or large font)
                let max_heading_len = if is_first_page && (is_top_of_page || is_large_font) {
                    150 // Document titles can be 80-120 chars
                } else {
                    80 // Section headers are shorter
                };

                // Guard: inline descriptions like "Author: John Doe" shouldn't be headers
                let has_inline_description = if let Some(colon_pos) = text.find(':') {
                    if colon_pos < 10 {
                        let key = &text[..colon_pos].trim();
                        let is_property_like = key
                            .chars()
                            .next()
                            .map(|c| c.is_lowercase())
                            .unwrap_or(false)
                            || key == &"doi"
                            || key == &"url"
                            || key == &"email";
                        is_property_like && text.len() > 50
                    } else {
                        false
                    }
                } else {
                    false
                };

                let is_short_for_heading =
                    text.len() < max_heading_len && !text.ends_with('.') && !has_inline_description;

                // Check for subsection pattern first (e.g., "1.1 Motivation")
                if is_short_for_heading && subsection_heading.is_match(text) {
                    let prefix: String = text
                        .chars()
                        .take_while(|c| c.is_ascii_digit() || *c == '.')
                        .collect();
                    let trimmed = prefix.trim_end_matches('.');
                    let dot_count = trimmed.chars().filter(|&c| c == '.').count() as u8;
                    // 1.1 → 1 dot → H3, 1.1.1 → 2 dots → H4
                    let level = (dot_count + 2).clamp(3, 6);
                    block.block_type = BlockType::SectionHeader;
                    block.level = Some(level);
                    continue;
                }

                // Single number patterns need additional validation (avoid list items)
                // Addresses like "353 Serra Mall, Stanford, CA" contain commas, skip them
                if is_short_for_heading
                    && single_number_heading.is_match(text)
                    && !text.contains(',')
                {
                    let after_number: String = text
                        .chars()
                        .skip_while(|c| c.is_ascii_digit() || *c == '.' || c.is_whitespace())
                        .collect();

                    let is_title_cased = after_number
                        .chars()
                        .next()
                        .map(|c| c.is_uppercase())
                        .unwrap_or(false);

                    if let Some(span) = block.spans.first() {
                        let size = span.style.size.unwrap_or(10.0);
                        let weight = span.style.weight.unwrap_or(400);
                        let is_bold = weight >= 600;
                        let is_larger = size > body_size * 1.15;

                        // Multi-signal: need font evidence AND structure
                        let is_likely_section =
                            (is_larger || is_bold) && is_title_cased || (is_larger && is_bold);

                        if is_likely_section {
                            block.block_type = BlockType::SectionHeader;
                            block.level = Some(2);
                            continue;
                        }
                    }
                }

                // Font-size based detection
                if let Some(span) = block.spans.first() {
                    let size = span.style.size.unwrap_or(10.0);
                    let weight = span.style.weight.unwrap_or(400);
                    let _is_bold = weight >= 600; // Reserved for potential future use

                    let text_lower = text.to_lowercase();
                    let is_arxiv_or_meta = text_lower.starts_with("arxiv:")
                        || text_lower.contains("arxiv.org")
                        || text_lower.starts_with("[cs.")
                        || text_lower.starts_with("[stat.")
                        || text_lower.starts_with("[math.");

                    let max_len_for_heading = if is_first_page && (is_top_of_page || is_large_font)
                    {
                        150
                    } else {
                        100
                    };

                    let headingish = !text.is_empty()
                        && text.len() < max_len_for_heading
                        && !text.contains('@')
                        && !text.ends_with('.')
                        && !text.contains(',')
                        && !is_arxiv_or_meta;

                    // Section headers start with digit OR are all-caps
                    let looks_like_section = text.starts_with(|c: char| c.is_ascii_digit())
                        || text.chars().all(|c| c.is_uppercase() || c.is_whitespace());

                    if headingish && size > body_size * 1.6 {
                        block.block_type = BlockType::SectionHeader;
                        block.level = Some(1);
                    } else if headingish && looks_like_section && size > body_size * 1.35 {
                        block.block_type = BlockType::SectionHeader;
                        block.level = Some(2);
                    } else if headingish && looks_like_section && size > body_size * 1.2 {
                        block.block_type = BlockType::SectionHeader;
                        block.level = Some(3);
                    }
                    // Note: We no longer convert all bold text to headers
                }
            }
        }

        Ok(document)
    }

    fn name(&self) -> &str {
        "HeaderDetectionProcessor"
    }
}

// =============================================================================
// CaptionDetectionProcessor
// =============================================================================

/// Detects figure and table captions.
///
/// **Pattern:** "Figure N:" or "Table N:" prefix.
///
/// **WHY regex, not font metrics:**
/// Captions have consistent naming conventions across papers.
pub struct CaptionDetectionProcessor {}

impl CaptionDetectionProcessor {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for CaptionDetectionProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for CaptionDetectionProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        let caption_regex = Regex::new(r"^(Figure|Fig\.|Table|Tab\.)\s*\d+[:.]").unwrap();

        for page in &mut document.pages {
            for block in &mut page.blocks {
                if block.block_type != BlockType::Text {
                    continue;
                }

                let text = block.text.trim();
                if caption_regex.is_match(text) {
                    block.block_type = BlockType::Caption;
                }
            }
        }
        Ok(document)
    }

    fn name(&self) -> &str {
        "CaptionDetectionProcessor"
    }
}

// =============================================================================
// ListDetectionProcessor
// =============================================================================

/// Detects bullet and numbered list items.
///
/// **Detection:**
/// - Bullet markers: -, *, •
/// - Number patterns: 1. or 1)
/// - Indentation level from left margin
///
/// **WHY indentation matters:**
/// Nested lists use increasing indentation. We compute level from x-offset.
pub struct ListDetectionProcessor {}

impl ListDetectionProcessor {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for ListDetectionProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for ListDetectionProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        let bullet_regex = Regex::new(r"^[-*•]\s+").unwrap();
        let number_regex = Regex::new(r"^\d+[\.)]\s+").unwrap();

        for page in &mut document.pages {
            // Find left margin for indentation calculation
            let min_x = page
                .blocks
                .iter()
                .map(|b| b.bbox.x1)
                .fold(f32::MAX, |a, b| a.min(b));

            for block in &mut page.blocks {
                if block.block_type != BlockType::Text {
                    continue;
                }

                let text = block.text.trim();
                if bullet_regex.is_match(text) || number_regex.is_match(text) {
                    block.block_type = BlockType::ListItem;

                    // Calculate indentation level (20pts per level)
                    let indent = block.bbox.x1 - min_x;
                    let level = (indent / 20.0).round() as i32;

                    block
                        .metadata
                        .insert("indent".to_string(), serde_json::json!(indent));
                    block
                        .metadata
                        .insert("level".to_string(), serde_json::json!(level));
                }
            }
        }
        Ok(document)
    }

    fn name(&self) -> &str {
        "ListDetectionProcessor"
    }
}

// =============================================================================
// CodeBlockDetectionProcessor
// =============================================================================

/// Detects and merges code blocks.
///
/// **Detection:** All spans use monospace/code-like fonts.
///
/// **Merging:** Consecutive code blocks are joined with newlines.
///
/// **WHY merge:**
/// PDF extracts each code line separately. We need coherent blocks.
pub struct CodeBlockDetectionProcessor {}

impl CodeBlockDetectionProcessor {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for CodeBlockDetectionProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for CodeBlockDetectionProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        for page in &mut document.pages {
            // 1. Identify code blocks by font
            for block in &mut page.blocks {
                if block.block_type != BlockType::Text {
                    continue;
                }

                let all_code = !block.spans.is_empty()
                    && block.spans.iter().all(|s| s.style.looks_like_code());

                if all_code {
                    block.block_type = BlockType::Code;
                }
            }

            // 2. Merge consecutive code blocks
            let mut merged = Vec::new();
            let mut current_code: Option<Block> = None;

            for block in std::mem::take(&mut page.blocks) {
                if block.block_type == BlockType::Code {
                    if let Some(mut cur) = current_code.take() {
                        cur.text.push('\n');
                        cur.text.push_str(&block.text);
                        cur.spans.extend(block.spans);
                        cur.bbox = cur.bbox.union(&block.bbox);
                        current_code = Some(cur);
                    } else {
                        current_code = Some(block);
                    }
                } else {
                    if let Some(cur) = current_code.take() {
                        merged.push(cur);
                    }
                    merged.push(block);
                }
            }

            if let Some(cur) = current_code {
                merged.push(cur);
            }

            page.blocks = merged;
        }
        Ok(document)
    }

    fn name(&self) -> &str {
        "CodeBlockDetectionProcessor"
    }
}

#[cfg(test)]
mod tests {
    #![allow(unused_imports)]

    use super::*;
    use crate::processors::test_helpers::{
        code_block as make_code_block, doc_with_blocks, styled_block, text_block,
    };
    use crate::schema::{BoundingBox, FontStyle, Page, TextSpan};

    /// Create a minimal test document with one paragraph.
    #[allow(dead_code)]
    fn create_test_document() -> Document {
        doc_with_blocks(vec![text_block(
            "Test paragraph",
            (72.0, 100.0, 540.0, 130.0),
        )])
    }

    #[test]
    fn test_caption_detection() {
        let doc = doc_with_blocks(vec![
            text_block("Test paragraph", (72.0, 100.0, 540.0, 130.0)),
            text_block("Figure 1: Test figure", (72.0, 200.0, 540.0, 220.0)),
        ]);

        let processor = CaptionDetectionProcessor::new();
        let result = processor.process(doc).unwrap();

        assert_eq!(result.pages[0].blocks[1].block_type, BlockType::Caption);
    }

    #[test]
    fn test_list_detection() {
        let doc = doc_with_blocks(vec![
            text_block("Test paragraph", (72.0, 100.0, 540.0, 130.0)),
            text_block("- First item", (72.0, 200.0, 540.0, 220.0)),
            text_block("1. Numbered item", (72.0, 230.0, 540.0, 250.0)),
        ]);

        let processor = ListDetectionProcessor::new();
        let result = processor.process(doc).unwrap();

        assert_eq!(result.pages[0].blocks[1].block_type, BlockType::ListItem);
        assert_eq!(result.pages[0].blocks[2].block_type, BlockType::ListItem);
    }

    #[test]
    fn test_code_block_detection() {
        use crate::processors::test_helpers::monospace_block;

        // Two monospace blocks should be merged into one Code block
        let doc = doc_with_blocks(vec![
            monospace_block("def hello():", (72.0, 100.0, 540.0, 115.0)),
            monospace_block("    print('Hello')", (72.0, 120.0, 540.0, 135.0)),
        ]);

        let processor = CodeBlockDetectionProcessor::new();
        let result = processor.process(doc).unwrap();

        // Should be merged into one code block
        assert_eq!(result.pages[0].blocks.len(), 1);
        assert_eq!(result.pages[0].blocks[0].block_type, BlockType::Code);
        assert!(result.pages[0].blocks[0].text.contains('\n'));
    }

    #[test]
    fn test_header_detection_numeric_sections() {
        // Body text to establish baseline, then bold section header
        let doc = doc_with_blocks(vec![
            styled_block("This is body text.", (72.0, 200.0, 540.0, 220.0), 10.0, 400),
            styled_block("1. Introduction", (72.0, 100.0, 540.0, 120.0), 10.0, 700),
        ]);

        let processor = HeaderDetectionProcessor::new();
        let result = processor.process(doc).unwrap();

        let intro = result.pages[0]
            .blocks
            .iter()
            .find(|b| b.text.trim() == "1. Introduction")
            .expect("missing heading block");
        assert_eq!(intro.block_type, BlockType::SectionHeader);
        assert_eq!(intro.level, Some(2));
    }
}
