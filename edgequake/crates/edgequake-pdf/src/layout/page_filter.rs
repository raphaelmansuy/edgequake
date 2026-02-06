//! Page header/footer detection and filtering.
//!
//! PDFs often contain repeated elements at the top/bottom of every page:
//! - Page numbers ("1", "2", "Page 3")
//! - Running headers ("Journal of AI Research")
//! - Conference headers ("NeurIPS 2024")
//! - Copyright notices
//!
//! These should be filtered from markdown output as they add noise.
//!
//! ## Algorithm
//!
//! ```text
//! 1. Group blocks by y-position (top margin, bottom margin)
//! 2. Track text that repeats across multiple pages
//! 3. Filter blocks that match repeated patterns
//! ```

use super::pymupdf_structs::Block;
use std::collections::HashMap;

/// Configuration for header/footer detection.
#[derive(Debug, Clone)]
pub struct HeaderFooterConfig {
    /// Margin from top/bottom of page (in points) to search for headers/footers.
    /// Default: 72pt (1 inch)
    pub margin_pts: f32,
    /// Minimum number of pages a text must appear on to be considered repeated.
    /// Default: 2
    pub min_pages: usize,
}

impl Default for HeaderFooterConfig {
    fn default() -> Self {
        Self {
            margin_pts: 72.0,
            min_pages: 2,
        }
    }
}

/// Detect and filter page headers and footers from blocks.
///
/// Returns a new vector of blocks with headers/footers removed.
/// Blocks are considered headers/footers if:
/// 1. They are within the top or bottom margin of the page
/// 2. Their text (or a normalized version) repeats across multiple pages
/// 3. They are short (< 80 chars) - excludes body text at margins
pub fn filter_headers_footers(
    blocks: &[Block],
    page_height: f32,
    config: &HeaderFooterConfig,
) -> Vec<Block> {
    if blocks.is_empty() || page_height <= 0.0 {
        return blocks.to_vec();
    }

    let top_threshold = page_height - config.margin_pts;
    let bottom_threshold = config.margin_pts;

    // Step 1: Find candidate texts in margins and count page occurrences
    let mut text_page_counts: HashMap<String, Vec<usize>> = HashMap::new();

    for block in blocks {
        let is_in_top_margin = block.y1 > top_threshold;
        let is_in_bottom_margin = block.y0 < bottom_threshold;

        if !is_in_top_margin && !is_in_bottom_margin {
            continue;
        }

        let text = block.text();
        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed.len() > 80 {
            continue;
        }

        // Normalize: strip page numbers and whitespace for comparison
        let normalized = normalize_for_comparison(trimmed);
        text_page_counts
            .entry(normalized)
            .or_default()
            .push(block.page_num);
    }

    // Step 2: Build set of repeated texts (appear on >= min_pages)
    let mut repeated_texts: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (normalized, pages) in &text_page_counts {
        // Count unique pages
        let mut unique_pages = pages.clone();
        unique_pages.sort();
        unique_pages.dedup();
        if unique_pages.len() >= config.min_pages {
            repeated_texts.insert(normalized.clone());
        }
    }

    // Also always filter standalone page numbers
    // (they don't need to repeat since each is unique)
    // Detected by: single number or "Page X" in margin

    // Step 3: Filter blocks
    blocks
        .iter()
        .filter(|block| {
            let is_in_top_margin = block.y1 > top_threshold;
            let is_in_bottom_margin = block.y0 < bottom_threshold;

            if !is_in_top_margin && !is_in_bottom_margin {
                return true; // Keep non-margin blocks
            }

            let text = block.text();
            let trimmed = text.trim();

            // Filter standalone page numbers
            if is_page_number(trimmed) {
                return false;
            }

            // Filter repeated header/footer text
            let normalized = normalize_for_comparison(trimmed);
            if repeated_texts.contains(&normalized) {
                return false;
            }

            true // Keep non-repeated margin text
        })
        .cloned()
        .collect()
}

/// Check if text is a standalone page number.
///
/// Patterns: "1", "123", "Page 1", "- 1 -", "1 of 10", "i", "ii", "iii"
fn is_page_number(text: &str) -> bool {
    let trimmed = text.trim();

    // Pure digit: "1", "42", "123"
    if trimmed.chars().all(|c| c.is_ascii_digit()) && !trimmed.is_empty() {
        return true;
    }

    // "Page X" or "page X"
    let lower = trimmed.to_lowercase();
    if lower.starts_with("page ") {
        let rest = lower.strip_prefix("page ").unwrap_or("");
        if rest.chars().all(|c| c.is_ascii_digit()) && !rest.is_empty() {
            return true;
        }
    }

    // "- X -" pattern
    if trimmed.starts_with("- ") && trimmed.ends_with(" -") && trimmed.len() > 4 {
        let inner = &trimmed[2..trimmed.len() - 2];
        if inner.chars().all(|c| c.is_ascii_digit()) {
            return true;
        }
    }

    // Roman numerals: "i", "ii", "iii", "iv", "v", etc.
    if !trimmed.is_empty()
        && trimmed.len() <= 5
        && trimmed.chars().all(|c| matches!(c, 'i' | 'v' | 'x'))
    {
        return true;
    }

    false
}

/// Normalize text for repeated header/footer comparison.
///
/// Removes digits and extra whitespace so "Journal Vol 1" and "Journal Vol 2"
/// both become "Journal Vol".
fn normalize_for_comparison(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_ascii_digit())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::pymupdf_structs::{BlockType, Line, Span};

    fn make_block(text: &str, page_num: usize, y0: f32, y1: f32) -> Block {
        Block {
            lines: vec![Line::from_span(Span {
                text: text.to_string(),
                x0: 50.0,
                y0,
                x1: 200.0,
                y1,
                font_size: 10.0,
                font_name: None,
                page_num,
                font_is_bold: None,
                font_is_italic: None,
                font_is_monospace: None,
            })],
            x0: 50.0,
            y0,
            x1: 200.0,
            y1,
            page_num,
            block_type: BlockType::Paragraph,
        }
    }

    #[test]
    fn test_is_page_number() {
        assert!(is_page_number("1"));
        assert!(is_page_number("42"));
        assert!(is_page_number("Page 3"));
        assert!(is_page_number("page 7"));
        assert!(is_page_number("- 5 -"));
        assert!(is_page_number("ii"));
        assert!(is_page_number("iv"));

        assert!(!is_page_number("Introduction"));
        assert!(!is_page_number("Table 3"));
        assert!(!is_page_number(""));
    }

    #[test]
    fn test_normalize_for_comparison() {
        assert_eq!(normalize_for_comparison("Journal Vol 1"), "journal vol");
        assert_eq!(normalize_for_comparison("  NeurIPS  2024  "), "neurips");
    }

    #[test]
    fn test_filter_page_numbers() {
        let page_height = 792.0; // Letter
        let config = HeaderFooterConfig::default();

        let blocks = vec![
            make_block("Body text", 0, 400.0, 412.0),   // Body (keep)
            make_block("1", 0, 20.0, 30.0),             // Page number at bottom (filter)
            make_block("Body text 2", 1, 400.0, 412.0), // Body (keep)
            make_block("2", 1, 20.0, 30.0),             // Page number at bottom (filter)
        ];

        let filtered = filter_headers_footers(&blocks, page_height, &config);
        assert_eq!(filtered.len(), 2, "Should keep only body text");
        assert_eq!(filtered[0].text(), "Body text");
        assert_eq!(filtered[1].text(), "Body text 2");
    }

    #[test]
    fn test_filter_repeated_header() {
        let page_height = 792.0;
        let config = HeaderFooterConfig::default();

        let blocks = vec![
            make_block("NeurIPS 2024", 0, 760.0, 770.0), // Top margin (filter)
            make_block("Body page 1", 0, 400.0, 412.0),
            make_block("NeurIPS 2024", 1, 760.0, 770.0), // Repeated header (filter)
            make_block("Body page 2", 1, 400.0, 412.0),
        ];

        let filtered = filter_headers_footers(&blocks, page_height, &config);
        assert_eq!(filtered.len(), 2, "Should filter repeated headers");
        assert_eq!(filtered[0].text(), "Body page 1");
        assert_eq!(filtered[1].text(), "Body page 2");
    }

    #[test]
    fn test_preserve_non_repeated_margin_text() {
        let page_height = 792.0;
        let config = HeaderFooterConfig::default();

        // Title on first page only (top margin, not repeated)
        let blocks = vec![
            make_block("Paper Title Here", 0, 760.0, 775.0), // Only page 0
            make_block("Body", 0, 400.0, 412.0),
            make_block("Body 2", 1, 400.0, 412.0),
        ];

        let filtered = filter_headers_footers(&blocks, page_height, &config);
        assert_eq!(filtered.len(), 3, "Non-repeated margin text should be kept");
    }

    #[test]
    fn test_empty_blocks() {
        let config = HeaderFooterConfig::default();
        let filtered = filter_headers_footers(&[], 792.0, &config);
        assert!(filtered.is_empty());
    }
}
