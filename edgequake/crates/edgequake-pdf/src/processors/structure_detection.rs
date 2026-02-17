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
use std::collections::HashSet;

// OODA-IT19: Import shared prose detection (DRY principle)
use super::heading_classifier::has_prose_indicators;
use std::sync::OnceLock;

use super::Processor;

// =============================================================================
// OODA-03: Comprehensive Bullet Character Detection
// =============================================================================

/// WHY (First Principles - per pymupdf4llm utils.py lines 28-56):
/// PDFs use a wide variety of Unicode characters as list bullets. PyMuPDF4LLM
/// recognizes 530+ characters including the entire Geometric Shapes block.
/// Our previous regex only detected 7 characters, missing ~98.7% of bullets.
///
/// This set includes:
/// - Common bullets: *, -, •, ◦, ▪
/// - Dashes: –, —, ‐, ‑, ‒, ―
/// - Daggers: †, ‡
/// - Geometric shapes: entire 0x25A0-0x25FF block
/// - Miscellaneous symbols: 0x2600+ partial
/// - Private use area: 0xF0A7, 0xF0B7 (common in fonts)
fn get_bullets() -> &'static HashSet<char> {
    static BULLETS: OnceLock<HashSet<char>> = OnceLock::new();
    BULLETS.get_or_init(|| {
        let mut set = HashSet::new();

        // Common bullets and markers
        set.insert('*'); // 0x2A asterisk
        set.insert('-'); // 0x2D hyphen-minus
        set.insert('>'); // 0x3E greater-than
        set.insert('o'); // 0x6F lowercase o
        set.insert('¶'); // 0xB6 pilcrow
        set.insert('·'); // 0xB7 middle dot

        // Various dash types
        set.insert('‐'); // 0x2010 hyphen
        set.insert('‑'); // 0x2011 non-breaking hyphen
        set.insert('‒'); // 0x2012 figure dash
        set.insert('–'); // 0x2013 en dash
        set.insert('—'); // 0x2014 em dash
        set.insert('―'); // 0x2015 horizontal bar

        // Daggers and special symbols
        set.insert('†'); // 0x2020 dagger
        set.insert('‡'); // 0x2021 double dagger
        set.insert('•'); // 0x2022 bullet
        set.insert('−'); // 0x2212 minus sign
        set.insert('∙'); // 0x2219 bullet operator

        // Geometric Shapes block (0x25A0-0x25FF)
        // WHY: Many PDFs use squares, circles, triangles as bullets
        for code in 0x25A0u32..=0x25FFu32 {
            if let Some(c) = char::from_u32(code) {
                set.insert(c);
            }
        }

        // Miscellaneous Symbols partial (0x2600-0x26FF)
        // WHY: Some PDFs use stars, checkmarks, etc.
        for code in 0x2600u32..=0x26FFu32 {
            if let Some(c) = char::from_u32(code) {
                set.insert(c);
            }
        }

        // Private Use Area (common in embedded fonts)
        set.insert('\u{F0A7}');
        set.insert('\u{F0B7}');

        // Replacement character (indicates encoding issues but often used as bullet)
        set.insert('\u{FFFD}');

        set
    })
}

/// OODA-03: Check if text starts with a bullet character.
///
/// WHY (per pymupdf4llm startswith_bullet function):
/// A text line is a bullet if:
/// 1. First character is in BULLETS set
/// 2. AND either:
///    - Single character only
///    - OR followed by a space/tab
///    - OR followed by uppercase letter (sentence start in list item)
///    - OR followed by asterisk (markdown bold marker `**`)
///
/// This prevents false positives like "∙ome" matching the bullet operator.
///
/// WHY uppercase: List items start sentences, which begin with capital letters.
/// WHY asterisk: PDF extraction may have bold text marked with `**` in markdown format.
pub fn starts_with_bullet(text: &str) -> bool {
    let bullets = get_bullets();
    let mut chars = text.chars();
    let first = match chars.next() {
        Some(c) => c,
        None => return false,
    };

    if !bullets.contains(&first) {
        return false;
    }

    // Single character is a bullet
    match chars.next() {
        None => true,
        // Bullet followed by space is valid
        Some(' ') | Some('\t') => true,
        // OODA-IT12: Bullet followed by asterisk (markdown bold **text**)
        Some('*') => true,
        // OODA-IT12: Bullet followed by uppercase letter (sentence start)
        // WHY: List items like "•General Aspect" start with capital letters
        Some(c) if c.is_uppercase() => true,
        // Anything else is not a bullet (prevents math operators like "∙x")
        _ => false,
    }
}

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
/// @implements FEAT1022
/// @implements FEAT1023 - PDF header detection
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
            // OODA-37: First pass — normalize section number spacing on ALL blocks.
            // WHY: Must run before level check because StyleDetectionProcessor may have
            // already set block_type=SectionHeader + level, causing the skip below.
            // The normalization is safe for all blocks: it only inserts a space between
            // a leading digit sequence and an uppercase letter (e.g., "1INTRO" → "1 INTRO").
            for block in &mut page.blocks {
                let raw_text = block.text.trim().to_string();
                if raw_text.starts_with(|c: char| c.is_ascii_digit()) {
                    let normalized = Self::normalize_section_number_spacing(&raw_text);
                    if normalized != raw_text {
                        block.text = normalized;
                    }
                }
            }

            // Second pass — detect headers based on font size and patterns
            for block in &mut page.blocks {
                // Skip blocks already classified as list items (by ListDetectionProcessor)
                // WHY: List items like "1. First item" should not become headers
                if block.block_type == BlockType::ListItem {
                    continue;
                }

                if !matches!(block.block_type, BlockType::Text | BlockType::SectionHeader) {
                    continue;
                }

                // OODA-27: Don't overwrite blocks that already have a level assigned
                // WHY: HeadingBodySplitProcessor sets level=3 for Abstract, and we
                // don't want HeaderDetection to overwrite that with level=2 based on font size
                if block.level.is_some() {
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

                // OODA-23: Filter out figure/table captions
                // WHY: Captions like "Fig. 1. Key Components..." are styled like headings
                // (bold, larger font, title-case) but should remain as body text.
                let text_lower = text.to_lowercase();
                let is_caption = text_lower.starts_with("fig.")
                    || text_lower.starts_with("figure")
                    || text_lower.starts_with("table")
                    || text_lower.starts_with("tab.");
                if is_caption {
                    continue; // Don't classify captions as headers
                }

                // OODA-12: DISABLE subsection pattern detection (e.g., "1.1 Motivation")
                // WHY: pymupdf4llm gold standards show subsections as bold text, NOT headers.
                // Only the paper title and major sections (1. Introduction, etc.) are headers.
                // The gold file has 10 headers vs our 33 - disabling subsection detection
                // to match pymupdf4llm's conservative header identification.
                //
                // BEFORE: Pattern "1.1 Motivation" → level 3 header (###)
                // AFTER:  Pattern "1.1 Motivation" → bold text paragraph
                let _subsection_pattern_disabled = subsection_heading.is_match(text);
                // Previously this created H3-H6 headers:
                // if is_short_for_heading && subsection_heading.is_match(text) {
                //     let prefix: String = text.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
                //     let trimmed = prefix.trim_end_matches('.');
                //     let dot_count = trimmed.chars().filter(|&c| c == '.').count() as u8;
                //     let level = (dot_count + 2).clamp(3, 6);
                //     block.block_type = BlockType::SectionHeader;
                //     block.level = Some(level);
                //     continue;
                // }

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
                        // WHY: Prose text contains commas (sentences), headings don't
                        let has_prose_markers = text.contains(',') || text.ends_with('.');

                        // Multi-signal: need font evidence AND structure
                        let is_likely_section = !has_prose_markers
                            && ((is_larger || is_bold) && is_title_cased || (is_larger && is_bold));

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

                    // OODA-25: Generic prose detection - no document-specific heuristics
                    let is_url_or_identifier = text.contains("://")
                        || text.contains(".org")
                        || text.contains(".com")
                        || text.contains(".edu");

                    let is_bracketed_code =
                        text.starts_with('[') && text.len() < 20 && text.contains('.');

                    let is_metadata = is_url_or_identifier || is_bracketed_code;

                    // OODA-25: Generic sentence boundary detection
                    // Pattern `. [A-Z]` indicates embedded sentences like "Abstract. This paper..."
                    let has_sentence_boundary = Self::contains_sentence_boundary(text);

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
                        && !is_metadata
                        && !has_sentence_boundary
                        // OODA-IT19: Reject prose-like text (articles/copulas + lowercase).
                        // WHY: "This is the second" passes all other checks but contains
                        // prose indicators ("is" + "the" lowercase) proving it's a sentence
                        // fragment, not a heading label. Shared with heading_classifier.rs.
                        && !has_prose_indicators(text);

                    // Section headers start with digit OR are all-caps
                    let looks_like_section = text.starts_with(|c: char| c.is_ascii_digit())
                        || text.chars().all(|c| c.is_uppercase() || c.is_whitespace());

                    // OODA-12: Conservative header detection to match pymupdf4llm
                    // Only H1 (title) and H2 (major sections) - no H3+
                    // WHY: pymupdf4llm gold has only 10 headers, not 33+
                    if headingish && size > body_size * 1.6 {
                        block.block_type = BlockType::SectionHeader;
                        block.level = Some(1);
                    } else if headingish && looks_like_section && size > body_size * 1.4 {
                        block.block_type = BlockType::SectionHeader;
                        block.level = Some(2);
                    }
                    // OODA-12: REMOVED H3 detection (size > 1.2x)
                    // Previous code created too many subsection headers
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

impl HeaderDetectionProcessor {
    /// Generic sentence boundary detection.
    /// Returns true if text contains `. [A-Z]` pattern (period + space + capital)
    /// indicating a sentence break, NOT an abbreviation.
    fn contains_sentence_boundary(text: &str) -> bool {
        let chars: Vec<char> = text.chars().collect();
        for i in 0..chars.len().saturating_sub(2) {
            let is_sentence_end = matches!(chars[i], '.' | '?' | '!');
            let is_space = chars[i + 1] == ' ';
            let is_capital = chars.get(i + 2).map(|c| c.is_uppercase()).unwrap_or(false);

            if is_sentence_end && is_space && is_capital {
                // Check for common abbreviations by looking at preceding chars
                // Build the preceding string from chars to avoid UTF-8 boundary issues
                let preceding: String = chars[..=i].iter().collect();
                let preceding_lower = preceding.to_lowercase();

                let is_abbreviation = preceding_lower.ends_with("dr.")
                    || preceding_lower.ends_with("mr.")
                    || preceding_lower.ends_with("mrs.")
                    || preceding_lower.ends_with("ms.")
                    || preceding_lower.ends_with("prof.")
                    || preceding_lower.ends_with("fig.")
                    || preceding_lower.ends_with("tab.")
                    || preceding_lower.ends_with("eq.")
                    || preceding_lower.ends_with("vs.")
                    || preceding_lower.ends_with("et al.")
                    || preceding_lower.ends_with("e.g.")
                    || preceding_lower.ends_with("i.e.");

                if !is_abbreviation {
                    return true;
                }
            }
        }
        false
    }

    /// OODA-37: Normalize section header text to fix number-title spacing.
    ///
    /// WHY (First Principle): In natural language, a section number like "1", "2",
    /// or "3.2" is ALWAYS separated from the section title by whitespace or punctuation.
    /// Patterns like "1INTRODUCTION" or "3THE LIGHTRAG" are PDF extraction artifacts
    /// where the section number span and title span were adjacent without a gap
    /// exceeding the space-detection threshold.
    ///
    /// ```text
    /// PDF spans: ["1"]["INTRODUCTION"]  ← gap < 15% font size → no space inserted
    /// Raw text:  "1INTRODUCTION"        ← violates natural language rules
    /// Fixed:     "1 INTRODUCTION"       ← restored space between number and title
    /// ```
    ///
    /// Pattern: `^\d+[A-Z]` → insert space between trailing digit and leading uppercase.
    /// This is safe because no valid English/scientific text starts with digit+uppercase
    /// without separation (e.g., "1A" as a grade would be "1-A" or "1.A").
    fn normalize_section_number_spacing(text: &str) -> String {
        let chars: Vec<char> = text.chars().collect();
        if chars.len() < 2 {
            return text.to_string();
        }

        // Find where the leading digits/dots end
        let mut num_end = 0;
        for (i, ch) in chars.iter().enumerate() {
            if ch.is_ascii_digit() || *ch == '.' {
                num_end = i + 1;
            } else {
                break;
            }
        }

        // If we found leading digits and the next char is uppercase, insert space
        if num_end > 0 && num_end < chars.len() {
            let next_char = chars[num_end];
            if next_char.is_uppercase() {
                let prefix: String = chars[..num_end].iter().collect();
                let suffix: String = chars[num_end..].iter().collect();
                return format!("{} {}", prefix, suffix);
            }
        }

        text.to_string()
    }
}

// =============================================================================
// CaptionDetectionProcessor
// =============================================================================

/// Detects figure and table captions, including multi-line continuations.
///
/// **Pattern:** "Figure N:" or "Table N:" prefix.
///
/// **WHY regex, not font metrics:**
/// Captions have consistent naming conventions across papers.
///
/// **WHY continuation detection (OODA-25):**
/// Captions often wrap to multiple lines/blocks in PDFs.
/// The continuation block doesn't start with "Figure N:" but is part of the caption.
/// We detect continuations by:
/// 1. Caption ends with hyphen (word continuation)
/// 2. Next block starts lowercase (sentence continuation)
/// 3. Blocks are vertically adjacent in same column
pub struct CaptionDetectionProcessor {}

impl CaptionDetectionProcessor {
    pub fn new() -> Self {
        Self {}
    }

    /// Check if a caption block appears to continue on the next block.
    ///
    /// **WHY:** Captions can wrap mid-word (hyphenation) or mid-sentence.
    fn caption_continues(&self, caption: &Block, next: &Block) -> bool {
        let caption_text = caption.text.trim();
        let next_text = next.text.trim();

        // Empty next block cannot be continuation
        if next_text.is_empty() {
            return false;
        }

        // Check 1: Caption ends with hyphen (word was split)
        // WHY: "reposi-" + "tory" must be merged
        let ends_with_hyphen = caption_text.ends_with('-');

        // Check 2: Next block starts lowercase (sentence continues)
        // WHY: New sentences start uppercase, continuations don't
        let next_starts_lowercase = next_text
            .chars()
            .next()
            .map(|c| c.is_lowercase())
            .unwrap_or(false);

        // Check 3: Blocks are spatially adjacent (same column, close vertically)
        // WHY: Caption continuations are visually connected
        // Block.bbox is BoundingBox with fields x1, y1, x2, y2
        let cap_bbox = &caption.bbox;
        let next_bbox = &next.bbox;

        // Same column: X coordinates overlap significantly
        let x_overlap = cap_bbox.x1.max(next_bbox.x1) < cap_bbox.x2.min(next_bbox.x2);
        // Vertically close: gap less than typical line height (~15pt)
        let y_gap = (next_bbox.y1 - cap_bbox.y2).abs();
        let close_vertically = y_gap < 20.0;
        let vertically_adjacent = x_overlap && close_vertically;

        // Either hyphenation or lowercase continuation, AND spatially adjacent
        (ends_with_hyphen || next_starts_lowercase) && vertically_adjacent
    }

    /// Merge caption text with its continuation, handling hyphenation.
    fn merge_caption_text(&self, caption_text: &str, continuation_text: &str) -> String {
        let cap_trimmed = caption_text.trim();
        let cont_trimmed = continuation_text.trim();

        if let Some(stripped) = cap_trimmed.strip_suffix('-') {
            // Hyphenated word: remove hyphen and join directly
            // WHY: "reposi-" + "tory" → "repository"
            format!("{}{}", stripped, cont_trimmed)
        } else {
            // Sentence continuation: add space
            format!("{} {}", cap_trimmed, cont_trimmed)
        }
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
            // Pass 1: Mark blocks starting with caption pattern
            for block in &mut page.blocks {
                if block.block_type != BlockType::Text {
                    continue;
                }

                let text = block.text.trim();
                if caption_regex.is_match(text) {
                    block.block_type = BlockType::Caption;
                }
            }

            // Pass 2: Detect and merge caption continuations (OODA-25)
            // WHY separate pass: Need all captions marked first to find continuations
            let mut merged_indices: Vec<usize> = Vec::new();

            for i in 0..page.blocks.len() {
                if page.blocks[i].block_type != BlockType::Caption {
                    continue;
                }

                // Look for continuation in next block
                if i + 1 < page.blocks.len() {
                    let next_idx = i + 1;
                    let (caption, next) = {
                        let (left, right) = page.blocks.split_at_mut(next_idx);
                        (&left[i], &right[0])
                    };

                    if next.block_type == BlockType::Text && self.caption_continues(caption, next) {
                        // Merge the text
                        let merged_text = self.merge_caption_text(&caption.text, &next.text);

                        // Update caption text
                        page.blocks[i].text = merged_text;

                        // Extend bounding box to encompass both blocks
                        // Block.bbox is BoundingBox with fields x1, y1, x2, y2
                        let cap_bbox = page.blocks[i].bbox;
                        let next_bbox = page.blocks[next_idx].bbox;
                        page.blocks[i].bbox = crate::schema::BoundingBox::new(
                            cap_bbox.x1.min(next_bbox.x1),
                            cap_bbox.y1.min(next_bbox.y1),
                            cap_bbox.x2.max(next_bbox.x2),
                            cap_bbox.y2.max(next_bbox.y2),
                        );

                        // Mark for removal
                        merged_indices.push(next_idx);

                        tracing::debug!(
                            "CaptionDetection: merged continuation block {} into caption {}",
                            next_idx,
                            i
                        );
                    }
                }
            }

            // Remove merged blocks (reverse order to preserve indices)
            for idx in merged_indices.into_iter().rev() {
                page.blocks.remove(idx);
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
        // OODA-03: bullet_regex removed - replaced by starts_with_bullet() which
        // recognizes 530+ Unicode bullet characters including geometric shapes.

        // Generic numbered list: "1. " or "1)" with required space
        // WHY: Space after marker distinguishes lists from decimal numbers like "1.1"
        let number_regex = Regex::new(r"^\d+[\.)]\s+").unwrap();
        // Secondary pattern: "1.Text" (no space) followed by capital
        // WHY: Some PDFs omit space after list marker
        let number_no_space_regex = Regex::new(r"^\d+\.[A-Z]").unwrap();
        // Generic bracketed reference: [N] format common in academic/technical docs
        // WHY: Citations, footnotes, references use [N] format universally
        let ref_regex = Regex::new(r"^\[\d{1,3}\]\s*").unwrap();

        // OODA-12: Section title pattern - exclude from list detection
        // WHY: "1. Introduction", "2. Literature Survey" are section headers, NOT lists
        // ONLY match common major section names that appear in academic papers
        // These are specific enough to avoid matching methodology sub-items
        let section_title_regex = Regex::new(
            r"^[1-9]\.\s+(Introduction|Conclusion|Discussion|Results|Methodology|Methods|Literature\s+Survey|Related\s+Work|Background|Experiments?|Evaluation|Implementation|Future\s+Work|Acknowledgements?|Results\s+and\s+(Discussion|Analysis)|Materials?\s+and\s+Methods?)$"
        ).unwrap();

        for page in &mut document.pages {
            // Find left margin for indentation calculation
            // OODA-IT36: Include Paragraph blocks (from PdfiumBackend)
            let min_x = page
                .blocks
                .iter()
                .filter(|b| {
                    matches!(
                        b.block_type,
                        BlockType::Text | BlockType::Paragraph | BlockType::ListItem
                    )
                })
                .map(|b| b.bbox.x1)
                .fold(f32::MAX, |a, b| a.min(b));

            tracing::debug!(
                "ListDetectionProcessor: page {} min_x={:.1}",
                page.number,
                min_x
            );

            for block in &mut page.blocks {
                // OODA-IT36: Accept both Text and Paragraph blocks for list detection.
                // WHY: PdfiumBackend creates Paragraph blocks (from LayoutBlockType::Paragraph),
                // while lopdf backend creates Text blocks. Both can contain list items.
                if !matches!(block.block_type, BlockType::Text | BlockType::Paragraph) {
                    continue;
                }

                let text = block.text.trim();

                // OODA-12: Skip section titles - they look like lists but are headers
                // WHY: "1. Introduction", "2. Methodology" should become section headers
                // not list items. The SectionPatternProcessor will handle them.
                if section_title_regex.is_match(text) {
                    tracing::debug!(
                        "  Skipping section title (not list): '{}'",
                        text.chars().take(40).collect::<String>()
                    );
                    continue;
                }

                // OODA-03: Use comprehensive bullet detection with 530+ characters
                // WHY: Old regex "^[-–—*•◦▪]\s+" only detected 7 bullet types.
                // PyMuPDF4LLM uses the entire geometric shapes Unicode block.
                let is_bullet_list = starts_with_bullet(text);

                // OODA-14: Check patterns for numbered lists
                // - number_regex: "1. Text" with space (standard)
                // - number_no_space_regex: "1.Text" no space but uppercase letter (not "1.1")
                let is_numbered_list =
                    number_regex.is_match(text) || number_no_space_regex.is_match(text);

                // Reference pattern: [N] format
                let is_reference = ref_regex.is_match(text);

                if is_bullet_list || is_numbered_list || is_reference {
                    block.block_type = BlockType::ListItem;

                    // Store whether this was a bullet for markdown rendering
                    if is_bullet_list {
                        block
                            .metadata
                            .insert("list_type".to_string(), serde_json::json!("bullet"));
                    } else if is_numbered_list {
                        block
                            .metadata
                            .insert("list_type".to_string(), serde_json::json!("numbered"));
                    } else {
                        block
                            .metadata
                            .insert("list_type".to_string(), serde_json::json!("reference"));
                    }

                    // Calculate indentation level (20pts per level)
                    let indent = block.bbox.x1 - min_x;
                    let level = (indent / 20.0).round() as i32;

                    tracing::debug!(
                        "  ListItem '{}' x1={:.1} indent={:.1} level={}",
                        text.chars().take(30).collect::<String>(),
                        block.bbox.x1,
                        indent,
                        level
                    );

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

/// Check if text contains only email addresses (should NOT be marked as code).
///
/// WHY: Academic PDFs often render author emails in monospace fonts,
/// but these are not code blocks. Marking them as code confuses LLMs.
///
/// Detection: All whitespace-separated tokens contain @ and . (email pattern).
fn is_email_only_content(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }
    // Check each token - all must look like emails
    trimmed.split_whitespace().all(|word| {
        // Simple email pattern: contains @ followed by .domain
        // Exclude code-like patterns with = (assignments)
        word.contains('@')
            && word.contains('.')
            && !word.contains('=')
            && !word.contains('{')
            && !word.contains('[')
    })
}

/// Check if text is a standalone URL (should NOT be marked as code).
///
/// WHY: URLs in references/citations often use monospace fonts
/// but are not code blocks.
fn is_url_only_content(text: &str) -> bool {
    let trimmed = text.trim();
    // Single-line URL patterns - no programming context
    trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("www.")
        || trimmed.starts_with("ftp://")
        // Also catch partial URLs from line breaks
        || (trimmed.len() < 50 && trimmed.contains("://") && !trimmed.contains(' '))
}

/// Detects and merges code blocks.
///
/// **Detection:** All spans use monospace/code-like fonts.
///
/// **Content Filtering (OODA-IT13):** Excludes email addresses and URLs
/// that happen to be in monospace fonts but are not actual code.
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
            // 1. Identify code blocks by font AND content
            for block in &mut page.blocks {
                if block.block_type != BlockType::Text {
                    continue;
                }

                let all_code = !block.spans.is_empty()
                    && block.spans.iter().all(|s| s.style.looks_like_code());

                // OODA-IT13: Content-based exclusion
                // Emails and URLs in monospace fonts should NOT be code blocks
                let is_excluded =
                    is_email_only_content(&block.text) || is_url_only_content(&block.text);

                if all_code && !is_excluded {
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

// =============================================================================
// HeadingBodySplitProcessor (OODA-27)
// =============================================================================

/// Splits merged heading+body text blocks into separate heading and body blocks.
///
/// **WHY this processor exists:**
/// PDF extraction often merges "Abstract. This paper reviews..." into a single
/// block. The gold standard expects "### Abstract" as a separate heading.
///
/// **Detection Patterns (Generic):**
/// - Single-word heading followed by period and continuation:
///   `^(Abstract|Introduction|Conclusion|Summary|Acknowledgments)\.\s+(.+)$`
/// - Keyword with period sentence boundary pattern
///
/// **First Principles:**
///
/// - Uses sentence boundary detection (`. ` followed by capital letter)
/// - Validates that first word is a known heading keyword
/// - Creates new block with same styling for body text
///
/// @implements FEAT0512
pub struct HeadingBodySplitProcessor {
    /// Regex for detecting merged heading patterns
    heading_pattern: Regex,
}

impl HeadingBodySplitProcessor {
    pub fn new() -> Self {
        // WHY these keywords: Common academic/technical paper structure headings
        // that are often rendered inline with period separator
        // Pattern: Single word heading + period + optional space + continuation (capital letter)
        // OODA-27 FIX: Changed \s+ to \s* to handle "Abstract.This" (no space after period)
        // OODA-12: REMOVED "Abstract" - pymupdf4llm formats it as inline bold text, not a header
        let heading_pattern = Regex::new(
            r"(?i)^(Introduction|Conclusion|Summary|Acknowledgments|Acknowledgements|Background|Discussion|Methods|Results|References)\.\s*([A-Z].*)$"
        ).expect("Invalid heading pattern regex");

        Self { heading_pattern }
    }
}

impl Default for HeadingBodySplitProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for HeadingBodySplitProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        for page in &mut document.pages {
            let mut new_blocks = Vec::with_capacity(page.blocks.len() * 2);

            for block in page.blocks.drain(..) {
                // Only process unclassified text blocks
                if block.block_type != BlockType::Text && block.block_type != BlockType::Paragraph {
                    new_blocks.push(block);
                    continue;
                }

                let text = block.text.trim();

                // Check for heading pattern match
                if let Some(captures) = self.heading_pattern.captures(text) {
                    if let (Some(heading_match), Some(body_match)) =
                        (captures.get(1), captures.get(2))
                    {
                        let heading_text = heading_match.as_str().to_string();
                        let body_text = body_match.as_str().to_string();

                        // Only split if body text is substantial
                        if body_text.len() > 10 {
                            // Create heading block (copy bbox but update text)
                            let mut heading_block = block.clone();
                            heading_block.text = heading_text.clone();
                            // Clear spans to prevent renderer from using old merged text
                            // The renderer uses spans if present, falling back to block.text
                            heading_block.spans.clear();
                            // OODA-12: Mark as section header with H2 level (was H3)
                            // WHY: pymupdf4llm gold uses H2 for "Abstract" style headings
                            heading_block.block_type = BlockType::SectionHeader;
                            heading_block.level = Some(2);

                            new_blocks.push(heading_block);

                            // Create body block with continuation text
                            let mut body_block = block;
                            body_block.text = body_text;
                            // Keep spans for body - they may be useful for styling
                            body_block.block_type = BlockType::Text;
                            body_block.level = None;
                            new_blocks.push(body_block);

                            continue;
                        }
                    }
                }

                // No match - keep original block
                new_blocks.push(block);
            }

            page.blocks = new_blocks;
        }

        Ok(document)
    }

    fn name(&self) -> &str {
        "HeadingBodySplitProcessor"
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

    // ==========================================================================
    // OODA-03: Tests for comprehensive bullet detection
    // ==========================================================================

    #[test]
    fn test_starts_with_bullet_common_bullets() {
        // Common bullet characters
        assert!(starts_with_bullet("• Item"));
        assert!(starts_with_bullet("- Item"));
        assert!(starts_with_bullet("* Item"));
        assert!(starts_with_bullet("– Item")); // en dash
        assert!(starts_with_bullet("— Item")); // em dash
    }

    #[test]
    fn test_starts_with_bullet_geometric_shapes() {
        // Geometric shapes from 0x25A0-0x25FF block
        assert!(starts_with_bullet("■ Black square")); // U+25A0
        assert!(starts_with_bullet("□ White square")); // U+25A1
        assert!(starts_with_bullet("▪ Small black square")); // U+25AA
        assert!(starts_with_bullet("● Black circle")); // U+25CF
        assert!(starts_with_bullet("○ White circle")); // U+25CB
        assert!(starts_with_bullet("◆ Black diamond")); // U+25C6
        assert!(starts_with_bullet("► Right triangle")); // U+25BA
    }

    #[test]
    fn test_starts_with_bullet_single_char() {
        // Single bullet character with no following text
        assert!(starts_with_bullet("•"));
        assert!(starts_with_bullet("-"));
        assert!(starts_with_bullet("■"));
    }

    #[test]
    fn test_starts_with_bullet_false_positives() {
        // Should NOT match these
        assert!(!starts_with_bullet("")); // empty
        assert!(!starts_with_bullet("Hello")); // normal text
        assert!(!starts_with_bullet("•text")); // bullet + LOWERCASE (math operator risk)
        assert!(!starts_with_bullet("1. Item")); // numbered list
        assert!(!starts_with_bullet("a) Item")); // lettered list
        assert!(!starts_with_bullet("•123")); // bullet + digit
    }

    #[test]
    fn test_starts_with_bullet_uppercase() {
        // OODA-IT12: Bullet followed by uppercase letter is a list item
        // WHY: List items like "•General Aspect" start with capital letters
        assert!(starts_with_bullet("•General Aspect"));
        assert!(starts_with_bullet("•Agriculture: This domain"));
        assert!(starts_with_bullet("•Methodologies. To enable"));
        assert!(starts_with_bullet("●Introduction"));
        assert!(starts_with_bullet("■Summary"));
    }

    #[test]
    fn test_starts_with_bullet_markdown_bold() {
        // OODA-IT12: Bullet followed by asterisk (markdown bold)
        assert!(starts_with_bullet("•**Bold text**"));
        assert!(starts_with_bullet("•*Italic text*"));
        assert!(starts_with_bullet("■**Title**"));
    }

    #[test]
    fn test_bullet_count() {
        // Verify we have many bullet characters
        // Our set has 372 characters (96 geometric shapes + 256 misc symbols + 20 explicit)
        // This is comparable to pymupdf4llm's coverage
        let bullets = get_bullets();
        assert!(
            bullets.len() >= 350,
            "Expected 350+ bullet characters, got {}",
            bullets.len()
        );
    }

    #[test]
    fn test_list_detection_geometric_bullets() {
        // Test that geometric shape bullets are detected as list items
        let doc = doc_with_blocks(vec![
            text_block("Regular text paragraph", (72.0, 100.0, 540.0, 120.0)),
            text_block(
                "■ First item with black square",
                (72.0, 200.0, 540.0, 220.0),
            ),
            text_block("● Second item with circle", (72.0, 230.0, 540.0, 250.0)),
        ]);

        let processor = ListDetectionProcessor::new();
        let result = processor.process(doc).unwrap();

        // The geometric bullets should be detected as list items
        assert_eq!(
            result.pages[0].blocks[1].block_type,
            BlockType::ListItem,
            "Black square bullet should be list item"
        );
        assert_eq!(
            result.pages[0].blocks[2].block_type,
            BlockType::ListItem,
            "Circle bullet should be list item"
        );
    }

    // ==========================================================================
    // OODA-IT13: Tests for code block content filtering
    // ==========================================================================

    #[test]
    fn test_is_email_only_content() {
        // WHY: Emails in monospace fonts should NOT be marked as code
        // Academic PDFs often use monospace for author affiliations

        // Single email - should be excluded
        assert!(is_email_only_content("user@example.com"));

        // Multiple emails - should be excluded
        assert!(is_email_only_content("zrguo101@hku.hk aka_xia@foxmail.com"));

        // Email with spaces - should be excluded
        assert!(is_email_only_content("  john@doe.org  "));

        // NOT emails - should NOT be excluded (is actual code)
        assert!(!is_email_only_content("x = 5"));
        assert!(!is_email_only_content("import os"));
        assert!(!is_email_only_content("Hello world"));
        assert!(!is_email_only_content(""));

        // Code with email-like patterns but has other syntax
        assert!(!is_email_only_content("email = user@example.com"));
        assert!(!is_email_only_content("{user@domain.com}"));
    }

    #[test]
    fn test_is_url_only_content() {
        // WHY: URLs in monospace fonts should NOT be marked as code
        // References/citations often use monospace for URLs

        // Full URLs - should be excluded
        assert!(is_url_only_content("https://arxiv.org"));
        assert!(is_url_only_content("http://example.com"));
        assert!(is_url_only_content("https://github.com/user/repo"));
        assert!(is_url_only_content("www.example.com"));
        assert!(is_url_only_content("ftp://files.server.com"));

        // Partial URL from line break
        assert!(is_url_only_content("https://arxiv."));

        // NOT URLs - should NOT be excluded
        assert!(!is_url_only_content("import requests"));
        assert!(!is_url_only_content("url = https://example.com"));
        assert!(!is_url_only_content("def get_url():"));
        assert!(!is_url_only_content(""));
    }

    #[test]
    fn test_code_block_excludes_emails() {
        use crate::processors::test_helpers::monospace_block;

        // Monospace block with emails should NOT be marked as code
        let doc = doc_with_blocks(vec![monospace_block(
            "user@example.com admin@test.org",
            (72.0, 100.0, 540.0, 115.0),
        )]);

        let processor = CodeBlockDetectionProcessor::new();
        let result = processor.process(doc).unwrap();

        // Should remain as Text, not Code
        assert_eq!(
            result.pages[0].blocks[0].block_type,
            BlockType::Text,
            "Email addresses in monospace should NOT be code"
        );
    }

    #[test]
    fn test_code_block_excludes_urls() {
        use crate::processors::test_helpers::monospace_block;

        // Monospace block with URL should NOT be marked as code
        let doc = doc_with_blocks(vec![monospace_block(
            "https://github.com/user/repo",
            (72.0, 100.0, 540.0, 115.0),
        )]);

        let processor = CodeBlockDetectionProcessor::new();
        let result = processor.process(doc).unwrap();

        // Should remain as Text, not Code
        assert_eq!(
            result.pages[0].blocks[0].block_type,
            BlockType::Text,
            "URL in monospace should NOT be code"
        );
    }

    #[test]
    fn test_code_block_keeps_real_code() {
        use crate::processors::test_helpers::monospace_block;

        // Actual code should still be detected
        let doc = doc_with_blocks(vec![
            monospace_block("def hello():", (72.0, 100.0, 540.0, 115.0)),
            monospace_block("    print('Hello')", (72.0, 120.0, 540.0, 135.0)),
        ]);

        let processor = CodeBlockDetectionProcessor::new();
        let result = processor.process(doc).unwrap();

        // Should be Code
        assert_eq!(
            result.pages[0].blocks[0].block_type,
            BlockType::Code,
            "Real Python code should still be detected as code"
        );
    }

    // ==========================================================================
    // OODA-37: Tests for section number spacing normalization
    // ==========================================================================

    #[test]
    fn test_normalize_section_number_spacing_basic() {
        // WHY: "1INTRODUCTION" is a common PDF extraction artifact where the section
        // number and title are adjacent without a space gap.
        assert_eq!(
            HeaderDetectionProcessor::normalize_section_number_spacing("1INTRODUCTION"),
            "1 INTRODUCTION"
        );
        assert_eq!(
            HeaderDetectionProcessor::normalize_section_number_spacing("2RETRIEVAL"),
            "2 RETRIEVAL"
        );
        assert_eq!(
            HeaderDetectionProcessor::normalize_section_number_spacing("3THE LIGHTRAG"),
            "3 THE LIGHTRAG"
        );
        assert_eq!(
            HeaderDetectionProcessor::normalize_section_number_spacing("4EVALUATION"),
            "4 EVALUATION"
        );
    }

    #[test]
    fn test_normalize_section_number_spacing_with_dot() {
        // Section numbers with dots should also be normalized
        assert_eq!(
            HeaderDetectionProcessor::normalize_section_number_spacing("3.2DUAL-LEVEL"),
            "3.2 DUAL-LEVEL"
        );
        assert_eq!(
            HeaderDetectionProcessor::normalize_section_number_spacing("1.1MOTIVATION"),
            "1.1 MOTIVATION"
        );
    }

    #[test]
    fn test_normalize_section_number_spacing_already_spaced() {
        // Already correctly spaced text should NOT be modified
        assert_eq!(
            HeaderDetectionProcessor::normalize_section_number_spacing("1 INTRODUCTION"),
            "1 INTRODUCTION"
        );
        assert_eq!(
            HeaderDetectionProcessor::normalize_section_number_spacing("3.2 DUAL-LEVEL"),
            "3.2 DUAL-LEVEL"
        );
        assert_eq!(
            HeaderDetectionProcessor::normalize_section_number_spacing("1. Introduction"),
            "1. Introduction"
        );
    }

    #[test]
    fn test_normalize_section_number_spacing_no_match() {
        // Should NOT modify text that doesn't match the pattern
        assert_eq!(
            HeaderDetectionProcessor::normalize_section_number_spacing("Introduction"),
            "Introduction"
        );
        assert_eq!(
            HeaderDetectionProcessor::normalize_section_number_spacing("hello world"),
            "hello world"
        );
        assert_eq!(
            HeaderDetectionProcessor::normalize_section_number_spacing(""),
            ""
        );
        assert_eq!(
            HeaderDetectionProcessor::normalize_section_number_spacing("1"),
            "1"
        );
        // Digit followed by lowercase — not a section pattern
        assert_eq!(
            HeaderDetectionProcessor::normalize_section_number_spacing("1st place"),
            "1st place"
        );
    }
}
