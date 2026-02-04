//! Text grouping algorithms for pymupdf4llm-inspired extraction.
//!
//! This module provides the `TextGrouper` that converts a stream of `RawChar`s
//! into structured `Span`s, `Line`s, and `Block`s.
//!
//! ## Algorithm
//!
//! 1. **Char → Span**: Group consecutive chars with same font style
//! 2. **Span → Line**: Group spans on same baseline (vertical tolerance)
//! 3. **Line → Block**: Group lines in same column with vertical proximity
//!
//! This mirrors the pymupdf4llm approach but implemented in pure Rust.

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
            column_overlap: 0.5,
        }
    }
}

/// Groups raw characters into spans, lines, and blocks.
pub struct TextGrouper {
    params: GroupingParams,
}

impl TextGrouper {
    /// Create a new text grouper with default parameters.
    pub fn new() -> Self {
        Self {
            params: GroupingParams::default(),
        }
    }

    /// Create a text grouper with custom parameters.
    pub fn with_params(params: GroupingParams) -> Self {
        Self { params }
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
        // TODO: Implement proper vertical text detection using character sequence analysis
        // The aspect ratio approach doesn't work because normal text often has height > width
        true
    }

    /// Group raw characters into spans.
    ///
    /// Characters are grouped when they have:
    /// - Same page
    /// - Same font name
    /// - Similar font size (within 0.5pt)
    /// - Horizontal adjacency (gap < 1.5 * char width)
    /// - Vertical alignment (within font_size * 0.3)
    pub fn chars_to_spans(&self, chars: &[RawChar]) -> Vec<Span> {
        if chars.is_empty() {
            return vec![];
        }

        let mut spans = Vec::new();
        let mut current_span = Span::new(chars[0].page_num);

        for ch in chars {
            // Skip control characters (except space) and zero-width chars
            if (ch.char.is_control() && !ch.char.is_whitespace()) || ch.x0 >= ch.x1 {
                continue;
            }

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

        // Sort lines by page, then top-to-bottom
        lines.sort_by(|a, b| {
            a.page_num
                .cmp(&b.page_num)
                .then(b.y1.partial_cmp(&a.y1).unwrap())
        });

        lines
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
                let page_blocks = self.group_lines_by_column(page_lines, &columns);
                all_blocks.extend(page_blocks);
            }
        }

        // WHY: Phase 2 normalization from pymupdf4llm (multi_column.py lines 213-245)
        // Normalizes x0/x1 boundaries within 3pt tolerance, then merges close blocks
        Self::join_blocks_phase2(&mut all_blocks);

        // Apply reading order sorting (left column first, then right)
        self.sort_blocks_reading_order(&mut all_blocks);

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
        let mut i = 0;
        while i < blocks.len().saturating_sub(1) {
            let can_merge = {
                let current = &blocks[i];
                let next = &blocks[i + 1];

                // Same page
                current.page_num == next.page_num
                    // Similar left boundary
                    && (current.x0 - next.x0).abs() <= BOUNDARY_TOLERANCE
                    // Similar right boundary
                    && (current.x1 - next.x1).abs() <= BOUNDARY_TOLERANCE
                    // Close vertically (current is above next, gap <= 10pt)
                    && (current.y0 - next.y1).abs() <= VERTICAL_GAP_MAX
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

    /// Detect column boundaries from lines.
    ///
    /// Algorithm:
    /// 1. Find horizontal gaps between line bounding boxes
    /// 2. If a gap appears consistently across many lines, it's a column gutter
    fn detect_columns(&self, lines: &[Line]) -> Vec<(f32, f32)> {
        if lines.len() < 4 {
            return vec![];
        }

        // Calculate page bounds
        let page_left = lines.iter().map(|l| l.x0).fold(f32::MAX, f32::min);
        let page_right = lines.iter().map(|l| l.x1).fold(f32::MIN, f32::max);
        let page_width = page_right - page_left;

        if page_width < 100.0 {
            return vec![];
        }

        // Collect all line boundaries
        let mut line_bounds: Vec<(f32, f32)> = lines.iter().map(|l| (l.x0, l.x1)).collect();
        line_bounds.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // Check for gaps near the center of the page
        let center = page_left + page_width / 2.0;
        let search_range = page_width * 0.2; // Search within ±20% of center

        // Scan for lines that don't cross the center region
        let mut left_count = 0;
        let mut right_count = 0;
        let mut gap_start = center - search_range;
        let mut gap_end = center + search_range;

        for &(x0, x1) in &line_bounds {
            // Skip lines that span most of the page (headers, etc.)
            if x1 - x0 > page_width * 0.8 {
                continue;
            }

            // Check if this line is fully to the left or right of center
            if x1 < center - search_range * 0.1 {
                left_count += 1;
                gap_start = gap_start.max(x1);
            } else if x0 > center + search_range * 0.1 {
                right_count += 1;
                gap_end = gap_end.min(x0);
            }
        }

        // Find candidate gutters
        let mut gutter_candidates: Vec<(f32, f32, usize)> = vec![]; // (start, end, count)

        // Need at least 3 lines on each side
        let min_lines_per_column = 3;

        if left_count >= min_lines_per_column && right_count >= min_lines_per_column {
            // Refine gutter bounds
            let gutter_width = gap_end - gap_start;
            if gutter_width >= 10.0 && gutter_width < page_width * 0.3 {
                gutter_candidates.push((gap_start, gap_end, left_count + right_count));
            }
        }

        // Return column boundaries
        if !gutter_candidates.is_empty() {
            // Sort by count (most evidence first)
            gutter_candidates.sort_by_key(|&(_, _, count)| std::cmp::Reverse(count));
            let best = gutter_candidates[0];
            let gutter_center = (best.0 + best.1) / 2.0;

            // Return two columns: left and right of gutter
            vec![(page_left, gutter_center), (gutter_center, page_right)]
        } else {
            vec![]
        }
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
        // Assign each line to a column
        let mut column_lines: Vec<Vec<Line>> = vec![vec![]; columns.len()];

        for line in lines {
            let line_center = (line.x0 + line.x1) / 2.0;

            // Find which column this line belongs to
            let mut assigned = false;
            for (i, &(col_start, col_end)) in columns.iter().enumerate() {
                if line_center >= col_start && line_center <= col_end {
                    column_lines[i].push(line.clone());
                    assigned = true;
                    break;
                }
            }

            // If line spans multiple columns (like a header), assign to first column
            if !assigned && !column_lines.is_empty() {
                column_lines[0].push(line);
            }
        }

        // Group lines within each column, then concatenate
        // Blocks are already in reading order within each column
        // We just need to process left column fully before right column
        let mut all_blocks = Vec::new();
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
            .enumerate()
            .map(|(_, (b, _))| blocks.iter().position(|x| std::ptr::eq(x, *b)).unwrap())
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
    pub fn group(&self, chars: &[RawChar]) -> Vec<Block> {
        let spans = self.chars_to_spans(chars);
        let lines = self.spans_to_lines(spans);
        self.lines_to_blocks(lines)
    }

    /// Detect block types based on content analysis.
    ///
    /// This analyzes:
    /// - Font size relative to body text → headers
    /// - Monospace fonts → code blocks
    /// - Bullet/number prefixes → list items
    pub fn classify_blocks(&self, blocks: &mut [Block], body_font_size: f32) {
        for block in blocks {
            block.block_type = self.classify_block(block, body_font_size);
        }
    }

    fn classify_block(&self, block: &Block, body_font_size: f32) -> BlockType {
        if block.lines.is_empty() {
            return BlockType::Paragraph;
        }

        // Check for code block (all monospace)
        let all_mono = block
            .lines
            .iter()
            .all(|line| line.spans.iter().all(|span| span.is_monospace()));
        if all_mono {
            return BlockType::Code;
        }

        // Get first line text for pattern matching
        let first_text = block.lines.first().map(|l| l.text()).unwrap_or_default();
        let trimmed = first_text.trim();

        // OODA-10: Pattern-based header detection for academic papers
        // WHY: Many IEEE-style papers use Roman numerals (I., II.) for sections
        // and letters (A., B.) for subsections, but these often have the SAME
        // font size as body text. Font-size detection fails for these.
        // Text pattern matching is more reliable for structured documents.
        if block.lines.len() <= 2 {
            // Check for Roman numeral section headers (level 2)
            // Patterns: "I. INTRODUCTION", "II. RELATED WORKS", "III. METHOD"
            if is_roman_numeral_header(trimmed) {
                return BlockType::Header(2);
            }

            // Check for letter subsection headers (level 3)
            // Patterns: "A. Background", "B. Policy Representations"
            if is_letter_subsection_header(trimmed) {
                return BlockType::Header(3);
            }
        }

        // Check for header (larger font size, single line usually)
        let dominant_size = block
            .lines
            .iter()
            .map(|l| l.dominant_font_size())
            .fold(0.0_f32, |a, b| a.max(b));

        if dominant_size > body_font_size * 1.2 && block.lines.len() <= 2 {
            // Map size ratio to header level
            // WHY adjusted thresholds: Academic papers often have title/section fonts
            // at 1.4-1.5x body size. The old thresholds assigned H3+ to these,
            // but pymupdf4llm treats the largest font as H1.
            // These adjusted thresholds better match pymupdf4llm's output.
            let ratio = dominant_size / body_font_size;
            let level = if ratio >= 1.8 {
                1
            } else if ratio >= 1.4 {
                2
            } else if ratio >= 1.3 {
                3
            } else if ratio >= 1.25 {
                4
            } else {
                5
            };
            return BlockType::Header(level);
        }

        // Check for list item
        if let Some(first_line) = block.lines.first() {
            let text = first_line.text();
            let trimmed = text.trim_start();
            if trimmed.starts_with("• ")
                || trimmed.starts_with("- ")
                || trimmed.starts_with("* ")
                || is_numbered_list_item(trimmed)
            {
                return BlockType::ListItem;
            }
        }

        BlockType::Paragraph
    }
}

/// Check if text starts with a Roman numeral section pattern.
/// OODA-10: Detects "I. INTRODUCTION", "II. RELATED WORKS", etc.
///
/// WHY: IEEE-style papers use Roman numerals (I-X) for major sections.
/// These sections are typically level 2 headings (##).
fn is_roman_numeral_header(text: &str) -> bool {
    // Must have at least 3 chars: "I. X"
    if text.len() < 4 {
        return false;
    }

    let mut chars = text.chars().peekable();

    // Collect Roman numeral characters (I, V, X)
    let mut has_roman = false;
    while let Some(&c) = chars.peek() {
        if c == 'I' || c == 'V' || c == 'X' {
            has_roman = true;
            chars.next();
        } else {
            break;
        }
    }

    if !has_roman {
        return false;
    }

    // Must be followed by "." and space
    match (chars.next(), chars.next()) {
        (Some('.'), Some(' ')) => {
            // Rest should be mostly uppercase (section title)
            let rest: String = chars.collect();
            let uppercase_count = rest.chars().filter(|c| c.is_uppercase()).count();
            let alpha_count = rest.chars().filter(|c| c.is_alphabetic()).count();
            // At least 50% uppercase indicates a section title
            alpha_count > 0 && (uppercase_count as f32 / alpha_count as f32) >= 0.5
        }
        _ => false,
    }
}

/// Check if text starts with a letter subsection pattern.
/// OODA-10: Detects "A. Background", "B. Policy Representations", etc.
///
/// WHY: IEEE-style papers use single letters (A-Z) for subsections.
/// These are typically level 3 headings (###).
///
/// NOTE: Excludes I, V, X which are Roman numerals (handled by is_roman_numeral_header).
fn is_letter_subsection_header(text: &str) -> bool {
    // Must have at least 4 chars: "A. X"
    if text.len() < 4 {
        return false;
    }

    let mut chars = text.chars();
    let first = chars.next();
    let second = chars.next();
    let third = chars.next();

    // Pattern: single uppercase letter + "." + space
    // Exclude I, V, X which are Roman numerals (they're handled by is_roman_numeral_header)
    match (first, second, third) {
        (Some(c), Some('.'), Some(' '))
            if c.is_ascii_uppercase() && c != 'I' && c != 'V' && c != 'X' =>
        {
            // Rest should start with uppercase (subsection title)
            // e.g., "A. Background" or "A. Humanoid Manipulation"
            chars.next().map(|c| c.is_uppercase()).unwrap_or(false)
        }
        _ => false,
    }
}

impl Default for TextGrouper {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if text starts with a numbered list item pattern.
fn is_numbered_list_item(text: &str) -> bool {
    let mut chars = text.chars().peekable();

    // Check for digit(s)
    let mut has_digit = false;
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            has_digit = true;
            chars.next();
        } else {
            break;
        }
    }

    if !has_digit {
        return false;
    }

    // Check for separator (., ), :)
    match chars.next() {
        Some('.') | Some(')') | Some(':') => true,
        _ => false,
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
        }
    }

    #[test]
    fn test_chars_to_spans() {
        let grouper = TextGrouper::new();

        // Create "Hi" on one line
        let chars = vec![
            make_char('H', 10.0, 100.0, 12.0, 0),
            make_char('i', 17.2, 100.0, 12.0, 0),
        ];

        let spans = grouper.chars_to_spans(&chars);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "Hi");
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
        let chars = vec![
            // Line 1: "Hi"
            make_char('H', 10.0, 100.0, 12.0, 0),
            make_char('i', 17.2, 100.0, 12.0, 0),
            // Line 2: "Bye" (lower y = below line 1)
            make_char('B', 10.0, 85.0, 12.0, 0),
            make_char('y', 17.2, 85.0, 12.0, 0),
            make_char('e', 24.4, 85.0, 12.0, 0),
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
    }

    #[test]
    fn test_block_classification() {
        let mut grouper = TextGrouper::new();
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
}
