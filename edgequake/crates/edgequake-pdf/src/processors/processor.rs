//! Core processor traits and utilities.
//!
//! **Single Responsibility:** Processor trait definition and chaining.
//!
//! This module defines:
//! - `Processor`: Core trait for document transformation
//! - `ProcessorChain`: Composes processors in sequence
//! - `SectionPatternProcessor`: Pattern-based section header detection
//! - `StyleDetectionProcessor`: Font-based style and header detection
//!
//! All other processors are extracted to focused modules:
//! - `layout_processing`: Layout, margins, block merging
//! - `structure_detection`: Headers, captions, lists, code blocks  
//! - `table_detection`: Table detection and reconstruction
//! - `text_cleanup`: Text normalization, OCR fixes, hyphenation

use crate::schema::{Block, BlockType, Document};
use crate::Result;
use regex::Regex;

// =============================================================================
// Processor Trait
// =============================================================================

/// Trait for document processors.
///
/// Processors transform documents in a chain-of-responsibility pattern.
/// Each processor can modify the document structure, blocks, or metadata.
///
/// **Implementations must be:**
/// - `Send + Sync` for parallel processing
/// - Idempotent where possible
/// - Error-tolerant (don't fail on edge cases)
pub trait Processor: Send + Sync {
    /// Process a document, returning the modified document.
    fn process(&self, document: Document) -> Result<Document>;

    /// Get the processor name for debugging/logging.
    fn name(&self) -> &str;
}

// =============================================================================
// ProcessorChain
// =============================================================================

/// Chain of processors applied sequentially.
///
/// **Usage:**
/// ```rust,ignore
/// let chain = ProcessorChain::new()
///     .add(LayoutProcessor::new())
///     .add(BlockMergeProcessor::new())
///     .add(PostProcessor::new());
///
/// let document = chain.process(document)?;
/// ```
pub struct ProcessorChain {
    processors: Vec<Box<dyn Processor>>,
}

impl ProcessorChain {
    /// Create a new empty processor chain.
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
        }
    }

    /// Add a processor to the chain.
    pub fn add<P: Processor + 'static>(mut self, processor: P) -> Self {
        self.processors.push(Box::new(processor));
        self
    }

    /// Process a document through the chain.
    pub fn process(&self, mut document: Document) -> Result<Document> {
        for processor in &self.processors {
            tracing::debug!("Running processor: {}", processor.name());

            // DEBUG: Track block 20 content through processors - check ALL pages
            let mut found_page = None;
            for (page_idx, page) in document.pages.iter().enumerate() {
                if page.blocks.iter().any(|b| b.text.contains("dering. Given")) {
                    found_page = Some(page_idx + 1);
                    break;
                }
            }
            if let Some(pn) = found_page {
                tracing::info!(
                    "CHAIN-TRACE [BEFORE {}]: 'dering. Given' on page {}",
                    processor.name(),
                    pn
                );
            } else {
                tracing::info!(
                    "CHAIN-TRACE [BEFORE {}]: 'dering. Given' MISSING from all pages",
                    processor.name()
                );
            }

            document = processor.process(document)?;

            // DEBUG: Check if block 20 survived - check ALL pages
            let mut found_page = None;
            for (page_idx, page) in document.pages.iter().enumerate() {
                if page.blocks.iter().any(|b| b.text.contains("dering. Given")) {
                    found_page = Some(page_idx + 1);
                    break;
                }
            }
            if let Some(pn) = found_page {
                tracing::info!(
                    "CHAIN-TRACE [AFTER {}]: 'dering. Given' on page {}",
                    processor.name(),
                    pn
                );
            } else {
                tracing::info!(
                    "CHAIN-TRACE [AFTER {}]: 'dering. Given' MISSING from all pages",
                    processor.name()
                );
            }
        }
        Ok(document)
    }

    /// Get the number of processors.
    pub fn len(&self) -> usize {
        self.processors.len()
    }

    /// Check if the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.processors.is_empty()
    }
}

impl Default for ProcessorChain {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// SectionPatternProcessor
// =============================================================================

/// Detects section headers from text patterns.
///
/// **Strategies (in priority order):**
/// 1. Running headers (text repeated across pages) → PageHeader
/// 2. Numbered sections ("1. Introduction", "3.2. Methods") → SectionHeader
/// 3. Special section names ("Abstract", "References") → SectionHeader
/// 4. Font-size based (HeadingClassifier geometric detection) → SectionHeader
///
/// **Single Responsibility:** Section header detection and classification.
/// Delegates font analysis to FontAnalyzer and heading classification to HeadingClassifier.
#[allow(dead_code)]
pub struct SectionPatternProcessor {
    section_regex: Regex,
    special_sections: Vec<&'static str>,
    font_analyzer: super::FontAnalyzer,
    heading_classifier: super::HeadingClassifier,
}

#[allow(dead_code)]
impl SectionPatternProcessor {
    pub fn new() -> Self {
        Self {
            // Match patterns like "1.", "3.2.", "A.1." followed by space and title
            section_regex: Regex::new(
                r"^([0-9A-Z]+\.(?:[0-9]+\.)*)\s+([A-Z][A-Za-z0-9\s,:\-\(\)]+)$",
            )
            .expect("Section regex should be valid"),
            special_sections: vec![
                "Abstract",
                "Introduction",
                "Related Work",
                "Background",
                "Methodology",
                "Methods",
                "Approach",
                "Experiments",
                "Results",
                "Discussion",
                "Conclusion",
                "Conclusions",
                "Future Work",
                "Acknowledgments",
                "Acknowledgements",
                "References",
                "Bibliography",
                "Appendix",
            ],
            font_analyzer: super::FontAnalyzer::new(),
            heading_classifier: super::HeadingClassifier::new(),
        }
    }

    /// Calculate heading level from section number.
    /// "1." → level 2 (H2, since H1 is title)
    /// "3.2." → level 3 (H3)
    /// "3.2.1." → level 4 (H4)
    fn calculate_level(&self, section_num: &str) -> u8 {
        let dots = section_num.matches('.').count();
        ((dots + 1) as u8).clamp(2, 6)
    }

    /// Check if text is a special section name.
    fn is_special_section(&self, text: &str) -> bool {
        let trimmed = text.trim();
        self.special_sections
            .iter()
            .any(|s| trimmed.eq_ignore_ascii_case(s))
    }

    /// Detect running headers (text repeated across multiple pages).
    fn find_running_headers(&self, document: &Document) -> std::collections::HashSet<String> {
        use std::collections::HashMap;

        let mut text_pages: HashMap<String, usize> = HashMap::new();

        for page in &document.pages {
            let mut seen_on_page = std::collections::HashSet::new();
            for block in &page.blocks {
                let text = block.text.trim().to_string();
                if text.len() > 10 && text.len() < 150 {
                    let normalized = text.to_lowercase();
                    if seen_on_page.insert(normalized.clone()) {
                        *text_pages.entry(normalized).or_insert(0) += 1;
                    }
                }
            }
        }

        let threshold = (document.pages.len() / 2).max(3);
        text_pages
            .into_iter()
            .filter(|(_, count)| *count >= threshold)
            .map(|(text, _)| text)
            .collect()
    }
}

impl Default for SectionPatternProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for SectionPatternProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        // First pass: detect body font size
        let body_font_size = self.font_analyzer.detect_body_font_size(&document);
        tracing::debug!("Detected body font size: {:.1}pt", body_font_size);

        // Second pass: identify running headers
        let running_headers = self.find_running_headers(&document);

        // Third pass: process blocks
        for page in &mut document.pages {
            for block in &mut page.blocks {
                // Skip blocks already classified as list items
                // WHY: List items like "1. Item" should not become headers
                if block.block_type == BlockType::ListItem {
                    continue;
                }

                if block.block_type != BlockType::Text && block.block_type != BlockType::Paragraph {
                    continue;
                }

                let text = block.text.trim();

                // Strategy 1: Check for running headers
                if running_headers.contains(&text.to_lowercase()) {
                    block.block_type = BlockType::PageHeader;
                    continue;
                }

                // Strategy 2: Check for numbered section headers
                if let Some(captures) = self.section_regex.captures(text) {
                    if let (Some(num), Some(title)) = (captures.get(1), captures.get(2)) {
                        let section_num = num.as_str();
                        let title_text = title.as_str();

                        if title_text.len() < 80 && !title_text.ends_with('.') {
                            let level = self.calculate_level(section_num);
                            block.block_type = BlockType::SectionHeader;
                            block.level = Some(level);
                        }
                    }
                }
                // Strategy 3: Check for special section names
                else if self.is_special_section(text) {
                    block.block_type = BlockType::SectionHeader;
                    block.level = Some(2);
                }
                // Strategy 4: Font-size based detection
                else {
                    let (is_heading, level) =
                        self.heading_classifier.classify(block, body_font_size);
                    if is_heading {
                        block.block_type = BlockType::SectionHeader;
                        block.level = Some(level);
                    }
                }
            }
        }

        Ok(document)
    }

    fn name(&self) -> &str {
        "SectionPatternProcessor"
    }
}

// =============================================================================
// StyleDetectionProcessor
// =============================================================================

/// Detects styles (bold, italic) and headers from font properties.
///
/// **Style Detection:**
/// - Bold: font weight >= 600 OR font name contains "Bold"
/// - Italic: font name contains "Italic" or "Oblique"
///
/// **Header Detection (font-size ratio to body):**
/// - H1: ratio > 1.5 AND short text (<80 chars)
/// - H2: ratio > 1.2 AND looks like section
/// - H3: ratio > 1.1 AND looks like section
///
/// **WHY no keyword matching:**
/// Section names vary by discipline. Font metrics are universal.
#[derive(Clone)]
pub struct StyleDetectionProcessor {
    body_size: f32,
}

impl StyleDetectionProcessor {
    pub fn new() -> Self {
        Self { body_size: 10.0 }
    }

    fn compute_body_size(&mut self, document: &Document) {
        use std::collections::HashMap;
        let mut size_counts: HashMap<i32, usize> = HashMap::new();

        for page in &document.pages {
            for block in &page.blocks {
                for span in &block.spans {
                    let size_key = (span.style.size.unwrap_or(10.0) * 10.0) as i32;
                    *size_counts.entry(size_key).or_insert(0) += 1;
                }
            }
        }

        self.body_size = size_counts
            .iter()
            .max_by_key(|&(_, count)| count)
            .map(|(s, _)| *s as f32 / 10.0)
            .unwrap_or(10.0);

        tracing::debug!("Computed body font size: {:.1}pt", self.body_size);
    }

    fn detect_styles(&self, block: &mut Block) {
        for span in &mut block.spans {
            let family_lower = span
                .style
                .family
                .as_ref()
                .map(|f| f.to_lowercase())
                .unwrap_or_default();

            let is_bold = span.style.weight.unwrap_or(400) >= 600 || family_lower.contains("bold");
            span.style.weight = Some(if is_bold { 700 } else { 400 });

            let is_italic = span.style.italic
                || family_lower.contains("italic")
                || family_lower.contains("oblique");
            span.style.italic = is_italic;
        }
    }

    /// Simple wrapper for detect_headers_with_context.
    /// Reserved for future use in contexts where page position is unknown.
    #[allow(dead_code)]
    fn detect_headers(&self, block: &mut Block) {
        self.detect_headers_with_context(block, false);
    }

    /// Detect headers with page/block context.
    ///
    /// **WHY context matters:**
    /// First block on first page is typically the document title,
    /// so we lower the font ratio threshold for H1 detection.
    fn detect_headers_with_context(&self, block: &mut Block, is_first_block_on_first_page: bool) {
        if block.block_type != BlockType::Text {
            return;
        }

        let size = block
            .spans
            .first()
            .map(|s| s.style.size.unwrap_or(10.0))
            .unwrap_or(10.0);

        let ratio = size / self.body_size;
        let text = block.text.trim();
        let text_lower = text.to_lowercase();
        let is_short = text.len() < 80;

        // Guards
        let is_arxiv_or_meta = text_lower.starts_with("arxiv:")
            || text_lower.contains("arxiv.org")
            || text_lower.starts_with("[cs.")
            || text_lower.starts_with("[stat.")
            || text_lower.starts_with("[math.");

        let looks_like_prose =
            text.contains('@') || text.ends_with('.') || text.contains(',') || is_arxiv_or_meta;
        if looks_like_prose {
            return;
        }

        // Detect list items (should not be headers)
        // Pattern: starts with "N." or "N)" where N is 1-3 digits
        let is_list_item = {
            let trimmed = text.trim();
            if let Some(first_word) = trimmed.split_whitespace().next() {
                // Matches: "1.", "2.", "10.", "1)", "2)"
                (first_word.ends_with('.') || first_word.ends_with(')'))
                    && first_word.len() >= 2
                    && first_word[..first_word.len() - 1]
                        .chars()
                        .all(|c| c.is_ascii_digit())
            } else {
                false
            }
        };

        if is_list_item {
            return; // Don't classify list items as headers
        }

        // Section pattern: starts with digit OR is all caps
        let looks_like_numbered_section = text.starts_with(|c: char| c.is_ascii_digit());
        let looks_like_caps_section = text
            .chars()
            .all(|c| c.is_uppercase() || c.is_whitespace() || c.is_ascii_digit());

        // Title case: first char uppercase, contains lowercase (mixed case)
        let has_uppercase_start = text
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false);
        let has_lowercase = text.chars().any(|c| c.is_lowercase());
        let looks_like_title_case = has_uppercase_start && has_lowercase;

        // Expanded section detection
        let looks_like_section = looks_like_numbered_section
            || looks_like_caps_section
            || (looks_like_title_case && is_short);

        let is_abstract_or_keywords = text_lower == "abstract"
            || text_lower.starts_with("abstract.")
            || text_lower == "keywords"
            || text_lower.starts_with("keywords:");

        if is_abstract_or_keywords && is_short {
            block.block_type = BlockType::SectionHeader;
            block.level = Some(3);
        } else if ratio > 1.5 && is_short {
            // Large font ratio (>=1.5x) is always H1
            block.block_type = BlockType::SectionHeader;
            block.level = Some(1);
        } else if is_first_block_on_first_page && ratio > 1.2 && is_short && looks_like_title_case {
            // WHY: First block on first page with title-case text and larger font
            // is almost always the document title, even if ratio < 1.5
            // Pandoc typically uses ~1.3x for H1 titles
            block.block_type = BlockType::SectionHeader;
            block.level = Some(1);
        } else if ratio > 1.2 && is_short && looks_like_section {
            block.block_type = BlockType::SectionHeader;
            block.level = Some(2);
        } else if ratio > 1.1 && is_short && looks_like_section {
            block.block_type = BlockType::SectionHeader;
            block.level = Some(3);
        } else {
            let is_bold = block
                .spans
                .first()
                .map(|s| s.style.weight.unwrap_or(400) >= 600)
                .unwrap_or(false);

            let is_first_char_upper = text
                .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ' ')
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false);

            // WHY: Bold text with body-sized font is typically H3 or H4
            // In LaTeX/pandoc, H3 is often rendered with same font size as body but bold
            // H2 is typically rendered with slightly larger font (ratio > 1.1)
            if is_bold
                && is_short
                && is_first_char_upper
                && looks_like_section
                && !looks_like_prose
                && !is_abstract_or_keywords
            {
                block.block_type = BlockType::SectionHeader;
                // If font is body-sized (ratio <= 1.1), it's H3; otherwise H2
                // WHY: Distinguishes "bold + slightly larger" (H2) from "bold + body-sized" (H3)
                if ratio <= 1.05 {
                    block.level = Some(3);
                } else {
                    block.level = Some(2);
                }
            }
        }
    }
}

impl Default for StyleDetectionProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for StyleDetectionProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        let mut processor = self.clone();
        processor.compute_body_size(&document);

        for page in &mut document.pages {
            let is_first_page = page.number == 1;

            for (block_idx, block) in page.blocks.iter_mut().enumerate() {
                processor.detect_styles(block);
                // Pass context: first block on first page is likely the title
                let is_first_block = is_first_page && block_idx == 0;
                processor.detect_headers_with_context(block, is_first_block);

                for child in &mut block.children {
                    processor.detect_styles(child);
                    processor.detect_headers_with_context(child, false);
                }
            }
        }

        Ok(document)
    }

    fn name(&self) -> &str {
        "StyleDetectionProcessor"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{BoundingBox, FontStyle, Page, TextSpan};

    fn create_test_document() -> Document {
        let mut doc = Document::new();
        let mut page = Page::new(1, 612.0, 792.0);

        page.add_block(Block::text(
            "First paragraph.",
            BoundingBox::new(72.0, 100.0, 540.0, 130.0),
        ));
        page.add_block(Block::text(
            "Second paragraph.",
            BoundingBox::new(72.0, 150.0, 540.0, 180.0),
        ));

        doc.add_page(page);
        doc
    }

    #[test]
    fn test_processor_chain() {
        use super::super::{BlockMergeProcessor, LayoutProcessor, PostProcessor};

        let chain = ProcessorChain::new()
            .add(LayoutProcessor::new())
            .add(BlockMergeProcessor::new())
            .add(PostProcessor::new());

        assert_eq!(chain.len(), 3);

        let doc = create_test_document();
        let result = chain.process(doc);
        assert!(result.is_ok());
    }

    #[test]
    fn test_style_detection_bold() {
        let processor = StyleDetectionProcessor::new();
        let mut doc = Document::new();
        let mut page = Page::new(1, 612.0, 792.0);

        let mut block = Block::text("Bold text", BoundingBox::new(72.0, 100.0, 200.0, 120.0));
        block.spans = vec![TextSpan::styled(
            "Bold text",
            FontStyle {
                family: Some("Times-Bold".to_string()),
                size: Some(10.0),
                weight: Some(400), // Will be detected from name
                ..Default::default()
            },
        )];
        page.add_block(block);
        doc.add_page(page);

        let result = processor.process(doc).unwrap();
        let span = &result.pages[0].blocks[0].spans[0];
        assert_eq!(span.style.weight, Some(700));
    }

    #[test]
    fn test_section_pattern_special_sections() {
        let processor = SectionPatternProcessor::new();
        assert!(processor.is_special_section("Abstract"));
        assert!(processor.is_special_section("REFERENCES"));
        assert!(!processor.is_special_section("Random Text"));
    }

    #[test]
    fn test_section_pattern_level_calculation() {
        let processor = SectionPatternProcessor::new();
        assert_eq!(processor.calculate_level("1."), 2);
        assert_eq!(processor.calculate_level("3.2."), 3);
        assert_eq!(processor.calculate_level("3.2.1."), 4);
    }
}
