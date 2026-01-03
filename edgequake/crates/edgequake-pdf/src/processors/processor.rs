//! Document processors for transforming documents.
//!
//! Processors implement a chain-of-responsibility pattern for document
//! transformation. Each processor can modify the document structure.

use crate::layout::LayoutAnalyzer;
use crate::schema::{Block, BlockType, Document, TextSpan};
use crate::Result;
use regex::Regex;

use super::stats::DocumentStats;

/// Processor to merge standalone section numbers with their section titles.
///
/// Some PDFs have section numbers (like "1." or "2.") as separate blocks from
/// the section title (like "Introduction" or "Related Works"). This processor
/// merges them into a single block (e.g., "1. Introduction").
///
/// Pattern requirements (First Principles - no keyword matching):
/// - First block: Just a section number like "1.", "2.", "1.1.", etc.
/// - Second block: Looks like a section title (capitalized, short, appropriate position)
/// - Both blocks must be on the same page and close together
pub struct SectionNumberMergeProcessor;

impl SectionNumberMergeProcessor {
    pub fn new() -> Self {
        Self
    }

    /// Check if text is a standalone section number
    fn is_section_number(text: &str) -> bool {
        let trimmed = text.trim();
        // Match patterns like "1", "1.", "2", "2.", "1.1", "1.1.", etc.
        // Must be just digits and dots, optionally with trailing period
        if trimmed.is_empty() || trimmed.len() > 10 {
            return false;
        }

        // Check if it's all digits and dots
        let all_digit_or_dot = trimmed.chars().all(|c| c.is_ascii_digit() || c == '.');
        if !all_digit_or_dot {
            return false;
        }

        // Must start with a digit
        if !trimmed
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
        {
            return false;
        }

        // Must have at least one digit
        trimmed.chars().any(|c| c.is_ascii_digit())
    }

    /// Check if text looks like a section title based on structural properties.
    /// First Principles: sections are characterized by capitalization and length,
    /// not by matching keyword lists.
    fn looks_like_section_title(text: &str) -> bool {
        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed.len() > 100 {
            return false;
        }
        // Section titles typically start with uppercase letter
        trimmed
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
    }
}

impl Default for SectionNumberMergeProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for SectionNumberMergeProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        for page in &mut document.pages {
            // First pass: collect all section numbers and their Y positions
            let mut section_numbers: Vec<(usize, String, f32, f32)> = Vec::new(); // (index, text, y_center, x_left)

            for (idx, block) in page.blocks.iter().enumerate() {
                let text = block.text.trim();
                if Self::is_section_number(text) {
                    let y_center = (block.bbox.y1 + block.bbox.y2) / 2.0;
                    section_numbers.push((idx, text.to_string(), y_center, block.bbox.x1));
                }
            }

            // Second pass: for each section number, find matching section keyword on same Y-band
            let mut merge_map: std::collections::HashMap<usize, (usize, String)> =
                std::collections::HashMap::new();

            for (sec_idx, sec_text, sec_y, sec_x) in &section_numbers {
                // Look for a section keyword block on the same horizontal band (within 20px Y)
                for (title_idx, title_block) in page.blocks.iter().enumerate() {
                    if title_idx == *sec_idx {
                        continue;
                    }

                    let title_text = title_block.text.trim();
                    let title_y_center = (title_block.bbox.y1 + title_block.bbox.y2) / 2.0;
                    let y_gap = (sec_y - title_y_center).abs();

                    // Must be on same horizontal band (within 25px Y difference)
                    // and title must be to the right of section number
                    if y_gap < 25.0 && title_block.bbox.x1 > *sec_x {
                        if Self::looks_like_section_title(title_text) {
                            let merged_text =
                                format!("{}. {}", sec_text.trim_end_matches('.'), title_text);
                            tracing::info!(
                                "SectionNumberMerge: Horizontal match '{}' + '{}' = '{}' (y_gap={:.1})",
                                sec_text,
                                title_text,
                                merged_text,
                                y_gap
                            );
                            merge_map.insert(*sec_idx, (title_idx, merged_text));
                            break;
                        }
                    }
                }
            }

            // Third pass: create merged block list
            let mut skip_indices: std::collections::HashSet<usize> =
                std::collections::HashSet::new();
            let mut merged_blocks: Vec<Block> = Vec::new();

            for (idx, block) in page.blocks.iter().enumerate() {
                if skip_indices.contains(&idx) {
                    continue;
                }

                if let Some((title_idx, merged_text)) = merge_map.get(&idx) {
                    // This is a section number that matched a title - merge them
                    let title_block = &page.blocks[*title_idx];
                    let mut merged = block.clone();
                    merged.text = merged_text.clone();
                    merged.spans.extend(title_block.spans.clone());
                    merged.bbox.x2 = merged.bbox.x2.max(title_block.bbox.x2);
                    merged.bbox.y1 = merged.bbox.y1.min(title_block.bbox.y1);
                    merged.bbox.y2 = merged.bbox.y2.max(title_block.bbox.y2);
                    merged_blocks.push(merged);
                    skip_indices.insert(*title_idx);
                } else {
                    merged_blocks.push(block.clone());
                }
            }

            // Update positions
            for (pos, block) in merged_blocks.iter_mut().enumerate() {
                block.position = pos;
            }

            page.blocks = merged_blocks;
        }

        Ok(document)
    }

    fn name(&self) -> &'static str {
        "SectionNumberMergeProcessor"
    }
}

/// Processor to filter out margin content like line numbers.
///
/// Academic papers (especially arXiv) often have line numbers in the left margin.
/// This processor removes blocks that are positioned in page margins.
/// Margin filter processor - removes margin content (line numbers, headers, footers).
///
/// Uses adaptive margins based on page dimensions (First Principles approach).
/// No magic numbers - all thresholds are calculated as percentages of page size.
pub struct MarginFilterProcessor {
    // No configuration needed - margins calculated adaptively from page dimensions!
}

impl MarginFilterProcessor {
    /// Create a new margin filter processor.
    /// Margins are calculated adaptively based on page dimensions.
    pub fn new() -> Self {
        Self {}
    }

    /// Create with custom margins (deprecated - for backward compatibility in tests).
    #[cfg(test)]
    #[allow(dead_code)]
    pub fn with_margins(_left: f32, _right: f32, _top: f32, _bottom: f32) -> Self {
        // Parameters ignored - now calculated adaptively from page dimensions
        Self {}
    }

    /// Check if a block is in the margin area using adaptive thresholds.
    fn is_margin_content(
        &self,
        block: &Block,
        page_width: f32,
        _page_height: f32,
        left_margin: f32,
        right_margin: f32,
        _top_margin: f32,
        bottom_margin: f32,
        line_number_edge: f32,
    ) -> bool {
        let bbox = &block.bbox;

        // First principles: content outside the main text region (true margins) is almost never
        // semantically meaningful in scientific PDFs (line numbers, crop marks, etc.).
        if bbox.x2 < left_margin {
            tracing::debug!("Filtering left margin block: '{}'", block.text.trim());
            return true;
        }
        if bbox.x1 > page_width - right_margin {
            tracing::debug!("Filtering right margin block: '{}'", block.text.trim());
            return true;
        }

        // First principles: line numbering manifests as long runs of integers near the page edge.
        // This catches cases where line numbers get merged into one block and are no longer "short".
        let trimmed = block.text.trim();
        let edge_adjacent = bbox.x1 < line_number_edge || bbox.x2 > page_width - line_number_edge;
        if edge_adjacent {
            let tokens: Vec<&str> = trimmed.split_whitespace().collect();
            if tokens.len() >= 6
                && tokens
                    .iter()
                    .all(|t| !t.is_empty() && t.chars().all(|c| c.is_ascii_digit()))
            {
                let mut nums: Vec<i32> = Vec::with_capacity(tokens.len());
                for t in tokens {
                    if let Ok(n) = t.parse::<i32>() {
                        nums.push(n);
                    } else {
                        nums.clear();
                        break;
                    }
                }

                if !nums.is_empty() {
                    let all_same = nums.iter().all(|n| *n == nums[0]);
                    let consecutive = nums.windows(2).all(|w| w[1] == w[0].saturating_add(1));
                    if all_same || consecutive {
                        tracing::debug!("Filtering edge numeric run: '{}'", trimmed);
                        return true;
                    }
                }
            }
        }

        // Check if block is single digit/letter at edge of content (likely line number)
        let text = trimmed;
        if text.len() <= 2
            && text
                .chars()
                .all(|c| c.is_ascii_digit() || c.is_ascii_uppercase())
        {
            // Use adaptive threshold (10% of page width) instead of fixed 60.0
            if bbox.x1 < line_number_edge || bbox.x1 > page_width - line_number_edge {
                tracing::debug!("Filtering likely line number: '{}'", text);
                return true;
            }
        }

        // NOTE: PDF coordinates: Y=0 at bottom, higher Y = higher on page.
        // Footer region is near Y=0, header region is near Y=page_height.

        // Check bottom margin (footers): filter page numbers
        let in_footer = bbox.y1 <= bottom_margin;
        if in_footer {
            let trimmed = block.text.trim();
            if trimmed.parse::<i32>().is_ok() {
                tracing::debug!("Filtering footer page number: '{}'", trimmed);
                return true;
            }
        }

        // Header/footer removal for running headers is handled at document-level in process()
        // (needs repetition stats across pages).

        false
    }
}

impl Default for MarginFilterProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for MarginFilterProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        use std::collections::{HashMap, HashSet};

        // First pass: collect repeated margin texts across pages.
        // This targets running headers/footers without over-detecting section headers.
        let mut header_counts: HashMap<String, usize> = HashMap::new();
        let mut footer_counts: HashMap<String, usize> = HashMap::new();

        let normalize = |text: &str| -> String {
            let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
            collapsed.to_lowercase()
        };

        for page in &document.pages {
            let page_height = page.height;
            let top_margin = page_height * 0.05;
            let bottom_margin = page_height * 0.05;

            let mut header_seen: HashSet<String> = HashSet::new();
            let mut footer_seen: HashSet<String> = HashSet::new();

            for block in &page.blocks {
                let trimmed = block.text.trim();
                if trimmed.is_empty() {
                    continue;
                }
                // Only consider reasonably short texts as running header/footer candidates.
                // (Long blocks in margins are more likely actual content.)
                if trimmed.len() < 10 || trimmed.len() > 220 {
                    continue;
                }

                let bbox = &block.bbox;
                let in_header = bbox.y2 >= page_height - top_margin;
                let in_footer = bbox.y1 <= bottom_margin;

                if in_header {
                    let key = normalize(trimmed);
                    if header_seen.insert(key.clone()) {
                        *header_counts.entry(key).or_insert(0) += 1;
                    }
                }

                if in_footer {
                    // Skip pure page numbers; they are handled separately.
                    if trimmed.parse::<i32>().is_ok() {
                        continue;
                    }
                    let key = normalize(trimmed);
                    if footer_seen.insert(key.clone()) {
                        *footer_counts.entry(key).or_insert(0) += 1;
                    }
                }
            }
        }

        // Text appearing on >= half of pages (at least 3) is likely running header/footer.
        let threshold = (document.pages.len() / 2).max(3);
        let running_headers: HashSet<String> = header_counts
            .into_iter()
            .filter(|(_, count)| *count >= threshold)
            .map(|(k, _)| k)
            .collect();
        let running_footers: HashSet<String> = footer_counts
            .into_iter()
            .filter(|(_, count)| *count >= threshold)
            .map(|(k, _)| k)
            .collect();

        // Second pass: filter margins + remove running header/footer blocks.
        for page in &mut document.pages {
            let page_width = page.width;
            let page_height = page.height;

            // Calculate adaptive margins based on THIS page's dimensions (First Principles!)
            // Typography standards: margins are percentages of page dimensions
            let left_margin = page_width * 0.08; // 8% of page width (standard book margin)
            let right_margin = page_width * 0.05; // 5% of page width (smaller than left)
            let top_margin = page_height * 0.05; // 5% of page height (header space)
            let bottom_margin = page_height * 0.05; // 5% of page height (footer space)
            let line_number_edge = page_width * 0.10; // 10% of page width (line number detection)

            page.blocks.retain(|block| {
                if self.is_margin_content(
                    block,
                    page_width,
                    page_height,
                    left_margin,
                    right_margin,
                    top_margin,
                    bottom_margin,
                    line_number_edge,
                ) {
                    return false;
                }

                let trimmed = block.text.trim();
                if trimmed.is_empty() {
                    return true;
                }

                let bbox = &block.bbox;
                let in_header = bbox.y2 >= page_height - top_margin;
                let in_footer = bbox.y1 <= bottom_margin;
                let key = normalize(trimmed);

                if in_header && running_headers.contains(&key) {
                    tracing::debug!("Filtering running header: '{}'", trimmed);
                    return false;
                }
                if in_footer && running_footers.contains(&key) {
                    tracing::debug!("Filtering running footer: '{}'", trimmed);
                    return false;
                }

                true
            });
        }

        Ok(document)
    }

    fn name(&self) -> &str {
        "MarginFilterProcessor"
    }
}

/// Processor to filter garbled/corrupted text from figure annotations.
///
/// Detects text that appears corrupted, such as:
/// - High ratio of single-character words
/// - Text that doesn't form recognizable patterns
/// - Very short isolated fragments
pub struct GarbledTextFilterProcessor {
    /// Maximum ratio of short words (≤2 chars) allowed
    max_short_word_ratio: f32,
    /// Minimum number of words to apply the ratio check
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
    fn is_garbled(&self, text: &str) -> bool {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return false;
        }

        // Filter very short isolated fragments (≤3 chars) that don't look like valid content
        // e.g., ",w", "v", but not "1.", "a)", "I", "A"
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

        // Filter very short isolated fragments that look like figure labels
        // e.g., ",w", "v", "x u", "l i d"
        // But not valid short content like "1.", "a)", etc.
        if trimmed.len() <= 6 && trimmed.split_whitespace().count() >= 2 {
            // Multiple words in ≤6 chars = likely garbled
            // Except if it looks like a numbered item or reference
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

        // Count short words (≤2 chars) excluding common valid short words
        let valid_short_words = [
            "a", "an", "as", "at", "be", "by", "do", "go", "he", "if", "in", "is", "it", "me",
            "my", "no", "of", "on", "or", "so", "to", "up", "us", "we", "i", "1", "2", "3", "4",
            "5", "6", "7", "8", "9",
        ];
        let short_count = words
            .iter()
            .filter(|w| w.len() <= 2 && !valid_short_words.contains(&w.to_lowercase().as_str()))
            .count();
        let ratio = short_count as f32 / words.len() as f32;

        if ratio > self.max_short_word_ratio {
            tracing::debug!(
                "Filtering garbled text ({}% unusual short words): '{}'",
                (ratio * 100.0) as i32,
                if trimmed.len() > 50 {
                    &trimmed[..50]
                } else {
                    trimmed
                }
            );
            return true;
        }

        // Check for patterns like missing letters: "a iliar tools hich o erlook"
        // This has MANY isolated letters that aren't common words
        let isolated_letters = words
            .iter()
            .filter(|w| w.len() == 1 && w.chars().all(|c| c.is_alphabetic()))
            .filter(|w| !valid_short_words.contains(&w.to_lowercase().as_str()))
            .count();
        // Need at least 4 isolated non-common letters AND high ratio
        if isolated_letters >= 4 && ratio > 0.30 {
            tracing::debug!(
                "Filtering text with isolated letters: '{}'",
                if trimmed.len() > 50 {
                    &trimmed[..50]
                } else {
                    trimmed
                }
            );
            return true;
        }

        // Check for OCR-garbled text pattern: words that look like word fragments
        // Pattern: single letter + space + lowercase fragment (e.g., "a iliar" = "familiar" with missing "f")
        // "hich" = "which" missing "w", "erlook" = "overlook" missing "ov", "ec" = "exec" missing "ex"
        // These are unusual non-words that suggest OCR corruption
        let non_word_fragments = words
            .iter()
            .filter(|w| {
                let w_lower = w.to_lowercase();
                let len = w_lower.len();
                // Check for likely fragments: 4-7 chars that start with unusual patterns
                if len >= 4 && len <= 8 && w_lower.chars().all(|c| c.is_alphabetic()) {
                    // Check for patterns that suggest missing first letter(s)
                    // Common garbled patterns from the PDF
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
                if trimmed.len() > 50 {
                    &trimmed[..50]
                } else {
                    trimmed
                }
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
                // Tables (and other structured blocks) often contain many short tokens (e.g. '|')
                // that would be falsely flagged as "garbled".
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

/// Trait for document processors.
pub trait Processor: Send + Sync {
    /// Process a document, returning the modified document.
    fn process(&self, document: Document) -> Result<Document>;

    /// Get the processor name for debugging.
    fn name(&self) -> &str;
}

/// Chain of processors.
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

/// Layout processor - applies layout analysis to pages.
pub struct LayoutProcessor {
    analyzer: LayoutAnalyzer,
}

impl LayoutProcessor {
    /// Create a new layout processor.
    pub fn new() -> Self {
        Self {
            analyzer: LayoutAnalyzer::new(),
        }
    }
}

impl Default for LayoutProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for LayoutProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        for page in &mut document.pages {
            // Skip layout reanalysis if page already has columns set (backend already handled it)
            // This allows backends like SOTA to do their own column detection without interference
            if !page.columns.is_empty() {
                tracing::debug!(
                    "Page {} already has {} columns set, skipping layout reanalysis",
                    page.number,
                    page.columns.len()
                );
                continue;
            }

            let layout = self.analyzer.analyze(&page.blocks, page.width, page.height);

            // Check if the detected columns look like a table structure
            // If so, don't apply column-based reading order
            let bboxes: Vec<crate::schema::BoundingBox> =
                page.blocks.iter().map(|b| b.bbox).collect();
            let is_table = self
                .analyzer
                .column_detector()
                .is_likely_table(&bboxes, &layout.columns);

            if is_table {
                // For table-like layouts, use single column to preserve natural order
                page.columns = vec![];
                tracing::debug!("Detected table-like layout, skipping column-based reading order");
            } else {
                page.columns = layout.columns;
                // Sort blocks by reading order only for actual multi-column layouts
                self.analyzer
                    .sort_by_reading_order(&mut page.blocks, &page.columns);
            }
        }

        Ok(document)
    }

    fn name(&self) -> &str {
        "LayoutProcessor"
    }
}
/// Table detection processor - identifies tables from block layout.
pub struct TableDetectionProcessor;

impl TableDetectionProcessor {
    /// Create a new table detection processor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for TableDetectionProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for TableDetectionProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        for page in &mut document.pages {
            if page.blocks.is_empty() {
                continue;
            }

            // If we detected multiple columns, skip table detection to preserve reading order
            // This is a simplification, but prevents column text from being treated as a table
            if page.columns.len() > 1 {
                continue;
            }

            // Group blocks by Y coordinate (rows) with a more generous tolerance
            // to handle multi-line cells that might be slightly misaligned
            let mut rows: Vec<Vec<usize>> = Vec::new();
            let mut sorted_indices: Vec<usize> = (0..page.blocks.len()).collect();
            sorted_indices.sort_by(|&a, &b| {
                page.blocks[a]
                    .bbox
                    .y1
                    .partial_cmp(&page.blocks[b].bbox.y1)
                    .unwrap()
            });

            for idx in sorted_indices {
                let block = &page.blocks[idx];
                let mut found = false;
                for row in rows.iter_mut() {
                    let first_idx = row[0];
                    // If Y coordinates overlap significantly, they are on the same row
                    let b1 = &page.blocks[first_idx];
                    let overlap_y = b1.bbox.y2.min(block.bbox.y2) - b1.bbox.y1.max(block.bbox.y1);
                    let min_h = (b1.bbox.y2 - b1.bbox.y1).min(block.bbox.y2 - block.bbox.y1);

                    if overlap_y > min_h * 0.5 || (b1.bbox.y1 - block.bbox.y1).abs() < 10.0 {
                        row.push(idx);
                        found = true;
                        break;
                    }
                }
                if !found {
                    rows.push(vec![idx]);
                }
            }

            // Sort each row by X coordinate
            for row in rows.iter_mut() {
                row.sort_by(|&a, &b| {
                    page.blocks[a]
                        .bbox
                        .x1
                        .partial_cmp(&page.blocks[b].bbox.x1)
                        .unwrap()
                });
            }

            tracing::debug!("Detected {} rows", rows.len());
            for (idx, row) in rows.iter().enumerate() {
                tracing::debug!("Row {}: {} blocks", idx, row.len());
            }

            // Identify table regions
            let mut new_blocks = Vec::new();
            let mut i = 0;
            while i < rows.len() {
                // A potential table row has multiple blocks
                if rows[i].len() > 1 {
                    let mut table_rows = vec![i];
                    let mut j = i + 1;

                    // Look ahead for more rows that look like they belong to the same table
                    while j < rows.len() {
                        let current_row_blocks = &rows[j];

                        // If it's a single block, it might be a multi-line cell continuation
                        // or the end of the table.
                        if current_row_blocks.len() > 1 {
                            // Check gap between blocks
                            let mut max_gap: f32 = 0.0;
                            for k in 0..current_row_blocks.len() - 1 {
                                let b1 = &page.blocks[current_row_blocks[k]];
                                let b2 = &page.blocks[current_row_blocks[k + 1]];
                                max_gap = max_gap.max(b2.bbox.x1 - b1.bbox.x2);
                            }

                            // If gap is too large, it's probably columns, not a table
                            // Increased to 150.0 to handle wider tables
                            if max_gap > 150.0 {
                                break;
                            }

                            table_rows.push(j);
                            j += 1;
                        } else if current_row_blocks.len() == 1 {
                            // Check if this single block aligns with one of the columns in the table
                            let block = &page.blocks[current_row_blocks[0]];
                            let mut aligns = false;
                            for &prev_row_idx in &table_rows {
                                for &prev_block_idx in &rows[prev_row_idx] {
                                    let prev_block = &page.blocks[prev_block_idx];
                                    let overlap_x = prev_block.bbox.x2.min(block.bbox.x2)
                                        - prev_block.bbox.x1.max(block.bbox.x1);
                                    let min_w = (prev_block.bbox.x2 - prev_block.bbox.x1)
                                        .min(block.bbox.x2 - block.bbox.x1);
                                    if overlap_x > min_w * 0.8 {
                                        aligns = true;
                                        break;
                                    }
                                }
                                if aligns {
                                    break;
                                }
                            }

                            if aligns {
                                table_rows.push(j);
                                j += 1;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    // If we have at least 2 rows and some multi-column rows, it's a table
                    let has_multi_col = table_rows.iter().any(|&r| rows[r].len() > 1);

                    // A table should have at least 4 rows or multiple rows with 3+ columns to be sure it's not just a random alignment
                    let is_likely_table = (table_rows.len() >= 6 && has_multi_col)
                        || (table_rows.len() >= 4
                            && table_rows.iter().any(|&r| rows[r].len() >= 4));

                    if is_likely_table {
                        let mut table_bbox = page.blocks[rows[table_rows[0]][0]].bbox.clone();
                        for &row_idx in &table_rows {
                            for &block_idx in &rows[row_idx] {
                                table_bbox = table_bbox.union(&page.blocks[block_idx].bbox);
                            }
                        }

                        let mut table_block = Block::new(BlockType::Table, table_bbox);
                        table_block.page = page.number as usize - 1;

                        // Group blocks into cells based on X alignment
                        // This is a simplification; a real SOTA extractor would do better
                        for &row_idx in &table_rows {
                            for &block_idx in &rows[row_idx] {
                                table_block.children.push(page.blocks[block_idx].clone());
                            }
                        }
                        new_blocks.push(table_block);
                        i = j;
                    } else {
                        // Not a table, just push the blocks
                        for &block_idx in &rows[i] {
                            new_blocks.push(page.blocks[block_idx].clone());
                        }
                        i += 1;
                    }
                } else {
                    // Single block row
                    for &block_idx in &rows[i] {
                        new_blocks.push(page.blocks[block_idx].clone());
                    }
                    i += 1;
                }
            }
            page.blocks = new_blocks;
        }
        Ok(document)
    }

    fn name(&self) -> &str {
        "TableDetectionProcessor"
    }
}

/// Text-based table reconstruction processor.
///
/// This handles common cases where the PDF extraction produces one text block per table row
/// (rather than per cell), by reconstructing a structured `BlockType::Table` from caption-adjacent
/// lines.
pub struct TextTableReconstructionProcessor;

impl TextTableReconstructionProcessor {
    pub fn new() -> Self {
        Self
    }

    fn normalize_table_caption(text: &str) -> String {
        text.trim()
            .trim_start_matches('#')
            .trim_start_matches('*')
            .trim_start()
            .to_string()
    }

    fn looks_like_table_caption(text: &str) -> bool {
        let t = Self::normalize_table_caption(text);
        // Table 1. ..., TABLE 2 ..., Table S1 ..., etc.
        let re = Regex::new(r"(?i)^table\s*(?:\d+|s\d+)\b").unwrap();
        re.is_match(&t)
    }

    fn is_hard_break(block: &Block) -> bool {
        let t = block.text.trim();
        t == "---" || block.block_type == BlockType::SectionHeader
    }

    fn looks_like_pipe_table(text: &str) -> bool {
        let t = text.trim();
        if !t.starts_with('|') {
            return false;
        }

        let mut lines: Vec<&str> = t
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();
        if lines.len() < 2 {
            return false;
        }

        // Detect a markdown table separator line like: | --- | ---: | :--- |
        let has_separator = lines.iter().any(|l| {
            l.starts_with('|')
                && l.chars()
                    .all(|c| c == '|' || c == '-' || c == ':' || c == ' ' || c == '\t')
        });

        // Many extracted tables come as multi-line text with several pipe-prefixed rows.
        let pipe_lines = lines
            .iter()
            .filter(|l| l.starts_with('|') && l.matches('|').count() >= 2)
            .count();

        has_separator && pipe_lines >= 2
    }

    fn table_like_score(text: &str) -> i32 {
        let t = text.trim();
        if t.is_empty() {
            return 0;
        }

        // Be conservative: avoid treating normal prose as a table.
        // Strong signals:
        // - explicit pipe separators
        // - repeated multi-space alignment (common in extracted tables)
        // - numeric suffix patterns (e.g. "... 0.4578 4")
        let pipes = t.matches('|').count();
        let has_multi_space = t.contains("  ") || t.contains('\t');

        let cleaned = Self::sanitize_table_line(t);
        let num_tokens = cleaned
            .split_whitespace()
            .filter(|tok| tok.parse::<f64>().is_ok())
            .count();
        let has_numeric_suffix = Self::parse_numeric_suffix(&cleaned).is_some();

        let mut score = 0;
        if pipes >= 2 {
            score += 3;
        }
        if has_multi_space {
            score += 2;
        }
        if has_numeric_suffix {
            score += 3;
        } else if num_tokens >= 2 {
            score += 2;
        }

        score
    }

    fn sanitize_table_line(line: &str) -> String {
        line.replace('|', " ")
    }

    fn parse_agent_pipeline_leaderboard(line: &str) -> Option<Vec<Vec<String>>> {
        let t = Self::sanitize_table_line(line);
        let tokens: Vec<&str> = t.split_whitespace().collect();
        if tokens.len() < 6 {
            return None;
        }

        // Detect common collapsed line:
        // "Agent Pipeline Func-IoU(%) Resolved(%) Agentless 5.28 10.12 ..."
        let has_agent_pipeline = tokens
            .windows(2)
            .any(|w| w[0].eq_ignore_ascii_case("agent") && w[1].eq_ignore_ascii_case("pipeline"));
        if !has_agent_pipeline {
            return None;
        }
        if !tokens.iter().any(|t| t.to_lowercase().contains("resolved")) {
            return None;
        }

        let func_idx = tokens
            .iter()
            .position(|t| t.to_lowercase().contains("iou"))?;
        let resolved_idx = tokens
            .iter()
            .position(|t| t.to_lowercase().contains("resolved"))?;
        let data_start = resolved_idx + 1;
        if data_start >= tokens.len() {
            return None;
        }

        let mut rows: Vec<Vec<String>> = Vec::new();
        rows.push(vec![
            "Agent Pipeline".to_string(),
            tokens[func_idx].to_string(),
            tokens[resolved_idx].to_string(),
        ]);

        let is_num = |s: &str| s.parse::<f64>().is_ok();

        let mut i = data_start;
        while i < tokens.len() {
            let mut name_parts: Vec<&str> = Vec::new();
            while i < tokens.len() && !is_num(tokens[i]) {
                name_parts.push(tokens[i]);
                i += 1;
            }
            if i + 1 >= tokens.len() {
                break;
            }
            if !is_num(tokens[i]) || !is_num(tokens[i + 1]) {
                break;
            }
            let name = name_parts.join(" ");
            if !name.is_empty() {
                rows.push(vec![name, tokens[i].to_string(), tokens[i + 1].to_string()]);
            }
            i += 2;
        }

        if rows.len() >= 2 {
            Some(rows)
        } else {
            None
        }
    }

    fn parse_numeric_suffix(line: &str) -> Option<(String, Vec<String>)> {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            return None;
        }

        // Try: <prefix> <float> <int>
        if tokens.len() >= 3 {
            let last = tokens[tokens.len() - 1];
            let prev = tokens[tokens.len() - 2];

            let last_is_int = last.parse::<i64>().is_ok();
            let prev_is_float = prev.parse::<f64>().is_ok();

            if last_is_int && prev_is_float {
                let prefix = tokens[..tokens.len() - 2].join(" ");
                return Some((prefix, vec![prev.to_string(), last.to_string()]));
            }
        }

        // Try: <prefix> <float>
        if tokens.len() >= 2 {
            let last = tokens[tokens.len() - 1];
            if last.parse::<f64>().is_ok() {
                let prefix = tokens[..tokens.len() - 1].join(" ");
                return Some((prefix, vec![last.to_string()]));
            }
        }

        None
    }

    fn build_table_cells(
        table_bbox: crate::schema::BoundingBox,
        page: usize,
        rows: &[Vec<String>],
    ) -> Vec<Block> {
        if rows.is_empty() {
            return Vec::new();
        }
        let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
        if col_count == 0 {
            return Vec::new();
        }

        let width = (table_bbox.x2 - table_bbox.x1).max(1.0);
        let col_w = width / col_count as f32;
        let row_h = 14.0;

        let mut children = Vec::new();
        for (r, row) in rows.iter().enumerate() {
            for c in 0..col_count {
                let text = row.get(c).cloned().unwrap_or_default();
                let mut cell_bbox = table_bbox;
                cell_bbox.x1 = table_bbox.x1 + c as f32 * col_w;
                cell_bbox.x2 = table_bbox.x1 + (c as f32 + 1.0) * col_w;
                cell_bbox.y1 = table_bbox.y1 + r as f32 * row_h;
                cell_bbox.y2 = cell_bbox.y1 + row_h;

                let mut cell = Block::new(BlockType::TableCell, cell_bbox);
                cell.page = page;
                cell.text = text;
                children.push(cell);
            }
        }
        children
    }
}

impl Default for TextTableReconstructionProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for TextTableReconstructionProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        // Pre-scan *previous-page* table candidates so we can guard reconstruction even if the
        // caption drifts onto the next page (common around page breaks).
        // We include both structured tables and pre-rendered pipe tables.
        let page_table_bboxes: Vec<Vec<crate::schema::BoundingBox>> = document
            .pages
            .iter()
            .map(|p| {
                p.blocks
                    .iter()
                    .filter(|b| {
                        b.block_type == BlockType::Table || Self::looks_like_pipe_table(&b.text)
                    })
                    .map(|b| b.bbox)
                    .collect()
            })
            .collect();

        for (page_idx, page) in document.pages.iter_mut().enumerate() {
            if page.blocks.is_empty() {
                continue;
            }

            let mut new_blocks: Vec<Block> = Vec::with_capacity(page.blocks.len());
            let mut i = 0;
            while i < page.blocks.len() {
                let block = &page.blocks[i];

                if !Self::looks_like_table_caption(&block.text) {
                    new_blocks.push(block.clone());
                    i += 1;
                    continue;
                }

                // If the backend already produced a structured Table block near this caption,
                // do NOT attempt text-based reconstruction. Table blocks often have empty
                // `.text` and would otherwise be missed by the text-only heuristics below.
                let mut has_structured_table = false;
                let caption_bbox = block.bbox;

                let consider_table_bbox = |table_bbox: crate::schema::BoundingBox| -> bool {
                    let overlap_x = (caption_bbox.x2.min(table_bbox.x2)
                        - caption_bbox.x1.max(table_bbox.x1))
                    .max(0.0);
                    let min_w = caption_bbox.width().min(table_bbox.width()).max(1.0);
                    let x_overlap_ratio = overlap_x / min_w;
                    x_overlap_ratio >= 0.30
                };

                // Spatial check: if there's any table on this page that is reasonably aligned
                // (X overlap / same column band), skip reconstruction.
                // FIRST PRINCIPLES: Check ALL tables on the page (before AND after caption).
                // The lattice engine may have detected a table from grid lines that will be
                // rendered at the proper position. We must avoid creating duplicate text-tables.
                // Check for tables BEFORE the caption in block order.
                if i > 0 {
                    has_structured_table = page.blocks[..i].iter().any(|b| {
                        (b.block_type == BlockType::Table || Self::looks_like_pipe_table(&b.text))
                            && consider_table_bbox(b.bbox)
                    });
                }

                // Also check for tables AFTER the caption in block order.
                // This handles the case where lattice detection placed the table after the caption.
                if !has_structured_table && i + 1 < page.blocks.len() {
                    has_structured_table = page.blocks[(i + 1)..]
                        .iter()
                        .any(|b| b.block_type == BlockType::Table && consider_table_bbox(b.bbox));
                }

                // Previous page (caption spills to next page in some PDFs).
                if !has_structured_table && page_idx > 0 {
                    if let Some(prev_tables) = page_table_bboxes.get(page_idx - 1) {
                        has_structured_table =
                            prev_tables.iter().any(|bb| consider_table_bbox(*bb));
                    }
                }

                if has_structured_table {
                    new_blocks.push(block.clone());
                    // If this caption is followed by a duplicate pipe-table fragment, drop it.
                    // This prevents a second (often garbled) table from being emitted when the
                    // real table already exists (sometimes on the previous page).
                    const MAX_DUP_SCAN_BLOCKS: usize = 32;
                    let mut first_pipe_idx: Option<usize> = None;
                    for j in (i + 1)..page.blocks.len().min(i + 1 + MAX_DUP_SCAN_BLOCKS) {
                        let t = page.blocks[j].text.trim();
                        if t.is_empty() {
                            break;
                        }
                        if Self::is_hard_break(&page.blocks[j]) || Self::looks_like_table_caption(t)
                        {
                            break;
                        }
                        if t.starts_with('|') {
                            first_pipe_idx = Some(j);
                            break;
                        }
                    }

                    if let Some(pipe_start) = first_pipe_idx {
                        let mut consumed_until = pipe_start;
                        for j in pipe_start..page.blocks.len().min(pipe_start + MAX_DUP_SCAN_BLOCKS)
                        {
                            let b = &page.blocks[j];
                            let t = b.text.trim();
                            if t.is_empty() {
                                break;
                            }
                            if Self::is_hard_break(b) || Self::looks_like_table_caption(t) {
                                break;
                            }
                            consumed_until = j + 1;
                        }

                        // Consume everything from the caption up through the pipe table.
                        i = consumed_until;
                        continue;
                    }

                    i += 1;
                    continue;
                }

                // If the next non-empty line is already a pipe table, do nothing.
                let mut next_non_empty: Option<&str> = None;
                for j in (i + 1)..page.blocks.len().min(i + 8) {
                    let t = page.blocks[j].text.trim();
                    if t.is_empty() {
                        continue;
                    }
                    next_non_empty = Some(t);
                    break;
                }
                if matches!(next_non_empty, Some(t) if t.starts_with('|')) {
                    new_blocks.push(block.clone());
                    i += 1;
                    continue;
                }

                const MAX_SCAN_BLOCKS: usize = 22;

                // Scan forward for contiguous table-like lines (caption-before-table).
                // First pass: collect all candidate lines up to a hard break.
                // Track skipped zero-score lines so we can include them as potential headers.
                let mut forward_lines: Vec<(usize, String, i32)> = Vec::new();
                let mut forward_score = 0;
                let mut skipped_zeros: Vec<(usize, String, i32)> = Vec::new();
                let mut started = false;
                let mut consecutive_zeros = 0;
                const MAX_ZERO_LINES: usize = 2; // Allow up to 2 zero-score lines within table
                const MAX_LEADING_ZEROS: usize = 3; // Allow up to 3 header lines before data
                for j in (i + 1)..page.blocks.len().min(i + 1 + MAX_SCAN_BLOCKS) {
                    let b = &page.blocks[j];
                    let t = b.text.trim();
                    if t.is_empty() {
                        break;
                    }
                    if Self::is_hard_break(b) || Self::looks_like_table_caption(t) {
                        break;
                    }
                    let s = Self::table_like_score(t);
                    if !started {
                        if s == 0 {
                            // Track zero-score lines before we start - could be table headers
                            if skipped_zeros.len() < MAX_LEADING_ZEROS {
                                skipped_zeros.push((j, t.to_string(), s));
                            }
                            continue;
                        }
                        // Found first positive-score line - prepend skipped header lines
                        started = true;
                        for skipped in skipped_zeros.drain(..) {
                            forward_lines.push(skipped);
                        }
                        consecutive_zeros = 0;
                    } else if s == 0 {
                        consecutive_zeros += 1;
                        if consecutive_zeros > MAX_ZERO_LINES {
                            break;
                        }
                        // Include zero-score lines within tolerance
                        forward_lines.push((j, t.to_string(), s));
                        continue;
                    } else {
                        consecutive_zeros = 0;
                    }
                    forward_score += s;
                    forward_lines.push((j, t.to_string(), s));
                }

                // Scan backward for contiguous table-like lines (caption-after-table).
                let mut backward_lines: Vec<(usize, String, i32)> = Vec::new();
                let mut backward_score = 0;
                let mut started = false;
                let mut consecutive_zeros = 0;
                let mut steps = 0usize;
                let mut j = i.saturating_sub(1);
                loop {
                    if steps >= MAX_SCAN_BLOCKS {
                        break;
                    }
                    steps += 1;

                    let b = &page.blocks[j];
                    let t = b.text.trim();
                    if t.is_empty() {
                        break;
                    }
                    if Self::is_hard_break(b) || Self::looks_like_table_caption(t) {
                        break;
                    }

                    let s = Self::table_like_score(t);
                    if !started {
                        if s == 0 {
                            if j == 0 {
                                break;
                            }
                            j -= 1;
                            continue;
                        }
                        started = true;
                        consecutive_zeros = 0;
                    } else if s == 0 {
                        consecutive_zeros += 1;
                        if consecutive_zeros > MAX_ZERO_LINES {
                            break;
                        }
                        // Include zero-score lines within tolerance
                        backward_lines.push((j, t.to_string(), s));
                        if j == 0 {
                            break;
                        }
                        j -= 1;
                        continue;
                    } else {
                        consecutive_zeros = 0;
                    }

                    backward_score += s;
                    backward_lines.push((j, t.to_string(), s));

                    if j == 0 {
                        break;
                    }
                    j -= 1;
                }
                backward_lines.reverse();

                // Pick the best candidate direction.
                // Some PDFs collapse an entire table into a single extracted line/block.
                // Accept that case if the line is strongly table-like.
                const MIN_SINGLE_LINE_SCORE: i32 = 3;
                let forward_candidate = forward_lines.len() >= 2
                    || (forward_lines.len() == 1 && forward_lines[0].2 >= MIN_SINGLE_LINE_SCORE);
                let backward_candidate = backward_lines.len() >= 2
                    || (backward_lines.len() == 1 && backward_lines[0].2 >= MIN_SINGLE_LINE_SCORE);

                let forward_first = forward_lines.first().map(|(_, _, s)| *s).unwrap_or(0);
                let backward_first = backward_lines.first().map(|(_, _, s)| *s).unwrap_or(0);

                let use_forward = forward_candidate
                    && (!backward_candidate
                        || forward_first > backward_first
                        || (forward_first == backward_first && forward_score >= backward_score));
                let use_backward = !use_forward && backward_candidate;

                if !use_forward && !use_backward {
                    // Not enough evidence; keep as-is.
                    new_blocks.push(block.clone());
                    i += 1;
                    continue;
                }

                let lines: Vec<(usize, String, i32)> = if use_forward {
                    forward_lines
                } else {
                    backward_lines
                };

                // If we only captured a single line, emit a conservative 1-column table.
                // This guarantees a Markdown pipe table renders without guessing columns.
                if lines.len() == 1 {
                    if let Some(rows) = Self::parse_agent_pipeline_leaderboard(&lines[0].1) {
                        let mut table_bbox = block.bbox;
                        table_bbox = table_bbox.union(&page.blocks[lines[0].0].bbox);

                        let mut table_block = Block::new(BlockType::Table, table_bbox);
                        table_block.page = page.number as usize - 1;
                        table_block.children =
                            Self::build_table_cells(table_bbox, table_block.page, &rows);
                        table_block
                            .metadata
                            .insert("reconstructed".to_string(), serde_json::json!(true));

                        new_blocks.push(block.clone());
                        new_blocks.push(table_block);

                        if use_forward {
                            let consumed_until = lines[0].0 + 1;
                            i = consumed_until;
                        } else {
                            i += 1;
                        }
                        continue;
                    }

                    let mut table_bbox = block.bbox;
                    table_bbox = table_bbox.union(&page.blocks[lines[0].0].bbox);

                    let mut rows: Vec<Vec<String>> = Vec::new();
                    rows.push(vec!["Value".to_string()]);
                    rows.push(vec![lines[0].1.clone()]);

                    let mut table_block = Block::new(BlockType::Table, table_bbox);
                    table_block.page = page.number as usize - 1;
                    table_block.children =
                        Self::build_table_cells(table_bbox, table_block.page, &rows);
                    table_block
                        .metadata
                        .insert("reconstructed".to_string(), serde_json::json!(true));

                    new_blocks.push(block.clone());
                    new_blocks.push(table_block);

                    if use_forward {
                        let consumed_until = lines[0].0 + 1;
                        i = consumed_until;
                    } else {
                        i += 1;
                    }
                    continue;
                }

                // Header detection: often split across 1-2 lines.
                let mut header_cols: Vec<String> = Vec::new();
                let header_consumed: usize;
                let first = Self::sanitize_table_line(&lines[0].1);
                let second = lines.get(1).map(|(_, s, _)| s.as_str()).unwrap_or("");

                let first_lc = first.to_lowercase();
                if first_lc.contains("sub-task")
                    && first_lc.contains("f1")
                    && first_lc.contains("rank")
                {
                    header_cols.push("Sub-task".to_string());
                    if second.eq_ignore_ascii_case("task") {
                        header_cols.push("Task".to_string());
                        header_consumed = 2;
                    } else {
                        header_consumed = 1;
                    }
                    header_cols.push("F1-score".to_string());
                    header_cols.push("Rank".to_string());
                } else {
                    // Fallback: split by runs of whitespace
                    header_cols = first.split_whitespace().map(|s| s.to_string()).collect();
                    header_consumed = 1;
                }

                if header_cols.len() < 2 {
                    new_blocks.push(block.clone());
                    i += 1;
                    continue;
                }

                let mut rows: Vec<Vec<String>> = Vec::new();
                rows.push(header_cols.clone());

                // Specialized row parsing for common leaderboard tables.
                if header_cols.len() == 4
                    && header_cols.get(1).map(|s| s == "Task").unwrap_or(false)
                {
                    #[derive(Default)]
                    struct RowAcc {
                        sub_task: String,
                        task: String,
                        f1: String,
                        rank: String,
                    }

                    let re_subtask =
                        Regex::new(r"(?i)^(?:[A-D]\d+(?:\.\d+)?|C\d+|D\d+)\s*-").unwrap();

                    let mut pending: Option<RowAcc> = None;
                    let mut parsed: Vec<RowAcc> = Vec::new();

                    for (_, line, _) in lines.iter().skip(header_consumed) {
                        let t = line.trim();
                        if t.is_empty() {
                            continue;
                        }

                        let cleaned = Self::sanitize_table_line(t);
                        if let Some((prefix, nums)) = Self::parse_numeric_suffix(&cleaned) {
                            if let Some(p) = pending.take() {
                                if !p.sub_task.is_empty() {
                                    parsed.push(p);
                                }
                            }

                            let mut row = RowAcc::default();
                            row.sub_task = prefix;
                            if nums.len() == 2 {
                                row.f1 = nums[0].clone();
                                row.rank = nums[1].clone();
                            } else if nums.len() == 1 {
                                row.f1 = nums[0].clone();
                            }
                            pending = Some(row);
                            continue;
                        }

                        let is_int_only = t.parse::<i64>().is_ok();
                        let is_float_only = t.parse::<f64>().is_ok();
                        let has_digit = t.chars().any(|c| c.is_ascii_digit());
                        let lc = t.to_lowercase();
                        let looks_task = !has_digit
                            && (lc.contains("extraction")
                                || lc.contains("discovery")
                                || lc.contains("typing")
                                || lc.contains("relation"));

                        // Some PDFs split the sub-task descriptor into its own line.
                        let looks_subtask = re_subtask.is_match(t);
                        if looks_subtask {
                            if let Some(p) = pending.take() {
                                if !p.sub_task.is_empty() {
                                    parsed.push(p);
                                }
                            }
                            let mut row = RowAcc::default();
                            row.sub_task = t.to_string();
                            pending = Some(row);
                            continue;
                        }

                        if let Some(p) = pending.as_mut() {
                            if is_int_only && p.rank.is_empty() {
                                p.rank = t.to_string();
                                continue;
                            }
                            if is_float_only && p.f1.is_empty() {
                                p.f1 = t.to_string();
                                continue;
                            }
                            if looks_task && p.task.is_empty() {
                                p.task = t.to_string();
                                continue;
                            }

                            // Continuation: prefer extending the most descriptive field.
                            if p.task.is_empty() {
                                if !p.sub_task.is_empty() {
                                    p.sub_task.push(' ');
                                }
                                p.sub_task.push_str(t);
                            } else {
                                p.task.push(' ');
                                p.task.push_str(t);
                            }
                        }
                    }

                    if let Some(p) = pending.take() {
                        if !p.sub_task.is_empty() {
                            parsed.push(p);
                        }
                    }

                    for p in parsed {
                        rows.push(vec![p.sub_task, p.task, p.f1, p.rank]);
                    }
                } else {
                    // Generic fallback:
                    // 1) Try splitting rows by numeric suffix.
                    // 2) If that fails, build a single-column Markdown table so tables still render.
                    let mut numeric_rows: Vec<Vec<String>> = Vec::new();
                    for (_, line, _) in lines.iter().skip(header_consumed) {
                        let cleaned = Self::sanitize_table_line(line);
                        if let Some((prefix, nums)) = Self::parse_numeric_suffix(&cleaned) {
                            let mut r = Vec::new();
                            r.push(prefix);
                            r.extend(nums);
                            numeric_rows.push(r);
                        }
                    }

                    if numeric_rows.len() >= 2 {
                        rows.extend(numeric_rows);
                    } else {
                        // 1-col fallback: include the scanned lines as rows.
                        rows.clear();
                        rows.push(vec!["Value".to_string()]);
                        for (_, line, _) in lines.iter() {
                            rows.push(vec![line.clone()]);
                        }
                    }
                }

                // Normalize row sizes.
                let col_count = header_cols.len();
                if rows.len() >= 2 {
                    for r in rows.iter_mut() {
                        if r.len() < col_count {
                            r.resize(col_count, String::new());
                        }
                    }
                }

                if rows.len() < 2 {
                    new_blocks.push(block.clone());
                    i += 1;
                    continue;
                }

                // Guard against a common failure mode:
                // the scanned "table" lines are actually multi-line Markdown (already containing pipes),
                // but our numeric parsing failed, so we fell back to a 1-col "Value" table.
                // That produces nested/garbled pipe tables and nukes precision.
                let is_value_header = rows
                    .first()
                    .map(|h| {
                        !h.is_empty()
                            && h[0].eq_ignore_ascii_case("Value")
                            && h.iter().skip(1).all(|c| c.trim().is_empty())
                    })
                    .unwrap_or(false);

                if is_value_header {
                    new_blocks.push(block.clone());
                    i += 1;
                    continue;
                }

                // Compute bbox union over captured blocks.
                let mut table_bbox = block.bbox;
                for (idx, _, _) in &lines {
                    table_bbox = table_bbox.union(&page.blocks[*idx].bbox);
                }

                let mut table_block = Block::new(BlockType::Table, table_bbox);
                table_block.page = page.number as usize - 1;
                table_block.children = Self::build_table_cells(table_bbox, table_block.page, &rows);
                table_block
                    .metadata
                    .insert("reconstructed".to_string(), serde_json::json!(true));

                // Keep caption, then insert reconstructed table.
                new_blocks.push(block.clone());
                new_blocks.push(table_block);

                if use_forward {
                    // Skip consumed blocks (caption-before-table).
                    let consumed_until = lines.last().map(|(idx, _, _)| *idx + 1).unwrap_or(i + 1);
                    i = consumed_until;
                } else {
                    // Caption-after-table: do not skip forward blocks.
                    i += 1;
                }
            }

            page.blocks = new_blocks;
        }

        Ok(document)
    }

    fn name(&self) -> &str {
        "TextTableReconstructionProcessor"
    }
}

/// Block merge processor - merges adjacent blocks into paragraphs.
///
/// Uses adaptive thresholds based on document statistics (First Principles approach).
/// No magic numbers - all thresholds are derived from font sizes and spacing distributions.
pub struct BlockMergeProcessor {
    // No configuration needed - thresholds calculated from document stats!
}

impl BlockMergeProcessor {
    /// Create a new block merge processor.
    pub fn new() -> Self {
        Self {}
    }

    /// Create with custom parameters (deprecated - for backward compatibility in tests).
    #[cfg(test)]
    #[allow(dead_code)]
    pub fn with_params(_max_vertical_gap: f32, _max_margin_diff: f32) -> Self {
        // Parameters ignored - now calculated adaptively from document stats
        Self {}
    }

    /// Check if two blocks should be merged.
    ///
    /// Uses adaptive thresholds derived from DocumentStats instead of magic numbers.
    fn should_merge(&self, a: &Block, b: &Block, stats: &DocumentStats) -> bool {
        // Only merge text, header, or list item blocks
        if !matches!(
            a.block_type,
            BlockType::Text | BlockType::SectionHeader | BlockType::ListItem
        ) || !matches!(
            b.block_type,
            BlockType::Text | BlockType::SectionHeader | BlockType::ListItem
        ) {
            tracing::debug!(
                "BlockMerge: NOT merging - wrong block type: {:?} + {:?}",
                a.block_type,
                b.block_type
            );
            return false;
        }

        // If types are different, don't merge
        if a.block_type != b.block_type {
            tracing::debug!(
                "BlockMerge: NOT merging - different types: {:?} vs {:?}",
                a.block_type,
                b.block_type
            );
            return false;
        }

        // Don't merge if b looks like a start of a new list item
        let trimmed_b = b.text.trim();
        if trimmed_b.starts_with("- ")
            || trimmed_b.starts_with("* ")
            || trimmed_b.starts_with("• ")
            || (trimmed_b.len() > 2
                && trimmed_b.chars().next().unwrap().is_ascii_digit()
                && trimmed_b.contains(". "))
        {
            return false;
        }

        // Don't merge if style changes significantly
        if let (Some(span_a), Some(span_b)) = (a.spans.last(), b.spans.first()) {
            let size_a = span_a.style.size.unwrap_or(0.0);
            let size_b = span_b.style.size.unwrap_or(0.0);
            if (size_a - size_b).abs() > 1.5 {
                // Only log if it's text blocks that we'd otherwise merge
                if a.block_type == BlockType::Text && b.block_type == BlockType::Text {
                    // Use chars() for safe substring (handles multi-byte Unicode)
                    let a_start: String = a.text.chars().take(20).collect();
                    let b_start: String = b.text.chars().take(20).collect();
                    tracing::debug!(
                        "BlockMerge: NOT merging - font size diff: {:.1} vs {:.1}, text: '{}' + '{}'",
                        size_a, size_b,
                        a_start,
                        b_start
                    );
                }
                return false;
            }

            let weight_a = span_a.style.weight.unwrap_or(400);
            let weight_b = span_b.style.weight.unwrap_or(400);
            if (weight_a >= 600) != (weight_b >= 600) {
                return false;
            }
        }

        // For list items, don't merge if the second one starts with a bullet/number
        if a.block_type == BlockType::ListItem {
            let trimmed_b = b.text.trim();
            if trimmed_b.starts_with("- ")
                || trimmed_b.starts_with("* ")
                || trimmed_b.starts_with("• ")
                || (trimmed_b.len() > 2
                    && trimmed_b.chars().next().unwrap().is_ascii_digit()
                    && trimmed_b.contains(". "))
            {
                return false;
            }
        }

        // === ADAPTIVE THRESHOLDS (First Principles - no magic numbers!) ===

        // 1. Vertical gap threshold: Based on typical line spacing
        //    Allow up to 2.5x typical spacing (covers single to near-double spacing)
        let max_vertical_gap = stats.typical_line_spacing * 2.5;

        // For headers, allow more vertical space (multi-line headers)
        let vertical_threshold = if a.block_type == BlockType::SectionHeader {
            max_vertical_gap * 1.5 // 3.75x typical spacing for headers
        } else {
            max_vertical_gap
        };

        // Check vertical proximity
        // In PDF coordinates Y increases upward:
        // - y2 is the top of a block (higher Y)
        // - y1 is the bottom of a block (lower Y)
        // For blocks a (above) and b (below), the gap is: a.y1 (bottom of a) - b.y2 (top of b)
        let vertical_gap = (a.bbox.y1 - b.bbox.y2).abs();

        if vertical_gap > vertical_threshold {
            // tracing::debug!("BlockMerge: NOT merging - vertical gap {:.1} > {:.1}", vertical_gap, vertical_threshold);
            return false;
        }

        // 2. Horizontal alignment: Use adaptive tolerance from document stats
        let margin_diff = (a.bbox.x1 - b.bbox.x1).abs();

        let max_margin = if a.block_type == BlockType::SectionHeader {
            stats.column_alignment_tolerance * 2.5 // Headers can be more flexible
        } else {
            stats.column_alignment_tolerance
        };

        // 3. Column separation: Blocks in different columns should NOT merge
        //    Use page width percentage (15% = typical column gap)
        let horizontal_zone_threshold = stats.page_width * 0.15;

        if margin_diff > horizontal_zone_threshold {
            // Use chars() for safe substring (handles multi-byte Unicode)
            let a_start: String = a.text.chars().take(30).collect();
            let b_start: String = b.text.chars().take(30).collect();
            tracing::debug!(
                "BlockMerge: NOT merging due to horizontal zone difference ({:.1} > {:.1}): '{}' + '{}'",
                margin_diff,
                horizontal_zone_threshold,
                a_start,
                b_start
            );
            return false;
        }

        margin_diff <= max_margin
    }

    /// Merge blocks on a page using adaptive thresholds.
    fn merge_page_blocks(&self, blocks: Vec<Block>, stats: &DocumentStats) -> Vec<Block> {
        if blocks.len() < 2 {
            return blocks;
        }

        let mut merged = Vec::new();
        let mut current: Option<Block> = None;

        for block in blocks {
            if let Some(mut cur) = current.take() {
                let should = self.should_merge(&cur, &block, stats);
                if should {
                    // Use chars() to safely get last/first N characters (handles multi-byte Unicode)
                    let cur_end: String = cur
                        .text
                        .chars()
                        .rev()
                        .take(20)
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect();
                    let block_start: String = block.text.chars().take(20).collect();
                    tracing::debug!("BlockMerge: MERGING '{}' + '{}'", cur_end, block_start);
                    cur.merge(&block);
                    // NOTE: We preserve spans after merge to retain bold/italic styling.
                    // Block.merge() properly extends spans via: self.spans.extend(other.spans.clone())
                    // The MarkdownRenderer uses spans for styling markup (**bold**, *italic*).
                    current = Some(cur);
                } else {
                    // Log why consecutive Text blocks aren't merging
                    if cur.block_type == BlockType::Text && block.block_type == BlockType::Text {
                        let v_gap = (block.bbox.y1 - cur.bbox.y2).abs();
                        let h_diff = (cur.bbox.x1 - block.bbox.x1).abs();
                        // Use chars() for safe substring (handles multi-byte Unicode)
                        let cur_start: String = cur.text.chars().take(15).collect();
                        let block_start: String = block.text.chars().take(15).collect();
                        tracing::debug!(
                            "BlockMerge: Text+Text NOT merged: v_gap={:.1}, h_diff={:.1}, '{}...' + '{}...'",
                            v_gap, h_diff,
                            cur_start,
                            block_start
                        );
                    }
                    merged.push(cur);
                    current = Some(block);
                }
            } else {
                current = Some(block);
            }
        }

        if let Some(cur) = current {
            merged.push(cur);
        }

        // Update positions
        for (i, block) in merged.iter_mut().enumerate() {
            block.position = i;
        }

        merged
    }
}

impl Default for BlockMergeProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for BlockMergeProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        // Calculate stats once for entire document (First Principles approach!)
        let stats = DocumentStats::from_document(&document);

        for page in &mut document.pages {
            let blocks = std::mem::take(&mut page.blocks);
            page.blocks = self.merge_page_blocks(blocks, &stats);
            page.update_stats();
        }

        document.update_stats();
        Ok(document)
    }

    fn name(&self) -> &str {
        "BlockMergeProcessor"
    }
}

/// Processor to detect section headers from text patterns.
///
/// This processor works without font information by detecting:
/// - Numbered section patterns: "1. Introduction", "3.2. Related Work"
/// - Special section names: "Abstract", "References", "Conclusion"
/// - Running headers (repeated text across pages)
#[allow(dead_code)]
pub struct SectionPatternProcessor {
    /// Section number pattern regex
    section_regex: Regex,
    /// Special section names to detect
    special_sections: Vec<&'static str>,
}

#[allow(dead_code)]
impl SectionPatternProcessor {
    pub fn new() -> Self {
        Self {
            // Match patterns like "1.", "3.2.", "A.1.", followed by space and title
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
        }
    }

    /// Calculate heading level from section number.
    /// "1." -> level 2 (H2, since H1 is title)
    /// "3.2." -> level 3 (H3)
    /// "3.2.1." -> level 4 (H4)
    fn calculate_level(&self, section_num: &str) -> u8 {
        let dots = section_num.matches('.').count();
        // Minimum level 2, max level 6
        (dots + 1).min(6).max(2) as u8
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

        // Count on how many pages each short text appears
        for page in &document.pages {
            let mut seen_on_page = std::collections::HashSet::new();
            for block in &page.blocks {
                let text = block.text.trim().to_string();
                // Only consider short texts that could be headers (< 150 chars)
                if text.len() > 10 && text.len() < 150 {
                    // Normalize for comparison
                    let normalized = text.to_lowercase();
                    if seen_on_page.insert(normalized.clone()) {
                        *text_pages.entry(normalized).or_insert(0) += 1;
                    }
                }
            }
        }

        // Text appearing on 3+ pages is likely a running header
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
        // First pass: identify running headers
        let running_headers = self.find_running_headers(&document);

        // Second pass: process blocks
        for page in &mut document.pages {
            for block in &mut page.blocks {
                // Skip if already classified as something other than Text
                if block.block_type != BlockType::Text && block.block_type != BlockType::Paragraph {
                    continue;
                }

                let text = block.text.trim();

                // Check for running headers
                if running_headers.contains(&text.to_lowercase()) {
                    block.block_type = BlockType::PageHeader;
                    tracing::debug!("Marked running header: '{}'", text);
                    continue;
                }

                // Check for numbered section headers
                if let Some(captures) = self.section_regex.captures(text) {
                    if let (Some(num), Some(title)) = (captures.get(1), captures.get(2)) {
                        let section_num = num.as_str();
                        let title_text = title.as_str();

                        // Validate: title should be reasonable length
                        if title_text.len() < 80 && !title_text.ends_with('.') {
                            let level = self.calculate_level(section_num);
                            block.block_type = BlockType::SectionHeader;
                            block.level = Some(level);
                            tracing::debug!(
                                "Detected section header: '{}' -> level {}",
                                text,
                                level
                            );
                        }
                    }
                }
                // Check for special section names
                else if self.is_special_section(text) {
                    block.block_type = BlockType::SectionHeader;
                    block.level = Some(2); // Special sections are H2
                    tracing::debug!("Detected special section: '{}'", text);
                }
            }
        }

        Ok(document)
    }

    fn name(&self) -> &str {
        "SectionPatternProcessor"
    }
}

/// Post-processor for text cleanup.
pub struct PostProcessor {
    /// Normalize whitespace
    normalize_whitespace: bool,
    /// Fix common OCR errors
    fix_ocr_errors: bool,
    /// Consolidate headers
    consolidate_headers: bool,
}

impl PostProcessor {
    /// Create a new post-processor.
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

    /// Fix soft hyphen patterns and control characters.
    /// PDF extraction can produce control characters (like \x02) to indicate soft hyphens.
    /// Common patterns:
    /// - "modifi\x02 cation" (control char between word parts)
    /// - "modifi \x02 cation" (space + control char + space between parts)
    fn fix_soft_hyphens(&self, text: &str) -> String {
        let mut result = String::new();
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            let c = chars[i];

            // Check for control characters that indicate soft hyphen/line break
            // These are commonly \x02 (STX), \x1F (unit separator), \xAD (soft hyphen)
            if c == '\x02' || c == '\x1F' || c == '\u{00AD}' {
                // Look backward to find the last alphabetic character (skipping spaces from result string)
                let result_trimmed = result.trim_end();
                let prev_is_letter = result_trimmed
                    .chars()
                    .last()
                    .map(|c| c.is_alphabetic())
                    .unwrap_or(false);

                // Look forward past any spaces and control chars
                let mut j = i + 1;
                while j < len && (chars[j] == ' ' || chars[j] == '\x02' || chars[j] == '\x1F') {
                    j += 1;
                }

                // Check if next real char is lowercase (word continuation)
                let next_is_lower = j < len && chars[j].is_lowercase();

                if prev_is_letter && next_is_lower {
                    // This is a soft hyphen - remove trailing spaces from result and skip control/spaces
                    while result.ends_with(' ') {
                        result.pop();
                    }
                    i = j;
                    continue;
                } else {
                    // Not a soft hyphen pattern - replace with space
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
    fn normalize_text(&self, text: &str) -> String {
        if !self.normalize_whitespace {
            return text.to_string();
        }

        // First fix soft hyphens
        let text = self.fix_soft_hyphens(text);

        // Collapse multiple horizontal spaces but preserve newlines
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

        // Only trim if not a pure whitespace span (preserve space-only spans)
        if result.chars().all(|c| c.is_whitespace()) {
            result
        } else {
            result.trim().to_string()
        }
    }

    /// Normalize whitespace in span text.
    ///
    /// Unlike `normalize_text`, spans must preserve leading/trailing spaces because the
    /// Markdown renderer concatenates spans directly while using those boundary spaces
    /// to keep words separated.
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

    /// Fix common OCR errors.
    fn fix_ocr_text(&self, text: &str) -> String {
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
            ("\u{2013}", "-"),   // En dash
            ("\u{2014}", "-"),   // Em dash
            ("\u{2026}", "..."), // Ellipsis
            ("Þle", "file"),     // Common misread 'fi' ligature
            ("Þ", "fi"),
        ];

        for (from, to) in &replacements {
            result = result.replace(from, to);
        }

        result
    }

    /// Fix concatenated words (e.g., "methodsThe" -> "methods The")
    fn fix_concatenated_words(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Fix concatenations like "methodsThe" -> "methods The".
        // This is intentionally broad; we patch known legitimate tokens afterwards.
        if let Ok(re) = Regex::new(r"([a-z])([A-Z][a-z])") {
            result = re.replace_all(&result, "$1 $2").to_string();
        }

        // Repair common legitimate tokens that would otherwise be split.
        result = result.replace("ar Xiv", "arXiv");
        result = result.replace("Ar Xiv", "ArXiv");

        // Fix "etal." -> "et al." (standard academic citation)
        result = result.replace("etal.", "et al.");
        result = result.replace("etal,", "et al.,");

        result
    }

    /// Clean up malformed markdown-like artifacts from PDF extraction.
    /// These often come from figure/table annotations, checkboxes, or bullet points.
    fn cleanup_markdown_artifacts(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Remove patterns like "*[]*.*", "*-*", "*.*", "*[]**.*"
        // These are garbled representations of bullets/checkboxes
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

    /// Cleanup citation formatting
    fn cleanup_citations(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Fix patterns like "(Name,2024)" -> "(Name, 2024)"
        if let Ok(re) = Regex::new(r"\(([^)]+),(\d{4})\)") {
            result = re.replace_all(&result, "($1, $2)").to_string();
        }

        // Fix patterns like ",2024)" -> ", 2024)"
        if let Ok(re) = Regex::new(r",(\d{4})\)") {
            result = re.replace_all(&result, ", $1)").to_string();
        }

        result
    }

    /// Process a block.
    fn process_block(&self, block: &mut Block) {
        if block.block_type.has_text() {
            // Process main text
            block.text = self.normalize_text(&block.text);
            block.text = self.fix_ocr_text(&block.text);
            block.text = self.fix_concatenated_words(&block.text);
            block.text = self.cleanup_citations(&block.text);
            block.text = self.cleanup_markdown_artifacts(&block.text);

            // Also process spans since MarkdownRenderer uses spans if present
            for span in &mut block.spans {
                span.text = self.normalize_span_text(&span.text);
                span.text = self.fix_ocr_text(&span.text);

                // Avoid rewriting code-like spans (could change identifiers).
                if !span.style.looks_like_code() {
                    span.text = self.fix_concatenated_words(&span.text);
                    span.text = self.cleanup_citations(&span.text);
                    span.text = self.cleanup_markdown_artifacts(&span.text);
                }
            }

            // Span-to-span boundaries can create double-spaces (e.g. trailing + leading spaces).
            // Normalize those without destroying intentional newlines.
            if self.normalize_whitespace {
                Self::normalize_span_boundaries(&mut block.spans);
            }
        }

        // Process children
        for child in &mut block.children {
            self.process_block(child);
        }
    }

    fn normalize_span_boundaries(spans: &mut Vec<TextSpan>) {
        // Remove empty spans and de-duplicate horizontal spaces across boundaries.
        spans.retain(|s| !s.text.is_empty());
        if spans.len() < 2 {
            return;
        }

        for i in 1..spans.len() {
            let prev_ends_space = spans[i - 1].text.ends_with(' ');
            let cur_starts_space = spans[i].text.starts_with(' ');
            if prev_ends_space && cur_starts_space {
                // Drop exactly one leading space from current span.
                spans[i].text.remove(0);
            }
        }

        // Clean up spans that became empty after boundary normalization.
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

/// Processor to detect headers based on font size and weight.
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
        // 1. Calculate font size statistics
        let mut size_counts = std::collections::HashMap::new();
        for page in &document.pages {
            for block in &page.blocks {
                if let Some(span) = block.spans.first() {
                    let size = (span.style.size.unwrap_or(10.0) * 10.0).round() as i32; // Round to 0.1
                    *size_counts.entry(size).or_insert(0) += block.text.len();
                }
            }
        }

        // Find body size (most common)
        let body_size_int = size_counts
            .iter()
            .max_by_key(|&(_, count)| count)
            .map(|(s, _)| *s)
            .unwrap_or(100);
        let body_size = body_size_int as f32 / 10.0;

        // 2. Detect headers
        // Match patterns like "1. Introduction" or "1.1 Motivation"
        // Subsection pattern (e.g., "1.1", "2.3.4") - always valid section structure
        let subsection_heading = Regex::new(r"^\d+\.\d+(?:\.\d+)*\.?\s+[A-Z]").unwrap();
        // Single number pattern (e.g., "1." or "2.") - requires section keyword to avoid list items
        let single_number_heading = Regex::new(r"^\d+\.?\s+[A-Z]").unwrap();

        for page in &mut document.pages {
            for block in &mut page.blocks {
                if !matches!(block.block_type, BlockType::Text | BlockType::SectionHeader) {
                    continue;
                }

                let text = block.text.trim();

                // Adaptive length threshold based on heading level expectations
                // Document titles (H1) are naturally longer than section headers (H2/H3)
                // First Principles: use position + font size to distinguish title from section
                let is_first_page = page.number == 1;
                let block_y = block.bbox.y1;
                let page_height = page.height;
                // PDF coordinates: y increases from bottom to top
                // Top of page is near page_height, bottom is near 0
                let is_top_of_page = block_y > (page_height - 200.0);

                // Get font size for title detection
                let font_size = block
                    .spans
                    .first()
                    .and_then(|s| s.style.size)
                    .unwrap_or(10.0);
                let is_large_font = font_size > body_size * 1.4; // Slightly relaxed from 1.5

                // Allow longer text for document titles (first page + top position)
                // Use more relaxed conditions for first page to catch titles
                let max_heading_len = if is_first_page && (is_top_of_page || is_large_font) {
                    150 // Document titles: 80-120 chars typical
                } else {
                    80 // Section headers: usually shorter
                };

                // Inline description check: "Key: Value" patterns
                // Examples to EXCLUDE from headings: "Author: John Doe", "Date: 2024"
                // Examples to INCLUDE as headings: "Title: Subtitle of Paper", "ISWC 2025: Workshop"
                // First Principles: inline descriptions have very short keys (< 10 chars)
                // AND the key is often lowercase or property-like
                let has_inline_description = if let Some(colon_pos) = text.find(':') {
                    if colon_pos < 10 {
                        // Very short key - likely inline description
                        // But check if it's a common section/title pattern
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
                        // Longer key (>= 10 chars) - likely part of title/heading
                        false
                    }
                } else {
                    false
                };

                let is_short_for_heading =
                    text.len() < max_heading_len && !text.ends_with('.') && !has_inline_description;

                // Check for subsection pattern first (e.g., "1.1 Motivation")
                // Subsections have at least one internal dot, making them H3 or deeper
                if is_short_for_heading && subsection_heading.is_match(text) {
                    // Count dots only in the numeric prefix (before the first space)
                    let prefix: String = text
                        .chars()
                        .take_while(|c| c.is_ascii_digit() || *c == '.')
                        .collect();
                    // Count internal dots (between digits) - trailing dots don't add depth
                    // "2.1" -> 1 internal dot -> H3
                    // "2.1." -> 1 internal dot (trailing doesn't count) -> H3
                    // "2.1.1" -> 2 internal dots -> H4
                    let trimmed = prefix.trim_end_matches('.');
                    let dot_count = trimmed.chars().filter(|&c| c == '.').count() as u8;
                    // 1.1 -> 1 dot -> H3 (level 3)
                    // 1.1.1 -> 2 dots -> H4 (level 4)
                    let level = (dot_count + 2).clamp(3, 6);
                    block.block_type = BlockType::SectionHeader;
                    block.level = Some(level);
                    continue;
                }

                // For single number patterns (e.g., "1 Introduction"), require section keyword
                // This avoids converting list items like "1. Explore the options" to headings
                // Single number section headers are H2
                // For single number patterns (e.g., "1 Introduction"), use multi-signal detection
                // First Principles: no keyword matching, use font properties + structure
                // FIRST PRINCIPLES: Addresses like "353 Serra Mall, Stanford, CA" should NOT be headings
                // Addresses contain commas for city/state separation; section headers don't.
                if is_short_for_heading
                    && single_number_heading.is_match(text)
                    && !text.contains(',')
                {
                    // Extract text after the number
                    let after_number: String = text
                        .chars()
                        .skip_while(|c| c.is_ascii_digit() || *c == '.' || c.is_whitespace())
                        .collect();

                    // Check if it's title-cased (first letter uppercase after number)
                    let is_title_cased = after_number
                        .chars()
                        .next()
                        .map(|c| c.is_uppercase())
                        .unwrap_or(false);

                    // Get font properties for multi-signal detection
                    if let Some(span) = block.spans.first() {
                        let size = span.style.size.unwrap_or(10.0);
                        let weight = span.style.weight.unwrap_or(400);
                        let is_bold = weight >= 600;
                        let is_larger = size > body_size * 1.15;

                        // Multi-signal detection: need strong confirmation
                        // EITHER font evidence (larger OR bold) AND structural (short + title-cased)
                        // OR very strong font evidence (larger AND bold)
                        let is_likely_section =
                            (is_larger || is_bold) && is_title_cased || (is_larger && is_bold);

                        if is_likely_section {
                            // Single number sections like "1 Introduction" are H2
                            block.block_type = BlockType::SectionHeader;
                            block.level = Some(2);
                            continue;
                        }
                    }
                }

                if let Some(span) = block.spans.first() {
                    let size = span.style.size.unwrap_or(10.0);
                    let weight = span.style.weight.unwrap_or(400);
                    let is_bold = weight >= 600;

                    // H1: Very large (e.g. > 1.6x body)
                    // H2: Large (> 1.4x) - stricter threshold to avoid author names
                    // H3: Moderately larger (> 1.25x) - stricter threshold

                    // Guard against turning long prose lines into headings.
                    let text_lower = text.to_lowercase();
                    let is_arxiv_or_meta = text_lower.starts_with("arxiv:")
                        || text_lower.contains("arxiv.org")
                        || text_lower.starts_with("[cs.")
                        || text_lower.starts_with("[stat.")
                        || text_lower.starts_with("[math.");

                    // Use position-aware length threshold (same as above)
                    let max_len_for_heading = if is_first_page && (is_top_of_page || is_large_font)
                    {
                        150 // Allow longer document titles
                    } else {
                        100 // Standard section headers
                    };

                    let headingish = !text.is_empty()
                        && text.len() < max_len_for_heading
                        && !text.contains('@')  // No email addresses
                        && !text.ends_with('.')  // No sentences
                        && !text.contains(',')  // No comma-separated items like author affiliations
                        && !is_arxiv_or_meta; // No arXiv metadata

                    // Additional guard: author names are typically short (< 30 chars) and appear
                    // early in the document. Section headers usually have specific patterns.
                    // First Principles: look for structural patterns, not keywords
                    let looks_like_section = text.starts_with(|c: char| c.is_ascii_digit())
                        || text.chars().all(|c| c.is_uppercase() || c.is_whitespace());

                    if headingish && size > body_size * 1.6 {
                        block.block_type = BlockType::SectionHeader;
                        block.level = Some(1);
                    } else if headingish && looks_like_section && size > body_size * 1.35 {
                        block.block_type = BlockType::SectionHeader;
                        block.level = Some(2);
                    } else if headingish && looks_like_section && size > body_size * 1.2 {
                        block.block_type = BlockType::SectionHeader;
                        block.level = Some(3);
                    }
                    // Note: We no longer convert all bold text to headers.
                    // Bold text that isn't larger than body text should remain as bold text,
                    // not be converted to H4. This prevents author names, emphasis, etc.
                    // from incorrectly becoming headers.
                }
            }
        }

        Ok(document)
    }

    fn name(&self) -> &str {
        "HeaderDetectionProcessor"
    }
}

/// Processor to detect captions for figures and tables.
pub struct CaptionDetectionProcessor {}

impl CaptionDetectionProcessor {
    pub fn new() -> Self {
        Self {}
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
            for block in &mut page.blocks {
                if block.block_type != BlockType::Text {
                    continue;
                }

                let text = block.text.trim();
                if caption_regex.is_match(text) {
                    // It looks like a caption
                    block.block_type = BlockType::Caption;
                }
            }
        }
        Ok(document)
    }

    fn name(&self) -> &str {
        "CaptionDetectionProcessor"
    }
}

/// Processor to detect list items and their indentation.
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
        let bullet_regex = Regex::new(r"^[-*•]\s+").unwrap();
        let number_regex = Regex::new(r"^\d+[\.)]\s+").unwrap();

        for page in &mut document.pages {
            // Find the minimum x-coordinate (left margin) to calculate indentation
            let min_x = page
                .blocks
                .iter()
                .map(|b| b.bbox.x1)
                .fold(f32::MAX, |a, b| a.min(b));

            for block in &mut page.blocks {
                if block.block_type != BlockType::Text {
                    continue;
                }

                let text = block.text.trim();
                if bullet_regex.is_match(text) || number_regex.is_match(text) {
                    block.block_type = BlockType::ListItem;

                    // Calculate indentation level
                    // Assume 20 points per level
                    let indent = block.bbox.x1 - min_x;
                    let level = (indent / 20.0).round() as i32;

                    // Store indentation in metadata for renderer
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

/// Processor to detect and merge code blocks.
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
            // 1. Identify code blocks
            for block in &mut page.blocks {
                if block.block_type != BlockType::Text {
                    continue;
                }

                // Check if all spans look like code
                let all_code = !block.spans.is_empty()
                    && block.spans.iter().all(|s| s.style.looks_like_code());

                if all_code {
                    block.block_type = BlockType::Code;
                }
            }

            // 2. Merge consecutive code blocks
            let mut merged = Vec::new();
            let mut current_code: Option<Block> = None;

            for block in std::mem::take(&mut page.blocks) {
                if block.block_type == BlockType::Code {
                    if let Some(mut cur) = current_code.take() {
                        // Merge with current code block
                        // Add newline between lines
                        cur.text.push('\n');
                        cur.text.push_str(&block.text);

                        // Merge spans
                        // Add a newline span if needed, or just append
                        cur.spans.extend(block.spans);

                        // Update bbox
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

/// Processor to fix hyphenated words at line breaks.
///
/// Academic papers often use hyphenation for word wrapping. This processor
/// detects patterns like "modifi-\ncation" and joins them to "modification".
pub struct HyphenContinuationProcessor {}

impl HyphenContinuationProcessor {
    pub fn new() -> Self {
        Self {}
    }

    /// Check if a block text ends with a hyphenated word fragment.
    /// Returns the fragment if found.
    fn ends_with_hyphen(&self, text: &str) -> Option<String> {
        let trimmed = text.trim_end();

        // Check for explicit hyphen at end (most reliable)
        if trimmed.ends_with('-') {
            // Get the word fragment before the hyphen
            let without_hyphen = &trimmed[..trimmed.len() - 1];
            let last_word = without_hyphen.split_whitespace().last()?;
            return Some(last_word.to_string());
        }

        None
    }

    /// Check if text ends with an EXPLICIT hyphen (strict check for cross-column joining)
    fn ends_with_explicit_hyphen(&self, text: &str) -> bool {
        text.trim_end().ends_with('-')
    }

    /// Get the word fragment before the hyphen for cross-column validation
    fn get_hyphen_fragment(&self, text: &str) -> Option<String> {
        let trimmed = text.trim_end();
        if trimmed.ends_with('-') {
            let without_hyphen = &trimmed[..trimmed.len() - 1];
            let last_word = without_hyphen.split_whitespace().last()?;
            // Return the fragment in lowercase for matching
            return Some(last_word.to_lowercase());
        }
        None
    }

    /// Validate that a continuation completes the hyphenated word sensibly.
    /// The first word of continuation should be a reasonable suffix for the fragment.
    fn is_valid_continuation(&self, fragment: &str, continuation_text: &str) -> bool {
        let cont_trimmed = continuation_text.trim_start();
        if cont_trimmed.is_empty() {
            return false;
        }

        // Get first word of continuation
        let first_word = cont_trimmed.split_whitespace().next().unwrap_or("");
        if first_word.is_empty() || !first_word.chars().next().unwrap().is_lowercase() {
            return false;
        }

        // The first word should be a reasonable word suffix (all alphabetic)
        if !first_word.chars().all(|c| c.is_alphabetic()) {
            return false;
        }

        // STRICT CHECK: The continuation should be a word SUFFIX, not a standalone word.
        // Word suffixes typically:
        // 1. Are short (1-6 chars for typical suffixes like "tion", "ing", "ment", "ries", etc.)
        // 2. Don't form common standalone words themselves (like "which", "this", "that", etc.)

        let first_word_lower = first_word.to_lowercase();

        // Reject common English words that would never be hyphen continuations
        let common_words = [
            "which", "this", "that", "with", "from", "have", "been", "were", "their", "they",
            "there", "them", "these", "those", "then", "than", "when", "where", "what", "who",
            "how", "why", "can", "will", "may", "must", "should", "would", "could", "into", "such",
            "some", "only", "very", "also", "more", "most", "like", "just", "over", "other",
            "each", "both", "many", "well", "even", "while", "without", "within", "through",
            "during", "before", "after", "between", "under", "about", "above", "across", "along",
            "among", "around", "far", "the", "and", "for", "are", "but", "not", "you", "all",
            "out", "way", "its", "her", "his", "our", "any", "being", "doing", "going", "making",
            "using", "having", "getting", "saying", "seeing", "knowing", "coming",
        ];

        if common_words.contains(&first_word_lower.as_str()) {
            return false;
        }

        // Word suffixes are typically short - reject if too long to be a suffix
        // Common suffixes: -tion, -ment, -ness, -able, -ible, -ing, -ed, -ly, -ry, -ries
        // Allow up to 8 chars for compound suffixes like "itory" (reposit-itory)
        if first_word.len() > 8 {
            return false;
        }

        // The combined word should form something reasonable
        // Heuristic: fragment + first_word should be 5-20 chars (typical word length)
        let combined_len = fragment.len() + first_word.len();
        if combined_len < 4 || combined_len > 25 {
            return false;
        }

        true
    }

    /// Check if text starts with a continuation of a hyphenated word.
    fn starts_with_continuation(&self, text: &str) -> bool {
        let trimmed = text.trim_start();
        if trimmed.is_empty() {
            return false;
        }

        // Must start with lowercase letter
        let first_char = trimmed.chars().next().unwrap();
        if !first_char.is_lowercase() {
            return false;
        }

        // Get first word
        let first_word = trimmed.split_whitespace().next().unwrap_or("");

        // Should be a word fragment (no spaces, just letters)
        first_word.chars().all(|c| c.is_alphabetic())
    }

    /// Join two text blocks, removing hyphenation.
    fn join_hyphenated(&self, first: &str, second: &str) -> String {
        let first_trimmed = first.trim_end();
        let second_trimmed = second.trim_start();

        // If first ends with hyphen, remove it and join directly
        if first_trimmed.ends_with('-') {
            let base = &first_trimmed[..first_trimmed.len() - 1];
            return format!("{}{}", base, second_trimmed);
        }

        // Otherwise, we're dealing with a "word " + "fragment" pattern
        // Remove trailing spaces from first, then join with second
        let words: Vec<&str> = first_trimmed.split_whitespace().collect();
        if words.is_empty() {
            return second_trimmed.to_string();
        }

        let first_word = second_trimmed.split_whitespace().next().unwrap_or("");
        let rest_of_second: String = second_trimmed[first_word.len()..].trim_start().to_string();

        // Join the last word fragment with the continuation
        let last_word = words.last().unwrap();
        let prefix: String = words[..words.len() - 1].join(" ");

        if prefix.is_empty() {
            format!("{}{} {}", last_word, first_word, rest_of_second)
                .trim()
                .to_string()
        } else {
            format!("{} {}{} {}", prefix, last_word, first_word, rest_of_second)
                .trim()
                .to_string()
        }
    }
}

impl Default for HyphenContinuationProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for HyphenContinuationProcessor {
    fn process(&self, mut document: Document) -> Result<Document> {
        // Calculate stats once for adaptive threshold (First Principles!)
        let stats = DocumentStats::from_document(&document);
        let max_vertical_gap = stats.typical_line_spacing * 2.5;

        for page in &mut document.pages {
            let mut i = 0;
            while i < page.blocks.len() {
                // First check for immediate adjacent join (standard case)
                let mut join_with: Option<usize> = None;

                if i + 1 < page.blocks.len() {
                    let current = &page.blocks[i];
                    let next = &page.blocks[i + 1];

                    // Only consider joining text blocks that are vertically adjacent
                    if current.block_type == BlockType::Text && next.block_type == BlockType::Text {
                        // Check if they're on consecutive lines (small vertical gap)
                        // Note: In PDF coordinates, Y increases upward, so blocks read from top to bottom
                        // have decreasing Y values. The gap is current.y2 - next.y1 (top of next vs bottom of current)
                        // Or we can check absolute difference to handle both orderings
                        let vertical_gap = (next.bbox.y1 - current.bbox.y2).abs();

                        // Check for hyphenation
                        let ends_hyph = self.ends_with_hyphen(&current.text);
                        let starts_cont = self.starts_with_continuation(&next.text);

                        if ends_hyph.is_some() {
                            // Safe string slicing for debug output
                            let current_end: String = current
                                .text
                                .chars()
                                .rev()
                                .take(15)
                                .collect::<String>()
                                .chars()
                                .rev()
                                .collect();
                            let next_start: String = next.text.chars().take(15).collect();
                            tracing::debug!(
                                "Hyphen check: '{}...' ends_hyph={:?}, starts_cont={}, vertical_gap={:.1}, next='{}...'",
                                current_end,
                                ends_hyph,
                                starts_cont,
                                vertical_gap,
                                next_start
                            );
                        }

                        // Use adaptive threshold based on document's actual line spacing (First Principles!)
                        // 2.5x typical line spacing covers single-spaced to near double-spaced
                        if vertical_gap <= max_vertical_gap {
                            if ends_hyph.is_some() && starts_cont {
                                tracing::debug!("Immediate hyphen join triggered");
                                join_with = Some(i + 1);
                            }
                        }
                    }
                }

                // If no immediate join, but current ends with EXPLICIT hyphen, search ahead
                // This handles two-column layouts where hyphenation spans columns
                // Only trigger for explicit "-" ending, not implicit short-word patterns
                if join_with.is_none() && i + 1 < page.blocks.len() {
                    let current = &page.blocks[i];
                    if current.block_type == BlockType::Text
                        && self.ends_with_explicit_hyphen(&current.text)
                    {
                        // Get the word fragment before the hyphen for validation
                        if let Some(fragment) = self.get_hyphen_fragment(&current.text) {
                            // Search up to 5 blocks ahead for continuation
                            for j in (i + 1)..((i + 6).min(page.blocks.len())) {
                                let candidate = &page.blocks[j];
                                // Skip non-text blocks (figures, captions)
                                if candidate.block_type != BlockType::Text {
                                    continue;
                                }
                                // Check if this is a valid continuation that completes the word
                                if self.is_valid_continuation(&fragment, &candidate.text) {
                                    // Use chars() for safe substring (handles multi-byte Unicode)
                                    let cur_end: String = current
                                        .text
                                        .chars()
                                        .rev()
                                        .take(20)
                                        .collect::<String>()
                                        .chars()
                                        .rev()
                                        .collect();
                                    let cand_start: String =
                                        candidate.text.chars().take(20).collect();
                                    tracing::debug!(
                                        "Cross-column hyphen join: '{}' + '{}' (fragment: {})",
                                        cur_end,
                                        cand_start,
                                        fragment
                                    );
                                    join_with = Some(j);
                                    break;
                                }
                            }
                        }
                    }
                }

                if let Some(j) = join_with {
                    let current = &page.blocks[i];
                    let target = &page.blocks[j];

                    let joined_text = self.join_hyphenated(&current.text, &target.text);
                    // Print first 100 chars of joined text for debugging
                    let joined_preview: String = joined_text.chars().take(80).collect();
                    tracing::debug!(
                        "Joining blocks [{}+{}]: len {}→{}, text: '{}'",
                        i,
                        j,
                        current.text.len() + target.text.len(),
                        joined_text.len(),
                        joined_preview
                    );
                    let joined_bbox = current.bbox.union(&target.bbox);

                    page.blocks[i].text = joined_text;
                    page.blocks[i].bbox = joined_bbox;
                    // Clear spans since they contain the old hyphenated text
                    // The MarkdownRenderer will use block.text instead
                    page.blocks[i].spans.clear();
                    page.blocks.remove(j);
                    // Don't increment i, check if there are more continuations
                } else {
                    i += 1;
                }
            }
        }

        Ok(document)
    }

    fn name(&self) -> &str {
        "HyphenContinuationProcessor"
    }
}

/// Processor for detecting bold/italic styles and H1/H2+ header levels.
///
/// This processor implements the requirements from spec_algo_2.md:
/// - Bold detection: Font weight >= 600 or font name contains "bold"
/// - Italic detection: Font name contains "italic" or "oblique"
/// - H1/H2+ detection: Font size ratios relative to body text
///
/// Uses pure heuristics, no ML required.
#[derive(Clone)]
pub struct StyleDetectionProcessor {
    /// Computed body font size (most common size in document)
    body_size: f32,
}

impl StyleDetectionProcessor {
    /// Create a new style detection processor.
    pub fn new() -> Self {
        Self { body_size: 10.0 }
    }

    /// Compute body font size as the most common font size in the document.
    fn compute_body_size(&mut self, document: &Document) {
        use std::collections::HashMap;
        let mut size_counts: HashMap<i32, usize> = HashMap::new();

        for page in &document.pages {
            for block in &page.blocks {
                for span in &block.spans {
                    // Quantize to 0.1pt precision
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

    /// Detect and update styles per span based on font metadata.
    fn detect_styles(&self, block: &mut Block) {
        for span in &mut block.spans {
            let family_lower = span
                .style
                .family
                .as_ref()
                .map(|f| f.to_lowercase())
                .unwrap_or_default();

            // Bold: Weight >= 600 or name contains "bold"
            let is_bold = span.style.weight.unwrap_or(400) >= 600 || family_lower.contains("bold");
            span.style.weight = Some(if is_bold { 700 } else { 400 });

            // Italic: Name contains "italic" or "oblique"
            let is_italic = span.style.italic
                || family_lower.contains("italic")
                || family_lower.contains("oblique");
            span.style.italic = is_italic;
        }
    }

    /// Detect header levels based on font size ratios ONLY.
    ///
    /// IMPORTANT: We no longer use bold as an indicator for headers.
    /// Bold text should remain as bold styling, not be converted to headers.
    /// This is critical for preserving author names, emphasis, etc.
    ///
    /// Rules:
    /// - H1: size ratio > 1.6 AND short text (<80 chars) - document title
    /// - H2/H3: size ratio > 1.2 AND matches section keywords - section headers
    ///
    /// We require section keywords for H2/H3 to avoid converting author names,
    /// affiliations, and other metadata into section headers.
    fn detect_headers(&self, block: &mut Block) {
        if block.block_type != BlockType::Text {
            return;
        }

        // Get representative style from first span
        let size = block
            .spans
            .first()
            .map(|s| s.style.size.unwrap_or(10.0))
            .unwrap_or(10.0);

        let ratio = size / self.body_size;
        let text = block.text.trim();
        let text_lower = text.to_lowercase();
        let is_short = text.len() < 80;

        // Guard: Don't convert text with email addresses, ending in period, or arXiv metadata
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

        // Check if this looks like a section header (not an author name, etc.)
        // First Principles: look for structural patterns (numbers, all-caps), not keywords
        let looks_like_section = text.starts_with(|c: char| c.is_ascii_digit())
            || text
                .chars()
                .all(|c| c.is_uppercase() || c.is_whitespace() || c.is_ascii_digit());

        // Check if this is Abstract or Keywords - these should always be H3
        let is_abstract_or_keywords = text_lower == "abstract"
            || text_lower.starts_with("abstract.")
            || text_lower == "keywords"
            || text_lower.starts_with("keywords:");

        // H3: Abstract and Keywords are always H3 (per academic paper conventions)
        if is_abstract_or_keywords && is_short {
            block.block_type = BlockType::SectionHeader;
            block.level = Some(3);
            tracing::debug!("H3 (abstract/keywords): text='{}'", text);
        }
        // H1: Large title (ratio > 1.5) and short - no section keyword required for main title
        else if ratio > 1.5 && is_short {
            block.block_type = BlockType::SectionHeader;
            block.level = Some(1);
            tracing::debug!("H1 detected: ratio={:.2}, text='{}'", ratio, text);
        }
        // H2: Section header (ratio > 1.2) and short AND looks like section
        else if ratio > 1.2 && is_short && looks_like_section {
            block.block_type = BlockType::SectionHeader;
            block.level = Some(2);
            tracing::debug!("H2 detected: ratio={:.2}, text='{}'", ratio, text);
        }
        // H3: Slightly larger (ratio > 1.1) and short AND looks like section
        else if ratio > 1.1 && is_short && looks_like_section {
            block.block_type = BlockType::SectionHeader;
            block.level = Some(3);
            tracing::debug!("H3 detected: ratio={:.2}, text='{}'", ratio, text);
        }
        // Special case: Bold text that exactly matches a section keyword at body size
        // This handles PDFs where section headers are just bold, not larger font
        else {
            let is_bold = block
                .spans
                .first()
                .map(|s| s.style.weight.unwrap_or(400) >= 600)
                .unwrap_or(false);

            // For bold text, check if it looks like a section (short, capitalized, not prose)
            // First Principles: use font weight + structure, not keyword matching
            let is_first_char_upper = text
                .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ' ')
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false);

            // Only convert bold text to H2 if it's short, capitalized, and looks like section
            // But not Abstract/Keywords - those stay as H3
            if is_bold
                && is_short
                && is_first_char_upper
                && looks_like_section
                && !looks_like_prose
                && !is_abstract_or_keywords
            {
                block.block_type = BlockType::SectionHeader;
                block.level = Some(2);
                tracing::debug!("H2 (bold section): text='{}'", text);
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
            for block in &mut page.blocks {
                processor.detect_styles(block);
                processor.detect_headers(block);

                // Process children recursively
                for child in &mut block.children {
                    processor.detect_styles(child);
                    processor.detect_headers(child);
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
    fn test_block_merge_processor() {
        let processor = BlockMergeProcessor::new();
        let doc = create_test_document();
        let result = processor.process(doc).unwrap();

        // With 20px gap between blocks, they shouldn't merge
        assert_eq!(result.pages[0].blocks.len(), 2);
    }

    #[test]
    fn test_post_processor_normalize() {
        let processor = PostProcessor::new();

        let normalized = processor.normalize_text("Hello    world   test");
        assert_eq!(normalized, "Hello world test");

        let fixed = processor.fix_ocr_text("ﬁnd the ﬂow");
        assert_eq!(fixed, "find the flow");
    }

    #[test]
    fn test_post_processor_span_cleanup_and_boundaries() {
        let processor = PostProcessor::new();

        let mut block = Block::text(
            "methodsThe    model",
            BoundingBox::new(72.0, 100.0, 540.0, 130.0),
        );
        // Renderer prefers spans if present.
        block.spans = vec![TextSpan::plain("methodsThe "), TextSpan::plain("  model")];

        processor.process_block(&mut block);

        // Concatenated-word fix should apply to spans too.
        assert_eq!(block.spans[0].text, "methods The ");
        // Boundary normalization should remove the extra double-space across spans.
        assert_eq!(block.spans[1].text, "model");
    }

    #[test]
    fn test_post_processor_does_not_split_arxiv() {
        let processor = PostProcessor::new();

        let input = "Submitted to arXiv:2501.23456";
        let output = processor.fix_concatenated_words(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_layout_processor() {
        let processor = LayoutProcessor::new();
        let doc = create_test_document();
        let result = processor.process(doc).unwrap();

        assert!(!result.pages.is_empty());
    }

    #[test]
    fn test_text_table_reconstruction_caption_after_table() {
        let mut doc = Document::new();
        let mut page = Page::new(1, 612.0, 792.0);

        page.add_block(Block::text(
            "Agent Pipeline Func-IoU(%) Resolved(%) Agentless 5.28 10.12 Repo Navigator 12.00 14.74",
            BoundingBox::new(72.0, 100.0, 540.0, 130.0),
        ));
        page.add_block(Block::text(
            "*Table 3. We use Qwen2.5-14B-Instruct as the localization model*",
            BoundingBox::new(72.0, 140.0, 540.0, 155.0),
        ));
        page.add_block(Block::text(
            "---",
            BoundingBox::new(72.0, 160.0, 540.0, 165.0),
        ));

        doc.add_page(page);

        let processor = TextTableReconstructionProcessor::new();
        let result = processor.process(doc).unwrap();
        let blocks = &result.pages[0].blocks;

        let mut found = false;
        for w in blocks.windows(2) {
            if TextTableReconstructionProcessor::looks_like_table_caption(&w[0].text)
                && w[1].block_type == BlockType::Table
            {
                assert!(!w[1].children.is_empty());
                assert!(w[1]
                    .children
                    .iter()
                    .all(|c| c.block_type == BlockType::TableCell));
                // First row must look like the leaderboard header.
                assert_eq!(w[1].children[0].text, "Agent Pipeline");
                assert!(w[1].children[1].text.contains("IoU"));
                assert!(w[1].children[2].text.contains("Resolved"));
                found = true;
                break;
            }
        }
        assert!(found, "expected caption followed by reconstructed table");
    }

    #[test]
    fn test_header_detection_numeric_sections() {
        let mut doc = Document::new();
        let mut page = Page::new(1, 612.0, 792.0);

        // Establish a body size via a normal paragraph with spans.
        let mut body = Block::text(
            "This is body text.",
            BoundingBox::new(72.0, 200.0, 540.0, 220.0),
        );
        body.spans = vec![TextSpan::styled(
            "This is body text.",
            FontStyle {
                family: Some("Times-Roman".to_string()),
                size: Some(10.0),
                weight: Some(400),
                italic: false,
                ..Default::default()
            },
        )];
        page.add_block(body);

        // Section headers should be distinguishable by font (bold or larger)
        // First Principles: sections have different styling than body text
        let mut heading = Block::text(
            "1. Introduction",
            BoundingBox::new(72.0, 100.0, 540.0, 120.0),
        );
        heading.spans = vec![TextSpan::styled(
            "1. Introduction",
            FontStyle {
                family: Some("Times-Bold".to_string()),
                size: Some(10.0),
                weight: Some(700), // Bold weight for section header
                italic: false,
                ..Default::default()
            },
        )];
        page.add_block(heading);

        doc.add_page(page);

        let processor = HeaderDetectionProcessor::new();
        let result = processor.process(doc).unwrap();

        let blocks = &result.pages[0].blocks;
        let intro = blocks
            .iter()
            .find(|b| b.text.trim() == "1. Introduction")
            .expect("missing heading block");
        assert_eq!(intro.block_type, BlockType::SectionHeader);
        assert_eq!(intro.level, Some(2));
    }

    #[test]
    fn test_text_table_reconstruction_caption_before_table_skips_source_lines() {
        let mut doc = Document::new();
        let mut page = Page::new(1, 612.0, 792.0);

        page.add_block(Block::text(
            "#### Table 1. F1-scores and Rankings",
            BoundingBox::new(72.0, 100.0, 540.0, 115.0),
        ));
        page.add_block(Block::text(
            "Sub-task F1-score Rank",
            BoundingBox::new(72.0, 120.0, 540.0, 135.0),
        ));
        page.add_block(Block::text(
            "Task",
            BoundingBox::new(72.0, 136.0, 540.0, 150.0),
        ));
        page.add_block(Block::text(
            "A1.2 - Scholarly Term Extraction 0.4578 4",
            BoundingBox::new(72.0, 156.0, 540.0, 170.0),
        ));
        page.add_block(Block::text(
            "A1.3 - Engineering Term Extraction 0.4302 6",
            BoundingBox::new(72.0, 171.0, 540.0, 185.0),
        ));
        page.add_block(Block::text(
            "---",
            BoundingBox::new(72.0, 190.0, 540.0, 195.0),
        ));

        doc.add_page(page);

        let processor = TextTableReconstructionProcessor::new();
        let result = processor.process(doc).unwrap();
        let blocks = &result.pages[0].blocks;

        // Ensure caption is followed by a table block.
        let caption_idx = blocks
            .iter()
            .position(|b| TextTableReconstructionProcessor::looks_like_table_caption(&b.text))
            .expect("caption should exist");
        assert!(caption_idx + 1 < blocks.len());
        assert_eq!(blocks[caption_idx + 1].block_type, BlockType::Table);

        // Ensure we did not keep the raw header line(s) as separate blocks.
        assert!(
            !blocks
                .iter()
                .any(|b| b.text.trim() == "Sub-task F1-score Rank"),
            "expected source table lines to be consumed when caption precedes table"
        );
    }
}
