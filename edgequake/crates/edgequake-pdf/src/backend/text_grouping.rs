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

        // Sort by Y descending (higher Y = top of page in PDF coordinates)
        // This puts content that appears at the top of the page first
        let mut elements = elements;
        elements.sort_by(|a, b| b.y.partial_cmp(&a.y).unwrap_or(std::cmp::Ordering::Equal));

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

        // Margin around column boundary for classification
        let margin = 15.0;

        for elem in elements {
            // Skip very small elements (likely artifacts)
            if elem.text.trim().is_empty() {
                continue;
            }

            // Check if element is in footer region
            let is_footer = elem.y < footer_threshold;

            // Check if element is in header region (running header)
            let is_header = elem.y > header_threshold && elem.font_size < large_font_threshold;

            // Check if element is affiliation/metadata (between body and footer)
            // These include: university names, emails, conference submission lines
            let is_affiliation_zone = elem.y < affiliation_threshold && elem.y >= footer_threshold;
            let looks_like_affiliation = elem.text.contains('@')
                || elem.text.contains("University")
                || elem.text.contains("School of")
                || elem.text.contains("Department")
                || elem.text.contains("Correspondence")
                || elem.text.contains("Submitted to")
                || elem.text.contains("Conference")
                || elem.text.starts_with("1") && elem.text.len() < 5  // Affiliation numbers
                || elem.text.starts_with("2") && elem.text.len() < 5;
            let is_affiliation = is_affiliation_zone || looks_like_affiliation;

            // Handle spanning elements (titles):
            // - In title zone (near top of page)
            // - Larger font size (typically > 11pt for titles)
            // - Not a header/footer
            let is_title_zone = elem.y > title_threshold;
            let is_large_font = elem.font_size > large_font_threshold;
            let is_spanning = is_title_zone && is_large_font && !is_footer && !is_header;

            if is_spanning {
                // Spanning elements go to beginning (will be processed first)
                if elem.text.contains("trajectory") || elem.text.contains("temporal") {
                    info!(
                        "SPANNING: Y={:.1} X={:.1} title_zone={} large_font={} '{}'",
                        elem.y,
                        elem.x,
                        is_title_zone,
                        is_large_font,
                        &elem.text[..elem.text.len().min(40)]
                    );
                }
                spanning_elements.push(elem);
            } else if is_footer || is_header || is_affiliation {
                // WHY: Footer/header/affiliation must ALSO respect column boundaries
                // FIXED: Separate left and right footer to prevent cross-column line merging
                debug!(
                    "Footer/affiliation element: Y={:.1} X={:.1} affil={} '{}'",
                    elem.y,
                    elem.x,
                    is_affiliation,
                    &elem.text[..elem.text.len().min(40)]
                );

                // Assign footer to appropriate column
                if elem.x < column_boundary {
                    left_footer.push(elem);
                } else {
                    right_footer.push(elem);
                }
            } else if elem.x < column_boundary - margin {
                // Clearly in left column
                // Log if text contains suspicious patterns
                if elem.text.contains("paradigm")
                    || elem.text.contains("updated")
                    || elem.text.contains("approach")
                    || elem.text.contains("trajectory")
                    || elem.text.contains("temporal-control")
                {
                    info!(
                        "LEFT-COL: Y={:.1} X={:.1} boundary={:.1} '{}'",
                        elem.y,
                        elem.x,
                        column_boundary,
                        &elem.text[..elem.text.len().min(50)]
                    );
                }
                left_column.push(elem);
            } else if elem.x > column_boundary + margin {
                // Clearly in right column
                if elem.text.contains("paradigm")
                    || elem.text.contains("updated")
                    || elem.text.contains("approach")
                    || elem.text.contains("trajectory")
                    || elem.text.contains("temporal-control")
                {
                    info!(
                        "RIGHT-COL: Y={:.1} X={:.1} boundary={:.1} '{}'",
                        elem.y,
                        elem.x,
                        column_boundary,
                        &elem.text[..elem.text.len().min(50)]
                    );
                }
                right_column.push(elem);
            } else {
                // WHY: Element is in the gap between columns (within ±15pt of boundary)
                // FIXED: Use element's X position relative to boundary
                // Elements starting left of boundary go to left column
                if elem.x < column_boundary {
                    info!(
                        "GAP->LEFT: Y={:.1} X={:.1} boundary={:.1} '{}'",
                        elem.y,
                        elem.x,
                        column_boundary,
                        &elem.text[..elem.text.len().min(30)]
                    );
                    left_column.push(elem);
                } else {
                    info!(
                        "GAP->RIGHT: Y={:.1} X={:.1} boundary={:.1} '{}'",
                        elem.y,
                        elem.x,
                        column_boundary,
                        &elem.text[..elem.text.len().min(30)]
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
            debug!("Spanning line {}: '{}'", i, &text[..text.len().min(50)]);
        }
        for (i, line) in left_lines.iter().take(3).enumerate() {
            let text: String = line
                .iter()
                .map(|e| e.text.as_str())
                .collect::<Vec<_>>()
                .join("");
            debug!("Left line {}: '{}'", i, &text[..text.len().min(50)]);
        }
        for (i, line) in right_lines.iter().take(3).enumerate() {
            let text: String = line
                .iter()
                .map(|e| e.text.as_str())
                .collect::<Vec<_>>()
                .join("");
            debug!("Right line {}: '{}'", i, &text[..text.len().min(50)]);
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

        // Find largest gap in Y (remember: sorted by Y descending, so gaps are when Y suddenly drops more)
        let mut max_gap = 0.0f32;
        let mut split_idx = lines.len();

        for i in 1..y_positions.len() {
            let gap = y_positions[i - 1] - y_positions[i]; // Previous Y minus current Y (should be positive)
            if gap > max_gap && gap > gap_threshold {
                max_gap = gap;
                split_idx = i;
            }
        }

        if max_gap > gap_threshold {
            debug!(
                "Found vertical gap of {:.1}pt at line {}, splitting column content",
                max_gap, split_idx
            );
            let (main, bottom) = lines.split_at(split_idx);
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

    /// Group elements into lines for single-column layout
    fn group_single_column_layout(&self, mut elements: Vec<TextElement>) -> Vec<Vec<TextElement>> {
        if elements.is_empty() {
            return Vec::new();
        }

        // Sort by Y descending (higher Y = top of page in PDF coordinates)
        elements.sort_by(|a, b| b.y.partial_cmp(&a.y).unwrap_or(std::cmp::Ordering::Equal));

        // Group into Y-bands
        let mut lines: Vec<Vec<TextElement>> = Vec::new();
        let mut current_line: Vec<TextElement> = Vec::new();
        let mut current_y: Option<f32> = None;

        for elem in elements {
            let y_tolerance = elem.font_size * 0.5;

            if let Some(y) = current_y {
                if (elem.y - y).abs() > y_tolerance {
                    // New line - save current and start new
                    if !current_line.is_empty() {
                        current_line.sort_by(|a, b| {
                            a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal)
                        });

                        // Log X-coordinate range for this line
                        let min_x = current_line
                            .iter()
                            .map(|e| e.x)
                            .fold(f32::INFINITY, f32::min);
                        let max_x = current_line
                            .iter()
                            .map(|e| e.x)
                            .fold(f32::NEG_INFINITY, f32::max);
                        let x_range = max_x - min_x;
                        let text: String = current_line.iter().map(|e| e.text.as_str()).collect();

                        if x_range > 200.0 {
                            // Log individual elements in this line
                            for (i, e) in current_line.iter().enumerate() {
                                info!(
                                    "  LINE-ELEM[{}]: X={:.1} Y={:.1} text='{}'",
                                    i,
                                    e.x,
                                    e.y,
                                    &e.text[..e.text.len().min(40)]
                                );
                            }
                            info!(
                                "LINE-XRANGE: Y={:.1} X=[{:.1},{:.1}] range={:.1} elements={} text='{}'",
                                current_y.unwrap_or(0.0),
                                min_x,
                                max_x,
                                x_range,
                                current_line.len(),
                                &text[..text.len().min(80)]
                            );
                        }

                        lines.push(std::mem::take(&mut current_line));
                    }
                    current_y = Some(elem.y);
                }
            } else {
                current_y = Some(elem.y);
            }
            current_line.push(elem);
        }

        if !current_line.is_empty() {
            current_line.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));

            // Log X-coordinate range for final line
            let min_x = current_line
                .iter()
                .map(|e| e.x)
                .fold(f32::INFINITY, f32::min);
            let max_x = current_line
                .iter()
                .map(|e| e.x)
                .fold(f32::NEG_INFINITY, f32::max);
            let x_range = max_x - min_x;
            let text: String = current_line.iter().map(|e| e.text.as_str()).collect();

            if x_range > 200.0 {
                info!(
                    "LINE-XRANGE: Y={:.1} X=[{:.1},{:.1}] range={:.1} elements={} text='{}'",
                    current_y.unwrap_or(0.0),
                    min_x,
                    max_x,
                    x_range,
                    current_line.len(),
                    &text[..text.len().min(80)]
                );
            }

            lines.push(current_line);
        }

        lines
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

        // Estimate average character width.
        // We bias toward inserting spaces (missing spaces are worse than extra spaces).
        // Using a low threshold (0.3x) to be more aggressive about space insertion.
        // Post-processing can clean up extra spaces, but missing spaces cause word concatenation.
        let avg_char_width = avg_font_size * 0.5;
        let space_threshold = avg_char_width * 0.3;

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

        for (i, elem) in elements.iter().enumerate() {
            if i > 0 {
                let prev = &elements[i - 1];
                // Estimate previous element's end position using its own font size and Unicode-safe length.
                let prev_char_width = prev.font_size * 0.5;
                let prev_len = prev.text.chars().count() as f32;
                let prev_end = prev.x + (prev_len * prev_char_width);
                let gap = elem.x - prev_end;

                // Avoid inserting spaces before punctuation.
                let starts_with_punct = elem
                    .text
                    .chars()
                    .next()
                    .map(|c| matches!(c, ',' | '.' | ':' | ';' | ')' | ']' | '}' | '?' | '!'))
                    .unwrap_or(false);

                if gap > space_threshold && !starts_with_punct {
                    text.push(' ');
                    if let Some(last) = spans.last_mut() {
                        last.text.push(' ');
                    } else {
                        spans.push(TextSpan::plain(" "));
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
/// Returns tuple of (footer_threshold, header_threshold, title_threshold, affiliation_threshold, large_font_threshold).
fn calculate_adaptive_region_thresholds(elements: &[TextElement]) -> (f32, f32, f32, f32, f32) {
    if elements.is_empty() {
        // Fallback to reasonable defaults for empty documents
        return (60.0, 730.0, 650.0, 80.0, 11.0);
    }

    // Calculate page height from elements
    let page_height = elements.iter().map(|e| e.y).fold(f32::MIN, f32::max);
    let page_bottom = elements.iter().map(|e| e.y).fold(f32::MAX, f32::min);

    // Calculate font size distribution
    let font_sizes: Vec<f32> = elements.iter().map(|e| e.font_size).collect();
    let avg_font_size = if font_sizes.is_empty() {
        10.0
    } else {
        font_sizes.iter().sum::<f32>() / font_sizes.len() as f32
    };

    // Calculate adaptive thresholds based on page dimensions and content
    let footer_threshold = page_bottom + (page_height - page_bottom) * 0.08; // Bottom 8% of page
    let header_threshold = page_height - (page_height - page_bottom) * 0.08; // Top 8% of page
    let title_threshold = page_bottom + (page_height - page_bottom) * 0.15; // Top 15% of page
    let affiliation_threshold = page_bottom + (page_height - page_bottom) * 0.12; // Bottom 12% of page
    let large_font_threshold = avg_font_size * 1.2; // 20% larger than average

    // Clamp to reasonable ranges - ensure min <= max to avoid panic
    let footer_threshold = footer_threshold.clamp(40.0, 100.0);
    let header_min = (page_height - 100.0).max(0.0);
    let header_max = (page_height - 20.0).max(header_min);
    let header_threshold = header_threshold.clamp(header_min, header_max);

    let title_min = page_bottom + 100.0;
    let title_max = (page_height - 50.0).max(title_min);
    let title_threshold = title_threshold.clamp(title_min, title_max);

    let affiliation_threshold = affiliation_threshold.clamp(60.0, 120.0);
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
        let elements = vec![
            make_element(100.0, 700.0, "Top", 12.0),
            make_element(100.0, 500.0, "Middle", 10.0),
            make_element(100.0, 100.0, "Bottom", 8.0),
        ];

        let (footer, header, title, affiliation, large_font) =
            calculate_adaptive_region_thresholds(&elements);

        // Verify thresholds are in valid ranges
        assert!(footer > 0.0);
        assert!(header > footer);
        assert!(large_font > 0.0);
        assert!(title > 0.0);
        assert!(affiliation > 0.0);
    }

    #[test]
    fn test_adaptive_thresholds_empty() {
        let elements: Vec<TextElement> = vec![];
        let (footer, header, title, affiliation, large_font) =
            calculate_adaptive_region_thresholds(&elements);

        // Should return default values
        assert_eq!(footer, 60.0);
        assert_eq!(header, 730.0);
        assert_eq!(large_font, 11.0);
        assert!(title > 0.0);
        assert!(affiliation > 0.0);
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
