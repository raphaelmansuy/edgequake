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

// OODA-IT19: Import shared prose detection (DRY principle)
use super::heading_classifier::has_prose_indicators;

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
            document = processor.process(document)?;
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
            // WHY: Match section numbers like "1.", "3.2.", "A.1." but NOT acronyms like "LLM."
            // - Allow single letter (A-Z) or single digit followed by optional sub-numbering
            // - Require sub-numbering to be digits only (e.g., "A.1." not "A.B.")
            // - Reject 2+ consecutive uppercase letters (those are acronyms, not section labels)
            section_regex: Regex::new(
                r"^((?:[A-Z]|\d+)\.(?:\d+\.)*)\s+([A-Z][A-Za-z0-9\s,:\-\(\)]+)$",
            )
            .expect("Section regex should be valid"),
            special_sections: vec![
                // OODA-12: "Abstract" removed - pymupdf4llm formats it as inline bold
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
                // OODA-12: Additional special sections from academic papers
                "Data availability statement",
                "Data Availability Statement",
                "Author Contributions",
                "Author contributions",
                "Competing interests",
                "Competing Interests",
                "Conflict of Interest",
                "Conflicts of Interest",
                "Funding",
                "Ethics Statement",
                "Supplementary Material",
                "Supplementary Materials",
                "Declarations",
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

        // Third pass: process blocks with index-based iteration
        // WHY: Index-based access allows us to peek at adjacent blocks for inline label detection
        for page in &mut document.pages {
            let block_count = page.blocks.len();
            for i in 0..block_count {
                // Skip blocks already classified as list items
                // WHY: List items like "1. Item" should not become headers
                if page.blocks[i].block_type == BlockType::ListItem {
                    continue;
                }

                if page.blocks[i].block_type != BlockType::Text
                    && page.blocks[i].block_type != BlockType::Paragraph
                {
                    continue;
                }

                let text = page.blocks[i].text.trim().to_string();

                // Strategy 1: Check for running headers
                if running_headers.contains(&text.to_lowercase()) {
                    page.blocks[i].block_type = BlockType::PageHeader;
                    continue;
                }

                // Strategy 2: Check for numbered section headers
                // OODA-23: Skip figure/table captions that look like numbered sections
                // OODA-12: Only top-level sections (1., 2., etc.) become headers
                // WHY: pymupdf4llm gold standards show subsections (1.1, 3.1.1) as bold text
                if let Some(captures) = self.section_regex.captures(&text) {
                    if let (Some(num), Some(title)) = (captures.get(1), captures.get(2)) {
                        let section_num = num.as_str();
                        let title_text = title.as_str();

                        // OODA-23: Filter out "Fig." and "Table" captions
                        // WHY: "Fig. 1. Title" matches section regex but isn't a section
                        let is_caption = section_num.to_lowercase().starts_with("fig.")
                            || section_num.to_lowercase().starts_with("table");

                        // OODA-12: Only create headers for top-level sections (no dots in number)
                        // "1. Introduction" → H2 header
                        // "3.2. Methods" → NOT a header (bold text instead)
                        let dots_count = section_num.matches('.').count();
                        let is_top_level = dots_count == 1; // e.g., "1." has 1 dot

                        if !is_caption
                            && title_text.len() < 80
                            && !title_text.ends_with('.')
                            && is_top_level
                        {
                            page.blocks[i].block_type = BlockType::SectionHeader;
                            page.blocks[i].level = Some(2); // Always H2 for major sections
                        }
                    }
                }
                // Strategy 3: Check for special section names
                else if self.is_special_section(&text) {
                    page.blocks[i].block_type = BlockType::SectionHeader;
                    page.blocks[i].level = Some(2);
                }
                // Strategy 4: Font-size based detection with adjacent block check
                else {
                    let (is_heading, level) = self
                        .heading_classifier
                        .classify(&page.blocks[i], body_font_size);

                    if is_heading {
                        // OODA-26: Check if next block indicates this is an inline label
                        // WHY: Inline labels like "**Categorizing Tools:**" may have their
                        // colon and continuation text split into the next block during extraction.
                        // If next block starts with continuation patterns, don't classify as heading.
                        let has_continuation_next = if i + 1 < block_count {
                            let next_text = page.blocks[i + 1].text.trim_start();
                            // Pattern 1: Next block starts with colon (split during extraction)
                            let starts_with_colon = next_text.starts_with(':');
                            // Pattern 2: Next block starts with article/continuation
                            // "The diverse landscape..." following "Categorizing Tools for Perception"
                            let next_lower = next_text.to_lowercase();
                            let starts_with_article = next_lower.starts_with("the ")
                                || next_lower.starts_with("a ")
                                || next_lower.starts_with("an ")
                                || next_lower.starts_with("this ");
                            // Pattern 3: Next block starts with lowercase (continuation text)
                            let starts_lowercase = next_text
                                .chars()
                                .next()
                                .map(|c| c.is_lowercase())
                                .unwrap_or(false);

                            starts_with_colon || starts_with_article || starts_lowercase
                        } else {
                            false
                        };

                        // Only classify as heading if no continuation pattern detected
                        if !has_continuation_next {
                            page.blocks[i].block_type = BlockType::SectionHeader;
                            page.blocks[i].level = Some(level);
                        }
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

        // OODA-25: Generic prose detection - no document-specific heuristics
        // These patterns universally indicate prose content, not headings:
        // 1. Email addresses (contains @)
        // 2. Sentence endings (ends with .)
        // 3. Complex punctuation (contains ,)
        // 4. URLs (contains :// or common TLDs)
        // 5. Bracketed category codes (e.g., [cs.AI], [math.ST])
        let is_url_or_identifier = text.contains("://")
            || text.contains(".org")
            || text.contains(".com")
            || text.contains(".edu");

        let is_bracketed_code = text.starts_with('[') && text.len() < 20 && text.contains('.');

        let looks_like_prose = text.contains('@')
            || text.ends_with('.')
            || text.contains(',')
            || is_url_or_identifier
            || is_bracketed_code;

        if looks_like_prose {
            return;
        }

        // OODA-32: Author block fragment detection
        // WHY: When table detection rejects author blocks (correctly), the individual
        // fragments may still be misclassified as headers. Author block fragments have:
        // - Single leading digit(s) directly attached to a name (no delimiter)
        //   e.g., "1Alois Knoll" (superscript ¹ rendered as "1")
        // - Just a standalone digit (affiliation number)
        // - Very short person name patterns (< 30 chars)
        // - Pattern "N. Name Name" where N is 1-2 digits (affiliation + name)
        // Real section headers have: "1. Introduction" (common section words, not names)
        let looks_like_author_fragment = {
            // Pattern 1: Starts with digit(s) immediately followed by uppercase letter
            // e.g., "1Alois Knoll" vs "1. Introduction" or "1 Introduction"
            let digit_end = text.chars().take_while(|c| c.is_ascii_digit()).count();
            let has_digit_prefix = digit_end > 0 && digit_end < text.len();
            let digit_attached_to_name = if has_digit_prefix {
                let after_digits = text[digit_end..].chars().next();
                matches!(after_digits, Some(c) if c.is_uppercase())
            } else {
                false
            };

            // Pattern 2: Just a standalone digit (1-3 chars, all digits)
            let is_standalone_digit = text.len() <= 3 && text.chars().all(|c| c.is_ascii_digit());

            // Pattern 3: "N. Name Name" - affiliation number + proper names
            // e.g., "1. Alois Knoll" vs "1. Introduction"
            // Detect: short text with digit prefix, followed by 2+ capitalized words
            // that don't look like section titles
            let is_numbered_name = {
                // Check for "N." or "N)" prefix
                let trimmed = text.trim();
                let prefix_end =
                    trimmed.find(|c: char| !c.is_ascii_digit() && c != '.' && c != ')' && c != ' ');

                if let Some(pos) = prefix_end {
                    // Check if starts with digit + delimiter pattern
                    let prefix = &trimmed[..pos];
                    let has_number_prefix = prefix.chars().any(|c| c.is_ascii_digit())
                        && (prefix.contains('.') || prefix.contains(')'));

                    if has_number_prefix && pos < trimmed.len() {
                        let after_prefix = trimmed[pos..].trim();
                        let words: Vec<&str> = after_prefix.split_whitespace().collect();

                        // Looks like names: 1-3 capitalized words, all short
                        let looks_like_names = !words.is_empty()
                            && words.len() <= 4
                            && words.iter().all(|w| {
                                let first_char = w.chars().next();
                                matches!(first_char, Some(c) if c.is_uppercase()) && w.len() <= 15
                                // Person name words are short
                            })
                            && after_prefix.len() <= 40; // Total name is short

                        // NOT a section header pattern
                        // Real sections have: Introduction, Motivation, Background, Methods, Results, etc.
                        let after_lower = after_prefix.to_lowercase();
                        let looks_like_section = after_lower.contains("introduction")
                            || after_lower.contains("motivation")
                            || after_lower.contains("background")
                            || after_lower.contains("method")
                            || after_lower.contains("result")
                            || after_lower.contains("conclusion")
                            || after_lower.contains("discussion")
                            || after_lower.contains("abstract")
                            || after_lower.contains("related")
                            || after_lower.contains("experiment")
                            || after_lower.contains("evaluation")
                            || after_lower.contains("overview")
                            || after_lower.contains("objective")
                            || after_lower.contains("problem")
                            || after_lower.contains("approach")
                            || after_lower.contains("system")
                            || after_lower.contains("framework")
                            || after_lower.contains("analysis")
                            || after_lower.contains("implementation")
                            || after_lower.contains("appendix")
                            || after_lower.contains("reference");

                        looks_like_names && !looks_like_section
                    } else {
                        false
                    }
                } else {
                    false
                }
            };

            digit_attached_to_name || is_standalone_digit || is_numbered_name
        };

        if looks_like_author_fragment {
            return; // Don't classify author fragments as headers
        }

        // Generic list item detection
        // Pattern: starts with "N." or "N)" where N is 1-3 digits
        let is_list_item = {
            let trimmed = text.trim();
            if let Some(first_word) = trimmed.split_whitespace().next() {
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

        // OODA-23: Filter out figure/table captions
        // WHY: Captions like "Fig. 1. Key Components..." are styled like headings
        // (bold, larger font, title-case) but should remain as body text.
        let is_caption = text_lower.starts_with("fig.")
            || text_lower.starts_with("figure")
            || text_lower.starts_with("table")
            || text_lower.starts_with("tab.");
        if is_caption {
            return; // Don't classify captions as headers
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

        // OODA-12: Removed Abstract/Keywords special handling
        // WHY: pymupdf4llm formats "Abstract" as inline bold, not as a header
        // let is_abstract_or_keywords = text_lower == "abstract" || text_lower == "abstract.";

        if ratio > 1.5 && is_short {
            // Large font ratio (>=1.5x) is always H1
            // OODA-IT19: BUT only if the text is NOT prose-like.
            // WHY: "This is the second" at 18pt (ratio 1.8x) passes font size
            // checks but contains prose indicators ("is" + "the" lowercase),
            // proving it's a sentence fragment, not a heading label.
            if !has_prose_indicators(text) {
                block.block_type = BlockType::SectionHeader;
                block.level = Some(1);
            }
        } else if is_first_block_on_first_page && ratio > 1.2 && is_short && looks_like_title_case {
            // WHY: First block on first page with title-case text and larger font
            // is almost always the document title, even if ratio < 1.5
            // Pandoc typically uses ~1.3x for H1 titles
            block.block_type = BlockType::SectionHeader;
            block.level = Some(1);
        } else if ratio > 1.4 && is_short && looks_like_section {
            // OODA-12: Raised threshold from 1.2 to 1.4 to be more conservative
            block.block_type = BlockType::SectionHeader;
            block.level = Some(2);
        }
        // OODA-12: REMOVED H3 detection (ratio > 1.1)
        // This was creating too many subsection headers
        else {
            let is_bold = block
                .spans
                .first()
                .map(|s| s.style.weight.unwrap_or(400) >= 600)
                .unwrap_or(false);

            let _is_first_char_upper = text
                .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ' ')
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false);

            // OODA-24: Keywords should never be headers (just bold body text)
            let is_keywords = text_lower == "keywords"
                || text_lower.starts_with("keywords:")
                || text_lower.starts_with("keywords.");
            if is_keywords {
                return; // Don't classify Keywords as a header
            }

            // OODA-24: Filter out inline definition labels
            // WHY: Patterns like "Reasoning System: The reasoning system receives..."
            // have bold label + colon + description. These are inline definitions, not headings.
            // Gold format: "**Reasoning System:** The reasoning system receives..."
            // Detection: if text contains ":" followed by lowercase after whitespace, it's inline.
            // Also detect: "Label Words Description text..." without colon (PDF lost the colon)
            let is_inline_label = {
                // Pattern 1: Contains colon with lowercase text after
                let has_colon_pattern = if let Some(colon_pos) = text.find(':') {
                    let after_colon = &text[colon_pos + 1..];
                    let trimmed_after = after_colon.trim_start();
                    trimmed_after
                        .chars()
                        .next()
                        .map(|c| c.is_lowercase())
                        .unwrap_or(false)
                        || trimmed_after.len() > 20
                } else {
                    false
                };

                // Pattern 2: Title-case words followed by lowercase description
                // E.g., "Reasoning System The reasoning system receives..."
                // The pattern: uppercase+lowercase words, then "The/A/An/This" or lowercase start
                let has_inline_description = {
                    // Split text by space and find where title-case ends
                    let words: Vec<&str> = text.split_whitespace().collect();
                    let mut found_inline = false;
                    if words.len() >= 3 {
                        // Check first 1-4 words for title-case pattern
                        // BUT: if we see common prose starters (The, A, An, This, etc.)
                        // followed by lowercase words, it's inline description
                        for i in 1..words.len().min(5) {
                            let word = words[i];
                            let word_lower = word.to_lowercase();

                            // Check if this word is a prose starter
                            let is_prose_starter = matches!(
                                word_lower.as_str(),
                                "the" | "a" | "an" | "this" | "it" | "in" | "is" | "are" | "as"
                            );

                            if is_prose_starter {
                                // Check if there are lowercase words after this
                                if i + 1 < words.len() {
                                    let next_word = words[i + 1];
                                    let starts_lower = next_word
                                        .chars()
                                        .next()
                                        .map(|c| c.is_lowercase())
                                        .unwrap_or(false);
                                    if starts_lower {
                                        found_inline = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    found_inline
                };

                // Pattern 3: Standalone label ending with colon (definition term)
                // E.g., "How to Implement a Reflection System:"
                // "Robotic and Physical System Control:"
                // "Categorizing Tools for Perception"
                // These are NOT section headings (which would be "4.1 Section Name")
                let is_definition_label = {
                    let trimmed = text.trim();
                    // Ends with colon and doesn't start with a number (section numbering)
                    // And has multiple words (not just "Abstract:" which IS a heading)
                    let ends_with_colon = trimmed.ends_with(':');
                    let word_count = trimmed.split_whitespace().count();
                    let starts_with_number = trimmed
                        .chars()
                        .next()
                        .map(|c| c.is_ascii_digit())
                        .unwrap_or(false);
                    // Definition labels have 3+ words and end with colon
                    ends_with_colon && word_count >= 3 && !starts_with_number
                };

                has_colon_pattern || has_inline_description || is_definition_label
            };
            if is_inline_label {
                return; // Don't classify inline labels as headers
            }

            // OODA-12: DISABLED bold body-sized text as headers
            // WHY: pymupdf4llm gold standards show subsections as bold text, NOT headers.
            // Previously this created H3 headers for bold body-sized text like "1.1 Motivation"
            // Now we don't create headers from just bold text - only from larger font sizes.
            //
            // BEFORE: Bold + body-sized + section-like → H3
            // AFTER:  Bold + body-sized + section-like → plain bold text (not a header)
            let _is_bold = is_bold;
            let _looks_like_section = looks_like_section;
            let _is_short = is_short;
            // Removed: if is_bold && is_short && is_first_char_upper && looks_like_section...
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
            let block_count = page.blocks.len();

            // OODA-26: Index-based iteration for adjacent block access
            for block_idx in 0..block_count {
                processor.detect_styles(&mut page.blocks[block_idx]);
                // Pass context: first block on first page is likely the title
                let is_first_block = is_first_page && block_idx == 0;
                processor.detect_headers_with_context(&mut page.blocks[block_idx], is_first_block);

                // OODA-26: Post-hoc inline label detection using adjacent block
                // WHY: Labels like "Categorizing Tools for Perception" followed by
                // continuation text should NOT be classified as headings.
                // OODA-27 FIX: Don't revert known section names (Abstract, Introduction, etc.)
                // These are valid headings even when followed by "This paper..."
                if page.blocks[block_idx].block_type == BlockType::SectionHeader
                    && page.blocks[block_idx].level == Some(3)
                {
                    let block_text = page.blocks[block_idx].text.trim().to_lowercase();
                    let is_known_section = block_text == "abstract"
                        || block_text == "introduction"
                        || block_text == "conclusion"
                        || block_text == "background"
                        || block_text == "methods"
                        || block_text == "results"
                        || block_text == "discussion"
                        || block_text == "acknowledgments"
                        || block_text == "acknowledgements"
                        || block_text == "references";

                    // Only apply inline label reversion to NON-section headers
                    if !is_known_section {
                        // Check if next block looks like continuation text
                        if block_idx + 1 < block_count {
                            let next_text = page.blocks[block_idx + 1].text.trim_start();
                            let next_lower = next_text.to_lowercase();

                            // Pattern 1: Next block starts with lowercase (continuation)
                            let starts_lowercase = next_text
                                .chars()
                                .next()
                                .map(|c| c.is_lowercase())
                                .unwrap_or(false);

                            // Pattern 2: Next block starts with article (continuation)
                            let starts_with_article = next_lower.starts_with("the ")
                                || next_lower.starts_with("a ")
                                || next_lower.starts_with("an ")
                                || next_lower.starts_with("this ");

                            // Pattern 3: Next block starts with colon (split label)
                            let starts_with_colon = next_text.starts_with(':');

                            if starts_lowercase || starts_with_article || starts_with_colon {
                                // Revert to text/paragraph - not a heading
                                page.blocks[block_idx].block_type = BlockType::Text;
                                page.blocks[block_idx].level = None;
                            }
                        }
                    }
                }

                // Process children
                for child in &mut page.blocks[block_idx].children {
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
        // Note: Abstract is intentionally NOT a special section - it's inline bold text per pymupdf4llm gold format
        assert!(!processor.is_special_section("Abstract"));
        assert!(processor.is_special_section("REFERENCES"));
        assert!(processor.is_special_section("Introduction"));
        assert!(processor.is_special_section("Conclusion"));
        assert!(!processor.is_special_section("Random Text"));
    }

    #[test]
    fn test_section_pattern_level_calculation() {
        let processor = SectionPatternProcessor::new();
        assert_eq!(processor.calculate_level("1."), 2);
        assert_eq!(processor.calculate_level("3.2."), 3);
        assert_eq!(processor.calculate_level("3.2.1."), 4);
    }

    // ==========================================================================
    // OODA-30: ProcessorChain and Default implementation tests
    // ==========================================================================

    #[test]
    fn test_processor_chain_empty() {
        let chain = ProcessorChain::new();
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);

        // WHY: Empty chain should pass document through unchanged
        let doc = create_test_document();
        let original_block_count = doc.pages[0].blocks.len();
        let result = chain.process(doc).unwrap();
        assert_eq!(result.pages[0].blocks.len(), original_block_count);
    }

    #[test]
    fn test_processor_chain_default() {
        let chain = ProcessorChain::default();
        assert!(chain.is_empty());
    }

    #[test]
    fn test_section_pattern_default() {
        let _processor = SectionPatternProcessor::default();
        // WHY: Just verify Default trait creates without panic
    }

    #[test]
    fn test_style_detection_default() {
        let processor = StyleDetectionProcessor::default();
        // WHY: Default body size is 10.0pt (common academic font)
        assert!((processor.body_size - 10.0).abs() < 0.001);
    }
}
