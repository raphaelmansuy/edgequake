//! Text grouping algorithms for pymupdf4llm-inspired extraction.
//!
//! This module provides the `TextGrouper` that converts a stream of `RawChar`s
//! into structured `Span`s, `Line`s, and `Block`s.
//!
//! ## Algorithm (OODA-45 SRP Refactoring)
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                    TEXT GROUPING PIPELINE                               │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  RawChar[] ──► chars_to_spans() ──► Span[]                              │
//! │                     │                                                   │
//! │  Span[]   ──► spans_to_lines() ──► Line[]                               │
//! │                     │                                                   │
//! │  Line[]   ──► lines_to_blocks() ──► Block[]                             │
//! │                     │                                                   │
//! │  Block[]  ──► classify_blocks() ──► Block[] (with BlockType)           │
//! │                     │                                                   │
//! │                     ▼                                                   │
//! │            Uses: BlockClassifier (from block_classifier.rs)            │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Font Style Preservation (OODA-46)
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                    FONT STYLE PROPAGATION                               │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  Step 1: RawChar carries style flags from PDFium                        │
//! │  ═══════════════════════════════════════════                            │
//! │                                                                         │
//! │  RawChar {                                                              │
//! │      char: 'T',                                                         │
//! │      is_bold: true,      ← from font_weight() >= 700                    │
//! │      is_italic: false,   ← from font_is_italic()                        │
//! │      font_name: "Arial-Bold",                                           │
//! │      ...                                                                │
//! │  }                                                                      │
//! │                                                                         │
//! │  Step 2: Span inherits style from first char                            │
//! │  ═══════════════════════════════════════════                            │
//! │                                                                         │
//! │  chars_to_spans() creates Span with:                                    │
//! │  - flags bit 4 = bold (from RawChar.is_bold)                            │
//! │  - flags bit 1 = italic (from RawChar.is_italic)                        │
//! │                                                                         │
//! │  Span {                                                                 │
//! │      text: "Title",                                                     │
//! │      flags: 0b10000,   ← bit 4 = bold                                   │
//! │      ...                                                                │
//! │  }                                                                      │
//! │                                                                         │
//! │  Step 3: Spans preserved through Line/Block grouping                    │
//! │  ═══════════════════════════════════════════════════                    │
//! │                                                                         │
//! │  Line { spans: [Span{flags: bold}, Span{flags: normal}, ...] }          │
//! │  Block { lines: [Line1, Line2, ...] }                                   │
//! │                                                                         │
//! │  Step 4: Renderer reads flags and applies markdown                      │
//! │  ══════════════════════════════════════════════                         │
//! │                                                                         │
//! │  pymupdf_renderer.rs:render_span()                                      │
//! │      if flags & 0b10000 → wrap with **bold**                            │
//! │      if flags & 0b00010 → wrap with *italic*                            │
//! │      if flags & 0b01000 → wrap with `code`                              │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! This mirrors the pymupdf4llm approach but implemented in pure Rust.

use super::block_classifier::{is_bullet_item, is_numbered_list_item, BlockClassifier};
use super::pymupdf_structs::{Block, BlockType, Line, Span};
use crate::backend::elements::RawChar;

/// Parameters for text grouping.
#[derive(Debug, Clone)]
pub struct GroupingParams {
    /// Vertical tolerance for same-line detection (in points)
    pub line_tolerance: f32,
    /// Maximum gap between lines in same block (in points)
    pub block_gap: f32,
    /// Minimum horizontal overlap for same-column detection (0.0-1.0)
    pub column_overlap: f32,
    /// OODA-07: Left margin width to exclude (in points).
    /// Text in the left margin (x < left_margin) is filtered out.
    /// WHY: pymupdf4llm filters rotated arXiv watermarks (line["dir"] check at get_text_lines.py:121).
    /// PDFium doesn't provide direction vectors, so we use position-based filtering.
    /// ArXiv watermarks are typically at x ≈ 10-40pt (well within 50pt threshold).
    pub left_margin: f32,
    /// OODA-07: Right margin width to exclude (in points).
    /// Text beyond (page_width - right_margin) is filtered out.
    pub right_margin: f32,
    /// OODA-IT01: Header margin height to exclude (in points).
    /// Text within header_margin from page top (y < header_margin) is filtered out.
    /// WHY: pymupdf4llm uses header_margin=50 to filter page numbers, chapter titles.
    /// See multi_column.py:column_boxes() - `clip.y0 += header_margin`.
    pub header_margin: f32,
    /// OODA-IT01: Footer margin height to exclude (in points).
    /// Text within footer_margin from page bottom is filtered out.
    /// WHY: pymupdf4llm uses footer_margin=50 to filter footnotes, page numbers.
    /// See multi_column.py:column_boxes() - `clip.y1 -= footer_margin`.
    pub footer_margin: f32,
    /// Page height for footer margin calculation.
    /// Must be set when using footer_margin filtering.
    pub page_height: f32,
}

impl Default for GroupingParams {
    fn default() -> Self {
        Self {
            // WHY: Increased from 3pt to 5pt to handle font style variations
            // (italic/bold fonts have different baseline positions).
            // PDFium character bboxes vary more than pymupdf's pre-grouped spans.
            // OODA-04: Changed from 5.0 to 3.0 to match pymupdf4llm default
            line_tolerance: 3.0,
            // WHY: pymupdf4llm uses 10pt as max vertical gap for joining blocks
            // (multi_column.py line 242: `abs(r0.y1 - r.y0) <= 10`)
            block_gap: 10.0,
            // WHY (OODA-10): 50% horizontal overlap threshold for same-column detection.
            // Two blocks are "same column" if X ranges overlap by 50%+.
            // This handles indented paragraphs while preventing adjacent column merging.
            column_overlap: 0.5,
            // OODA-07: Filter left margin (arXiv watermarks at x ≈ 10-40pt)
            // Using 50pt to catch all rotated margin text
            left_margin: 50.0,
            // OODA-07: No right margin filtering by default
            right_margin: 0.0,
            // OODA-IT01: Header margin = 50pt matches pymupdf4llm default
            // See multi_column.py:column_boxes() default parameter
            header_margin: 50.0,
            // OODA-IT01: Footer margin = 50pt matches pymupdf4llm default
            footer_margin: 50.0,
            // OODA-IT01: Default page height (US Letter = 792pt)
            // Updated per-page during extraction
            page_height: 792.0,
        }
    }
}

/// Groups raw characters into spans, lines, and blocks.
///
/// OODA-45 SRP: This struct handles ONLY grouping logic.
/// Classification is delegated to `BlockClassifier` from `block_classifier.rs`.
pub struct TextGrouper {
    params: GroupingParams,
    /// OODA-45: Block classifier for type detection
    classifier: BlockClassifier,
}

impl TextGrouper {
    /// Create a new text grouper with default parameters.
    pub fn new() -> Self {
        Self {
            params: GroupingParams::default(),
            classifier: BlockClassifier::default(),
        }
    }

    /// Create a text grouper with custom parameters.
    pub fn with_params(params: GroupingParams) -> Self {
        Self {
            params,
            classifier: BlockClassifier::default(),
        }
    }

    /// Check if a character is horizontal text (not rotated/vertical).
    ///
    /// Rotated text (like arXiv margin dates) has bbox where height >> width.
    /// For horizontal text, width is typically similar to or greater than height.
    ///
    /// WHY: pymupdf4llm filters non-horizontal text at get_text_lines.py:121
    /// `if abs(1 - line_dir[0]) > 1e-3: continue`
    /// Since PDFium doesn't give us direction vectors, we approximate using bbox aspect ratio.
    ///
    /// NOTE: This filter is currently disabled because PDFium character bboxes
    /// often have height > width even for horizontal text. Need better heuristic.
    #[allow(dead_code)]
    fn is_horizontal_char(_ch: &RawChar) -> bool {
        // KNOWN LIMITATION: Vertical text detection not implemented
        // WHY: PDFium character bboxes don't indicate text direction reliably
        // Aspect ratio heuristics fail because normal chars often have height > width
        // WORKAROUND: ArXiv watermarks are filtered by margin position instead (OODA-07)
        // FUTURE: Analyze character sequence patterns to detect vertical text runs
        true
    }

    /// OODA-IT32: Sort characters into reading order (top-to-bottom, left-to-right).
    ///
    /// WHY: PDFium emits characters in PDF content stream order, not reading order.
    /// We need them sorted for proper span grouping.
    ///
    /// ```text
    /// ┌──────────────────────────────────────────────────┐
    /// │  Algorithm: Two-pass clustering sort              │
    /// │                                                  │
    /// │  Pass 1: Sort all chars by (page, y0)            │
    /// │  Pass 2: Scan sorted list, group consecutive     │
    /// │          chars with |Δy| ≤ tolerance into        │
    /// │          line clusters. Sort each cluster by x0. │
    /// │                                                  │
    /// │  Tolerance = max(font_size) * 0.3 within cluster │
    /// │  (same as Span::can_append y_tolerance)          │
    /// │                                                  │
    /// │  WHY clustering > fixed buckets:                 │
    /// │  Fixed buckets have boundary artifacts:          │
    /// │    y=614.9 → bucket A, y=615.1 → bucket B       │
    /// │    Same line, different buckets! ✗               │
    /// │  Clustering groups by actual proximity. ✓        │
    /// └──────────────────────────────────────────────────┘
    /// ```
    fn sort_chars_reading_order(chars: &[RawChar]) -> Vec<&RawChar> {
        if chars.is_empty() {
            return vec![];
        }

        // Pass 1: Sort by (page, y0) for vertical ordering
        let mut refs: Vec<&RawChar> = chars.iter().collect();
        refs.sort_by(|a, b| {
            a.page_num
                .cmp(&b.page_num)
                .then_with(|| a.y0.partial_cmp(&b.y0).unwrap_or(std::cmp::Ordering::Equal))
        });

        // Pass 2: Group consecutive chars into line clusters, sort each by x
        let mut result: Vec<&RawChar> = Vec::with_capacity(refs.len());
        let mut cluster_start = 0;

        for i in 1..=refs.len() {
            let should_break = if i == refs.len() {
                true // Flush last cluster
            } else {
                let prev = refs[i - 1];
                let curr = refs[i];
                // Break cluster on page change or significant y-gap
                if prev.page_num != curr.page_num {
                    true
                } else {
                    // WHY 0.3 * font_size: Same tolerance as Span::can_append.
                    // Subscripts/superscripts shift by ~0.33-0.5em; 0.3 allows
                    // minor baseline drift while separating distinct lines.
                    let tolerance = prev.font_size.max(curr.font_size) * 0.3;
                    (curr.y0 - prev.y0).abs() > tolerance
                }
            };

            if should_break {
                // Sort this cluster by x0 (left-to-right reading order)
                let cluster = &mut refs[cluster_start..i];
                cluster
                    .sort_by(|a, b| a.x0.partial_cmp(&b.x0).unwrap_or(std::cmp::Ordering::Equal));
                result.extend_from_slice(cluster);
                cluster_start = i;
            }
        }

        result
    }

    /// Group raw characters into spans.
    ///
    /// Characters are grouped when they have:
    /// - Same page
    /// - Same font name
    /// - Similar font size (within 0.5pt)
    /// - Horizontal adjacency (gap < 1.5 * char width)
    /// - Vertical alignment (within font_size * 0.3)
    ///
    /// OODA-07: Characters in the left/right margins are filtered out.
    /// WHY: pymupdf4llm filters non-horizontal text (get_text_lines.py:121).
    /// ArXiv watermarks are rotated 90° and appear in the left margin.
    pub fn chars_to_spans(&self, chars: &[RawChar]) -> Vec<Span> {
        if chars.is_empty() {
            return vec![];
        }

        // OODA-IT32: Sort characters by reading order before grouping.
        //
        // WHY: PDFium emits characters in PDF content stream order, which is
        // NOT necessarily left-to-right reading order. For example, all 'g'
        // characters on a line may be emitted together, then all 'a' chars, etc.
        // Without sorting, chars_to_spans creates fragmented single-char spans
        // because can_append() sees huge gaps between non-adjacent characters.
        //
        // ┌───────────────────────────────────────────────────────┐
        // │  PDF stream order:  g(x=123) g(x=316) g(x=334) ...  │
        // │  Reading order:     z(x=114) r(x=119) g(x=123) ...  │
        // │                                                       │
        // │  Without sort: each char → 1 span → spurious spaces  │
        // │  With sort:    consecutive chars → proper word spans  │
        // └───────────────────────────────────────────────────────┘
        //
        // Algorithm: two-pass clustering sort.
        //   Pass 1: Sort by (page, y0) to get chars in vertical order.
        //   Pass 2: Group consecutive chars with similar y into "line clusters",
        //           then sort each cluster by x0 for left-to-right reading order.
        //
        // WHY two-pass instead of fixed buckets: Fixed y-buckets suffer from
        // boundary artifacts (chars at y=614.9 and y=615.1 split into different
        // buckets despite being on the same line). Dynamic clustering avoids this.
        let sorted_chars = Self::sort_chars_reading_order(chars);

        let mut spans = Vec::new();
        let mut current_span = Span::new(sorted_chars[0].page_num);

        for ch in &sorted_chars {
            // Skip control characters (except space) and zero-width chars
            if (ch.char.is_control() && !ch.char.is_whitespace()) || ch.x0 >= ch.x1 {
                continue;
            }

            // OODA-12: Filter characters with tiny/zero font size
            // WHY: PDFs contain metadata characters with font_size=0 or very small.
            // These appear as noise artifacts (like "*y*") in the output.
            // pymupdf4llm filters these via MuPDF's text extraction which ignores them.
            if ch.font_size < 3.0 {
                continue;
            }

            // OODA-07: Filter left margin text (arXiv watermarks, etc.)
            // WHY: pymupdf4llm uses line["dir"] to filter rotated text (get_text_lines.py:121).
            // PDFium doesn't provide direction vectors, so we filter by position.
            // ArXiv watermarks are at x ≈ 10-40pt, well within the 50pt threshold.
            if ch.x1 < self.params.left_margin {
                continue;
            }

            // OODA-IT01: Filter header margin text (page numbers, chapter titles at top)
            // WHY: pymupdf4llm uses header_margin parameter to exclude top region.
            // See multi_column.py:column_boxes() - `clip.y0 += header_margin`.
            // In PDF coordinates, Y=0 is at BOTTOM, but after normalization in extraction_engine.rs,
            // Y=0 is at TOP and increases downward. So header region is y < header_margin.
            if ch.y0 < self.params.header_margin {
                continue;
            }

            // OODA-IT01: Filter footer margin text (footnotes, page numbers at bottom)
            // WHY: pymupdf4llm uses footer_margin parameter to exclude bottom region.
            // See multi_column.py:column_boxes() - `clip.y1 -= footer_margin`.
            // After normalization, footer region is y > (page_height - footer_margin).
            if self.params.footer_margin > 0.0
                && self.params.page_height > 0.0
                && ch.y1 > self.params.page_height - self.params.footer_margin
            {
                continue;
            }

            // OODA-07: Filter right margin text (if right_margin > 0)
            // Currently disabled by default (right_margin = 0)
            // Note: Would need page width to implement properly

            // WHY: Spaces are word boundary markers - they should break spans but not be included
            // This is how pymupdf4llm handles spaces: they mark word boundaries in the text stream
            if ch.char.is_whitespace() {
                // Space character forces word boundary - save current span and start fresh
                if !current_span.text.is_empty() {
                    spans.push(current_span);
                }
                current_span = Span::new(ch.page_num);
                continue; // Don't include the space in any span
            }

            if current_span.can_append(ch) {
                current_span.append(ch);
            } else {
                // Save current span if non-empty
                if !current_span.text.is_empty() {
                    spans.push(current_span);
                }
                // Start new span
                current_span = Span::new(ch.page_num);
                current_span.append(ch);
            }
        }

        // Don't forget the last span
        if !current_span.text.is_empty() {
            spans.push(current_span);
        }

        spans
    }

    /// Group spans into lines based on vertical alignment.
    ///
    /// Spans are grouped on the same line if their baseline (y0) or
    /// top (y1) coordinates are within the tolerance.
    pub fn spans_to_lines(&self, spans: Vec<Span>) -> Vec<Line> {
        if spans.is_empty() {
            return vec![];
        }

        // Sort spans by page, then by y (descending = top first), then by x
        let mut sorted_spans = spans;
        sorted_spans.sort_by(|a, b| {
            a.page_num
                .cmp(&b.page_num)
                .then(b.y1.partial_cmp(&a.y1).unwrap()) // descending y
                .then(a.x0.partial_cmp(&b.x0).unwrap()) // ascending x
        });

        let mut lines = Vec::new();
        let mut current_line = Line::from_span(sorted_spans.remove(0));

        for span in sorted_spans {
            if current_line.can_add_span(&span, self.params.line_tolerance) {
                current_line.add_span(span);
            } else {
                // Finalize current line
                current_line.sort_spans();
                lines.push(current_line);
                // Start new line
                current_line = Line::from_span(span);
            }
        }

        // Don't forget the last line
        current_line.sort_spans();
        lines.push(current_line);

        // OODA-07: Split lines that span multiple columns
        // WHY: In two-column layouts, spans at the same Y level from both columns
        // get merged into one line. We detect large gaps between spans and split.
        let lines = self.split_multi_column_lines(lines);

        // Sort lines by page, then top-to-bottom
        let mut lines = lines;
        lines.sort_by(|a, b| {
            a.page_num
                .cmp(&b.page_num)
                .then(b.y1.partial_cmp(&a.y1).unwrap())
        });

        lines
    }

    /// Split lines that span multiple columns.
    ///
    /// OODA-07: Detects large gaps between consecutive spans that indicate
    /// column boundaries. Typical column gutters are 14-20pt, while word gaps
    /// are < 5pt.
    fn split_multi_column_lines(&self, lines: Vec<Line>) -> Vec<Line> {
        // WHY (OODA-10): 10pt is less than typical column gutter (14-20pt) but larger
        // than word gaps (<5pt). Provides margin for detection uncertainty.
        const COLUMN_GAP_THRESHOLD: f32 = 10.0;

        let mut result = Vec::new();

        for mut line in lines {
            // Sort spans left-to-right
            line.sort_spans();

            if line.spans.len() < 2 {
                result.push(line);
                continue;
            }

            // Find large gaps that indicate column boundaries
            let mut split_points: Vec<usize> = Vec::new();
            for i in 1..line.spans.len() {
                let gap = line.spans[i].x0 - line.spans[i - 1].x1;
                if gap > COLUMN_GAP_THRESHOLD {
                    split_points.push(i);
                }
            }

            if split_points.is_empty() {
                // No column boundary found
                result.push(line);
            } else {
                // Split the line at column boundaries
                let mut start = 0;
                for &split_at in &split_points {
                    let split_spans: Vec<Span> = line.spans[start..split_at].to_vec();
                    if !split_spans.is_empty() {
                        result.push(Line::from_spans(split_spans, line.page_num));
                    }
                    start = split_at;
                }
                // Don't forget the last segment
                let split_spans: Vec<Span> = line.spans[start..].to_vec();
                if !split_spans.is_empty() {
                    result.push(Line::from_spans(split_spans, line.page_num));
                }
            }
        }

        result
    }

    /// Group lines into blocks based on column alignment and vertical proximity.
    ///
    /// This method now includes column detection to handle multi-column layouts:
    /// 1. Separate lines by page
    /// 2. For each page, detect column boundaries
    /// 3. Group lines within each column independently
    /// 4. Process columns in reading order (left to right)
    pub fn lines_to_blocks(&self, lines: Vec<Line>) -> Vec<Block> {
        if lines.is_empty() {
            return vec![];
        }

        // Group lines by page
        let mut pages: std::collections::HashMap<usize, Vec<Line>> =
            std::collections::HashMap::new();
        for line in lines {
            pages.entry(line.page_num).or_default().push(line);
        }

        let mut all_blocks: Vec<Block> = Vec::new();
        // OODA-53: Track if any page has multi-column layout
        let mut has_columns = false;

        // Get sorted page numbers for deterministic output
        let mut page_nums: Vec<usize> = pages.keys().cloned().collect();
        page_nums.sort();

        // Process each page in order
        for page_num in page_nums {
            let page_lines = pages.remove(&page_num).unwrap();
            // Detect columns for this page
            let columns = self.detect_columns(&page_lines);

            if columns.is_empty() {
                // Single column - use simple grouping
                let page_blocks = self.group_lines_simple(page_lines);
                all_blocks.extend(page_blocks);
            } else {
                // Multi-column - assign lines to columns, then group within each
                has_columns = true;
                let page_blocks = self.group_lines_by_column(page_lines, &columns);
                all_blocks.extend(page_blocks);
            }
        }

        // WHY: Phase 2 normalization from pymupdf4llm (multi_column.py lines 213-245)
        // Normalizes x0/x1 boundaries within 3pt tolerance, then merges close blocks
        Self::join_blocks_phase2(&mut all_blocks);

        // OODA-53: Only apply smart sort if NO columns were detected.
        // WHY: group_lines_by_column() already produces blocks in correct column order
        // (left column first, then right). sort_blocks_reading_order() re-sorts by Y
        // coordinate, which interleaves left/right column blocks at the same Y level.
        // This causes "generative ren-" (col 1) → "Given a monocular" (col 2) →
        // "dering" (col 1) instead of reading column 1 fully then column 2.
        if !has_columns {
            self.sort_blocks_reading_order(&mut all_blocks);
        }

        all_blocks
    }

    /// Phase 2 block joining from pymupdf4llm (multi_column.py lines 213-245).
    ///
    /// Algorithm:
    /// 1. Normalize x0/x1 boundaries: align to nearest neighbor within 3pt
    /// 2. Merge blocks with same boundaries and vertical gap <= 10pt
    ///
    /// WHY: This reduces fragmentation by merging paragraphs that should be together.
    fn join_blocks_phase2(blocks: &mut Vec<Block>) {
        const BOUNDARY_TOLERANCE: f32 = 3.0;
        const VERTICAL_GAP_MAX: f32 = 10.0;

        if blocks.len() < 2 {
            return;
        }

        // Phase 2a: Normalize x0/x1 boundaries
        // For each block, find the most common x0/x1 within tolerance and align to it
        let x0_values: Vec<f32> = blocks.iter().map(|b| b.x0).collect();
        let x1_values: Vec<f32> = blocks.iter().map(|b| b.x1).collect();

        for block in blocks.iter_mut() {
            // Normalize x0 to min of nearby values
            let min_x0 = x0_values
                .iter()
                .filter(|&&x| (x - block.x0).abs() <= BOUNDARY_TOLERANCE)
                .fold(block.x0, |acc, &x| acc.min(x));
            block.x0 = min_x0;

            // Normalize x1 to max of nearby values
            let max_x1 = x1_values
                .iter()
                .filter(|&&x| (x - block.x1).abs() <= BOUNDARY_TOLERANCE)
                .fold(block.x1, |acc, &x| acc.max(x));
            block.x1 = max_x1;
        }

        // Sort by (page, x0, y1 descending)
        blocks.sort_by(|a, b| {
            a.page_num
                .cmp(&b.page_num)
                .then(a.x0.partial_cmp(&b.x0).unwrap())
                .then(b.y1.partial_cmp(&a.y1).unwrap()) // top to bottom
        });

        // Phase 2b: Merge blocks with similar boundaries and close Y
        // OODA-IT36: EXCEPT when next block starts with a list marker
        let mut i = 0;
        while i < blocks.len().saturating_sub(1) {
            let can_merge = {
                let current = &blocks[i];
                let next = &blocks[i + 1];

                // OODA-IT36: Never merge a block that starts with a list marker.
                // WHY: join_blocks_phase2 re-merges blocks that can_add_line()
                // already separated. Without this check, bullet items like
                // "• Item A" and "• Item B" get re-merged into one block,
                // making them invisible to ListDetectionProcessor.
                let next_starts_with_bullet = next
                    .lines
                    .first()
                    .map(|l| l.starts_with_list_marker())
                    .unwrap_or(false);

                if next_starts_with_bullet {
                    false
                } else {
                    // Same page
                    current.page_num == next.page_num
                        // Similar left boundary
                        && (current.x0 - next.x0).abs() <= BOUNDARY_TOLERANCE
                        // Similar right boundary
                        && (current.x1 - next.x1).abs() <= BOUNDARY_TOLERANCE
                        // Close vertically (current is above next, gap <= 10pt)
                        && (current.y0 - next.y1).abs() <= VERTICAL_GAP_MAX
                }
            };

            if can_merge {
                // Merge next into current
                let next = blocks.remove(i + 1);
                let current = &mut blocks[i];
                current.lines.extend(next.lines);
                current.y0 = current.y0.min(next.y0);
                current.y1 = current.y1.max(next.y1);
                current.x0 = current.x0.min(next.x0);
                current.x1 = current.x1.max(next.x1);
                // Don't increment i - check the merged block again
            } else {
                i += 1;
            }
        }

        // Re-sort lines within each block
        for block in blocks.iter_mut() {
            block.sort_lines();
        }
    }

    /// Detect column boundaries from lines using histogram-based gutter detection.
    ///
    /// ## OODA-IT07: N-Column Support
    ///
    /// Previous algorithm only found ONE gutter near center (50%), breaking 3-column layouts.
    /// New algorithm scans full width to find ALL gutters, supporting any number of columns.
    ///
    /// ## Algorithm
    ///
    /// ```text
    /// 1. Build coverage histogram (how many lines cover each X position)
    /// 2. Find runs of zero coverage (gutters)
    /// 3. Convert gutters to column boundaries
    ///
    /// Example (3-column):
    ///   Lines:    ████████    ████████    ████████
    ///   X:        0   100  120  220  240   340
    ///   Gutters:        [100-120]   [220-240]
    ///   Columns:  [0,110]   [110,230]   [230,340]
    /// ```
    fn detect_columns(&self, lines: &[Line]) -> Vec<(f32, f32)> {
        if lines.len() < 4 {
            return vec![];
        }

        // Calculate page bounds
        let page_left = lines.iter().map(|l| l.x0).fold(f32::MAX, f32::min);
        let page_right = lines.iter().map(|l| l.x1).fold(f32::MIN, f32::max);
        let page_width = page_right - page_left;

        // WHY (OODA-10): 100pt ≈ 1.4 inches is too small for readable content.
        // Typical pages: US Letter (612pt), A4 (595pt). Skip column detection for
        // unusually narrow content (might be figure captions or marginal notes).
        if page_width < 100.0 {
            return vec![];
        }

        // Build coverage histogram with 5pt resolution
        // WHY 5pt: Balances accuracy vs noise. Smaller catches narrow gutters,
        // but also catches inter-word gaps. Typical gutters are 14-20pt.
        let bucket_width = 5.0;
        let num_buckets = ((page_width / bucket_width).ceil() as usize).max(1);
        let mut coverage = vec![0usize; num_buckets];

        // Count how many lines cover each bucket
        for line in lines {
            // Skip lines that span most of the page (headers, titles)
            if line.x1 - line.x0 > page_width * 0.8 {
                continue;
            }

            let start = ((line.x0 - page_left) / bucket_width) as usize;
            let end = ((line.x1 - page_left) / bucket_width) as usize;
            // WHY slice iteration: Clippy says loop variable is only used for indexing.
            // Using slice iterator avoids the indexing warning.
            for count in coverage[start..=end.min(num_buckets - 1)].iter_mut() {
                *count += 1;
            }
        }

        // Find gutters: runs of zero (or very low) coverage
        // WHY: Gutters are regions where no text exists
        let mut gutters: Vec<f32> = vec![];
        let min_gutter_buckets = 2; // ~10pt minimum gutter width
        let max_coverage = 1; // Allow up to 1 line to cross (tolerates stray chars)
        let mut gutter_start: Option<usize> = None;

        for (i, &count) in coverage.iter().enumerate() {
            if count <= max_coverage {
                if gutter_start.is_none() {
                    gutter_start = Some(i);
                }
            } else if let Some(start) = gutter_start {
                let gutter_width_buckets = i - start;
                if gutter_width_buckets >= min_gutter_buckets {
                    // Gutter center position
                    let gutter_x = page_left + ((start + i) as f32 / 2.0) * bucket_width;
                    // Verify there are lines on both sides
                    let has_left = coverage[..start].iter().any(|&c| c >= 2);
                    let has_right = coverage[i..].iter().any(|&c| c >= 2);
                    if has_left && has_right {
                        gutters.push(gutter_x);
                    }
                }
                gutter_start = None;
            }
        }

        // Handle gutter at end of page
        if let Some(start) = gutter_start {
            let gutter_width_buckets = num_buckets - start;
            if gutter_width_buckets >= min_gutter_buckets {
                let gutter_x = page_left + ((start + num_buckets) as f32 / 2.0) * bucket_width;
                let has_left = coverage[..start].iter().any(|&c| c >= 2);
                if has_left {
                    gutters.push(gutter_x);
                }
            }
        }

        // Convert gutters to column boundaries
        if gutters.is_empty() {
            return vec![];
        }

        // Build columns from gutters
        let mut columns: Vec<(f32, f32)> = vec![];
        let mut prev = page_left;
        for gutter in &gutters {
            columns.push((prev, *gutter));
            prev = *gutter;
        }
        columns.push((prev, page_right));

        // Filter out tiny columns (less than 50pt ≈ 0.7 inch)
        columns.retain(|(x0, x1)| x1 - x0 >= 50.0);

        // If filtering removed too many, fall back to single column
        if columns.len() < 2 {
            return vec![];
        }

        tracing::debug!(
            "OODA-IT07: Detected {} columns from {} gutters",
            columns.len(),
            gutters.len()
        );

        columns
    }

    /// Simple grouping without column detection.
    fn group_lines_simple(&self, mut lines: Vec<Line>) -> Vec<Block> {
        // Sort lines top-to-bottom
        lines.sort_by(|a, b| b.y1.partial_cmp(&a.y1).unwrap());

        let mut blocks: Vec<Block> = Vec::new();

        for line in lines {
            let mut added = false;
            for block in &mut blocks {
                if block.can_add_line(&line, self.params.block_gap) {
                    block.add_line(line.clone());
                    added = true;
                    break;
                }
            }

            if !added {
                blocks.push(Block::from_line(line));
            }
        }

        for block in &mut blocks {
            block.sort_lines();
        }

        blocks
    }

    /// Group lines by column, then within each column.
    fn group_lines_by_column(&self, lines: Vec<Line>, columns: &[(f32, f32)]) -> Vec<Block> {
        // OODA-56: Separate full-width lines from column-specific lines.
        // WHY: Pages with figures often have full-width content (title, figure text,
        // captions) above the two-column body text. Without separation, these lines
        // get assigned to columns incorrectly, scattering title/author/figure content.
        //
        // Strategy:
        // 1. Full-width lines (spanning across the gutter): sort by Y, output first
        // 2. Column-specific lines: group by column, left column first, then right
        //
        // A line is "full-width" if it extends past the gutter region,
        // i.e., it doesn't fit entirely within any single column.

        let mut full_width_lines = Vec::new();
        let mut column_lines: Vec<Vec<Line>> = vec![vec![]; columns.len()];

        for line in lines {
            let line_center = (line.x0 + line.x1) / 2.0;

            // Check if line fits within any single column
            let mut assigned = false;
            for (i, &(col_start, col_end)) in columns.iter().enumerate() {
                // Line center must be within column, AND line must not span too far
                // beyond column boundaries (allow 10% overflow for edge alignment)
                let col_width = col_end - col_start;
                let overflow_tolerance = col_width * 0.10;
                if line_center >= col_start
                    && line_center <= col_end
                    && line.x0 >= col_start - overflow_tolerance
                    && line.x1 <= col_end + overflow_tolerance
                {
                    column_lines[i].push(line.clone());
                    assigned = true;
                    break;
                }
            }

            if !assigned {
                // Line spans multiple columns or doesn't fit any column
                full_width_lines.push(line);
            }
        }

        // Build output: full-width blocks first (in Y order), then column blocks
        let mut all_blocks = Vec::new();

        // Full-width blocks sorted top-to-bottom
        if !full_width_lines.is_empty() {
            let mut fw_blocks = self.group_lines_simple(full_width_lines);
            fw_blocks.sort_by(|a, b| b.y1.partial_cmp(&a.y1).unwrap());
            all_blocks.extend(fw_blocks);
        }

        // Column blocks: left column first, then right column
        for col_lines in column_lines {
            if !col_lines.is_empty() {
                let mut col_blocks = self.group_lines_simple(col_lines);
                // Sort blocks within column: top to bottom
                col_blocks.sort_by(|a, b| b.y1.partial_cmp(&a.y1).unwrap());
                all_blocks.extend(col_blocks);
            }
        }

        all_blocks
    }

    /// Sort blocks in reading order using pymupdf4llm's smart sort key.
    ///
    /// WHY: pymupdf4llm uses a sophisticated reading order algorithm (multi_column.py lines 283-305):
    /// For each block Q, find the left-most block P with vertical overlap.
    /// Sort key = (P.y0, Q.x0), ensuring Q comes after P in reading order.
    ///
    /// ```text
    ///        Q +---------+
    ///          | next is |
    ///    P +-------+  this  |   For block Q: sort key = (P.y0, Q.x0)
    ///      | left  |  block |   This ensures Q comes after P
    ///      | block |        |
    ///      +-------+--------+
    /// ```
    fn sort_blocks_reading_order(&self, blocks: &mut [Block]) {
        if blocks.is_empty() {
            return;
        }

        // Create blocks with computed sort keys
        let mut keyed_blocks: Vec<(&Block, (usize, i32, i32))> = blocks
            .iter()
            .enumerate()
            .map(|(idx, block)| {
                let key = self.compute_smart_sort_key(idx, blocks);
                (block, key)
            })
            .collect();

        // Sort by computed key (page, y_key, x_key)
        keyed_blocks.sort_by_key(|(_, key)| *key);

        // Get the sorted indices
        let sorted_indices: Vec<usize> = keyed_blocks
            .iter()
            .map(|(b, _)| blocks.iter().position(|x| std::ptr::eq(x, *b)).unwrap())
            .collect();

        // Reorder blocks in-place using the sorted order
        // Create a new sorted vector and swap
        let mut sorted: Vec<Block> = Vec::with_capacity(blocks.len());
        for &idx in &sorted_indices {
            sorted.push(blocks[idx].clone());
        }
        blocks.clone_from_slice(&sorted);
    }

    /// Compute smart sort key for a block using pymupdf4llm algorithm.
    ///
    /// WHY: (multi_column.py lines 283-305)
    /// Find the right-most block that is:
    /// 1. To the left of current block (x1 < current.x0)
    /// 2. Has vertical overlap with current block
    ///
    /// Sort key = (left_block.y0, current.x0) if found
    /// Otherwise = (current.y0, current.x0)
    fn compute_smart_sort_key(&self, block_idx: usize, blocks: &[Block]) -> (usize, i32, i32) {
        let block = &blocks[block_idx];

        // Find blocks to the left with vertical overlap
        let left_blocks: Vec<&Block> = blocks
            .iter()
            .filter(|b| {
                // Must be to the left
                b.x1 < block.x0
                    // Same page
                    && b.page_num == block.page_num
                    // Must have vertical overlap
                    && Self::has_vertical_overlap(b, block)
            })
            .collect();

        // Find the right-most of the left blocks (highest x1)
        // WHY (OODA-07): Use y1 (TOP of block) for PDFium coords, not y0 (BOTTOM)
        // PyMuPDF uses y0=TOP (origin at top-left), PDFium uses y1=TOP (origin at bottom-left)
        let y_key = if let Some(left_block) = left_blocks
            .iter()
            .max_by(|a, b| a.x1.partial_cmp(&b.x1).unwrap())
        {
            // Use left block's top Y as the sort key Y
            left_block.y1 as i32 // y1 = TOP in PDFium coords
        } else {
            // No left block found, use own Y
            block.y1 as i32 // y1 = TOP in PDFium coords
        };

        // Convert to integers for stable sorting (Y is inverted because PDF Y=0 is at bottom)
        let y_inverted = -y_key; // Higher Y (top of page) should come first
        let x_key = block.x0 as i32;

        (block.page_num, y_inverted, x_key)
    }

    /// Check if two blocks have vertical overlap using pymupdf4llm's check.
    ///
    /// WHY: pymupdf4llm uses (box.y0 <= r.y0 <= box.y1 or box.y0 <= r.y1 <= box.y1)
    /// This checks if either the top (y0) or bottom (y1) of block `a` falls within
    /// the vertical range of block `b`.
    fn has_vertical_overlap(a: &Block, b: &Block) -> bool {
        // Either a's top is within b's vertical range, or a's bottom is within b's range
        (b.y0 <= a.y0 && a.y0 <= b.y1) || (b.y0 <= a.y1 && a.y1 <= b.y1)
    }

    /// Full pipeline: chars → spans → lines → blocks
    /// OODA-07: Added column split step to handle two-column layouts
    pub fn group(&self, chars: &[RawChar]) -> Vec<Block> {
        let spans = self.chars_to_spans(chars);
        let lines = self.spans_to_lines(spans);
        // Split multi-column lines at large horizontal gaps
        let split_lines = self.split_multi_column_lines(lines);
        self.lines_to_blocks(split_lines)
    }

    /// Detect block types based on content analysis.
    ///
    /// OODA-45 SRP: Delegates to `BlockClassifier` from `block_classifier.rs`.
    ///
    /// This analyzes:
    /// - Font size relative to body text → headers
    /// - Monospace fonts → code blocks
    /// - Bullet/number prefixes → list items
    pub fn classify_blocks(&self, blocks: &mut [Block], body_font_size: f32) {
        // OODA-45: Delegate to BlockClassifier for DRY compliance
        self.classifier.classify_blocks(blocks, body_font_size);
    }

    /// OODA-09: Classify blocks with page awareness for footnote detection.
    /// WHY: Footnotes require page_height to determine if a block is at the
    /// bottom of the page. This groups blocks by page_num and estimates
    /// page_height from block coordinates.
    pub fn classify_blocks_page_aware(&self, blocks: &mut [Block], body_font_size: f32) {
        // Estimate page_height per page from max y coordinate + margin
        let mut page_heights: std::collections::HashMap<usize, f32> =
            std::collections::HashMap::new();
        for block in blocks.iter() {
            let entry = page_heights.entry(block.page_num).or_insert(0.0_f32);
            *entry = entry.max(block.y1);
        }
        // Add margin estimate (1 inch = 72pt) to approximate full page height
        for val in page_heights.values_mut() {
            *val += 72.0;
        }

        // Classify each block with its page's estimated height
        for block in blocks.iter_mut() {
            let page_height = page_heights.get(&block.page_num).copied().unwrap_or(0.0);
            block.block_type = self
                .classifier
                .classify_block(block, body_font_size, page_height);
        }
    }

    /// OODA-11: Split blocks DISABLED - was causing header over-detection.
    ///
    /// PREVIOUS BEHAVIOR: This function scanned multi-line blocks and split off
    /// lines matching header patterns (section numbers, roman numerals, etc.)
    /// creating separate header blocks.
    ///
    /// WHY DISABLED: Comparison with pymupdf4llm gold standards shows that this
    /// aggressive splitting creates ~40 headers when gold has only 1-2.
    /// pymupdf4llm uses MuPDF's native layout detection which is VERY conservative.
    ///
    /// The function now returns blocks unchanged.
    pub fn split_header_blocks(&self, blocks: Vec<Block>) -> Vec<Block> {
        // OODA-11: Return blocks unchanged - no pattern-based header splitting
        blocks
    }

    /// OODA-12: Merge consecutive header blocks that belong to the same title.
    ///
    /// WHY: Paper titles often wrap across multiple lines, creating separate blocks
    /// due to the vertical gap between lines. pymupdf4llm renders these as a single
    /// header line (e.g., "### **Title Part 1** **Title Part 2**").
    ///
    /// We detect title continuation by:
    /// 1. Consecutive blocks both classified as Header
    /// 2. Same header level
    /// 3. Similar dominant font size (within 10%)
    /// 4. Blocks are on the same page
    /// 5. Second block starts within 2x line height of first block's bottom
    pub fn merge_title_blocks(&self, blocks: Vec<Block>) -> Vec<Block> {
        if blocks.len() < 2 {
            return blocks;
        }

        let mut result: Vec<Block> = Vec::with_capacity(blocks.len());
        let mut iter = blocks.into_iter().peekable();

        while let Some(mut current) = iter.next() {
            // Check if we can merge with the next block
            while let Some(next) = iter.peek() {
                // Both must be headers with same level
                let (BlockType::Header(level1), BlockType::Header(level2)) =
                    (&current.block_type, &next.block_type)
                else {
                    break;
                };
                if level1 != level2 {
                    break;
                }

                // Same page
                if current.page_num != next.page_num {
                    break;
                }

                // Similar font size (within 10%)
                let cur_size = current
                    .lines
                    .iter()
                    .map(|l| l.dominant_font_size())
                    .fold(0.0_f32, |a, b| a.max(b));
                let next_size = next
                    .lines
                    .iter()
                    .map(|l| l.dominant_font_size())
                    .fold(0.0_f32, |a, b| a.max(b));
                if cur_size > 0.0 && (cur_size - next_size).abs() / cur_size > 0.1 {
                    break;
                }

                // Vertical proximity: next block should start within 2x line height
                // In PDF coords, current.y0 < next.y1 (next is below current)
                let vertical_gap = current.y0 - next.y1;
                let max_gap = cur_size * 2.5; // Allow 2.5x font size gap for title continuation
                if vertical_gap < 0.0 || vertical_gap > max_gap {
                    break;
                }

                // Merge: take ownership of next block and absorb its lines
                let next_block = iter.next().unwrap();
                for line in next_block.lines {
                    current.add_line(line);
                }
            }

            result.push(current);
        }

        result
    }

    /// OODA-12: Split blocks at bullet/list item lines.
    ///
    /// WHY: Block grouping often merges bullet items with preceding text because
    /// they're vertically close. This splits blocks so each bullet line becomes
    /// the start of a new block, enabling proper list detection.
    ///
    /// Example transformation:
    /// Before: Block["intro text", "• item 1", "• item 2"]
    /// After:  Block["intro text"], Block["• item 1"], Block["• item 2"]
    pub fn split_at_bullet_lines(&self, blocks: Vec<Block>) -> Vec<Block> {
        let mut result = Vec::with_capacity(blocks.len() * 2);

        for block in blocks {
            if block.lines.len() <= 1 {
                result.push(block);
                continue;
            }

            // Look for bullet lines that aren't the first line
            let mut current_lines = Vec::new();
            let page_num = block.page_num;

            for line in block.lines {
                let text = line.text();
                let trimmed = text.trim_start();
                let is_bullet = is_bullet_item(trimmed) || is_numbered_list_item(trimmed);

                if is_bullet && !current_lines.is_empty() {
                    // Save the accumulated lines as a block
                    result.push(Block::from_lines(current_lines.clone(), page_num));
                    current_lines.clear();
                }

                current_lines.push(line);
            }

            // Push remaining lines
            if !current_lines.is_empty() {
                result.push(Block::from_lines(current_lines, page_num));
            }
        }

        result
    }
}

// =============================================================================
// OODA-45: Pattern detection functions moved to block_classifier.rs
// =============================================================================
// The following functions are now imported from super::block_classifier:
//   - is_bullet_item
//   - is_numbered_list_item
//   - is_roman_numeral_header (not exported, used internally by BlockClassifier)
//   - is_letter_subsection_header (not exported, used internally)
//   - is_numeric_section_header (not exported, used internally)
//   - is_numeric_subsection_header (not exported, used internally)
//   - is_abstract_header (not exported, used internally)
//
// This follows SRP: TextGrouper handles grouping, BlockClassifier handles classification.
// =============================================================================

impl Default for TextGrouper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_char(c: char, x0: f32, y0: f32, font_size: f32, page: usize) -> RawChar {
        let width = font_size * 0.6; // Approximate character width
        RawChar {
            char: c,
            x0,
            y0,
            x1: x0 + width,
            y1: y0 + font_size,
            font_size,
            font_name: Some("Arial".to_string()),
            page_num: page,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
        }
    }

    #[test]
    fn test_chars_to_spans() {
        let grouper = TextGrouper::new();

        // Create "Hi" on one line
        // WHY: x positions must be > left_margin (50pt) to avoid filtering
        let chars = vec![
            make_char('H', 60.0, 100.0, 12.0, 0),
            make_char('i', 67.2, 100.0, 12.0, 0),
        ];

        let spans = grouper.chars_to_spans(&chars);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "Hi");
    }

    /// OODA-07: Test mixed font styles in a single line.
    /// WHY: Verifies that chars_to_spans correctly splits on style changes,
    /// not just font name/size changes. This is the integration test for
    /// OODA-02 (bold/italic) and OODA-03 (monospace) style checks.
    #[test]
    fn test_mixed_style_chars_to_spans() {
        let grouper = TextGrouper::new();

        // Create helper for styled chars
        fn make_styled_char(
            c: char,
            x0: f32,
            y0: f32,
            font_size: f32,
            is_bold: bool,
            is_italic: bool,
            is_monospace: bool,
        ) -> RawChar {
            let width = font_size * 0.6;
            RawChar {
                char: c,
                x0,
                y0,
                x1: x0 + width,
                y1: y0 + font_size,
                font_size,
                font_name: Some("Arial".to_string()),
                page_num: 0,
                is_bold,
                is_italic,
                is_monospace,
            }
        }

        // Create "AB" (bold) + "cd" (italic) - adjacent characters with style change
        // WHY: x positions must be adjacent (no word break) but style differs
        let chars = vec![
            make_styled_char('A', 60.0, 100.0, 12.0, true, false, false),
            make_styled_char('B', 67.2, 100.0, 12.0, true, false, false),
            make_styled_char('c', 74.4, 100.0, 12.0, false, true, false), // Style change!
            make_styled_char('d', 81.6, 100.0, 12.0, false, true, false),
        ];

        let spans = grouper.chars_to_spans(&chars);

        // Should produce 2 spans due to style change
        assert_eq!(
            spans.len(),
            2,
            "Expected 2 spans for mixed bold/italic, got {}",
            spans.len()
        );
        assert_eq!(spans[0].text, "AB");
        assert_eq!(spans[1].text, "cd");

        // Verify style flags are preserved
        assert_eq!(
            spans[0].font_is_bold,
            Some(true),
            "First span should be bold"
        );
        assert_eq!(
            spans[0].font_is_italic,
            Some(false),
            "First span should not be italic"
        );
        assert_eq!(
            spans[1].font_is_bold,
            Some(false),
            "Second span should not be bold"
        );
        assert_eq!(
            spans[1].font_is_italic,
            Some(true),
            "Second span should be italic"
        );
    }

    /// OODA-08: Test monospace style transitions.
    /// WHY: Validates chars_to_spans correctly splits on monospace boundaries,
    /// essential for rendering inline code with backticks in Markdown.
    #[test]
    fn test_monospace_style_chars_to_spans() {
        let grouper = TextGrouper::new();

        // Helper for styled chars (monospace-focused)
        fn make_styled_char(
            c: char,
            x0: f32,
            y0: f32,
            font_size: f32,
            is_monospace: bool,
        ) -> RawChar {
            let width = font_size * 0.6;
            RawChar {
                char: c,
                x0,
                y0,
                x1: x0 + width,
                y1: y0 + font_size,
                font_size,
                font_name: Some("Arial".to_string()),
                page_num: 0,
                is_bold: false,
                is_italic: false,
                is_monospace,
            }
        }

        let char_width = 12.0 * 0.6; // 7.2

        // Create "Hi" (normal) + "code" (monospace) + "!" (normal)
        // WHY: Tests both normal→mono and mono→normal transitions
        let chars = vec![
            make_styled_char('H', 60.0, 100.0, 12.0, false),
            make_styled_char('i', 60.0 + char_width, 100.0, 12.0, false),
            make_styled_char('c', 60.0 + char_width * 2.0, 100.0, 12.0, true), // mono start
            make_styled_char('o', 60.0 + char_width * 3.0, 100.0, 12.0, true),
            make_styled_char('d', 60.0 + char_width * 4.0, 100.0, 12.0, true),
            make_styled_char('e', 60.0 + char_width * 5.0, 100.0, 12.0, true),
            make_styled_char('!', 60.0 + char_width * 6.0, 100.0, 12.0, false), // back to normal
        ];

        let spans = grouper.chars_to_spans(&chars);

        // Should produce 3 spans: "Hi" (normal) + "code" (mono) + "!" (normal)
        assert_eq!(
            spans.len(),
            3,
            "Expected 3 spans for normal/mono/normal, got {}",
            spans.len()
        );
        assert_eq!(spans[0].text, "Hi");
        assert_eq!(spans[1].text, "code");
        assert_eq!(spans[2].text, "!");

        // Verify monospace flags
        assert_eq!(
            spans[0].font_is_monospace,
            Some(false),
            "First span should not be monospace"
        );
        assert_eq!(
            spans[1].font_is_monospace,
            Some(true),
            "Second span should be monospace"
        );
        assert_eq!(
            spans[2].font_is_monospace,
            Some(false),
            "Third span should not be monospace"
        );
    }

    #[test]
    fn test_spans_to_lines() {
        let grouper = TextGrouper::new();

        // Two spans on same line
        let spans = vec![
            Span {
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
            },
            Span {
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
            },
        ];

        let lines = grouper.spans_to_lines(spans);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text(), "Hello World");
    }

    #[test]
    fn test_full_pipeline() {
        let grouper = TextGrouper::new();

        // Create two lines of text
        // WHY: x positions must be > left_margin (50pt) to avoid filtering
        let chars = vec![
            // Line 1: "Hi"
            make_char('H', 60.0, 100.0, 12.0, 0),
            make_char('i', 67.2, 100.0, 12.0, 0),
            // Line 2: "Bye" (lower y = below line 1)
            make_char('B', 60.0, 85.0, 12.0, 0),
            make_char('y', 67.2, 85.0, 12.0, 0),
            make_char('e', 74.4, 85.0, 12.0, 0),
        ];

        let blocks = grouper.group(&chars);

        // Should produce one block with two lines
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].lines.len(), 2);
    }

    #[test]
    fn test_numbered_list_detection() {
        assert!(is_numbered_list_item("1. First item"));
        assert!(is_numbered_list_item("23) Item"));
        assert!(is_numbered_list_item("5: Something"));
        assert!(!is_numbered_list_item("No number here"));
        assert!(!is_numbered_list_item("a. Letter prefix"));
        // OODA-10: Section headers should NOT match as list items
        assert!(!is_numbered_list_item("2.1. Agentic Training"));
        assert!(!is_numbered_list_item("3.2. Agent Architecture"));
        assert!(!is_numbered_list_item("10.5 Something"));
    }

    #[test]
    fn test_block_classification() {
        let grouper = TextGrouper::new();
        let body_size = 12.0;

        // Header block (larger font)
        let mut header_block = Block::from_line(Line {
            spans: vec![Span {
                text: "Title".to_string(),
                x0: 10.0,
                y0: 100.0,
                x1: 100.0,
                y1: 130.0,
                font_size: 24.0,
                font_name: Some("Arial-Bold".to_string()),
                page_num: 0,
                font_is_bold: Some(true),
                font_is_italic: None,
                font_is_monospace: None,
            }],
            x0: 10.0,
            y0: 100.0,
            x1: 100.0,
            y1: 130.0,
            page_num: 0,
        });

        grouper.classify_blocks(std::slice::from_mut(&mut header_block), body_size);
        assert!(matches!(header_block.block_type, BlockType::Header(1)));

        // List item block
        let mut list_block = Block::from_line(Line {
            spans: vec![Span {
                text: "• Item one".to_string(),
                x0: 10.0,
                y0: 50.0,
                x1: 100.0,
                y1: 62.0,
                font_size: 12.0,
                font_name: Some("Arial".to_string()),
                page_num: 0,
                font_is_bold: None,
                font_is_italic: None,
                font_is_monospace: None,
            }],
            x0: 10.0,
            y0: 50.0,
            x1: 100.0,
            y1: 62.0,
            page_num: 0,
        });

        grouper.classify_blocks(std::slice::from_mut(&mut list_block), body_size);
        assert_eq!(list_block.block_type, BlockType::ListItem);
    }

    /// OODA-IT01: Test header margin filtering.
    /// WHY: Verifies that characters in the header margin (top of page) are filtered out.
    /// This matches pymupdf4llm's header_margin parameter.
    #[test]
    fn test_header_margin_filtering() {
        let mut params = GroupingParams::default();
        params.header_margin = 50.0; // Filter top 50pt
        params.footer_margin = 0.0; // Disable footer filtering for this test
        params.page_height = 792.0; // US Letter height
        let grouper = TextGrouper::with_params(params);

        // Create chars: one in header margin (y=30), one in main content (y=100)
        let chars = vec![
            // This char is at y=30, which is < header_margin=50, should be filtered
            make_char('H', 60.0, 30.0, 12.0, 0),
            make_char('i', 67.2, 30.0, 12.0, 0),
            // This char is at y=100, which is > header_margin=50, should be kept
            make_char('O', 60.0, 100.0, 12.0, 0),
            make_char('K', 67.2, 100.0, 12.0, 0),
        ];

        let spans = grouper.chars_to_spans(&chars);

        // Only "OK" should remain (header text "Hi" filtered out)
        assert_eq!(spans.len(), 1, "Expected 1 span after header filtering");
        assert_eq!(
            spans[0].text, "OK",
            "Expected 'OK', got '{}'",
            spans[0].text
        );
    }

    /// OODA-IT01: Test footer margin filtering.
    /// WHY: Verifies that characters in the footer margin (bottom of page) are filtered out.
    /// This matches pymupdf4llm's footer_margin parameter.
    #[test]
    fn test_footer_margin_filtering() {
        let mut params = GroupingParams::default();
        params.header_margin = 0.0; // Disable header filtering for this test
        params.footer_margin = 50.0; // Filter bottom 50pt
        params.page_height = 792.0; // US Letter height
        let grouper = TextGrouper::with_params(params);

        // Create chars: one in main content (y=100), one in footer margin (y=760)
        let chars = vec![
            // This char is at y=100, well within content area, should be kept
            make_char('O', 60.0, 100.0, 12.0, 0),
            make_char('K', 67.2, 100.0, 12.0, 0),
            // This char is at y=760, which is > (page_height - footer_margin) = 742, should be filtered
            make_char('P', 60.0, 760.0, 12.0, 0),
            make_char('g', 67.2, 760.0, 12.0, 0),
        ];

        let spans = grouper.chars_to_spans(&chars);

        // Only "OK" should remain (footer text "Pg" filtered out)
        assert_eq!(spans.len(), 1, "Expected 1 span after footer filtering");
        assert_eq!(
            spans[0].text, "OK",
            "Expected 'OK', got '{}'",
            spans[0].text
        );
    }

    /// OODA-IT01: Test combined header and footer margin filtering.
    /// WHY: Verifies that both margins work together to filter page chrome.
    #[test]
    fn test_header_and_footer_margin_filtering() {
        let mut params = GroupingParams::default();
        params.header_margin = 50.0;
        params.footer_margin = 50.0;
        params.page_height = 792.0;
        let grouper = TextGrouper::with_params(params);

        // Create chars: header, content, footer
        let chars = vec![
            // Header (y=30 < 50) - should be filtered
            make_char('H', 60.0, 30.0, 12.0, 0),
            make_char('D', 67.2, 30.0, 12.0, 0),
            // Content (y=400, middle of page) - should be kept
            make_char('O', 60.0, 400.0, 12.0, 0),
            make_char('K', 67.2, 400.0, 12.0, 0),
            // Footer (y=760 > 742) - should be filtered
            make_char('F', 60.0, 760.0, 12.0, 0),
            make_char('T', 67.2, 760.0, 12.0, 0),
        ];

        let spans = grouper.chars_to_spans(&chars);

        // Only "OK" should remain
        assert_eq!(
            spans.len(),
            1,
            "Expected 1 span after header+footer filtering"
        );
        assert_eq!(spans[0].text, "OK");
    }

    /// OODA-IT07: Test two-column detection with histogram algorithm.
    /// WHY: Validates that detect_columns correctly finds a gutter between two text columns.
    #[test]
    fn test_detect_two_columns() {
        let grouper = TextGrouper::new();

        // Create two columns of lines:
        // Column 1: x=[50, 200], Column 2: x=[300, 450]
        // Gutter at x=[200, 300] (100pt gap)
        let lines = vec![
            // Left column lines
            Line::new_with_bbox(50.0, 100.0, 200.0, 112.0),
            Line::new_with_bbox(50.0, 120.0, 200.0, 132.0),
            Line::new_with_bbox(50.0, 140.0, 200.0, 152.0),
            Line::new_with_bbox(50.0, 160.0, 200.0, 172.0),
            // Right column lines
            Line::new_with_bbox(300.0, 100.0, 450.0, 112.0),
            Line::new_with_bbox(300.0, 120.0, 450.0, 132.0),
            Line::new_with_bbox(300.0, 140.0, 450.0, 152.0),
            Line::new_with_bbox(300.0, 160.0, 450.0, 172.0),
        ];

        let columns = grouper.detect_columns(&lines);

        // Should detect 2 columns
        assert_eq!(
            columns.len(),
            2,
            "Expected 2 columns, got {}",
            columns.len()
        );

        // First column should cover [50, ~250] (left edge to mid-gutter)
        assert!(
            columns[0].0 < 100.0 && columns[0].1 > 200.0 && columns[0].1 < 300.0,
            "Column 1 bounds unexpected: {:?}",
            columns[0]
        );

        // Second column should cover [~250, 450] (mid-gutter to right edge)
        assert!(
            columns[1].0 > 200.0 && columns[1].0 < 300.0 && columns[1].1 > 400.0,
            "Column 2 bounds unexpected: {:?}",
            columns[1]
        );
    }

    /// OODA-IT07: Test three-column detection with histogram algorithm.
    /// WHY: Previous detect_columns only supported 2 columns (searched near center).
    /// This test verifies the histogram-based algorithm finds multiple gutters.
    #[test]
    fn test_detect_three_columns() {
        let grouper = TextGrouper::new();

        // Create three columns of lines:
        // Column 1: x=[50, 150], Column 2: x=[200, 300], Column 3: x=[350, 450]
        // Gutters at x=[150, 200] and x=[300, 350] (50pt gaps)
        let lines = vec![
            // Left column lines
            Line::new_with_bbox(50.0, 100.0, 150.0, 112.0),
            Line::new_with_bbox(50.0, 120.0, 150.0, 132.0),
            Line::new_with_bbox(50.0, 140.0, 150.0, 152.0),
            Line::new_with_bbox(50.0, 160.0, 150.0, 172.0),
            // Middle column lines
            Line::new_with_bbox(200.0, 100.0, 300.0, 112.0),
            Line::new_with_bbox(200.0, 120.0, 300.0, 132.0),
            Line::new_with_bbox(200.0, 140.0, 300.0, 152.0),
            Line::new_with_bbox(200.0, 160.0, 300.0, 172.0),
            // Right column lines
            Line::new_with_bbox(350.0, 100.0, 450.0, 112.0),
            Line::new_with_bbox(350.0, 120.0, 450.0, 132.0),
            Line::new_with_bbox(350.0, 140.0, 450.0, 152.0),
            Line::new_with_bbox(350.0, 160.0, 450.0, 172.0),
        ];

        let columns = grouper.detect_columns(&lines);

        // Should detect 3 columns
        assert_eq!(
            columns.len(),
            3,
            "Expected 3 columns, got {}",
            columns.len()
        );

        // Verify column order (left to right)
        assert!(
            columns[0].0 < columns[1].0 && columns[1].0 < columns[2].0,
            "Columns should be ordered left to right: {:?}",
            columns
        );

        // Each column should have reasonable bounds
        for (i, col) in columns.iter().enumerate() {
            assert!(col.0 < col.1, "Column {} has invalid bounds: {:?}", i, col);
        }
    }

    /// OODA-IT07: Test single-column (no gutter) detection.
    /// WHY: When lines span the full width, should return empty (single column = no detection needed).
    #[test]
    fn test_detect_single_column() {
        let grouper = TextGrouper::new();

        // Create full-width lines (no gutter)
        let lines = vec![
            Line::new_with_bbox(50.0, 100.0, 450.0, 112.0),
            Line::new_with_bbox(50.0, 120.0, 450.0, 132.0),
            Line::new_with_bbox(50.0, 140.0, 450.0, 152.0),
            Line::new_with_bbox(50.0, 160.0, 450.0, 172.0),
        ];

        let columns = grouper.detect_columns(&lines);

        // Should return empty (no columns detected = single column layout)
        // WHY: detect_columns returns [] for single-column, not [(0, width)]
        assert!(
            columns.is_empty(),
            "Expected empty columns for full-width text, got {:?}",
            columns
        );
    }
}
