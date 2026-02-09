//! Layout structures for pymupdf4llm-inspired text extraction.
//!
//! This module implements the core text hierarchy used by pymupdf4llm:
//!
//! ```text
//! RawChar → Span → Line → Block → Page
//! ```
//!
//! ## Font Style Detection Pipeline (OODA-02, OODA-03)
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                      Font Style Data Flow                                    │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │                                                                             │
//! │  1. PDFium Backend (backend/pdfium.rs)                                      │
//! │     ├─ font_weight() ────────────────→ is_bold: bool                        │
//! │     ├─ font_is_italic() ─────────────→ is_italic: bool                      │
//! │     └─ font_is_fixed_pitch() ────────→ is_monospace: bool                   │
//! │                                                                             │
//! │  2. RawChar (backend/elements.rs)                                           │
//! │     ├─ is_bold: bool      ← From PDFium font descriptor                     │
//! │     ├─ is_italic: bool    ← From PDFium font descriptor                     │
//! │     └─ is_monospace: bool ← From PDFium FixedPitch flag (bit 1)             │
//! │                                                                             │
//! │  3. Span (this file)                                                        │
//! │     ├─ font_is_bold: Option<bool>     ← Copied from first char              │
//! │     ├─ font_is_italic: Option<bool>   ← Copied from first char              │
//! │     └─ font_is_monospace: Option<bool> ← Copied from first char             │
//! │                                                                             │
//! │  4. Markdown Rendering (layout/pymupdf_renderer.rs)                         │
//! │     ├─ is_bold() ────────→ **text**                                         │
//! │     ├─ is_italic() ──────→ _text_                                           │
//! │     └─ is_monospace() ───→ `text`                                           │
//! │                                                                             │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │  Fallback Chain (when PDFium flag unavailable):                             │
//! │  ├─ Bold:     font name contains "bold" or "heavy"                          │
//! │  ├─ Italic:   font name contains "italic" or "oblique"                      │
//! │  └─ Monospace: font name contains "mono", "courier", "consolas"             │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Key Concepts
//!
//! - **Span**: Contiguous characters with same font style (name, size, flags)
//! - **Line**: Spans on same baseline (within vertical tolerance)
//! - **Block**: Lines in same column/region
//! - **Page**: All blocks on a page, with layout metadata
//!
//! ## Algorithm Overview
//!
//! 1. Group consecutive `RawChar`s with same style → `Span`
//! 2. Group `Span`s on same baseline (±tolerance) → `Line`
//! 3. Detect columns using gap analysis
//! 4. Group `Line`s in same column → `Block`
//! 5. Render `Block`s as Markdown with proper reading order

use crate::backend::elements::RawChar;

/// A span is a contiguous sequence of characters with the same font style.
///
/// This corresponds to PyMuPDF's "span" in the DICT extraction format.
#[derive(Debug, Clone)]
pub struct Span {
    /// The text content of this span
    pub text: String,
    /// Bounding box: left edge
    pub x0: f32,
    /// Bounding box: bottom edge (PDF coordinates, origin at bottom-left)
    pub y0: f32,
    /// Bounding box: right edge
    pub x1: f32,
    /// Bounding box: top edge
    pub y1: f32,
    /// Font size in points
    pub font_size: f32,
    /// Font name (e.g., "Arial-Bold", "Times-Italic")
    pub font_name: Option<String>,
    /// Page number (0-indexed)
    pub page_num: usize,
    /// Bold flag from font descriptor (populated from RawChar)
    /// WHY: Font name matching is unreliable. PDFium provides accurate
    /// font weight from the font descriptor.
    pub font_is_bold: Option<bool>,
    /// Italic flag from font descriptor (populated from RawChar)
    /// WHY: Font name matching is unreliable. PDFium provides accurate
    /// italic flag from the font descriptor.
    pub font_is_italic: Option<bool>,
    /// Monospace (fixed-pitch) flag from font descriptor (populated from RawChar)
    /// OODA-03: WHY: Font name matching ("Mono", "Courier") is unreliable.
    /// PDFium provides accurate fixed-pitch flag from font descriptor.
    pub font_is_monospace: Option<bool>,
}

impl Span {
    /// Create an empty span at the given position.
    pub fn new(page_num: usize) -> Self {
        Self {
            text: String::new(),
            x0: f32::MAX,
            y0: f32::MAX,
            x1: f32::MIN,
            y1: f32::MIN,
            font_size: 0.0,
            font_name: None,
            page_num,
            font_is_bold: None,
            font_is_italic: None,
            font_is_monospace: None,
        }
    }

    /// Check if a character can be appended to this span (same style, same word).
    ///
    /// Returns false if:
    /// - Different page
    /// - Different font style (name, size, **bold, italic**)
    /// - Large horizontal gap (word boundary)
    /// - Vertical misalignment
    ///
    /// ## OODA-02: Style Check
    ///
    /// Characters with different bold/italic flags MUST create separate spans.
    /// Without this check, mixed-style text would inherit the first char's style:
    /// ```text
    /// Input: 'T'(bold) 'h'(bold) 'i'(normal) 's'(normal)
    /// Bad:   Span{text:"This", is_bold:true} → "**This**" (WRONG)
    /// Good:  Span{text:"Th", is_bold:true}, Span{text:"is", is_bold:false}
    ///        → "**Th**is" (CORRECT)
    /// ```
    pub fn can_append(&self, ch: &RawChar) -> bool {
        if self.text.is_empty() {
            return true;
        }

        // Same page
        if self.page_num != ch.page_num {
            return false;
        }

        // Same font size (within tolerance)
        // WHY 0.5pt: Rounding errors in PDF coordinate systems can cause ~0.1-0.3pt
        // variation for the same logical font size. 0.5pt is generous but catches
        // real size changes (e.g., 12pt body vs 10pt footnote).
        if (self.font_size - ch.font_size).abs() > 0.5 {
            return false;
        }

        // Same font name
        if self.font_name != ch.font_name {
            return false;
        }

        // OODA-02: Same font style (bold/italic)
        // WHY: A span must have homogeneous style for correct markdown rendering.
        // Without this check, mixed bold/normal text would all become bold.
        if let Some(span_bold) = self.font_is_bold {
            if span_bold != ch.is_bold {
                return false;
            }
        }
        if let Some(span_italic) = self.font_is_italic {
            if span_italic != ch.is_italic {
                return false;
            }
        }
        // OODA-03: Same monospace status
        // WHY: Monospace and proportional text must be in separate spans
        // for correct markdown code rendering (backticks).
        if let Some(span_mono) = self.font_is_monospace {
            if span_mono != ch.is_monospace {
                return false;
            }
        }

        // Vertically aligned (baseline within tolerance)
        // WHY 0.3 * font_size: Subscript/superscript shift is typically 0.33-0.5em.
        // Using 0.3 allows minor baseline drift from kerning but catches
        // intentional vertical positioning changes.
        let y_tolerance = self.font_size * 0.3;
        if (self.y0 - ch.y0).abs() > y_tolerance {
            return false;
        }

        // Check horizontal gap for word boundary detection
        // WHY: Characters within a word have minimal gaps (kerning, ~0-15% of font size)
        // Word boundaries have larger gaps (space character ~20-33% of font size)
        //
        // OODA-IT40: Font-aware threshold to handle both monospace and proportional fonts.
        // - Monospace fonts (Courier, Inconsolata): inter-char gaps ~25-28%, space ~26%
        //   → Use 33% threshold to avoid false word boundaries (OODA-IT32 fix)
        // - Proportional fonts (Arial, Times): inter-char gaps ~5-15%, space ~20-25%
        //   → Use 22% threshold to catch word boundaries without splitting kerned pairs
        //
        // OODA-IT41: URL/path punctuation-aware threshold adjustment.
        // URLs and technical text (https://github.com, file.txt, user@email.com) have
        // specific punctuation chars with kerning gaps that exceed the 22% proportional
        // threshold but are NOT word boundaries.
        //
        // Only apply higher threshold for URL/path punctuation: : / . @
        // Keep lower threshold for general punctuation: & , ; ! ? etc. which typically
        // ARE word boundaries (e.g., "A & B" should have spaces around &).
        //
        // Explicit space chars in the PDF stream are the PRIMARY word boundary signal
        // (handled in chars_to_spans). This gap check is the SECONDARY signal.
        let last_char = self.text.chars().last();

        // URL/path punctuation that should bind tightly to adjacent characters
        fn is_url_punctuation(c: char) -> bool {
            matches!(c, ':' | '/' | '.' | '@' | '-' | '_')
        }

        let is_url_boundary =
            last_char.map(is_url_punctuation).unwrap_or(false) || is_url_punctuation(ch.char);

        let space_threshold = if self.font_is_monospace.unwrap_or(false) || is_url_boundary {
            // Monospace OR URL punctuation: use higher threshold to avoid false splits
            self.font_size * 0.33
        } else {
            // Proportional non-URL: lower threshold for better word detection
            self.font_size * 0.22
        };
        let gap = ch.x0 - self.x1;

        // If gap is larger than threshold, it's a word boundary → new span
        if gap > space_threshold {
            return false;
        }

        // If gap is negative (overlapping or backwards), reject
        // unless it's minor overlap from kerning
        // WHY 0.3 * avg_char_width: Kerning in proportional fonts can cause
        // characters to overlap slightly (e.g., "AV", "To"). Allowing 30%
        // overlap tolerance preserves kerned pairs while rejecting truly
        // overlapping text (which indicates layout issues or vertical text).
        let avg_char_width = (self.x1 - self.x0) / self.text.len().max(1) as f32;
        if gap < -avg_char_width * 0.3 {
            return false;
        }

        true
    }

    /// Append a character to this span.
    pub fn append(&mut self, ch: &RawChar) {
        if self.text.is_empty() {
            self.font_size = ch.font_size;
            self.font_name = ch.font_name.clone();
            // Copy font flags from first character
            self.font_is_bold = Some(ch.is_bold);
            self.font_is_italic = Some(ch.is_italic);
            self.font_is_monospace = Some(ch.is_monospace);
        }

        self.text.push(ch.char);
        self.x0 = self.x0.min(ch.x0);
        self.y0 = self.y0.min(ch.y0);
        self.x1 = self.x1.max(ch.x1);
        self.y1 = self.y1.max(ch.y1);
    }

    /// Width of the span in points.
    pub fn width(&self) -> f32 {
        self.x1 - self.x0
    }

    /// Height of the span in points.
    pub fn height(&self) -> f32 {
        self.y1 - self.y0
    }

    /// Check if this span is bold.
    ///
    /// **Priority:** Uses font descriptor flag from PDFium if available (most accurate),
    /// otherwise falls back to font name pattern matching.
    ///
    /// WHY font flags are preferred:
    /// - PyMuPDF4llm uses font descriptor flags (like `flags & 16` for bold)
    /// - Font name matching ("bold" in name) is unreliable for many PDFs
    /// - The PDF 2900_Goyal_et_al has only NimbusSanL-Regu and NimbusSanL-Bold fonts
    ///   but PDFium correctly identifies bold via the font weight
    ///
    /// Falls back to font name patterns if flags not available:
    /// - "bold" - Standard bold indicator
    /// - "black" - Heavy weight (bolder than bold)
    /// - "heavy" - Heavy weight
    /// - "medi" - Medium weight (often used for emphasis in academic papers)
    /// - "semi" - SemiBold weight
    /// - "demi" - DemiBold weight
    pub fn is_bold(&self) -> bool {
        // Prefer font descriptor flag from PDFium (most accurate)
        if let Some(is_bold) = self.font_is_bold {
            return is_bold;
        }

        // Fallback to font name pattern matching
        self.font_name
            .as_ref()
            .map(|n| {
                let lower = n.to_lowercase();
                lower.contains("bold")
                    || lower.contains("black")
                    || lower.contains("heavy")
                    || lower.contains("medi") // Medium (NimbusRomNo9L-Medi)
                    || lower.contains("semi") // SemiBold
                    || lower.contains("demi") // DemiBold
            })
            .unwrap_or(false)
    }

    /// Check if this span is italic.
    ///
    /// **Priority:** Uses font descriptor flag from PDFium if available (most accurate),
    /// otherwise falls back to font name pattern matching.
    ///
    /// WHY font flags are preferred:
    /// - PyMuPDF4llm uses font descriptor flags (like `flags & 2` for italic)
    /// - Font name matching is unreliable - many PDFs don't use "italic" in font name
    /// - The PDF 2900_Goyal_et_al has NO italic font, so font flags will correctly
    ///   return false, avoiding false positives from journal names in references
    ///
    /// Falls back to font name patterns if flags not available:
    /// - "italic" - Standard italic indicator
    /// - "oblique" - Oblique (similar to italic)
    /// - "ital" - Abbreviated form (Nimbus fonts like NimbusRomNo9L-ReguItal)
    pub fn is_italic(&self) -> bool {
        // Prefer font descriptor flag from PDFium (most accurate)
        if let Some(is_italic) = self.font_is_italic {
            return is_italic;
        }

        // Fallback to font name pattern matching
        self.font_name
            .as_ref()
            .map(|n| {
                let lower = n.to_lowercase();
                lower.contains("italic") || lower.contains("oblique") || lower.contains("ital")
                // OODA-09: Abbreviated form (Nimbus fonts)
            })
            .unwrap_or(false)
    }

    /// Check if this span uses a monospace font.
    ///
    /// **Priority:** Uses font descriptor flag from PDFium if available (most accurate),
    /// otherwise falls back to font name pattern matching.
    ///
    /// OODA-03: WHY font descriptor flag is preferred:
    /// - PDFium reads the PDF font descriptor's FixedPitch flag (bit 1 of Flags)
    /// - This is the same data PyMuPDF uses for monospace detection
    /// - Font name matching ("Mono", "Courier") misses fonts like "F1", "CMR10"
    /// - Using font_is_fixed_pitch() gives ~99% accuracy vs ~70% for name matching
    pub fn is_monospace(&self) -> bool {
        // OODA-03: Prefer font descriptor flag from PDFium (most accurate)
        if let Some(is_mono) = self.font_is_monospace {
            return is_mono;
        }
        // Fallback: font name pattern matching (for legacy/lopdf data)
        self.font_name
            .as_ref()
            .map(|n| {
                let lower = n.to_lowercase();
                lower.contains("mono")
                    || lower.contains("courier")
                    || lower.contains("consolas")
                    || lower.contains("menlo")
                    || lower.contains("source code")
            })
            .unwrap_or(false)
    }

    /// OODA-04: Check if this span is superscript relative to a reference font size.
    ///
    /// A span is considered superscript when its font size is less than 70% of the
    /// reference (dominant line) font size AND its text is short (< 5 chars).
    /// Common for footnote markers like "1", "*", "†".
    ///
    /// REF: pymupdf4llm document_layout.py:172-184 (is_superscripted)
    pub fn is_superscript(&self, reference_font_size: f32) -> bool {
        if reference_font_size <= 0.0 {
            return false;
        }
        let ratio = self.font_size / reference_font_size;
        ratio < 0.7 && self.text.chars().count() < 5
    }

    /// OODA-19: Check if this span is subscript relative to a reference font size.
    ///
    /// A span is considered subscript when its font size is less than 70% of the
    /// reference (dominant line) font size AND its text is short (< 5 chars),
    /// AND it is NOT already detected as superscript (superscripts sit above baseline).
    ///
    /// Common for chemical formulas (H₂O, CO₂) and mathematical subscripts (x_i, a_n).
    /// Subscripts typically have a lower baseline (higher y0) than the reference text.
    pub fn is_subscript(&self, reference_font_size: f32, ref_y1: f32) -> bool {
        if reference_font_size <= 0.0 {
            return false;
        }
        let ratio = self.font_size / reference_font_size;
        // Small font, short text, AND positioned near/below the baseline of reference
        // Subscripts have y1 close to or below the ref_y1 (bottom of reference text)
        ratio < 0.7
            && self.text.chars().count() < 5
            && self.y1 >= ref_y1 - reference_font_size * 0.1
    }
}

/// A line is a sequence of spans on the same baseline.
#[derive(Debug, Clone)]
pub struct Line {
    /// Spans in this line, sorted left-to-right
    pub spans: Vec<Span>,
    /// Bounding box: left edge
    pub x0: f32,
    /// Bounding box: bottom edge
    pub y0: f32,
    /// Bounding box: right edge
    pub x1: f32,
    /// Bounding box: top edge
    pub y1: f32,
    /// Page number (0-indexed)
    pub page_num: usize,
}

impl Line {
    /// Create a new line from a single span.
    pub fn from_span(span: Span) -> Self {
        Self {
            x0: span.x0,
            y0: span.y0,
            x1: span.x1,
            y1: span.y1,
            page_num: span.page_num,
            spans: vec![span],
        }
    }

    /// Create a new line from multiple spans.
    /// OODA-07: Used when splitting multi-column lines.
    pub fn from_spans(spans: Vec<Span>, page_num: usize) -> Self {
        if spans.is_empty() {
            return Self {
                x0: 0.0,
                y0: 0.0,
                x1: 0.0,
                y1: 0.0,
                page_num,
                spans: vec![],
            };
        }

        let x0 = spans.iter().map(|s| s.x0).fold(f32::MAX, f32::min);
        let y0 = spans.iter().map(|s| s.y0).fold(f32::MAX, f32::min);
        let x1 = spans.iter().map(|s| s.x1).fold(f32::MIN, f32::max);
        let y1 = spans.iter().map(|s| s.y1).fold(f32::MIN, f32::max);

        Self {
            x0,
            y0,
            x1,
            y1,
            page_num,
            spans,
        }
    }

    /// OODA-IT07: Create an empty line with explicit bounding box.
    ///
    /// WHY: Used in tests to create lines with known positions
    /// for column detection algorithm verification.
    #[cfg(test)]
    pub fn new_with_bbox(x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        Self {
            x0,
            y0,
            x1,
            y1,
            page_num: 0,
            spans: vec![],
        }
    }

    /// Check if a span belongs on this line (same baseline).
    ///
    /// OODA-04 FIX: Only checks vertical alignment.
    /// OODA-07 NOTE: Horizontal gap check was attempted but caused issues.
    /// Column separation is now handled at the block level in lines_to_blocks.
    ///
    /// Returns false if:
    /// - Span is on a different page
    /// - Vertical alignment differs by more than tolerance
    pub fn can_add_span(&self, span: &Span, tolerance: f32) -> bool {
        if self.page_num != span.page_num {
            return false;
        }

        // Compare baseline (y0) or top (y1) - matches pymupdf4llm get_raw_lines.py:178
        // "if any of top or bottom coordinates are close enough, join..."
        if (self.y0 - span.y0).abs() <= tolerance || (self.y1 - span.y1).abs() <= tolerance {
            return true;
        }

        // OODA-IT28: Also check vertical overlap for narrow-height glyphs.
        // WHY (First Principles): Characters like em dash (—), en dash (–), bullet (•),
        // and other punctuation have glyph heights much smaller than regular letters.
        // For example, an em dash at font_size=30 has only ~3.7pt height vs ~22pt for 'A'.
        // Their baseline/top won't match regular letters within tolerance, but they are
        // visually on the same line. If the span is fully contained within the line's
        // y-range, it belongs to this line.
        let y_overlap_start = self.y0.max(span.y0);
        let y_overlap_end = self.y1.min(span.y1);
        let has_overlap = y_overlap_end > y_overlap_start;
        if has_overlap {
            let span_height = (span.y1 - span.y0).max(0.1);
            let overlap_amount = y_overlap_end - y_overlap_start;
            // If >80% of the span's height is within the line, it belongs here
            if overlap_amount / span_height > 0.8 {
                return true;
            }
        }

        false
    }

    /// Add a span to this line.
    pub fn add_span(&mut self, span: Span) {
        self.x0 = self.x0.min(span.x0);
        self.y0 = self.y0.min(span.y0);
        self.x1 = self.x1.max(span.x1);
        self.y1 = self.y1.max(span.y1);
        self.spans.push(span);
    }

    /// Sort spans left-to-right.
    pub fn sort_spans(&mut self) {
        self.spans.sort_by(|a, b| a.x0.partial_cmp(&b.x0).unwrap());
    }

    /// Get the full text of this line with appropriate spacing between spans.
    ///
    /// Uses gap analysis to determine if a space is needed between spans.
    /// Avoids adding spaces before hyphens/dashes to preserve hyphenated words.
    pub fn text(&self) -> String {
        if self.spans.is_empty() {
            return String::new();
        }

        if self.spans.len() == 1 {
            return self.spans[0].text.clone();
        }

        let mut result = String::new();
        for (i, span) in self.spans.iter().enumerate() {
            if i > 0 {
                // Check gap between this span and previous
                let prev = &self.spans[i - 1];
                let gap = span.x0 - prev.x1;

                // Only add space if there's a significant gap
                // Use the average font size for threshold
                let avg_size = (prev.font_size + span.font_size) / 2.0;
                let space_threshold = avg_size * 0.15; // ~15% of font size

                // Don't add space if current span starts with hyphen/dash
                // This preserves hyphenated words like "Qwen2.5-7B-Instruct"
                // WHY: Hyphens and en-dashes join compound terms without spaces.
                // Em dashes (—) are EXCLUDED: they are sentence-level punctuation
                // that typically has spaces around them (e.g., "AI Services — Elitizon").
                let starts_with_hyphen = span.text.starts_with('-') || span.text.starts_with('–'); // en-dash only, NOT em-dash

                // Don't add space if previous span ends with hyphen
                let ends_with_hyphen = prev.text.ends_with('-') || prev.text.ends_with('–');

                if gap > space_threshold && !starts_with_hyphen && !ends_with_hyphen {
                    result.push(' ');
                }
            }
            result.push_str(&span.text);
        }
        result
    }

    /// Width of the line in points.
    pub fn width(&self) -> f32 {
        self.x1 - self.x0
    }

    /// Height of the line in points.
    pub fn height(&self) -> f32 {
        self.y1 - self.y0
    }

    /// Get the dominant font size in this line.
    pub fn dominant_font_size(&self) -> f32 {
        if self.spans.is_empty() {
            return 0.0;
        }

        // Weight by text length
        let mut total_weight = 0.0;
        let mut weighted_size = 0.0;
        for span in &self.spans {
            let weight = span.text.len() as f32;
            weighted_size += span.font_size * weight;
            total_weight += weight;
        }

        if total_weight > 0.0 {
            weighted_size / total_weight
        } else {
            self.spans[0].font_size
        }
    }

    /// OODA-IT36: Check if this line starts with a list marker (bullet or number).
    ///
    /// WHY (First Principles): PDFs render each bullet item as a separate visual
    /// line starting with a bullet character (•, -, *, ▪, etc.) or a number
    /// pattern ("1.", "2)", "(a)"). When the TextGrouper merges these lines into
    /// one block, the downstream ListDetectionProcessor can only see the FIRST
    /// bullet — all subsequent items become invisible paragraph text.
    ///
    /// By detecting list markers at the line level, we can prevent merging and
    /// preserve each list item as a separate block.
    ///
    /// ```text
    /// Before (merged):           After (split):
    /// ┌──────────────────┐       ┌──────────────────┐
    /// │ • Item A text    │       │ • Item A text    │  ← Block 1
    /// │ • Item B text    │  →    └──────────────────┘
    /// │ • Item C text    │       ┌──────────────────┐
    /// └──────────────────┘       │ • Item B text    │  ← Block 2
    ///                            └──────────────────┘
    ///                            ┌──────────────────┐
    ///                            │ • Item C text    │  ← Block 3
    ///                            └──────────────────┘
    /// ```
    pub fn starts_with_list_marker(&self) -> bool {
        let text = self.text();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return false;
        }

        // Check for common bullet characters
        // WHY: This is a subset of the full 530+ character set in
        // structure_detection.rs. We only need the most common ones here
        // to prevent block merging. The full set is used by
        // ListDetectionProcessor for final classification.
        let first = trimmed.chars().next().unwrap();
        let is_bullet = matches!(
            first,
            '•' | '◦'
                | '▪'
                | '▸'
                | '▹'
                | '►'
                | '▻'
                | '●'
                | '○'
                | '■'
                | '□'
                | '▲'
                | '△'
                | '▶'
                | '▷'
                | '★'
                | '☆'
                | '✦'
                | '✧'
                | '✓'
                | '✔'
                | '✗'
                | '✘'
                | '➤'
                | '➢'
                | '➣'
                | '➜'
                | '➡'
                | '⁃'
                | '∙'
                | '·'
                | '†'
                | '‡'
        );

        if is_bullet {
            // Bullet followed by space, end, uppercase, or asterisk
            let rest = &trimmed[first.len_utf8()..];
            return rest.is_empty()
                || rest.starts_with(' ')
                || rest.starts_with('\t')
                || rest.starts_with('*')
                || rest.starts_with(|c: char| c.is_uppercase());
        }

        // Check for numbered list: "1. " or "1) " or "(1)" etc.
        // WHY: Numbered items like "1. Introduction" also get merged
        // Use a simple check: digit(s) followed by . or ) then space
        let bytes = trimmed.as_bytes();
        if bytes[0].is_ascii_digit() {
            // Find end of digits
            let digit_end = bytes.iter().position(|b| !b.is_ascii_digit()).unwrap_or(0);
            if digit_end > 0 && digit_end < bytes.len() {
                let after_digit = bytes[digit_end];
                // "1. text" or "1) text" — must have space after
                if (after_digit == b'.' || after_digit == b')') && digit_end + 1 < bytes.len() {
                    let after_marker = bytes[digit_end + 1];
                    if after_marker == b' ' || after_marker == b'\t' {
                        return true;
                    }
                }
            }
        }

        false
    }
}

/// A block is a collection of lines in the same column/region.
#[derive(Debug, Clone)]
pub struct Block {
    /// Lines in this block, sorted top-to-bottom
    pub lines: Vec<Line>,
    /// Bounding box: left edge
    pub x0: f32,
    /// Bounding box: bottom edge  
    pub y0: f32,
    /// Bounding box: right edge
    pub x1: f32,
    /// Bounding box: top edge
    pub y1: f32,
    /// Page number (0-indexed)
    pub page_num: usize,
    /// Block type for markdown rendering
    pub block_type: BlockType,
}

/// Type of content block for markdown rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    /// Regular paragraph text
    Paragraph,
    /// Header (h1-h6)
    Header(u8),
    /// Code block (monospace)
    Code,
    /// List item (bullet or numbered)
    ListItem,
    /// Table content
    Table,
    /// Footnote (rendered as blockquote)
    /// OODA-08: Footnotes detected by position + font size + marker
    Footnote,
}

impl Block {
    /// Create a new block from a single line.
    pub fn from_line(line: Line) -> Self {
        Self {
            x0: line.x0,
            y0: line.y0,
            x1: line.x1,
            y1: line.y1,
            page_num: line.page_num,
            lines: vec![line],
            block_type: BlockType::Paragraph,
        }
    }

    /// OODA-12: Create a new block from multiple lines.
    ///
    /// WHY: When splitting blocks at bullet points, we need to construct
    /// new blocks from a subset of lines. The bbox is computed as the
    /// union of all line bboxes.
    pub fn from_lines(lines: Vec<Line>, page_num: usize) -> Self {
        if lines.is_empty() {
            return Self {
                x0: 0.0,
                y0: 0.0,
                x1: 0.0,
                y1: 0.0,
                page_num,
                lines: vec![],
                block_type: BlockType::Paragraph,
            };
        }

        // Compute bounding box as union of all line bboxes
        let x0 = lines.iter().map(|l| l.x0).fold(f32::INFINITY, f32::min);
        let y0 = lines.iter().map(|l| l.y0).fold(f32::INFINITY, f32::min);
        let x1 = lines.iter().map(|l| l.x1).fold(f32::NEG_INFINITY, f32::max);
        let y1 = lines.iter().map(|l| l.y1).fold(f32::NEG_INFINITY, f32::max);

        Self {
            x0,
            y0,
            x1,
            y1,
            page_num,
            lines,
            block_type: BlockType::Paragraph,
        }
    }

    /// Check if a line belongs to this block.
    ///
    /// Uses horizontal overlap and vertical proximity.
    ///
    /// OODA-IT36: Also rejects lines that start with list markers (bullets,
    /// numbers) to prevent merging separate list items into one block.
    pub fn can_add_line(&self, line: &Line, line_gap_tolerance: f32) -> bool {
        if self.page_num != line.page_num {
            return false;
        }

        // OODA-IT36: Lines starting with a list marker ALWAYS start a new block.
        // WHY (First Principles): Each bullet/numbered item is a separate
        // semantic element. Merging "• Item A" with "• Item B" into one block
        // makes them invisible to ListDetectionProcessor, which only checks
        // block.text.starts_with(bullet).
        if line.starts_with_list_marker() {
            return false;
        }

        // Check vertical proximity (line should be below current block)
        // Note: PDF y-coordinates increase upward, so y0 of new line should be less
        let vertical_gap = self.y0 - line.y1;
        if vertical_gap < 0.0 || vertical_gap > line_gap_tolerance {
            return false;
        }

        // Check horizontal overlap
        let overlap_start = self.x0.max(line.x0);
        let overlap_end = self.x1.min(line.x1);
        let overlap = overlap_end - overlap_start;

        // Require significant horizontal overlap (at least 50% of narrower element)
        let min_width = self.width().min(line.width());
        overlap >= min_width * 0.5
    }

    /// Add a line to this block.
    pub fn add_line(&mut self, line: Line) {
        self.x0 = self.x0.min(line.x0);
        self.y0 = self.y0.min(line.y0);
        self.x1 = self.x1.max(line.x1);
        self.y1 = self.y1.max(line.y1);
        self.lines.push(line);
    }

    /// Sort lines top-to-bottom (decreasing y in PDF coordinates).
    pub fn sort_lines(&mut self) {
        // In PDF, larger y = higher on page, so sort descending by y1
        self.lines.sort_by(|a, b| b.y1.partial_cmp(&a.y1).unwrap());
    }

    /// Width of the block in points.
    pub fn width(&self) -> f32 {
        self.x1 - self.x0
    }

    /// Height of the block in points.
    pub fn height(&self) -> f32 {
        self.y1 - self.y0
    }

    /// Get the full text of this block.
    pub fn text(&self) -> String {
        self.lines
            .iter()
            .map(|l| l.text())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_append() {
        let mut span = Span::new(0);
        let ch1 = RawChar {
            char: 'H',
            x0: 10.0,
            y0: 100.0,
            x1: 18.0,
            y1: 112.0,
            font_size: 12.0,
            font_name: Some("Arial".to_string()),
            page_num: 0,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
        };
        let ch2 = RawChar {
            char: 'i',
            x0: 18.0,
            y0: 100.0,
            x1: 22.0,
            y1: 112.0,
            font_size: 12.0,
            font_name: Some("Arial".to_string()),
            page_num: 0,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
        };

        assert!(span.can_append(&ch1));
        span.append(&ch1);
        assert!(span.can_append(&ch2));
        span.append(&ch2);

        assert_eq!(span.text, "Hi");
        assert_eq!(span.x0, 10.0);
        assert_eq!(span.x1, 22.0);
    }

    #[test]
    fn test_span_style_detection() {
        let bold_span = Span {
            text: "Bold".to_string(),
            x0: 0.0,
            y0: 0.0,
            x1: 50.0,
            y1: 12.0,
            font_size: 12.0,
            font_name: Some("Arial-Bold".to_string()),
            page_num: 0,
            font_is_bold: Some(true),
            font_is_italic: Some(false),
            font_is_monospace: None,
        };

        let italic_span = Span {
            text: "Italic".to_string(),
            x0: 0.0,
            y0: 0.0,
            x1: 50.0,
            y1: 12.0,
            font_size: 12.0,
            font_name: Some("Arial-Italic".to_string()),
            page_num: 0,
            font_is_bold: Some(false),
            font_is_italic: Some(true),
            font_is_monospace: None,
        };

        let mono_span = Span {
            text: "code".to_string(),
            x0: 0.0,
            y0: 0.0,
            x1: 50.0,
            y1: 12.0,
            font_size: 12.0,
            font_name: Some("Courier".to_string()),
            page_num: 0,
            font_is_bold: Some(false),
            font_is_italic: Some(false),
            font_is_monospace: None,
        };

        assert!(bold_span.is_bold());
        assert!(!bold_span.is_italic());

        assert!(italic_span.is_italic());
        assert!(!italic_span.is_bold());

        assert!(mono_span.is_monospace());
    }

    #[test]
    fn test_line_from_spans() {
        let span1 = Span {
            text: "Hello".to_string(),
            x0: 10.0,
            y0: 100.0,
            x1: 50.0,
            y1: 112.0,
            font_size: 12.0,
            font_name: Some("Arial".to_string()),
            page_num: 0,
            font_is_bold: None,
            font_is_italic: None,
            font_is_monospace: None,
        };

        let span2 = Span {
            text: "World".to_string(),
            x0: 55.0,
            y0: 100.0,
            x1: 95.0,
            y1: 112.0,
            font_size: 12.0,
            font_name: Some("Arial".to_string()),
            page_num: 0,
            font_is_bold: None,
            font_is_italic: None,
            font_is_monospace: None,
        };

        let mut line = Line::from_span(span1);
        assert!(line.can_add_span(&span2, 3.0));
        line.add_span(span2);
        line.sort_spans();

        assert_eq!(line.text(), "Hello World");
        assert_eq!(line.spans.len(), 2);
    }

    #[test]
    fn test_block_from_lines() {
        let line1 = Line {
            spans: vec![Span {
                text: "First line".to_string(),
                x0: 10.0,
                y0: 100.0,
                x1: 100.0,
                y1: 112.0,
                font_size: 12.0,
                font_name: None,
                page_num: 0,
                font_is_bold: None,
                font_is_italic: None,
                font_is_monospace: None,
            }],
            x0: 10.0,
            y0: 100.0,
            x1: 100.0,
            y1: 112.0,
            page_num: 0,
        };

        let line2 = Line {
            spans: vec![Span {
                text: "Second line".to_string(),
                x0: 10.0,
                y0: 85.0,
                x1: 110.0,
                y1: 97.0,
                font_size: 12.0,
                font_name: None,
                page_num: 0,
                font_is_bold: None,
                font_is_italic: None,
                font_is_monospace: None,
            }],
            x0: 10.0,
            y0: 85.0,
            x1: 110.0,
            y1: 97.0,
            page_num: 0,
        };

        let mut block = Block::from_line(line1);
        // Line gap is 100 - 97 = 3 points
        assert!(block.can_add_line(&line2, 5.0));
        block.add_line(line2);
        block.sort_lines();

        assert_eq!(block.lines.len(), 2);
        assert!(block.text().contains("First line"));
        assert!(block.text().contains("Second line"));
    }

    /// OODA-02: Test that spans split correctly when font style changes.
    /// This ensures mixed bold/normal text creates separate spans.
    #[test]
    fn test_span_rejects_different_style() {
        use crate::backend::elements::RawChar;

        // Create a span starting with bold text
        let mut span = Span::new(0);
        let bold_char = RawChar {
            char: 'T',
            x0: 10.0,
            y0: 100.0,
            x1: 18.0,
            y1: 112.0,
            font_size: 12.0,
            font_name: Some("Arial".to_string()),
            page_num: 0,
            is_bold: true,
            is_italic: false,
            is_monospace: false,
        };
        span.append(&bold_char);

        // Now try to append a non-bold character
        let normal_char = RawChar {
            char: 'h',
            x0: 18.0, // Adjacent position
            y0: 100.0,
            x1: 26.0,
            y1: 112.0,
            font_size: 12.0,
            font_name: Some("Arial".to_string()),
            page_num: 0,
            is_bold: false, // Different style!
            is_italic: false,
            is_monospace: false,
        };

        // Should reject because bold differs
        assert!(
            !span.can_append(&normal_char),
            "Span should reject character with different bold flag"
        );

        // Create a span starting with italic text
        let mut italic_span = Span::new(0);
        let italic_char = RawChar {
            char: 'A',
            x0: 10.0,
            y0: 100.0,
            x1: 18.0,
            y1: 112.0,
            font_size: 12.0,
            font_name: Some("Times".to_string()),
            page_num: 0,
            is_bold: false,
            is_italic: true,
            is_monospace: false,
        };
        italic_span.append(&italic_char);

        // Try to append non-italic character
        let non_italic_char = RawChar {
            char: 'B',
            x0: 18.0,
            y0: 100.0,
            x1: 26.0,
            y1: 112.0,
            font_size: 12.0,
            font_name: Some("Times".to_string()),
            page_num: 0,
            is_bold: false,
            is_italic: false, // Different style!
            is_monospace: false,
        };

        // Should reject because italic differs
        assert!(
            !italic_span.can_append(&non_italic_char),
            "Span should reject character with different italic flag"
        );

        // But same style should still be accepted
        let same_style_char = RawChar {
            char: 'B',
            x0: 18.0,
            y0: 100.0,
            x1: 26.0,
            y1: 112.0,
            font_size: 12.0,
            font_name: Some("Times".to_string()),
            page_num: 0,
            is_bold: false,
            is_italic: true, // Same style!
            is_monospace: false,
        };

        assert!(
            italic_span.can_append(&same_style_char),
            "Span should accept character with same italic flag"
        );

        // OODA-04: Test monospace span rejection
        // Create a span starting with monospace text
        let mut mono_span = Span::new(0);
        let mono_char = RawChar {
            char: 'x',
            x0: 10.0,
            y0: 100.0,
            x1: 18.0,
            y1: 112.0,
            font_size: 12.0,
            font_name: Some("Courier".to_string()),
            page_num: 0,
            is_bold: false,
            is_italic: false,
            is_monospace: true, // Monospace font
        };
        mono_span.append(&mono_char);

        // Try to append non-monospace character
        let non_mono_char = RawChar {
            char: 'y',
            x0: 18.0,
            y0: 100.0,
            x1: 26.0,
            y1: 112.0,
            font_size: 12.0,
            font_name: Some("Arial".to_string()),
            page_num: 0,
            is_bold: false,
            is_italic: false,
            is_monospace: false, // Different style!
        };

        // Should reject because monospace differs
        assert!(
            !mono_span.can_append(&non_mono_char),
            "Span should reject character with different monospace flag"
        );

        // Same monospace style should be accepted
        let same_mono_char = RawChar {
            char: 'z',
            x0: 18.0,
            y0: 100.0,
            x1: 26.0,
            y1: 112.0,
            font_size: 12.0,
            font_name: Some("Courier".to_string()),
            page_num: 0,
            is_bold: false,
            is_italic: false,
            is_monospace: true, // Same style!
        };

        assert!(
            mono_span.can_append(&same_mono_char),
            "Span should accept character with same monospace flag"
        );
    }
}
