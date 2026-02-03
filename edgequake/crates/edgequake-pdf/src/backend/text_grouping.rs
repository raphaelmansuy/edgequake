//! Text grouping and line merging utilities.
//!
//! This module handles grouping text elements into lines and handling column layouts.
//! It includes:
//! - Single-column text grouping
//! - Two-column layout detection and handling
//! - Line merging with proper spacing
//! - Vertical gap detection for separating content regions

use super::elements::TextElement;
use crate::schema::{FontStyle, TextSpan};
use tracing::{debug, info};

/// Merged line with text, font size, and style spans
#[derive(Debug, Clone)]
pub struct MergedLine {
    pub text: String,
    pub avg_font_size: f32,
    pub spans: Vec<TextSpan>,
}

/// Text grouping engine for organizing text elements into lines
pub struct TextGrouper {
    // Configuration could be added here if needed
}

impl TextGrouper {
    pub fn new() -> Self {
        Self {}
    }

    /// Safely truncate a string to a maximum byte length without breaking UTF-8.
    /// Returns a string slice that ends at a valid UTF-8 character boundary.
    /// OODA-03: Fixes panic when slicing multibyte characters like box-drawing '─'.
    #[inline]
    fn safe_truncate(s: &str, max_bytes: usize) -> &str {
        if s.len() <= max_bytes {
            return s;
        }
        // Find the last valid char boundary at or before max_bytes
        let mut end = max_bytes;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        &s[..end]
    }

    /// Group text elements into lines with proper column handling.
    /// For two-column layouts: reads left column top-to-bottom, then right column.
    /// Returns (lines, detected_columns) where detected_columns are BoundingBoxes for each column.
    pub fn group_into_lines(
        &self,
        elements: Vec<TextElement>,
        page_width: f32,
        _page_height: f32,
        column_boundary: Option<f32>,
    ) -> Vec<Vec<TextElement>> {
        if elements.is_empty() {
            return Vec::new();
        }

        // Sort by Y ascending (lower Y = top of page after normalization)
        // After Y-normalization in extraction engine, Y=0 is at top of page
        let mut elements = elements;
        elements.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal));

        // If two-column layout detected, separate columns first
        if let Some(boundary) = column_boundary {
            info!(
                "TG-TWOCOL: Using two-column layout with boundary={:.1}",
                boundary
            );
            return self.group_two_column_layout(elements, boundary, page_width);
        }

        info!("TG-SINGLE: Using single-column layout");
        // Single-column layout: group into Y-bands
        self.group_single_column_layout(elements)
    }

    /// Handle two-column layout: separate left and right columns, then process each.
    /// Uses footer filtering and handles spanning elements.
    fn group_two_column_layout(
        &self,
        elements: Vec<TextElement>,
        column_boundary: f32,
        _page_width: f32,
    ) -> Vec<Vec<TextElement>> {
        let mut left_column: Vec<TextElement> = Vec::new();
        let mut right_column: Vec<TextElement> = Vec::new();
        let mut spanning_elements: Vec<TextElement> = Vec::new();
        let mut left_footer: Vec<TextElement> = Vec::new();
        let mut right_footer: Vec<TextElement> = Vec::new();

        // Calculate adaptive thresholds based on actual content distribution
        // This is a first-principles approach that adapts to different document layouts
        let (
            footer_threshold,
            header_threshold,
            title_threshold,
            affiliation_threshold,
            large_font_threshold,
        ) = calculate_adaptive_region_thresholds(&elements);

        // OODA-05 DEBUG: Log thresholds for first 3 elements
        if !elements.is_empty() {
            info!(
                "TG-THRESHOLDS: footer={:.1} header={:.1} title={:.1} affil={:.1} large_font={:.1} elem_count={}",
                footer_threshold, header_threshold, title_threshold, affiliation_threshold, large_font_threshold, elements.len()
            );
            // Log first few elements with Y and font_size
            for (i, elem) in elements.iter().take(3).enumerate() {
                info!(
                    "TG-ELEM[{}]: Y={:.1} font={:.1} text='{}'",
                    i,
                    elem.y,
                    elem.font_size,
                    Self::safe_truncate(&elem.text, 40)
                );
            }
        }

        // Margin around column boundary for classification
        let margin = 15.0;

        for elem in elements {
            // Skip very small elements (likely artifacts)
            if elem.text.trim().is_empty() {
                continue;
            }

            // OODA-13 DEBUG: Track specific element
            if elem.text.contains("Groom") {
                info!(
                    "GROOM-TRACK: Y={:.1} X={:.1} font={:.1} header_thresh={:.1} footer_thresh={:.1} text='{}'",
                    elem.y, elem.x, elem.font_size, header_threshold, footer_threshold,
                    Self::safe_truncate(&elem.text, 40)
                );
            }

            // WHY (OODA-05): After Y-normalization, Y=0 is at TOP of page, Y increases downward.
            // So footer (visual bottom) = LARGE Y, and title (visual top) = SMALL Y.

            // Check if element is in footer region (visual bottom = large Y)
            let is_footer = elem.y > footer_threshold;

            // Check if element is in header region (visual top but small font = running header)
            // Running headers are at the very top (small Y) but with small font size
            // OODA-13 FIX: Also check that text LOOKS like a running header
            // Running headers are typically:
            // - Short (< 80 chars) - paper titles, page numbers, journal names
            // - Don't continue sentences (don't end with continuation chars like comma, hyphen, "the", etc.)
            // - Often contain page numbers, dates, or all-caps words
            // Body text at Y=0 (paragraph continuation from previous page) should NOT be classified as header
            let looks_like_running_header = {
                let text_len = elem.text.len();
                let trimmed = elem.text.trim();
                // Short text is more likely a header
                let is_short = text_len < 80;
                // Check if text looks like paragraph continuation (ends with hyphen, comma, lowercase)
                let is_continuation = trimmed.ends_with('-')
                    || trimmed.ends_with(',')
                    || trimmed.ends_with("the")
                    || trimmed.ends_with("a")
                    || trimmed.ends_with("and")
                    || trimmed.ends_with("or")
                    || trimmed.ends_with("of");
                // Headers often have numbers (page numbers) or are all caps
                let has_page_number = trimmed.chars().any(|c| c.is_ascii_digit());
                let has_uppercase_word = trimmed
                    .split_whitespace()
                    .any(|w| w.len() > 2 && w.chars().all(|c| c.is_uppercase()));

                // It's a header if it's short AND NOT a continuation AND has header-like features
                // OR if it's very short (likely just a page number)
                (is_short && !is_continuation && (has_page_number || has_uppercase_word))
                    || text_len < 15 // Very short = likely page number or header
            };
            let is_header = elem.y < header_threshold
                && elem.font_size < large_font_threshold
                && looks_like_running_header;

            // Check if element is affiliation/metadata (between body and footer)
            // These include: university names, emails, conference submission lines
            // Affiliation zone is near footer (large Y) but not quite at the bottom
            let _is_affiliation_zone = elem.y > affiliation_threshold && elem.y <= footer_threshold;
            let looks_like_affiliation = elem.text.contains('@')
                || elem.text.contains("University")
                || elem.text.contains("School of")
                || elem.text.contains("Department")
                || elem.text.contains("Correspondence")
                || elem.text.contains("Submitted to")
                // REMOVED: "Conference" - this matches paper title lines like "Published at ICLR"
                || elem.text.starts_with("1") && elem.text.len() < 5  // Affiliation numbers
                || elem.text.starts_with("2") && elem.text.len() < 5;

            // WHY (OODA-13): REFERENCES section often has content near page bottom
            // that's in the affiliation_zone by Y-position but is NOT affiliation.
            // A reference line starts with "[number]" pattern - these should NOT be
            // classified as affiliations even if in the affiliation zone.
            let is_reference_content = {
                let trimmed = elem.text.trim();
                // Check for reference patterns: [1], [12], [123], etc.
                trimmed.starts_with('[') && trimmed.len() > 2 && {
                    let after_bracket = &trimmed[1..];
                    after_bracket
                        .chars()
                        .take_while(|c| c.is_ascii_digit())
                        .count()
                        >= 1
                        && after_bracket
                            .chars()
                            .skip_while(|c| c.is_ascii_digit())
                            .next()
                            == Some(']')
                }
            };

            // WHY (OODA-13 FIX): PREVIOUS BUG was `is_affiliation_zone || looks_like_affiliation`
            // which treated ALL content in the bottom 12% of page as affiliations.
            // This caused REFERENCES content near page bottom to be routed to footer processing,
            // where elements from left and right columns at similar Y got merged.
            //
            // FIX: Only treat as affiliation if it LOOKS like an affiliation (content-based).
            // Zone check is intentionally NOT used - position alone is unreliable for affiliations
            // since REFERENCES can also appear at page bottom.
            let is_affiliation = looks_like_affiliation && !is_reference_content;

            // Handle spanning elements (titles):
            // - In title zone (near top of page = small Y after normalization)
            // - Larger font size (typically > 11pt for titles)
            // - Not a header/footer
            let is_title_zone = elem.y < title_threshold;
            let is_large_font = elem.font_size > large_font_threshold;
            let is_spanning = is_title_zone && is_large_font && !is_footer && !is_header;

            if is_spanning {
                // Spanning elements go to beginning (will be processed first)
                // OODA-05 DEBUG: Log ALL spanning elements to verify title detection
                info!(
                    "SPANNING: Y={:.1} X={:.1} font={:.1} title_zone={} large_font={} '{}'",
                    elem.y,
                    elem.x,
                    elem.font_size,
                    is_title_zone,
                    is_large_font,
                    Self::safe_truncate(&elem.text, 50)
                );
                spanning_elements.push(elem);
            } else if is_footer || is_header || is_affiliation {
                // WHY: Footer/header/affiliation must ALSO respect column boundaries
                // FIXED: Separate left and right footer to prevent cross-column line merging
                // OODA-05 DEBUG: Log elements at top of page that are NOT spanning
                if elem.y < 100.0 {
                    info!(
                        "TOP-NON-SPAN: Y={:.1} font={:.1} header={} affil={} '{}'",
                        elem.y,
                        elem.font_size,
                        is_header,
                        is_affiliation,
                        Self::safe_truncate(&elem.text, 50)
                    );
                }
                debug!(
                    "Footer/affiliation element: Y={:.1} X={:.1} affil={} '{}'",
                    elem.y,
                    elem.x,
                    is_affiliation,
                    Self::safe_truncate(&elem.text, 40)
                );

                // Assign footer to appropriate column
                if elem.x < column_boundary {
                    left_footer.push(elem);
                } else {
                    right_footer.push(elem);
                }
            } else if elem.x < column_boundary - margin {
                // Clearly in left column
                left_column.push(elem);
            } else if elem.x > column_boundary + margin {
                // Clearly in right column
                right_column.push(elem);
            } else {
                // WHY: Element is in the gap between columns (within ±15pt of boundary)
                // OODA-12 FIX: Use midpoint between left column right edge and right column left edge
                // The "boundary" is the gap center. Elements close to boundary need smarter assignment.
                //
                // Observation: Reference [25] at X=313.2 with boundary=320 was incorrectly assigned
                // to LEFT because 313.2 < 320. But X=313 is clearly the START of right column text.
                //
                // FIX: Use page half as tie-breaker. If X > page_width/2, it's right column.
                // This handles cases where column boundary detection is slightly off.
                let page_center = _page_width / 2.0;
                if elem.x < page_center {
                    info!(
                        "GAP->LEFT: Y={:.1} X={:.1} boundary={:.1} center={:.1} '{}'",
                        elem.y,
                        elem.x,
                        column_boundary,
                        page_center,
                        Self::safe_truncate(&elem.text, 30)
                    );
                    left_column.push(elem);
                } else {
                    info!(
                        "GAP->RIGHT: Y={:.1} X={:.1} boundary={:.1} center={:.1} '{}'",
                        elem.y,
                        elem.x,
                        column_boundary,
                        page_center,
                        Self::safe_truncate(&elem.text, 30)
                    );
                    right_column.push(elem);
                }
            }
        }

        debug!(
            "Two-column separation: spanning={}, left={}, right={}, left_footer={}, right_footer={}",
            spanning_elements.len(),
            left_column.len(),
            right_column.len(),
            left_footer.len(),
            right_footer.len()
        );

        // Log statistics about element lengths
        let left_avg_len: f32 = if !left_column.is_empty() {
            left_column.iter().map(|e| e.text.len()).sum::<usize>() as f32
                / left_column.len() as f32
        } else {
            0.0
        };
        let right_avg_len: f32 = if !right_column.is_empty() {
            right_column.iter().map(|e| e.text.len()).sum::<usize>() as f32
                / right_column.len() as f32
        } else {
            0.0
        };

        // Check X-coordinate ranges in each column
        let left_min_x = left_column
            .iter()
            .map(|e| e.x)
            .fold(f32::INFINITY, f32::min);
        let left_max_x = left_column
            .iter()
            .map(|e| e.x)
            .fold(f32::NEG_INFINITY, f32::max);
        let right_min_x = right_column
            .iter()
            .map(|e| e.x)
            .fold(f32::INFINITY, f32::min);
        let right_max_x = right_column
            .iter()
            .map(|e| e.x)
            .fold(f32::NEG_INFINITY, f32::max);

        info!(
            "Column stats: left avg_len={:.1} X=[{:.1},{:.1}], right avg_len={:.1} X=[{:.1},{:.1}]",
            left_avg_len, left_min_x, left_max_x, right_avg_len, right_min_x, right_max_x
        );

        // Process spanning elements first (titles, etc.)
        let spanning_lines = self.group_single_column_layout(spanning_elements);

        // Process each column into lines
        let left_lines = self.group_single_column_layout(left_column);
        let right_lines = self.group_single_column_layout(right_column);

        // WHY: Process footer elements separately per column to prevent cross-column line merging
        let left_footer_lines = self.group_single_column_layout(left_footer);
        let right_footer_lines = self.group_single_column_layout(right_footer);

        debug!(
            "Grouped: spanning={}, left={} lines, right={} lines, left_footer={}, right_footer={} lines",
            spanning_lines.len(),
            left_lines.len(),
            right_lines.len(),
            left_footer_lines.len(),
            right_footer_lines.len()
        );

        // Log first few lines of each section
        for (i, line) in spanning_lines.iter().take(2).enumerate() {
            let text: String = line
                .iter()
                .map(|e| e.text.as_str())
                .collect::<Vec<_>>()
                .join("");
            debug!("Spanning line {}: '{}'", i, Self::safe_truncate(&text, 50));
        }
        for (i, line) in left_lines.iter().take(3).enumerate() {
            let text: String = line
                .iter()
                .map(|e| e.text.as_str())
                .collect::<Vec<_>>()
                .join("");
            debug!("Left line {}: '{}'", i, Self::safe_truncate(&text, 50));
        }
        for (i, line) in right_lines.iter().take(3).enumerate() {
            let text: String = line
                .iter()
                .map(|e| e.text.as_str())
                .collect::<Vec<_>>()
                .join("");
            debug!("Right line {}: '{}'", i, Self::safe_truncate(&text, 50));
        }

        // WHY: Two-column reading order must interleave by Y-coordinate
        // Academic papers should read: left-top, right-top, left-next, right-next, etc.
        // NOT: all-left-lines, then all-right-lines (which causes cross-column merging)
        let mut result = Vec::new();
        result.extend(spanning_lines);

        // Before adding left/right columns, detect and move isolated bottom content to footer
        // This handles affiliations, figure captions that are below the main column content
        let (left_main, left_bottom) = self.split_by_vertical_gap(left_lines, 30.0);
        let (right_main, right_bottom) = self.split_by_vertical_gap(right_lines, 30.0);

        // OODA-13 DEBUG: Log bottom content
        if !left_bottom.is_empty() {
            for line in &left_bottom {
                let text: String = line
                    .iter()
                    .map(|e| e.text.as_str())
                    .collect::<Vec<_>>()
                    .join("");
                info!("LEFT-BOTTOM: '{}'", Self::safe_truncate(&text, 60));
            }
        }
        if !right_bottom.is_empty() {
            for line in &right_bottom {
                let text: String = line
                    .iter()
                    .map(|e| e.text.as_str())
                    .collect::<Vec<_>>()
                    .join("");
                info!("RIGHT-BOTTOM: '{}'", Self::safe_truncate(&text, 60));
            }
        }

        info!(
            "TG-BEFORE-CONCAT: left_main={} right_main={}",
            left_main.len(),
            right_main.len()
        );

        // WHY: Academic papers are read column-by-column, NOT interleaved by Y
        // Correct reading order: ALL left column (top to bottom), THEN ALL right column (top to bottom)
        // WRONG approach: Interleaving by Y causes block_builder to merge lines from different columns
        result.extend(left_main);
        result.extend(right_main);

        info!("TG-AFTER-CONCAT: total={}", result.len());

        // WHY: Footer lines must also follow column order: left footer, then right footer
        result.extend(left_footer_lines);
        result.extend(right_footer_lines);
        result.extend(left_bottom); // Bottom content after footer
        result.extend(right_bottom); // Bottom content after footer

        result
    }

    /// Split lines into main content and bottom-isolated content.
    /// If there's a vertical gap > threshold between content regions, the lower content is separated.
    fn split_by_vertical_gap(
        &self,
        lines: Vec<Vec<TextElement>>,
        gap_threshold: f32,
    ) -> (Vec<Vec<TextElement>>, Vec<Vec<TextElement>>) {
        if lines.len() < 2 {
            return (lines, Vec::new());
        }

        // Find lines' Y positions (use first element's Y as representative)
        let y_positions: Vec<f32> = lines
            .iter()
            .map(|line| line.first().map(|e| e.y).unwrap_or(0.0))
            .collect();

        // Find largest gap in Y (after normalization: Y ascending, gaps are when Y increases more than usual)
        let mut max_gap = 0.0f32;
        let mut split_idx = lines.len();

        for i in 1..y_positions.len() {
            let gap = y_positions[i] - y_positions[i - 1]; // Current Y minus previous Y (should be positive)
            if gap > max_gap && gap > gap_threshold {
                max_gap = gap;
                split_idx = i;
            }
        }

        if max_gap > gap_threshold {
            info!(
                "SPLIT-GAP: gap={:.1}pt at line {} (threshold={:.1})",
                max_gap, split_idx, gap_threshold
            );
            let (main, bottom) = lines.split_at(split_idx);

            // OODA-13 DEBUG: Log if Groom gets split
            for line in bottom {
                let text: String = line
                    .iter()
                    .map(|e| e.text.as_str())
                    .collect::<Vec<_>>()
                    .join("");
                if text.contains("Groom") {
                    info!(
                        "GROOM-SPLIT-TO-BOTTOM: '{}'",
                        Self::safe_truncate(&text, 60)
                    );
                }
            }

            (main.to_vec(), bottom.to_vec())
        } else {
            (lines, Vec::new())
        }
    }

    /// Interleave left and right column lines by Y-coordinate.
    /// WHY: Academic two-column papers should read left-top, right-top, left-next, right-next
    /// NOT all-left then all-right (which causes cross-column text merging).
    ///
    /// Algorithm:
    /// 1. Collect lines with their Y-coordinates (use first element's Y)
    /// 2. Sort all lines by Y descending (top to bottom)
    /// 3. Return sorted sequence
    ///
    /// This ensures proper reading order and prevents backend from creating blocks
    /// that span column boundaries when text at similar Y-coordinates exists in both columns.
    ///
    /// Reserved for potential Y-coordinate-based column interleaving optimization.
    #[allow(dead_code)]
    fn interleave_columns_by_y(
        &self,
        left_lines: Vec<Vec<TextElement>>,
        right_lines: Vec<Vec<TextElement>>,
    ) -> Vec<Vec<TextElement>> {
        // Create (Y, line) tuples for sorting
        let mut all_lines: Vec<(f32, Vec<TextElement>)> = Vec::new();

        for line in left_lines {
            let y = line.first().map(|e| e.y).unwrap_or(0.0);
            all_lines.push((y, line));
        }

        for line in right_lines {
            let y = line.first().map(|e| e.y).unwrap_or(0.0);
            all_lines.push((y, line));
        }

        // Sort by Y descending (top of page first)
        all_lines.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        debug!(
            "Interleaved {} lines from left and right columns by Y-coordinate",
            all_lines.len()
        );

        // Extract just the lines (drop Y values)
        all_lines.into_iter().map(|(_, line)| line).collect()
    }

    /// Group elements into lines for single-column layout.
    ///
    /// # Algorithm (OODA-04 fix)
    ///
    /// 1. Sort by Y descending to get top-to-bottom order
    /// 2. Group elements with similar Y into lines
    /// 3. Within each line, detect "runs" of text (contiguous groups with small X gaps)
    /// 4. Sort elements by X within each run, but preserve run order
    /// 5. Treat runs with large X gaps as separate logical units
    ///
    /// This fixes the issue where elements at the same Y but from different columns
    /// were being incorrectly interleaved when sorted purely by X.
    fn group_single_column_layout(&self, mut elements: Vec<TextElement>) -> Vec<Vec<TextElement>> {
        if elements.is_empty() {
            return Vec::new();
        }

        // OODA-13 DEBUG: Log elements containing Groom
        for elem in &elements {
            if elem.text.contains("Groom") {
                info!(
                    "GROOM-IN-GROUP: Y={:.1} X={:.1} text='{}'",
                    elem.y,
                    elem.x,
                    Self::safe_truncate(&elem.text, 40)
                );
            }
        }

        // Sort by Y ascending (lower Y = top of page after normalization)
        // After Y-normalization in extraction engine, Y=0 is at top of page
        elements.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal));

        // Group into Y-bands
        let mut lines: Vec<Vec<TextElement>> = Vec::new();
        let mut current_line: Vec<TextElement> = Vec::new();
        let mut current_y: Option<f32> = None;

        for elem in elements {
            let y_tolerance = elem.font_size * 0.5;

            if let Some(y) = current_y {
                let y_diff = (elem.y - y).abs();
                if y_diff > y_tolerance {
                    // New line - save current and start new
                    if !current_line.is_empty() {
                        // OODA-04 FIX: Use run-aware sorting instead of pure X-sort
                        let sorted_line = self.sort_line_by_runs(current_line);
                        lines.push(sorted_line);
                        current_line = Vec::new();
                    }
                    current_y = Some(elem.y);
                }
            } else {
                current_y = Some(elem.y);
            }
            current_line.push(elem);
        }

        if !current_line.is_empty() {
            // OODA-04 FIX: Use run-aware sorting for final line
            let sorted_line = self.sort_line_by_runs(current_line);
            lines.push(sorted_line);
        }

        lines
    }

    /// Sort elements within a line using run-aware sorting.
    ///
    /// # Algorithm
    ///
    /// 1. First, assign each element to a "run" based on X proximity
    /// 2. Sort elements within each run by X
    /// 3. Return elements in run order (preserving logical grouping)
    ///
    /// This prevents interleaving of elements from different columns/regions
    /// that happen to share the same Y coordinate.
    fn sort_line_by_runs(&self, mut elements: Vec<TextElement>) -> Vec<TextElement> {
        if elements.len() <= 1 {
            return elements;
        }

        // First, sort by X to identify runs
        elements.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));

        // Detect runs: groups of elements with small X gaps
        // A large gap (> 100pt or > 10x average char width) indicates a new run
        let avg_font_size =
            elements.iter().map(|e| e.font_size).sum::<f32>() / elements.len() as f32;
        let large_gap_threshold = (avg_font_size * 10.0).max(100.0);

        let mut runs: Vec<Vec<TextElement>> = Vec::new();
        let mut current_run: Vec<TextElement> = Vec::new();
        let mut prev_end_x: Option<f32> = None;

        for elem in elements {
            if let Some(prev_x) = prev_end_x {
                let gap = elem.x - prev_x;
                if gap > large_gap_threshold {
                    // Large gap - start a new run
                    if !current_run.is_empty() {
                        runs.push(std::mem::take(&mut current_run));
                    }
                }
            }

            // Estimate element end position
            let char_width = elem.font_size * 0.5;
            let elem_end = elem.x + (elem.text.chars().count() as f32 * char_width);
            prev_end_x = Some(elem_end);

            current_run.push(elem);
        }

        if !current_run.is_empty() {
            runs.push(current_run);
        }

        // Flatten runs back into a single line
        // Each run is already sorted by X; runs are in left-to-right order
        runs.into_iter().flatten().collect()
    }

    /// Merge line elements into text with proper spacing while preserving style runs as spans.
    pub fn merge_line(&self, elements: &[TextElement]) -> MergedLine {
        if elements.is_empty() {
            return MergedLine {
                text: String::new(),
                avg_font_size: 12.0,
                spans: Vec::new(),
            };
        }

        let avg_font_size =
            elements.iter().map(|e| e.font_size).sum::<f32>() / elements.len() as f32;

        // Estimate space character width.
        // A space character is typically about 0.25-0.33 of the font size.
        // We use 0.25x as the minimum gap that should be treated as a space.
        // This prevents inserting spaces between tightly-kerned letters while
        // still catching actual word breaks.
        //
        let mut text = String::new();
        let mut spans: Vec<TextSpan> = Vec::new();

        let push_to_spans = |spans: &mut Vec<TextSpan>, chunk: &str, style: FontStyle| {
            if chunk.is_empty() {
                return;
            }
            if let Some(last) = spans.last_mut() {
                if last.style == style {
                    last.text.push_str(chunk);
                    return;
                }
            }
            spans.push(TextSpan {
                text: chunk.to_string(),
                bbox: None,
                style,
            });
        };

        // Calculate typical letter spacing from NON-WHITESPACE elements only
        // Exclude spacing to/from explicit space characters
        // This helps detect whether we have tightly-spaced letters or wide gaps
        let typical_spacing = if elements.len() > 2 {
            let mut spacings: Vec<f32> = Vec::new();
            for i in 1..elements.len() {
                // Only count spacing between non-whitespace elements
                let curr_is_ws = elements[i].text.trim().is_empty();
                let prev_is_ws = elements[i - 1].text.trim().is_empty();
                if !curr_is_ws && !prev_is_ws {
                    let spacing = elements[i].x - elements[i - 1].x;
                    if spacing > 0.0 && spacing < avg_font_size * 2.0 {
                        spacings.push(spacing);
                    }
                }
            }
            if spacings.is_empty() {
                avg_font_size * 0.6
            } else {
                spacings.iter().sum::<f32>() / spacings.len() as f32
            }
        } else {
            avg_font_size * 0.6
        };

        // A "word gap" is significantly larger than typical letter spacing
        // Typically 2-3x the typical letter spacing
        let word_gap_threshold = typical_spacing * 1.5;

        for (i, elem) in elements.iter().enumerate() {
            if i > 0 {
                let prev = &elements[i - 1];

                // Skip space insertion if current element is already a space/whitespace.
                // PDF content often has explicit space characters at the correct positions.
                let is_whitespace = elem.text.trim().is_empty();

                // Skip space insertion if previous element was already a space.
                let prev_is_whitespace = prev.text.trim().is_empty();

                // Skip space insertion if previous element already ends with a space.
                // This prevents double spaces when elements like 'ABOUT ' merge with 'THE '.
                let prev_ends_with_space = prev.text.ends_with(' ');

                // Skip space insertion if current element starts with a space.
                let elem_starts_with_space = elem.text.starts_with(' ');

                // If previous element CONTAINS a space anywhere, be more cautious.
                // The space in ' Mansu' already accounts for visual separation from previous element.
                // Don't add more spaces after elements that have internal spaces, unless gap is HUGE.
                let prev_has_space = prev.text.contains(' ');

                if !is_whitespace
                    && !prev_is_whitespace
                    && !prev_ends_with_space
                    && !elem_starts_with_space
                {
                    // Calculate actual gap between elements
                    let spacing = elem.x - prev.x;

                    // Avoid inserting spaces before punctuation.
                    let starts_with_punct = elem
                        .text
                        .chars()
                        .next()
                        .map(|c| matches!(c, ',' | '.' | ':' | ';' | ')' | ']' | '}' | '?' | '!'))
                        .unwrap_or(false);

                    // If prev already has a space, require a much larger gap to insert another.
                    // This handles cases like ' Mansu' -> 'y' where the leading space in prev
                    // already accounts for word separation.
                    let effective_threshold = if prev_has_space {
                        word_gap_threshold * 2.0 // Much stricter: 3x typical spacing
                    } else {
                        word_gap_threshold // Normal: 1.5x typical spacing
                    };

                    // Only insert space if spacing exceeds the effective threshold
                    if spacing > effective_threshold && !starts_with_punct {
                        text.push(' ');
                        if let Some(last) = spans.last_mut() {
                            last.text.push(' ');
                        } else {
                            spans.push(TextSpan::plain(" "));
                        }
                    }
                }
            }

            text.push_str(&elem.text);
            // FontStyle with weight and italic are used - this flows to output correctly!
            let style = FontStyle {
                family: Some(elem.font_name.clone()),
                size: Some(elem.font_size),
                weight: Some(if elem.is_bold { 700 } else { 400 }),
                italic: elem.is_italic,
                ..Default::default()
            };
            push_to_spans(&mut spans, &elem.text, style);
        }

        MergedLine {
            text,
            avg_font_size,
            spans,
        }
    }
}

/// Calculate adaptive region thresholds based on content distribution.
///
/// WHY (OODA-05): After Y-normalization in extraction_engine.rs, coordinates are:
/// - Y=0 at visual TOP of page
/// - Y=max at visual BOTTOM of page
///
/// Therefore:
/// - Footer region (visual bottom): elements with Y > footer_threshold (large Y)
/// - Title region (visual top): elements with Y < title_threshold (small Y)  
/// - Header region (running headers at very top): elements with Y < header_threshold (small Y) AND small font
/// - Affiliation region (near footer): elements with Y > affiliation_threshold (large Y)
///
/// Returns tuple of (footer_threshold, header_threshold, title_threshold, affiliation_threshold, large_font_threshold).
fn calculate_adaptive_region_thresholds(elements: &[TextElement]) -> (f32, f32, f32, f32, f32) {
    if elements.is_empty() {
        // Fallback to reasonable defaults for empty documents
        // After normalization: Y range is typically 0 to ~700-800
        // footer_threshold: 92% of page = large Y (visual bottom)
        // header_threshold: 8% of page = small Y (visual top)
        // title_threshold: 15% of page = small Y (visual top with title font)
        // affiliation_threshold: 88% of page = large Y (just above footer)
        return (650.0, 60.0, 120.0, 600.0, 11.0);
    }

    // After Y-normalization: min_y is near 0 (visual top), max_y is the visual bottom
    let min_y = elements.iter().map(|e| e.y).fold(f32::MAX, f32::min);
    let max_y = elements.iter().map(|e| e.y).fold(f32::MIN, f32::max);
    let y_range = max_y - min_y;

    // Calculate font size distribution
    let font_sizes: Vec<f32> = elements.iter().map(|e| e.font_size).collect();
    let avg_font_size = if font_sizes.is_empty() {
        10.0
    } else {
        font_sizes.iter().sum::<f32>() / font_sizes.len() as f32
    };

    // Calculate adaptive thresholds based on normalized coordinates
    // Footer threshold: Y > 92% = visual bottom (page number, copyright)
    let footer_threshold = min_y + y_range * 0.92;
    // Header threshold: Y < 8% = visual top (running headers with small font)
    let header_threshold = min_y + y_range * 0.08;
    // Title threshold: Y < 15% = visual top (titles with large font)
    let title_threshold = min_y + y_range * 0.15;
    // Affiliation threshold: Y > 88% = just above footer (author affiliations)
    let affiliation_threshold = min_y + y_range * 0.88;
    // Large font threshold: 20% larger than average indicates title/heading
    let large_font_threshold = avg_font_size * 1.2;

    // Clamp to reasonable ranges
    // Footer should be near bottom (large Y), so clamp to high values
    let footer_threshold = footer_threshold.clamp(y_range * 0.85, y_range * 0.98);
    // Header should be near top (small Y), so clamp to low values
    let header_threshold = header_threshold.clamp(y_range * 0.02, y_range * 0.12);
    // Title zone should be in top portion (small Y)
    let title_threshold = title_threshold.clamp(y_range * 0.08, y_range * 0.25);
    // Affiliation should be near footer but not at very bottom
    let affiliation_threshold = affiliation_threshold.clamp(y_range * 0.80, y_range * 0.92);
    // Font size threshold
    let large_font_threshold = large_font_threshold.clamp(10.0, 14.0);

    (
        footer_threshold,
        header_threshold,
        title_threshold,
        affiliation_threshold,
        large_font_threshold,
    )
}

impl Default for TextGrouper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_element(x: f32, y: f32, text: &str, font_size: f32) -> TextElement {
        TextElement {
            text: text.to_string(),
            x,
            y,
            font_name: "test".to_string(),
            font_size,
            is_bold: false,
            is_italic: false,
        }
    }

    #[test]
    fn test_text_grouper_default() {
        let grouper = TextGrouper::default();
        // Just verify it can be created
        assert!(std::mem::size_of_val(&grouper) >= 0);
    }

    #[test]
    fn test_group_empty_elements() {
        let grouper = TextGrouper::new();
        let elements: Vec<TextElement> = vec![];

        let result = grouper.group_into_lines(elements, 600.0, 800.0, None);
        assert!(result.is_empty());
    }

    #[test]
    fn test_group_single_element() {
        let grouper = TextGrouper::new();
        let elements = vec![make_element(100.0, 500.0, "Hello", 12.0)];

        let result = grouper.group_into_lines(elements, 600.0, 800.0, None);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 1);
    }

    #[test]
    fn test_group_same_line_elements() {
        let grouper = TextGrouper::new();
        let elements = vec![
            make_element(100.0, 500.0, "Hello", 12.0),
            make_element(160.0, 500.0, "World", 12.0),
        ];

        let result = grouper.group_into_lines(elements, 600.0, 800.0, None);
        // Same Y = same line
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 2);
    }

    #[test]
    fn test_group_different_lines() {
        let grouper = TextGrouper::new();
        let elements = vec![
            make_element(100.0, 600.0, "Line 1", 12.0),
            make_element(100.0, 580.0, "Line 2", 12.0),
            make_element(100.0, 560.0, "Line 3", 12.0),
        ];

        let result = grouper.group_into_lines(elements, 600.0, 800.0, None);
        // Three different Y values = three lines
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_group_two_column_layout() {
        let grouper = TextGrouper::new();
        let elements = vec![
            // Left column
            make_element(100.0, 500.0, "Left 1", 10.0),
            make_element(100.0, 480.0, "Left 2", 10.0),
            // Right column
            make_element(400.0, 500.0, "Right 1", 10.0),
            make_element(400.0, 480.0, "Right 2", 10.0),
        ];

        let result = grouper.group_into_lines(elements, 600.0, 800.0, Some(300.0));
        // Should separate columns and process left first, then right
        assert!(result.len() >= 2);
    }

    #[test]
    fn test_merged_line_struct() {
        let line = MergedLine {
            text: "Test line".to_string(),
            avg_font_size: 12.0,
            spans: vec![],
        };
        assert_eq!(line.text, "Test line");
        assert_eq!(line.avg_font_size, 12.0);
    }

    #[test]
    fn test_adaptive_thresholds() {
        // After Y-normalization: Y=0 is at TOP of page, Y increases downward
        // So elements with lower Y are at the TOP (title zone), higher Y at bottom (footer zone)
        let elements = vec![
            make_element(100.0, 50.0, "Top (title zone)", 14.0), // Y=50 = near top
            make_element(100.0, 400.0, "Middle", 10.0),          // Y=400 = middle
            make_element(100.0, 700.0, "Bottom (footer)", 8.0),  // Y=700 = near bottom
        ];

        let (footer, header, title, affiliation, large_font) =
            calculate_adaptive_region_thresholds(&elements);

        // After my fix: footer threshold should be LARGE (near bottom)
        // header threshold should be SMALL (near top)
        // title threshold should be SMALL (near top)
        assert!(
            footer > 500.0,
            "footer should be large Y (near bottom): got {}",
            footer
        );
        assert!(
            header < 100.0,
            "header should be small Y (near top): got {}",
            header
        );
        assert!(
            title < 200.0,
            "title should be small Y (near top): got {}",
            title
        );
        assert!(
            affiliation > 400.0,
            "affiliation should be large Y (above footer): got {}",
            affiliation
        );
        assert!(large_font > 0.0);
    }

    #[test]
    fn test_adaptive_thresholds_empty() {
        let elements: Vec<TextElement> = vec![];
        let (footer, header, title, affiliation, large_font) =
            calculate_adaptive_region_thresholds(&elements);

        // Updated default values after OODA-05 fix:
        // footer_threshold: 92% of page = large Y (visual bottom)
        // header_threshold: 8% of page = small Y (visual top)
        // title_threshold: 15% of page = small Y (visual top with title font)
        // affiliation_threshold: 88% of page = large Y (just above footer)
        assert_eq!(footer, 650.0, "footer default should be 650.0 (large Y)");
        assert_eq!(header, 60.0, "header default should be 60.0 (small Y)");
        assert_eq!(title, 120.0, "title default should be 120.0 (small Y)");
        assert_eq!(
            affiliation, 600.0,
            "affiliation default should be 600.0 (large Y)"
        );
        assert_eq!(large_font, 11.0);
    }

    #[test]
    fn test_skip_empty_text_elements() {
        let grouper = TextGrouper::new();
        let elements = vec![
            make_element(100.0, 500.0, "", 12.0),
            make_element(100.0, 500.0, "   ", 12.0),
            make_element(200.0, 500.0, "Valid", 12.0),
        ];

        let result = grouper.group_into_lines(elements, 600.0, 800.0, Some(300.0));
        // Empty elements should be skipped
        assert!(result.len() <= 2);
    }
}
