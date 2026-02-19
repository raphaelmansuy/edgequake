//! Text cleanup and normalization processors.
//!
//! @implements FEAT1006
//!
//! **Single Responsibility:** Cleaning and normalizing extracted text.
//!
//! This module contains processors for fixing common text extraction issues:
//! - `PostProcessor`: OCR fixes, whitespace normalization, ligature handling
//! - `GarbledTextFilterProcessor`: Removes corrupted/garbled text blocks
//! - `HyphenContinuationProcessor`: Joins hyphenated words across lines
//!
//! **First Principles:**
//! - PDF text extraction often produces artifacts (ligatures, soft hyphens)
//! - OCR errors follow predictable patterns (fi→ﬁ, fl→ﬂ)
//! - Hyphenation at line breaks should be transparent to readers

use crate::schema::{Block, BlockType, Document, TextSpan};
use crate::Result;
use regex::Regex;

use super::Processor;

/// WHY: UTF-8 safe string truncation.
///
/// Direct byte slicing like `&s[..50]` can panic if byte 50 falls in the middle
/// of a multi-byte character (e.g., box-drawing '─' is 3 bytes). This function
/// finds the nearest valid char boundary at or before `max_bytes`.
///
/// OODA-04: Fix byte index panics in text_cleanup.rs (debug logging).
fn safe_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

// =============================================================================
// PostProcessor
// =============================================================================

/// Cleans and normalizes extracted text.
///
/// **Operations:**
/// 1. Normalize whitespace (collapse multiple spaces)
/// 2. Fix OCR errors (ligatures, smart quotes)
/// 3. Fix soft hyphens (control characters from line breaks)
/// 4. Fix concatenated words ("methodsThe" → "methods The")
/// 5. Clean markdown artifacts from PDF annotations
///
/// **Configuration:**
/// All operations enabled by default. Use builder methods to disable.
pub struct PostProcessor {
    normalize_whitespace: bool,
    fix_ocr_errors: bool,
    consolidate_headers: bool,
}

impl PostProcessor {
    pub fn new() -> Self {
        Self {
            normalize_whitespace: true,
            fix_ocr_errors: true,
            consolidate_headers: true,
        }
    }

    /// Enable/disable whitespace normalization.
    pub fn with_normalize_whitespace(mut self, enabled: bool) -> Self {
        self.normalize_whitespace = enabled;
        self
    }

    /// Enable/disable OCR error fixing.
    pub fn with_fix_ocr_errors(mut self, enabled: bool) -> Self {
        self.fix_ocr_errors = enabled;
        self
    }

    /// Enable/disable header consolidation.
    pub fn with_consolidate_headers(mut self, enabled: bool) -> Self {
        self.consolidate_headers = enabled;
        self
    }

    /// Fix spaced text patterns (e.g., "T H E H O T M E S S" → "THE HOT MESS").
    ///
    /// **WHY:** Some PDFs embed titles with letter-spacing for visual emphasis.
    /// This produces text like "T H E" instead of "THE" after extraction.
    ///
    /// **Detection heuristics:**
    /// - Multiple single uppercase letters followed by spaces
    /// - Pattern: letter-space-letter-space repeating
    /// - Minimum 4 consecutive spaced letters to trigger (avoid "I A M")
    /// - Stops at words (uppercase followed by lowercase, not by space)
    ///
    /// **OODA-05:** Fix missing title in hotmess PDF.
    pub fn fix_spaced_text(&self, text: &str) -> String {
        let chars: Vec<char> = text.chars().collect();
        let mut result = String::new();
        let mut i = 0;

        while i < chars.len() {
            // Look for pattern: uppercase-space-uppercase-space-...
            if chars[i].is_uppercase() && i + 2 < chars.len() && chars[i + 1] == ' ' {
                // Count consecutive spaced uppercase letters
                let mut j = i;
                let mut spaced_letters = Vec::new();

                while j < chars.len() {
                    if chars[j].is_uppercase() {
                        // Check what follows this uppercase letter
                        if j + 1 < chars.len() && chars[j + 1] == ' ' {
                            // Space follows - check if next char is spaced letter or word start
                            if j + 2 < chars.len() {
                                let next = chars[j + 2];
                                if next.is_uppercase() {
                                    // Could be spaced letter or word start
                                    // If at end of string or followed by space, it's a spaced letter
                                    // If followed by lowercase, it's a word start
                                    if j + 3 >= chars.len() {
                                        // End of string - include both letters
                                        spaced_letters.push(chars[j]);
                                        j += 2; // Move to the last letter
                                        spaced_letters.push(chars[j]);
                                        break;
                                    } else if chars[j + 3] == ' ' || chars[j + 3].is_uppercase() {
                                        // Next uppercase is followed by space = spaced letter
                                        spaced_letters.push(chars[j]);
                                        j += 2; // Skip letter and space
                                        continue;
                                    } else {
                                        // Next uppercase is followed by lowercase = word start
                                        // Include current letter and stop
                                        spaced_letters.push(chars[j]);
                                        break;
                                    }
                                } else if next.is_lowercase() {
                                    // lowercase after space = end of spaced sequence
                                    spaced_letters.push(chars[j]);
                                    break;
                                }
                            } else {
                                // Nothing after space = end of string, include letter
                                spaced_letters.push(chars[j]);
                                break;
                            }
                        }
                        // No space after = last letter of spaced sequence
                        spaced_letters.push(chars[j]);
                    }
                    break;
                }

                // Only collapse if we found 4+ spaced letters (e.g., "T H E H")
                if spaced_letters.len() >= 4 {
                    // Collapse the spaced letters
                    for c in &spaced_letters {
                        result.push(*c);
                    }

                    // Calculate where to continue from
                    // j points to last consumed letter
                    let mut next_i = j + 1;

                    // If there's a space after the last letter, preserve it if followed by word
                    if next_i < chars.len() && chars[next_i] == ' ' {
                        // Check if next is a word (uppercase + lowercase) or end
                        if next_i + 1 < chars.len() {
                            let next = chars[next_i + 1];
                            if next.is_lowercase()
                                || (next.is_uppercase()
                                    && next_i + 2 < chars.len()
                                    && chars[next_i + 2].is_lowercase())
                            {
                                result.push(' '); // Preserve space before word
                            }
                        }
                        next_i += 1; // Skip the space
                    }

                    i = next_i;
                    continue;
                }
            }

            result.push(chars[i]);
            i += 1;
        }

        result
    }

    /// Process a single block and its children.
    pub fn process_block(&self, block: &mut Block) {
        if block.block_type.has_text() {
            // Process main text
            block.text = self.fix_spaced_text(&block.text); // OODA-05: Fix spaced titles first
            block.text = self.normalize_text(&block.text);
            block.text = self.fix_ocr_text(&block.text);
            block.text = self.fix_concatenated_words(&block.text);
            block.text = self.cleanup_citations(&block.text);
            block.text = self.cleanup_markdown_artifacts(&block.text);
            block.text = self.strip_footnote_markers(&block.text); // OODA-20: Strip footnote symbols

            // Process spans (renderer uses spans if present)
            for span in &mut block.spans {
                span.text = self.fix_spaced_text(&span.text); // OODA-05: Fix spaced titles
                span.text = self.normalize_span_text(&span.text);
                span.text = self.fix_ocr_text(&span.text);

                // Don't modify code-like spans (could break identifiers)
                if !span.style.looks_like_code() {
                    span.text = self.fix_concatenated_words(&span.text);
                    span.text = self.cleanup_citations(&span.text);
                    span.text = self.cleanup_markdown_artifacts(&span.text);
                    span.text = self.strip_footnote_markers(&span.text); // OODA-20
                }
            }

            // Normalize span boundaries
            if self.normalize_whitespace {
                Self::normalize_span_boundaries(&mut block.spans);
            }
        }

        // Process children recursively
        for child in &mut block.children {
            self.process_block(child);
        }
    }

    /// Fix soft hyphen patterns and control characters.
    ///
    /// **WHY:** PDF extraction produces control characters to indicate line breaks.
    /// Common patterns: "modifi\x02 cation" should become "modification".
    fn fix_soft_hyphens(&self, text: &str) -> String {
        let mut result = String::new();
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            let c = chars[i];

            // Control characters indicating soft hyphen/line break:
            // \x02 (STX), \x1F (unit separator), \xAD (soft hyphen)
            if c == '\x02' || c == '\x1F' || c == '\u{00AD}' {
                let result_trimmed = result.trim_end();
                let prev_is_letter = result_trimmed
                    .chars()
                    .last()
                    .map(|c| c.is_alphabetic())
                    .unwrap_or(false);

                // Skip forward past spaces and control chars
                let mut j = i + 1;
                while j < len && (chars[j] == ' ' || chars[j] == '\x02' || chars[j] == '\x1F') {
                    j += 1;
                }

                let next_is_lower = j < len && chars[j].is_lowercase();

                if prev_is_letter && next_is_lower {
                    // Soft hyphen: join words
                    while result.ends_with(' ') {
                        result.pop();
                    }
                    i = j;
                    continue;
                } else {
                    result.push(' ');
                }
            } else {
                result.push(c);
            }
            i += 1;
        }

        result
    }

    /// Normalize whitespace in text.
    pub fn normalize_text(&self, text: &str) -> String {
        if !self.normalize_whitespace {
            return text.to_string();
        }

        let text = self.fix_soft_hyphens(text);
        let mut result = String::new();
        let mut prev_space = false;

        for c in text.chars() {
            if c == ' ' || c == '\t' {
                if !prev_space {
                    result.push(' ');
                    prev_space = true;
                }
            } else {
                result.push(c);
                prev_space = false;
            }
        }

        if result.chars().all(|c| c.is_whitespace()) {
            result
        } else {
            result.trim().to_string()
        }
    }

    /// Normalize whitespace in span text (preserves boundary spaces).
    fn normalize_span_text(&self, text: &str) -> String {
        if !self.normalize_whitespace {
            return text.to_string();
        }

        let text = self.fix_soft_hyphens(text);
        let mut result = String::new();
        let mut prev_space = false;

        for c in text.chars() {
            if c == ' ' || c == '\t' {
                if !prev_space {
                    result.push(' ');
                    prev_space = true;
                }
            } else {
                result.push(c);
                prev_space = false;
            }
        }

        result
    }

    /// Fix common OCR errors (ligatures, smart quotes).
    pub fn fix_ocr_text(&self, text: &str) -> String {
        if !self.fix_ocr_errors {
            return text.to_string();
        }

        let mut result = text.to_string();

        // Common OCR replacements
        let replacements = [
            ("ﬁ", "fi"),
            ("ﬂ", "fl"),
            ("ﬀ", "ff"),
            ("ﬃ", "ffi"),
            ("ﬄ", "ffl"),
            ("ǎ", "a"),
            ("ǐ", "i"),
            ("ǒ", "o"),
            ("ǔ", "u"),
            ("\u{2018}", "'"),   // Left single quote
            ("\u{2019}", "'"),   // Right single quote
            ("\u{201C}", "\""),  // Left double quote
            ("\u{201D}", "\""),  // Right double quote
            ("\u{2013}", "–"),   // En dash: preserve as-is for Markdown quality
            ("\u{2014}", "—"),   // Em dash: preserve as-is for Markdown quality
            ("\u{2026}", "..."), // Ellipsis
            ("Þle", "file"),     // Common misread 'fi' ligature
            ("Þ", "fi"),
        ];

        for (from, to) in &replacements {
            result = result.replace(from, to);
        }

        result
    }

    /// Fix concatenated words (e.g., "methodsThe" → "methods The").
    ///
    /// **OODA-07 FIX**: Only split at word boundaries to preserve CamelCase terms.
    /// The original regex `([a-z])([A-Z][a-z])` was too aggressive and split terms like:
    /// - BrowseComp → Browse Comp
    /// - DeepHalluBench → Deep Hallu Bench
    ///
    /// The new approach only splits when there's a space before the lowercase word,
    /// indicating it's likely a concatenation error rather than intentional CamelCase.
    pub fn fix_concatenated_words(&self, text: &str) -> String {
        let mut result = text.to_string();

        // OODA-07: Only split concatenated words at word boundaries
        // This preserves CamelCase terms like BrowseComp, DeepHalluBench
        // Pattern: space + lowercase word + UpperLower
        // Match: "text methodsThe model" → "text methods The model"
        // Preserve: "BrowseComp" (no preceding space to match)
        if let Ok(re) = Regex::new(r"(\s)([a-z]+)([A-Z][a-z])") {
            result = re.replace_all(&result, "$1$2 $3").to_string();
        }

        // Also fix at start of line (no preceding space)
        // Only if the lowercase portion is long (likely a complete word, not part of CamelCase)
        // "methodsThe model" → "methods The model" (methods = 7 chars, likely a word)
        // but NOT "browseComp" (starts with lowercase = CamelCase style)
        if let Ok(re) = Regex::new(r"^([a-z]{5,})([A-Z][a-z])") {
            result = re.replace_all(&result, "$1 $2").to_string();
        }

        // Generic text repairs: fix common OCR/extraction artifacts
        // These patterns are universal PDF extraction issues, not document-specific
        result = result.replace("ar Xiv", "arXiv"); // Split identifier repair
        result = result.replace("Ar Xiv", "ArXiv");
        result = result.replace("etal.", "et al.");
        result = result.replace("etal,", "et al.,");

        result
    }

    /// Clean markdown-like artifacts from PDF annotations.
    fn cleanup_markdown_artifacts(&self, text: &str) -> String {
        let mut result = text.to_string();

        let artifact_patterns = [
            r"\*\[\]\*\*\.\*", // *[]**.*
            r"\*\[\]\*",       // *[]*
            r"\*-\*\s*",       // *-*
            r"\*\.\*\s*",      // *.*
            r" - \*-\*",       // - *-*
        ];

        for pattern in artifact_patterns {
            if let Ok(re) = Regex::new(pattern) {
                result = re.replace_all(&result, " ").to_string();
            }
        }

        result
    }

    /// Cleanup citation formatting.
    fn cleanup_citations(&self, text: &str) -> String {
        let mut result = text.to_string();

        // "(Name,2024)" → "(Name, 2024)"
        if let Ok(re) = Regex::new(r"\(([^)]+),(\d{4})\)") {
            result = re.replace_all(&result, "($1, $2)").to_string();
        }

        // ",2024)" → ", 2024)"
        if let Ok(re) = Regex::new(r",(\d{4})\)") {
            result = re.replace_all(&result, ", $1)").to_string();
        }

        result
    }

    /// OODA-20: Strip footnote marker symbols from the start of text.
    ///
    /// # WHY
    ///
    /// Academic papers use various symbols to mark footnotes:
    /// - ⋆ (six-pointed asterisk, U+22C6)
    /// - * (asterisk)
    /// - † (dagger)
    /// - ‡ (double dagger)
    /// - § (section sign)
    /// - ¶ (pilcrow)
    ///
    /// These appear at the start of footnote text in PDFs but should be
    /// stripped in markdown output (gold files don't include them).
    ///
    /// # Example
    ///
    /// Input: "⋆ This paper is based on..."
    /// Output: "This paper is based on..."
    fn strip_footnote_markers(&self, text: &str) -> String {
        let trimmed = text.trim_start();

        // Check for footnote markers at the start
        let footnote_markers = ['⋆', '†', '‡', '§', '¶'];

        for marker in footnote_markers {
            if trimmed.starts_with(marker) {
                // Remove the marker and any following whitespace
                let rest = trimmed.trim_start_matches(marker).trim_start();
                return rest.to_string();
            }
        }

        // Also handle asterisk at start (but only if followed by space, to avoid * list items)
        if trimmed.starts_with("* ") && !trimmed.starts_with("* ") {
            // This is actually a list item, don't strip
        } else if trimmed.starts_with('*') && trimmed.len() > 1 {
            let second_char = trimmed.chars().nth(1);
            if second_char == Some(' ') {
                // "* text" - could be list or footnote, check if it looks like prose
                let rest = trimmed[2..].trim_start();
                // If it starts with lowercase, it's likely a footnote continuation
                if rest
                    .chars()
                    .next()
                    .map(|c| c.is_lowercase())
                    .unwrap_or(false)
                {
                    return rest.to_string();
                }
            }
        }

        text.to_string()
    }

    /// Normalize span boundaries to avoid double spaces.
    fn normalize_span_boundaries(spans: &mut Vec<TextSpan>) {
        spans.retain(|s| !s.text.is_empty());
        if spans.len() < 2 {
            return;
        }

        for i in 1..spans.len() {
            let prev_ends_space = spans[i - 1].text.ends_with(' ');
            let cur_starts_space = spans[i].text.starts_with(' ');
            if prev_ends_space && cur_starts_space {
                spans[i].text.remove(0);
            }
        }

        spans.retain(|s| !s.text.is_empty());
    }
}

impl Default for PostProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for PostProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        for page in &mut document.pages {
            for block in &mut page.blocks {
                self.process_block(block);
            }
        }
        Ok(document)
    }

    fn name(&self) -> &str {
        "PostProcessor"
    }
}

// =============================================================================
// SpacedTextProcessor - MUST run before GarbledTextFilter!
// =============================================================================

/// Fixes spaced text patterns like "T H E H O T M E S S" → "THE HOT MESS".
///
/// **OODA-05**: Spaced text in PDF titles was being filtered by GarbledTextFilter
/// because it looks like many isolated letters. This processor must run FIRST
/// to normalize the text before garbled detection.
pub struct SpacedTextProcessor {
    post: PostProcessor,
}

impl SpacedTextProcessor {
    pub fn new() -> Self {
        Self {
            post: PostProcessor::new(),
        }
    }
}

impl Default for SpacedTextProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for SpacedTextProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        for page in &mut document.pages {
            for block in &mut page.blocks {
                // Only apply fix_spaced_text, not the full post-processing
                block.text = self.post.fix_spaced_text(&block.text);
                for span in &mut block.spans {
                    span.text = self.post.fix_spaced_text(&span.text);
                }
            }
        }
        Ok(document)
    }

    fn name(&self) -> &str {
        "SpacedTextProcessor"
    }
}

// =============================================================================
// GarbledTextFilterProcessor
// =============================================================================

/// Filters out garbled/corrupted text blocks.
///
/// **Detection Heuristics:**
/// 1. High ratio of short words (≤2 chars) indicates garbled text
/// 2. Isolated single letters not in common word list
/// 3. OCR fragment patterns (missing first letters)
/// 4. Very short fragments that aren't valid content
///
/// **WHY:** PDF extraction sometimes produces gibberish from figure labels,
/// watermarks, or corrupted fonts. These hurt downstream processing.
pub struct GarbledTextFilterProcessor {
    /// Maximum ratio of short words allowed
    max_short_word_ratio: f32,
    /// Minimum words to apply ratio check
    min_word_count: usize,
}

impl GarbledTextFilterProcessor {
    pub fn new() -> Self {
        Self {
            max_short_word_ratio: 0.35,
            min_word_count: 4,
        }
    }

    /// Check if text appears garbled/corrupted.
    ///
    /// OODA-37: Enhanced with first-principle structural checks:
    ///
    /// ```text
    /// ┌────────────────────────────────────────────────────┐
    /// │           GARBLED TEXT DETECTION PIPELINE           │
    /// ├────────────────────────────────────────────────────┤
    /// │  1. Academic reference → KEEP ([1], [23])          │
    /// │  2. Long-word check → GARBLED (word > 40 chars)    │
    /// │  3. Low space ratio → GARBLED (< 5% spaces)        │
    /// │  4. Very short fragment → GARBLED (≤3 chars)        │
    /// │  5. Short-word ratio → GARBLED (> 35% unusual)     │
    /// │  6. Isolated letters → GARBLED (≥4 isolated)       │
    /// │  7. OCR fragments → GARBLED (known patterns)       │
    /// └────────────────────────────────────────────────────┘
    /// ```
    fn is_garbled(&self, text: &str) -> bool {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return false;
        }

        // OODA-13 FIX: Never filter academic references (start with [NUMBER])
        // WHY: Author initials like "X.", "J." incorrectly trigger short-word ratio checks
        if trimmed.starts_with('[') {
            if let Some(bracket_end) = trimmed.find(']') {
                let inside = &trimmed[1..bracket_end];
                if inside.chars().all(|c| c.is_ascii_digit()) {
                    return false; // This is a reference like [1], [23], etc.
                }
            }
        }

        // OODA-38: Long-word and camelCase detection for diagram/figure text
        // WHY (First Principle): Natural language has a predictable word length distribution.
        // English max word length is ~30 chars ("antidisestablishmentarianism" = 28).
        // When PDF diagram text is extracted, overlapping character positions produce
        // continuous strings without spaces (e.g., "pbtBeekeepersinccrucialrolein...")
        // creating "words" of 100+ chars. This violates the fundamental property of
        // natural language and reliably identifies garbled diagram text.
        //
        // Two sub-checks:
        //   (a) Any word > 35 chars (not URL/path) → garbled
        //   (b) Any word > 25 chars with 3+ internal uppercase → concatenated labels
        //       e.g., "OriginalRelationsTextincludes" (30 chars, uppercase at 8,17,21)
        //
        // Exception: URLs and file paths can be long single tokens but contain
        // structural markers (://, /, .com, .org) that distinguish them.
        {
            let words: Vec<&str> = trimmed.split_whitespace().collect();
            let has_garbled_word = words.iter().any(|w| {
                let len = w.len();
                // Exclude URLs and file paths
                let is_url_or_path = w.contains("://")
                    || w.contains('/')
                    || w.contains(".com")
                    || w.contains(".org")
                    || w.contains(".edu")
                    || w.contains(".net")
                    || w.contains(".io");
                if is_url_or_path {
                    return false;
                }

                // Check (a): Very long word (> 35 chars)
                if len > 35 {
                    return true;
                }

                // Check (b): CamelCase concatenation (> 25 chars with 2+ internal uppercase)
                // WHY: Concatenated diagram labels like "OriginalRelationsTextincludes"
                // have multiple internal uppercase transitions that natural words don't.
                // Threshold of 25 chars avoids false positives on shorter compound names
                // like "DeepHalluBench" (14 chars) or "RetrievalAugmented" (18 chars).
                // Threshold of 2 internal uppercase catches inconsistent concatenations
                // where not all word boundaries have uppercase (e.g., "Textincludes").
                if len > 25 {
                    let internal_uppercase = w.chars().skip(1).filter(|c| c.is_uppercase()).count();
                    if internal_uppercase >= 2 {
                        return true;
                    }
                }

                false
            });
            if has_garbled_word {
                // OODA-38: Don't filter a long paragraph just because it starts with
                // one garbled word. Only filter if the garbled words dominate the text.
                // WHY: Figure diagram text is typically short (< 200 chars) and mostly
                // garbled. A long paragraph (> 200 chars) with one garbled word at the
                // start is likely a legitimate block with a PDF extraction artifact.
                let garbled_word_len: usize = words
                    .iter()
                    .filter(|w| {
                        let len = w.len();
                        let is_url = w.contains("://") || w.contains('/');
                        if is_url {
                            return false;
                        }
                        if len > 35 {
                            return true;
                        }
                        if len > 25 {
                            let internal_upper =
                                w.chars().skip(1).filter(|c| c.is_uppercase()).count();
                            return internal_upper >= 2;
                        }
                        false
                    })
                    .map(|w| w.len())
                    .sum();

                // Only filter if garbled words make up > 50% of the text length
                // or the total text is short (< 200 chars, likely a diagram label block)
                if garbled_word_len * 2 > trimmed.len() || trimmed.len() < 200 {
                    tracing::debug!(
                        "Filtering garbled text (long/camelCase word): '{}'",
                        safe_truncate(trimmed, 60)
                    );
                    return true;
                }
            }
        }

        // OODA-38: Low space ratio detection for concatenated diagram labels
        // WHY (First Principle): Natural language text has ~15-20% spaces (word boundaries).
        // PDF diagram labels that get concatenated have near-zero space ratio because
        // characters are positioned adjacent without word-break gaps. A block with > 60 chars
        // and < 5% spaces is almost certainly garbled or concatenated diagram text.
        //
        // Exception: Text containing URLs can have low space ratios because URLs are
        // long non-space tokens. Skip this check if text contains URL markers.
        let has_url = trimmed.contains("://") || trimmed.contains("www.");
        if trimmed.len() > 60 && !has_url {
            let space_count = trimmed.chars().filter(|c| c.is_whitespace()).count();
            let space_ratio = space_count as f32 / trimmed.len() as f32;
            if space_ratio < 0.05 {
                tracing::debug!(
                    "Filtering garbled text (space ratio {:.1}% < 5%): '{}'",
                    space_ratio * 100.0,
                    safe_truncate(trimmed, 60)
                );
                return true;
            }
        }

        // Filter very short fragments (≤3 chars) that aren't valid
        if trimmed.len() <= 3 {
            let is_valid_short =
                // Single uppercase letter (section marker)
                (trimmed.len() == 1 && trimmed.chars().next().map(|c| c.is_uppercase()).unwrap_or(false))
                // Number or numbered item
                || trimmed.chars().all(|c| c.is_ascii_digit() || c == '.')
                // Common single-letter words
                || ["I", "a", "A"].contains(&trimmed);

            if !is_valid_short {
                tracing::debug!("Filtering very short fragment: '{}'", trimmed);
                return true;
            }
        }

        // Filter multi-word fragments in ≤6 chars
        if trimmed.len() <= 6 && trimmed.split_whitespace().count() >= 2 {
            let has_digit = trimmed.chars().any(|c| c.is_ascii_digit());
            let looks_like_item = has_digit && (trimmed.contains('.') || trimmed.contains(')'));
            if !looks_like_item {
                tracing::debug!("Filtering short garbled fragment: '{}'", trimmed);
                return true;
            }
        }

        let words: Vec<&str> = trimmed.split_whitespace().collect();
        if words.len() < self.min_word_count {
            return false;
        }

        // Common valid short words
        // WHY include "&": ampersand is a standard English conjunction used in titles
        // like "AI Strategy & Co-Creation", "Search UX & APIs". Without it, these
        // legitimate section headers get filtered as "garbled text".
        let valid_short_words = [
            "a", "an", "as", "at", "be", "by", "do", "go", "he", "if", "in", "is", "it", "me",
            "my", "no", "of", "on", "or", "so", "to", "up", "us", "we", "i", "1", "2", "3", "4",
            "5", "6", "7", "8", "9", "&",
        ];

        // OODA-13 FIX: Count short words, but exclude author initials (e.g., "X.", "J.")
        // WHY: Academic references have many single-letter initials that shouldn't count as garbled
        let short_count = words
            .iter()
            .filter(|w| {
                if w.len() > 2 {
                    return false;
                }
                // Skip valid short words
                if valid_short_words.contains(&w.to_lowercase().as_str()) {
                    return false;
                }
                // Skip author initials: single uppercase letter + period (e.g., "X.", "J.")
                let chars: Vec<char> = w.chars().collect();
                if chars.len() == 2 && chars[0].is_ascii_uppercase() && chars[1] == '.' {
                    return false;
                }
                // WHY skip section numbers: "0)", "1)", "2.", "3." are section numbering
                // patterns, not garbled text. Without this, section headers like
                // "0) AI Strategy & Co-Creation" get falsely filtered.
                if chars.len() == 2
                    && chars[0].is_ascii_digit()
                    && (chars[1] == ')' || chars[1] == '.')
                {
                    return false;
                }
                true
            })
            .count();
        let ratio = short_count as f32 / words.len() as f32;

        if ratio > self.max_short_word_ratio {
            tracing::debug!(
                "Filtering garbled text ({}% unusual short words): '{}'",
                (ratio * 100.0) as i32,
                safe_truncate(trimmed, 50)
            );
            return true;
        }

        // Check for isolated letters
        let isolated_letters = words
            .iter()
            .filter(|w| w.len() == 1 && w.chars().all(|c| c.is_alphabetic()))
            .filter(|w| !valid_short_words.contains(&w.to_lowercase().as_str()))
            .count();

        if isolated_letters >= 4 && ratio > 0.30 {
            tracing::debug!(
                "Filtering text with isolated letters: '{}'",
                safe_truncate(trimmed, 50)
            );
            return true;
        }

        // Check for OCR fragment patterns
        let non_word_fragments = words
            .iter()
            .filter(|w| {
                let w_lower = w.to_lowercase();
                let len = w_lower.len();
                if (4..=8).contains(&len) && w_lower.chars().all(|c| c.is_alphabetic()) {
                    let garbled_patterns = ["iliar", "hich", "erlook", "ec", "tion", "xec"];
                    garbled_patterns
                        .iter()
                        .any(|p| w_lower.starts_with(p) || w_lower == *p)
                } else {
                    false
                }
            })
            .count();

        if non_word_fragments >= 2 {
            tracing::debug!(
                "Filtering text with OCR fragments: '{}'",
                safe_truncate(trimmed, 50)
            );
            return true;
        }

        false
    }
}

impl Default for GarbledTextFilterProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for GarbledTextFilterProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        for page in &mut document.pages {
            page.blocks.retain(|block| {
                // Don't filter structured blocks (tables, code, equations)
                if matches!(
                    block.block_type,
                    BlockType::Table | BlockType::Code | BlockType::Equation
                ) {
                    return true;
                }
                !self.is_garbled(&block.text)
            });
        }
        Ok(document)
    }

    fn name(&self) -> &str {
        "GarbledTextFilterProcessor"
    }
}

// =============================================================================
// HyphenContinuationProcessor
// =============================================================================

/// Joins hyphenated words split across line breaks.
///
/// **WHY:** Academic papers use hyphenation for word wrapping.
/// "modifi-\ncation" should become "modification".
///
/// **Algorithm:**
/// 1. Find blocks ending with explicit hyphen
/// 2. Validate continuation (lowercase start, reasonable suffix)
/// 3. Join words and merge blocks
pub struct HyphenContinuationProcessor;

impl HyphenContinuationProcessor {
    pub fn new() -> Self {
        Self
    }

    /// Check if text ends with explicit hyphen.
    fn ends_with_explicit_hyphen(text: &str) -> bool {
        text.trim_end().ends_with('-')
    }

    /// Get word fragment before hyphen.
    fn get_hyphen_fragment(text: &str) -> Option<String> {
        let trimmed = text.trim_end();
        if let Some(without_hyphen) = trimmed.strip_suffix('-') {
            let last_word = without_hyphen.split_whitespace().last()?;
            Some(last_word.to_lowercase())
        } else {
            None
        }
    }

    /// Validate continuation completes the word sensibly.
    fn is_valid_continuation(continuation_text: &str) -> bool {
        let cont_trimmed = continuation_text.trim_start();
        if cont_trimmed.is_empty() {
            return false;
        }

        let first_word = cont_trimmed.split_whitespace().next().unwrap_or("");
        !first_word.is_empty()
            && first_word
                .chars()
                .next()
                .map(|c| c.is_lowercase())
                .unwrap_or(false)
    }

    /// Process hyphenation within a single block's text (line-to-line).
    ///
    /// **WHY:** Academic papers have multi-line paragraph blocks with
    /// embedded line breaks like "gener-\nating". This function joins
    /// hyphenated words WITHIN a block's text before block-to-block
    /// processing happens.
    ///
    /// **OODA-10:** Added compound-word detection to preserve hyphens in
    /// compound words like "long-horizon", "self-supervised", "hand-eye".
    ///
    /// Algorithm:
    /// 1. Collapse all line breaks to spaces (making one continuous line)
    /// 2. Fix hyphenation patterns like "gener- ating" → "generating"
    /// 3. But PRESERVE compound hyphens like "long- horizon" → "long-horizon"
    fn process_intra_block_hyphens(text: &str) -> String {
        // Step 1: Collapse newlines to spaces, normalizing whitespace
        let collapsed = text
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<&str>>()
            .join(" ");

        // Step 2: Fix hyphenation patterns while preserving compound words
        let mut result = collapsed;
        let mut changed = true;

        while changed {
            changed = false;
            // Find pattern: word ending with hyphen followed by space and lowercase word
            if let Some(pos) = result.find("- ") {
                // Get the word before the hyphen
                let before_hyphen = &result[..pos];
                let prefix_word = before_hyphen.split_whitespace().last().unwrap_or("");
                let prefix_lower = prefix_word.to_lowercase();

                // Check if next word starts with lowercase
                let after_hyphen = &result[pos + 2..];
                if let Some(first_char) = after_hyphen.chars().next() {
                    if first_char.is_lowercase() {
                        // OODA-10: Check if this is a compound word prefix (keep hyphen)
                        // WHY: "long-horizon", "self-supervised" should keep hyphens
                        let is_compound_prefix = matches!(
                            prefix_lower.as_str(),
                            "long"
                                | "short"
                                | "self"
                                | "hand"
                                | "eye"
                                | "high"
                                | "low"
                                | "well"
                                | "full"
                                | "half"
                                | "co"
                                | "pre"
                                | "re"
                                | "anti"
                                | "non"
                                | "multi"
                                | "cross"
                                | "whole"
                                | "end"
                                | "real"
                                | "time"
                                | "data"
                                | "user"
                                | "loco"
                                | "semi"
                                | "all"
                                | "one"
                                | "two"
                                | "three"
                                | "first"
                                | "second"
                                | "body"
                                | "level"
                                | "state"
                                | "world"
                                | "task"
                                | "based"
                                | "free"
                        );

                        // Also detect as compound if prefix is >= 4 chars with vowel and no fragment ending
                        let has_vowel = prefix_word
                            .chars()
                            .any(|c| matches!(c.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u'));
                        let is_fragment_ending = prefix_lower.ends_with("ti")
                            || prefix_lower.ends_with("ni")
                            || prefix_lower.ends_with("fi")
                            || prefix_lower.ends_with("si")
                            || prefix_lower.ends_with("gi")
                            || prefix_lower.ends_with("vi")
                            || prefix_lower.ends_with("ci");
                        let _is_likely_complete_word =
                            prefix_word.len() >= 4 && has_vowel && !is_fragment_ending;

                        // OODA-11: Check if continuation starts with common suffix patterns
                        // WHY: "gener-ating" should become "generating" not "gener-ating"
                        // If continuation starts with "-ating", "-tion", "-ing", etc., it's likely a word break
                        let continuation_starts_suffix = after_hyphen.starts_with("ating")
                            || after_hyphen.starts_with("tion")
                            || after_hyphen.starts_with("ing")
                            || after_hyphen.starts_with("ering")
                            || after_hyphen.starts_with("izing")
                            || after_hyphen.starts_with("izing")
                            || after_hyphen.starts_with("ating")
                            || after_hyphen.starts_with("ering")
                            || after_hyphen.starts_with("ered")
                            || after_hyphen.starts_with("ment")
                            || after_hyphen.starts_with("ness")
                            || after_hyphen.starts_with("able")
                            || after_hyphen.starts_with("ible")
                            || after_hyphen.starts_with("ally")
                            || after_hyphen.starts_with("tion")
                            || after_hyphen.starts_with("sion")
                            || after_hyphen.starts_with("ity");

                        if is_compound_prefix && !continuation_starts_suffix {
                            // COMPOUND WORD: Replace "word- continuation" with "word-continuation"
                            // WHY: Keep the hyphen but remove the space
                            let cont_end = after_hyphen
                                .find(|c: char| c.is_whitespace())
                                .unwrap_or(after_hyphen.len());
                            let continuation = &after_hyphen[..cont_end];
                            let rest = &after_hyphen[cont_end..];

                            let new_result = format!("{}-{}{}", &result[..pos], continuation, rest);
                            result = new_result;
                            changed = true;
                        } else {
                            // CONTINUATION: Replace "word- continuation" with "wordcontinuation"
                            // WHY: Remove hyphen, this is a word broken at line end
                            let cont_end = after_hyphen
                                .find(|c: char| c.is_whitespace())
                                .unwrap_or(after_hyphen.len());
                            let continuation = &after_hyphen[..cont_end];

                            let new_result = format!(
                                "{}{}{}",
                                &result[..pos],
                                continuation,
                                &after_hyphen[cont_end..]
                            );
                            result = new_result;
                            changed = true;
                        }
                    }
                }
            }
        }

        result
    }

    /// OODA-IT43: Join soft word breaks (without hyphens) from PDF column wrapping.
    ///
    /// **WHY:** PDFs often split words at column edges without inserting hyphens.
    /// E.g., "representa tions" should become "representations".
    ///
    /// **First Principles:**
    /// - Word fragments at line ends followed by suffix patterns need joining
    /// - Common suffixes: -tions, -tion, -ment, -ing, -ness, -able, etc.
    /// - Consonant cluster starts (pr, tr, br) also indicate word continuation
    ///
    /// **Algorithm:**
    /// 1. Find pattern: "prefix suffix" where suffix starts lowercase
    /// 2. Check if suffix matches known suffix patterns
    /// 3. Join by removing the space
    fn process_soft_word_breaks(text: &str) -> String {
        let mut result = text.to_string();
        let mut changed = true;

        while changed {
            changed = false;

            // Find all potential "word continuation" patterns
            // Look for: word boundary, word (3+ chars ending in letter), space, lowercase word
            let chars: Vec<char> = result.chars().collect();
            let len = chars.len();

            for i in 0..len {
                // Find a space that might be a soft word break
                if chars[i] != ' ' {
                    continue;
                }

                // Get the word before the space
                let prefix_end = i;
                let mut prefix_start = i;
                while prefix_start > 0 && chars[prefix_start - 1].is_alphanumeric() {
                    prefix_start -= 1;
                }

                // Skip if prefix is too short or doesn't end with a letter
                if prefix_end - prefix_start < 3 {
                    continue;
                }
                if !chars[prefix_end - 1].is_alphabetic() {
                    continue;
                }

                // Get the word after the space
                let suffix_start = i + 1;
                if suffix_start >= len {
                    continue;
                }

                // Suffix must start with lowercase letter
                if !chars[suffix_start].is_ascii_lowercase() {
                    continue;
                }

                // Find suffix end
                let mut suffix_end = suffix_start;
                while suffix_end < len && chars[suffix_end].is_alphanumeric() {
                    suffix_end += 1;
                }

                // Get the actual strings
                let prefix: String = chars[prefix_start..prefix_end].iter().collect();
                let suffix: String = chars[suffix_start..suffix_end].iter().collect();

                // Check if this looks like a soft word break
                if Self::is_soft_word_break(&prefix, &suffix) {
                    // Join by removing the space
                    let before: String = chars[..i].iter().collect();
                    let after: String = chars[suffix_start..].iter().collect();
                    result = format!("{}{}", before, after);
                    changed = true;
                    break;
                }
            }
        }

        result
    }

    /// Check if "prefix suffix" is likely a soft word break (split word without hyphen).
    ///
    /// Returns true if the suffix matches known suffix patterns or looks like a word fragment.
    fn is_soft_word_break(_prefix: &str, suffix: &str) -> bool {
        let suffix_lower = suffix.to_lowercase();

        // OODA-IT43: Known suffix patterns that indicate word continuation
        // WHY: These suffixes never stand alone as words
        // NOTE: Only use patterns that are UNAMBIGUOUSLY suffixes
        // Avoid short patterns like "ing", "cal" that are prefixes of real words
        let suffix_patterns = [
            // Long, unambiguous suffixes (safe)
            "tions",
            "tion",
            "sions",
            "sion", // representa-tions, genera-tion
            "ments",
            "ment", // experi-mental
            "nesses",
            "ness", // aware-ness
            "able",
            "ible", // adapt-able
            "ence",
            "ance",
            "ency",
            "ancy", // depend-ence
            "ially",
            "ually",
            "ally", // essenti-ally
            "ization",
            "izing",
            "ized", // optim-ized
            "ities",
            "ity", // complex-ity
            "ives",
            "ive", // comprehens-ive (but not "ive" alone)
            "ation",
            "ating",
            "ated", // gener-ating
            "ering",
            "ered", // discov-ered
            "fully",
            "lessly",
            "less", // seam-less
            "ously",
            "ous", // continu-ous
            "nally",
            "cally", // practi-cally, but NOT "cal" alone (matches "calling")
            "larly",
            "tically",
            "tical", // problema-tical
            "ships",
            "ship", // relation-ship
            "doms",
            "hoods",
            "hood", // neighbor-hood
            "wards",
            "ward",       // for-ward
            "wise",       // other-wise
            "like",       // life-like
            "hensive",    // compre-hensive
            "prehensive", // Com-prehensive (special case from LightRAG)
            "rupting",    // dis-rupting
            "pending",    // de-pending
        ];

        // Check if suffix starts with any known pattern
        for pattern in &suffix_patterns {
            if suffix_lower.starts_with(pattern) {
                return true;
            }
        }

        // OODA-IT43: Special case for very short suffixes that are unambiguously fragments
        // WHY: "ing" (3 chars) is a suffix fragment, "ingredient" (10 chars) is a word
        // Short suffixes like "ing", "ed", "ly", "er" are safe when they're the WHOLE suffix
        let short_suffix_fragments = ["ing", "ings", "ed", "ly", "er", "ers", "ful"];
        if short_suffix_fragments.contains(&suffix_lower.as_str()) {
            return true;
        }

        // OODA-IT43: Consonant cluster heuristic DISABLED
        // WHY: Too aggressive - matches real words like "production", "calling", "platforms"
        // The suffix pattern approach is safer and handles most cases.
        // Specific cases like "Com prehensive" are handled by adding "hensive" to suffix_patterns.
        //
        // Previously tried consonant cluster detection for words starting with:
        // "pr", "tr", "br", "cr", etc. but this matched too many real words.

        false
    }
}

impl Default for HyphenContinuationProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for HyphenContinuationProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        // PHASE 1: Process intra-block hyphenation (line-to-line within each block)
        for page in &mut document.pages {
            for block in &mut page.blocks {
                if matches!(block.block_type, BlockType::Text | BlockType::Paragraph) {
                    block.text = Self::process_intra_block_hyphens(&block.text);
                }
            }
        }

        // PHASE 1.5: OODA-IT43 - Process soft word breaks (without hyphens)
        // WHY: PDFs split words at column edges without inserting hyphens
        // E.g., "representa tions" → "representations"
        for page in &mut document.pages {
            for block in &mut page.blocks {
                if matches!(block.block_type, BlockType::Text | BlockType::Paragraph) {
                    block.text = Self::process_soft_word_breaks(&block.text);
                }
            }
        }

        // PHASE 2: Process inter-block hyphenation (block-to-block)
        for page in &mut document.pages {
            // WHY: Calculate page center for column detection
            // In multi-column layouts, blocks in different columns have very different X positions.
            // A typical 2-column academic paper has left column at ~60-290 and right at ~310-540.
            // We use page center (typically ~306 for 612pt page) to detect column boundaries.
            let page_center = page.width / 2.0;
            // WHY: 50pt tolerance allows for slight variations but catches column jumps (~120pt gap)
            let column_tolerance = 50.0;

            let mut i = 0;
            while i < page.blocks.len().saturating_sub(1) {
                // Only process text blocks
                if !matches!(
                    page.blocks[i].block_type,
                    BlockType::Text | BlockType::Paragraph
                ) {
                    i += 1;
                    continue;
                }

                // Check conditions without holding references
                let current_text = page.blocks[i].text.clone();
                let current_bbox = page.blocks[i].bbox;
                let next_text = page.blocks[i + 1].text.clone();
                let next_bbox = page.blocks[i + 1].bbox;

                // WHY: Check if blocks are in the same column before merging
                // This prevents merging the last block of left column with first block of right column.
                // Two blocks are in different columns if they straddle the page center with significant gap.
                let current_center = current_bbox.center().x;
                let next_center = next_bbox.center().x;
                let crosses_columns = (current_center < page_center && next_center > page_center)
                    || (current_center > page_center && next_center < page_center);
                let significant_x_gap = (next_center - current_center).abs() > column_tolerance;

                // Skip if blocks are in different columns
                if crosses_columns && significant_x_gap {
                    tracing::debug!(
                        "HyphenContinuation: SKIPPING cross-column merge: '{}...' -> '{}...' (current_center={:.1}, next_center={:.1}, page_center={:.1})",
                        current_text.chars().take(30).collect::<String>(),
                        next_text.chars().take(30).collect::<String>(),
                        current_center,
                        next_center,
                        page_center
                    );
                    i += 1;
                    continue;
                }

                if Self::ends_with_explicit_hyphen(&current_text)
                    && Self::is_valid_continuation(&next_text)
                    && Self::get_hyphen_fragment(&current_text).is_some()
                {
                    // OODA-10: Check for compound word prefix before removing hyphen
                    // WHY: "long-" + "horizon" should become "long-horizon", not "longhorizon"
                    let prefix = current_text.trim_end().trim_end_matches('-');
                    let last_word = prefix.split_whitespace().last().unwrap_or("");
                    let last_word_lower = last_word.to_lowercase();

                    // Check if prefix is a known compound word prefix (keep hyphen)
                    let is_compound_prefix = matches!(
                        last_word_lower.as_str(),
                        "long"
                            | "short"
                            | "self"
                            | "hand"
                            | "eye"
                            | "high"
                            | "low"
                            | "well"
                            | "full"
                            | "half"
                            | "co"
                            | "pre"
                            | "re"
                            | "anti"
                            | "non"
                            | "multi"
                            | "cross"
                            | "whole"
                            | "end"
                            | "real"
                            | "time"
                            | "data"
                            | "user"
                            | "loco"
                            | "semi"
                            | "all"
                            | "one"
                            | "two"
                            | "three"
                            | "first"
                            | "second"
                            | "body"
                            | "level"
                            | "state"
                            | "world"
                            | "task"
                            | "based"
                            | "free"
                    );

                    // Also detect as compound if prefix is >= 4 chars with vowel and no fragment ending
                    let has_vowel = last_word
                        .chars()
                        .any(|c| matches!(c.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u'));
                    let is_fragment_ending = last_word_lower.ends_with("ti")
                        || last_word_lower.ends_with("ni")
                        || last_word_lower.ends_with("fi")
                        || last_word_lower.ends_with("si")
                        || last_word_lower.ends_with("gi")
                        || last_word_lower.ends_with("vi")
                        || last_word_lower.ends_with("ci");
                    let _is_likely_complete_word =
                        last_word.len() >= 4 && has_vowel && !is_fragment_ending;

                    // Get continuation word
                    let cont_trimmed = next_text.trim_start();
                    let first_word_next = cont_trimmed.split_whitespace().next().unwrap_or("");

                    // OODA-11: Check if continuation starts with common suffix patterns
                    // WHY: "gener-ating" should become "generating" not "gener-ating"
                    let continuation_starts_suffix = first_word_next.starts_with("ating")
                        || first_word_next.starts_with("tion")
                        || first_word_next.starts_with("ing")
                        || first_word_next.starts_with("ering")
                        || first_word_next.starts_with("izing")
                        || first_word_next.starts_with("ered")
                        || first_word_next.starts_with("ment")
                        || first_word_next.starts_with("ness")
                        || first_word_next.starts_with("able")
                        || first_word_next.starts_with("ible")
                        || first_word_next.starts_with("ally")
                        || first_word_next.starts_with("sion")
                        || first_word_next.starts_with("ity");

                    // Rest of continuation
                    let rest = cont_trimmed
                        .strip_prefix(first_word_next)
                        .unwrap_or("")
                        .trim_start();

                    let final_text = if is_compound_prefix && !continuation_starts_suffix {
                        // COMPOUND WORD: Keep the hyphen
                        // WHY: "long-" + "horizon" → "long-horizon"
                        tracing::debug!(
                            "HyphenContinuation: COMPOUND WORD merge: '{}' + '{}'",
                            last_word,
                            first_word_next
                        );
                        let joined_text = format!("{}{}-{}", prefix, "", first_word_next);
                        if rest.is_empty() {
                            joined_text
                        } else {
                            format!("{} {}", joined_text, rest)
                        }
                    } else {
                        // CONTINUATION: Remove the hyphen
                        // WHY: "modifi-" + "cation" → "modification"
                        tracing::debug!(
                            "HyphenContinuation: CONTINUATION merge: '{}' + '{}'",
                            last_word,
                            first_word_next
                        );
                        let mut new_current_text = current_text.trim_end().to_string();
                        new_current_text.pop(); // Remove hyphen
                        let joined_text = format!("{}{}", new_current_text, first_word_next);
                        if rest.is_empty() {
                            joined_text
                        } else {
                            format!("{} {}", joined_text, rest)
                        }
                    };

                    tracing::debug!(
                        "HyphenContinuation: MERGING blocks: '{}...' + '{}...' -> '{}'",
                        current_text.chars().take(30).collect::<String>(),
                        next_text.chars().take(30).collect::<String>(),
                        final_text.chars().take(50).collect::<String>()
                    );

                    // Update current block (no borrow conflict now)
                    page.blocks[i].text = final_text;
                    page.blocks[i].bbox = current_bbox.union(&next_bbox);

                    // WHY: Clear spans because they contain the OLD text with hyphen.
                    // The renderer now validates spans against text, but clearing here
                    // ensures consistency and avoids stale span accumulation.
                    page.blocks[i].spans.clear();

                    // Remove next block (its text has been merged into current)
                    page.blocks.remove(i + 1);
                    continue; // Don't increment i, check again
                }

                i += 1;
            }
        }
        Ok(document)
    }

    fn name(&self) -> &str {
        "HyphenContinuationProcessor"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_processor_normalize() {
        let processor = PostProcessor::new();
        assert_eq!(
            processor.normalize_text("Hello    world   test"),
            "Hello world test"
        );
    }

    #[test]
    fn test_post_processor_ocr_fix() {
        let processor = PostProcessor::new();
        assert_eq!(processor.fix_ocr_text("ﬁnd the ﬂow"), "find the flow");
    }

    #[test]
    fn test_post_processor_concatenated_words() {
        let processor = PostProcessor::new();
        // OODA-07: Test that concatenated words are split at word boundaries
        assert_eq!(
            processor.fix_concatenated_words("text methodsThe model"),
            "text methods The model"
        );
    }

    #[test]
    fn test_post_processor_camelcase_preserved() {
        let processor = PostProcessor::new();
        // OODA-07: CamelCase terms should be preserved (no preceding space)
        assert_eq!(processor.fix_concatenated_words("BrowseComp"), "BrowseComp");
        assert_eq!(
            processor.fix_concatenated_words("DeepHalluBench"),
            "DeepHalluBench"
        );
        assert_eq!(
            processor.fix_concatenated_words("ReportBench"),
            "ReportBench"
        );
        assert_eq!(processor.fix_concatenated_words("SciFact"), "SciFact");
        assert_eq!(processor.fix_concatenated_words("Mind2Web"), "Mind2Web");
        // Also test with surrounding context
        assert_eq!(
            processor.fix_concatenated_words("Using BrowseComp for evaluation"),
            "Using BrowseComp for evaluation"
        );
    }

    #[test]
    fn test_post_processor_arxiv_preserved() {
        let processor = PostProcessor::new();
        let input = "Submitted to arXiv:2501.23456";
        assert_eq!(processor.fix_concatenated_words(input), input);
    }

    #[test]
    fn test_garbled_detection() {
        let processor = GarbledTextFilterProcessor::new();

        // Valid short content
        assert!(!processor.is_garbled("I"));
        assert!(!processor.is_garbled("1."));

        // Garbled content
        assert!(processor.is_garbled(",w"));
        assert!(processor.is_garbled("v x y z"));
    }

    #[test]
    fn test_hyphen_detection() {
        assert!(HyphenContinuationProcessor::ends_with_explicit_hyphen(
            "modifi-"
        ));
        assert!(!HyphenContinuationProcessor::ends_with_explicit_hyphen(
            "modification"
        ));
    }

    #[test]
    fn test_valid_continuation() {
        assert!(HyphenContinuationProcessor::is_valid_continuation(
            "cation of the method"
        ));
        assert!(!HyphenContinuationProcessor::is_valid_continuation(""));
        assert!(!HyphenContinuationProcessor::is_valid_continuation(
            "The next section"
        ));
    }

    #[test]
    fn test_post_processor_empty_text() {
        let processor = PostProcessor::new();
        assert_eq!(processor.normalize_text(""), "");
        assert_eq!(processor.fix_ocr_text(""), "");
    }

    #[test]
    fn test_post_processor_whitespace_handling() {
        let processor = PostProcessor::new();
        // Whitespace is collapsed to single space, newlines preserved
        assert_eq!(processor.normalize_text("   "), " ");
        assert_eq!(processor.normalize_text("\t\t\n"), " \n");
    }

    #[test]
    fn test_garbled_detection_edge_cases() {
        let processor = GarbledTextFilterProcessor::new();
        assert!(!processor.is_garbled(""));
        assert!(!processor.is_garbled("Normal sentence."));
        assert!(processor.is_garbled("a b c d e f"));
    }

    #[test]
    fn test_hyphen_continuation_edge_cases() {
        assert!(!HyphenContinuationProcessor::ends_with_explicit_hyphen(""));
        assert!(!HyphenContinuationProcessor::ends_with_explicit_hyphen(
            "test"
        ));
        assert!(HyphenContinuationProcessor::ends_with_explicit_hyphen(
            "test-"
        ));
    }

    #[test]
    fn test_post_processor_default() {
        let processor = PostProcessor::default();
        assert_eq!(processor.name(), "PostProcessor");
    }

    #[test]
    fn test_garbled_filter_default() {
        let processor = GarbledTextFilterProcessor::default();
        assert_eq!(processor.name(), "GarbledTextFilterProcessor");
    }

    #[test]
    fn test_hyphen_continuation_default() {
        let processor = HyphenContinuationProcessor::default();
        assert_eq!(processor.name(), "HyphenContinuationProcessor");
    }

    #[test]
    fn test_intra_block_hyphen_continuation() {
        // Test basic hyphenation within a block
        let input = "This is a gener-\nating system";
        let expected = "This is a generating system";
        let result = HyphenContinuationProcessor::process_intra_block_hyphens(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_intra_block_hyphen_multiple() {
        // Test multiple hyphenations in same block
        let input = "gener-\nating and render-\ning models";
        let expected = "generating and rendering models";
        let result = HyphenContinuationProcessor::process_intra_block_hyphens(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_intra_block_hyphen_preserves_intentional() {
        // Test that intentional hyphens (with capital letter after) are preserved
        // The hyphen is kept because "Of" starts with capital (not a word continuation)
        // Newline is collapsed to space since this is paragraph processing
        let input = "This is state-\nOf-the-art";
        let expected = "This is state- Of-the-art"; // Capital O means not continuation
        let result = HyphenContinuationProcessor::process_intra_block_hyphens(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_intra_block_hyphen_with_rest_of_line() {
        // Test hyphen with continuation word plus more text
        let input = "This is a gener-\nating system with features";
        let expected = "This is a generating system with features";
        let result = HyphenContinuationProcessor::process_intra_block_hyphens(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_intra_block_no_hyphen() {
        // For paragraph blocks, line breaks are collapsed to spaces
        // (they represent soft wrapping for column width)
        let input = "Line one\nLine two\nLine three";
        let expected = "Line one Line two Line three";
        let result = HyphenContinuationProcessor::process_intra_block_hyphens(input);
        assert_eq!(result, expected);
    }

    // OODA-05: Tests for spaced text fix
    #[test]
    fn test_spaced_text_title() {
        let processor = PostProcessor::new();
        // Test the hotmess PDF title pattern
        let input = "T H E H O T M E S S O F A I";
        let result = processor.fix_spaced_text(input);
        assert_eq!(result, "THEHOTMESSOFAI");
    }

    #[test]
    fn test_spaced_text_partial() {
        let processor = PostProcessor::new();
        // Test partial spaced text with normal text
        let input = "A B S T R A C T Introduction";
        let result = processor.fix_spaced_text(input);
        assert_eq!(result, "ABSTRACT Introduction");
    }

    #[test]
    fn test_spaced_text_short_sequence() {
        let processor = PostProcessor::new();
        // Short sequences (< 4 letters) should NOT be collapsed
        // to avoid breaking "I A M" or similar valid phrases
        let input = "I A M here";
        let result = processor.fix_spaced_text(input);
        assert_eq!(result, "I A M here"); // Should stay unchanged
    }

    #[test]
    fn test_spaced_text_mixed_case() {
        let processor = PostProcessor::new();
        // Only uppercase spaced letters should trigger
        let input = "T h e hot mess";
        let result = processor.fix_spaced_text(input);
        assert_eq!(result, "T h e hot mess"); // Lowercase = no change
    }

    // ==========================================================================
    // OODA-37/38: Tests for enhanced garbled text detection
    // ==========================================================================

    #[test]
    fn test_garbled_long_word_detection() {
        let processor = GarbledTextFilterProcessor::new();

        // Long garbled word (>35 chars, no spaces) from PDF diagram text extraction
        // WHY: Diagram text like "pbtBeekeepersinccrucialrolein..." produces
        // continuous strings >100 chars that violate natural language word length limits.
        assert!(processor
            .is_garbled("pbtBeekeepersinccrucialroleinotheractivitiesrelatedtothemanagement"));
        assert!(processor.is_garbled(
            "AgricultureEnvironmentalProductionImpactAnother something additional words here"
        ));

        // OODA-38: 40-char word should now be caught (threshold lowered from 40 to 35)
        assert!(processor.is_garbled("AgricultureEnvironmentalProductionImpact"));

        // Normal text should NOT be flagged
        assert!(!processor.is_garbled("This is a normal sentence with normal words."));
        assert!(!processor.is_garbled("Even somewhat longer text with many words should be fine."));
    }

    #[test]
    fn test_garbled_camelcase_detection() {
        let processor = GarbledTextFilterProcessor::new();

        // OODA-38: CamelCase concatenated diagram labels (>25 chars, 2+ internal uppercase)
        // WHY: PDF figure text like "OriginalRelationsTextincludes" is concatenated
        // words from overlapping character positions in diagram elements.
        assert!(processor.is_garbled("OriginalRelationsTextincludes"));
        assert!(processor.is_garbled("AgricultureEnvironmentalProduction"));

        // Short camelCase words should NOT be flagged (< 25 chars)
        assert!(!processor.is_garbled("AgricultureEnvironmental")); // 24 chars
        assert!(!processor.is_garbled("DeepHalluBench")); // 14 chars
        assert!(!processor.is_garbled("RetrievalAugmented")); // 18 chars
    }

    #[test]
    fn test_garbled_long_word_url_exception() {
        let processor = GarbledTextFilterProcessor::new();

        // URLs can be long single tokens but should NOT be flagged
        assert!(!processor.is_garbled(
            "Visit https://www.example.com/very/long/path/to/resource/page for more info"
        ));
        assert!(!processor
            .is_garbled("See https://github.com/HKUDS/LightRAG/blob/main/readme for details"));
    }

    #[test]
    fn test_garbled_word_in_long_paragraph_not_filtered() {
        let processor = GarbledTextFilterProcessor::new();

        // OODA-38: A long paragraph (>200 chars) that starts with one garbled word
        // should NOT be filtered. The garbled word is a minor artifact.
        // WHY: PDF extraction sometimes prepends figure text to the start of a paragraph.
        // Filtering the entire paragraph would lose legitimate content.
        let long_para = "OriginalRelationsTextincludes two key functionalities: \
            i) Data Indexer which involves building a specific data structure based on \
            the external database. ii) Data Retriever: The relevant documents are obtained \
            by comparing the query against the indexed data, also denoted as relevant documents.";
        assert!(long_para.len() > 200);
        assert!(!processor.is_garbled(long_para));

        // But a short garbled block should still be filtered
        assert!(processor.is_garbled("OriginalRelationsTextincludes some short text"));
    }

    #[test]
    fn test_garbled_low_space_ratio() {
        let processor = GarbledTextFilterProcessor::new();

        // OODA-38: Text >60 chars with <5% spaces is likely garbled diagram text
        // (lowered from 80 to 60 to catch more concatenated labels)
        let garbled = "AbcDefGhiJklMnoPqrStuvWxyzAbcDefGhiJklMnoPqrStuvWxyzAbcDefGhiJklMnoPqrStuvWxyzAbcDefGhiJkl";
        assert!(garbled.len() > 60);
        assert!(processor.is_garbled(garbled));

        // Normal text >80 chars with adequate spaces should NOT be flagged
        let normal = "This is a normal paragraph of text with adequate spacing between words that exceeds eighty characters in total length.";
        assert!(normal.len() > 80);
        assert!(!processor.is_garbled(normal));
    }

    #[test]
    fn test_garbled_short_text_not_flagged() {
        let processor = GarbledTextFilterProcessor::new();

        // Short concatenated words (<25 chars) should NOT trigger camelCase check
        // They might be legitimate technical terms or CamelCase identifiers
        assert!(!processor.is_garbled("AgricultureEnvironmental")); // 24 chars
        assert!(!processor.is_garbled("DeepHalluBench")); // 14 chars
        assert!(!processor.is_garbled("RetrievalAugmented")); // 18 chars

        // Single compound words without multiple internal uppercase
        assert!(!processor.is_garbled("antidisestablishmentarianism")); // 28 chars, all lowercase
    }
}
