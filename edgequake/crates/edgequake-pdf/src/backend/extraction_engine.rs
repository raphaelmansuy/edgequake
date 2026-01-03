//! PDF extraction engine using lopdf with proper character encoding.
//!
//! This module provides production-quality PDF text extraction with:
//! - Proper font encoding support (WinAnsi, ToUnicode CMap, etc.)
//! - Font size tracking for header detection
//! - Bold/italic detection from font names
//! - Section pattern detection
//! - Running header removal
//! - Two-column layout detection
//! - Table detection using lattice analysis

#![cfg(feature = "lopdf")]

use async_trait::async_trait;
use std::collections::BTreeMap;
use tracing::{debug, info, warn};

use super::PdfBackend;
use crate::config::PdfConfig;
use crate::error::PdfError;
use crate::extractor::PdfInfo;
use crate::schema::{
    Block, BlockId, BlockType, BoundingBox, Document, ExtractionMethod, Page, PageStats,
    Point,
};
use crate::{DocumentMetadata, Result};

use lopdf::{Document as LopdfDocument, Object, ObjectId};

use super::column_detection::ColumnDetector;
use super::content_parser::ContentParser;
use super::elements::TextElement;
use super::font_handling::FontInfo;
use super::lattice::LatticeEngine;
use super::text_grouping::{MergedLine, TextGrouper};

/// PDF extraction engine with proper encoding support.
///
/// Uses lopdf for parsing and provides:
/// - Column detection (single/multi-column layouts)
/// - Text grouping into logical lines and blocks
/// - Font style detection (bold/italic)
/// - Table detection via lattice analysis
pub struct ExtractionEngine {
    config: PdfConfig,
    lattice_engine: LatticeEngine,
    text_grouper: TextGrouper,
    column_detector: ColumnDetector,
    content_parser: ContentParser,
}

impl ExtractionEngine {
    pub fn new() -> Self {
        Self::with_config(PdfConfig::default())
    }

    pub fn with_config(config: PdfConfig) -> Self {
        Self {
            config,
            lattice_engine: LatticeEngine::new(),
            text_grouper: TextGrouper::new(),
            column_detector: ColumnDetector::new(),
            content_parser: ContentParser::new(),
        }
    }

    /// Get fonts from page resources
    fn get_page_fonts(
        &self,
        doc: &LopdfDocument,
        page_id: ObjectId,
    ) -> Result<BTreeMap<Vec<u8>, FontInfo>> {
        let mut fonts = BTreeMap::new();

        let page_dict = doc
            .get_dictionary(page_id)
            .map_err(|e| PdfError::PdfParse(format!("Failed to get page: {}", e)))?;

        // Get Resources
        let resources = match page_dict.get(b"Resources") {
            Ok(Object::Reference(id)) => doc.get_dictionary(*id).ok(),
            Ok(Object::Dictionary(d)) => Some(d),
            _ => None,
        };

        if let Some(resources) = resources {
            // Get Font dictionary
            let font_dict = match resources.get(b"Font") {
                Ok(Object::Reference(id)) => doc.get_dictionary(*id).ok(),
                Ok(Object::Dictionary(d)) => Some(d),
                _ => None,
            };

            if let Some(font_dict) = font_dict {
                for (name, value) in font_dict.iter() {
                    let font = match value {
                        Object::Reference(id) => doc.get_dictionary(*id).ok(),
                        Object::Dictionary(d) => Some(d),
                        _ => None,
                    };

                    if let Some(font) = font {
                        fonts.insert(name.clone(), FontInfo::from_dict(doc, font));
                    }
                }
            }
        }

        Ok(fonts)
    }

    /// Get page content stream
    fn get_page_content(&self, doc: &LopdfDocument, page_id: ObjectId) -> Result<Vec<u8>> {
        let page_dict = doc
            .get_dictionary(page_id)
            .map_err(|e| PdfError::PdfParse(format!("Failed to get page: {}", e)))?;

        let contents = page_dict
            .get(b"Contents")
            .map_err(|_| PdfError::PdfParse("No Contents in page".to_string()))?;

        match contents {
            Object::Reference(id) => {
                let stream = doc
                    .get_object(*id)
                    .map_err(|e| PdfError::PdfParse(format!("Failed to get content: {}", e)))?;
                if let Object::Stream(s) = stream {
                    s.decompressed_content()
                        .map_err(|e| PdfError::PdfParse(format!("Failed to decompress: {}", e)))
                } else {
                    Err(PdfError::PdfParse("Content is not a stream".to_string()))
                }
            }
            Object::Array(arr) => {
                let mut content = Vec::new();
                for obj in arr {
                    if let Object::Reference(id) = obj {
                        if let Ok(Object::Stream(s)) = doc.get_object(*id) {
                            if let Ok(bytes) = s.decompressed_content() {
                                content.extend(bytes);
                                content.push(b'\n');
                            }
                        }
                    }
                }
                Ok(content)
            }
            Object::Stream(s) => s
                .decompressed_content()
                .map_err(|e| PdfError::PdfParse(format!("Failed to decompress: {}", e))),
            _ => Err(PdfError::PdfParse("Invalid Contents type".to_string())),
        }
    }

    /// Deduplicate text elements that are identical and at the same position.
    ///
    /// **WHY deduplication is critical:**
    /// - PDF files often contain invisible text layers (e.g., OCR layer + visible layer)
    /// - Without dedup, we get doubled text like "TheThe ProblemProblem"
    /// - Position tolerance of 2pt handles slight rendering variations
    /// - Keep element with more text if one is prefix of another (OCR sometimes partial)
    fn deduplicate_elements(&self, elements: Vec<TextElement>) -> Vec<TextElement> {
        if elements.is_empty() {
            return Vec::new();
        }

        // Sort by Y (descending), then X (ascending)
        let mut sorted = elements;
        sorted.sort_by(|a, b| {
            b.y.partial_cmp(&a.y)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
        });

        let mut unique = Vec::new();
        unique.push(sorted[0].clone());

        for elem in sorted.into_iter().skip(1) {
            let prev = unique.last().unwrap();

            // Check for overlap
            let same_pos = (elem.x - prev.x).abs() < 2.0 && (elem.y - prev.y).abs() < 2.0;

            if same_pos {
                // If text is identical, skip
                if elem.text == prev.text {
                    continue;
                }
                // If one contains the other, keep the longer one
                if elem.text.contains(&prev.text) {
                    unique.pop(); // Remove shorter prev
                    unique.push(elem);
                    continue;
                }
                if prev.text.contains(&elem.text) {
                    continue; // Skip shorter elem
                }
            }

            unique.push(elem);
        }

        unique
    }

    /// Merge text elements that are physically adjacent on the same line.
    ///
    /// **WHY merging is essential:**
    /// - PDF operators (Tj, TJ) emit individual words or even characters
    /// - "Hello World" might come as ["Hello", " ", "World"] at different positions
    /// - Merge threshold uses font size to estimate character width
    /// - Result: contiguous text runs for proper word/sentence extraction
    fn merge_text_elements(&self, elements: Vec<TextElement>) -> Vec<TextElement> {
        if elements.is_empty() {
            return Vec::new();
        }

        // Sort by Y (descending), then X (ascending)
        let mut sorted = elements;
        sorted.sort_by(|a, b| {
            b.y.partial_cmp(&a.y)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
        });

        let mut merged = Vec::new();
        let mut current = sorted[0].clone();

        for next in sorted.into_iter().skip(1) {
            // Check if on same line
            if (next.y - current.y).abs() < 2.0 {
                // Check horizontal distance
                // Use font size from current element to estimate char width
                let char_width = if current.font_size > 0.0 {
                    current.font_size * 0.4 // Conservative estimate
                } else {
                    4.0
                };

                let current_width = current.text.len() as f32 * char_width;
                let current_end = current.x + current_width;
                let gap = next.x - current_end;

                // If gap is small (e.g. < 2 chars), merge
                // Allow slight negative gap (overlap) due to kerning
                if gap > -char_width && gap < char_width * 2.5 {
                    // Merge!
                    // Add space if gap is significant (> 0.3 char width)
                    if gap > char_width * 0.3 {
                        current.text.push(' ');
                    }
                    current.text.push_str(&next.text);
                    continue;
                }
            }

            // Push current and start new
            merged.push(current);
            current = next;
        }
        merged.push(current);

        merged
    }

    /// Detect if page has two-column layout.
    /// Delegates to ColumnDetector for projection histogram analysis.
    fn detect_columns(&self, elements: &[TextElement], page_width: f32) -> Option<f32> {
        self.column_detector.detect_columns(elements, page_width)
    }

    /// Group text elements into lines with proper column handling
    /// For two-column layouts: reads left column top-to-bottom, then right column
    /// Returns (lines, detected_columns) where detected_columns are BoundingBoxes for each column
    fn group_into_lines(
        &self,
        elements: Vec<TextElement>,
        page_width: f32,
        page_height: f32,
    ) -> (Vec<Vec<TextElement>>, Vec<BoundingBox>) {
        if elements.is_empty() {
            return (Vec::new(), Vec::new());
        }

        // First, detect if this is a two-column layout
        let column_boundary = self.detect_columns(&elements, page_width);

        // Use TextGrouper to group elements into lines
        let lines = self.text_grouper.group_into_lines(
            elements,
            page_width,
            page_height,
            column_boundary,
        );

        // Create column bounding boxes if two-column layout
        let columns = if let Some(boundary) = column_boundary {
            let left_column = BoundingBox::new(0.0, 0.0, boundary, page_height);
            let right_column = BoundingBox::new(boundary, 0.0, page_width, page_height);
            vec![left_column, right_column]
        } else {
            Vec::new()
        };

        (lines, columns)
    }

    /// Convert lines to blocks with type detection
    fn lines_to_blocks(
        &self,
        lines: Vec<Vec<TextElement>>,
        page_width: f32,
        _page_height: f32,
    ) -> Vec<Block> {
        let mut blocks = Vec::new();

        // Debug: Log first 10 lines being processed
        debug!("Converting {} lines to blocks", lines.len());
        for (i, line) in lines.iter().take(10).enumerate() {
            let text: String = line
                .iter()
                .map(|e| e.text.as_str())
                .collect::<Vec<_>>()
                .join("");
            let y = line.first().map(|e| e.y).unwrap_or(0.0);
            let x = line.first().map(|e| e.x).unwrap_or(0.0);
            let preview: String = text.chars().take(40).collect();
            debug!(
                "  Block input line {}: Y={:.1} X={:.1} '{}'",
                i, y, x, preview
            );
        }

        // Calculate body font size (most common)
        let mut font_size_counts: BTreeMap<i32, usize> = BTreeMap::new();
        for line in &lines {
            for elem in line {
                let key = (elem.font_size * 10.0) as i32;
                *font_size_counts.entry(key).or_insert(0) += 1;
            }
        }
        let _body_size = font_size_counts
            .iter()
            .max_by_key(|&(_, count)| count)
            .map(|(&size, _)| size as f32 / 10.0)
            .unwrap_or(12.0);

        // Track text occurrences for running header detection
        let mut text_occurrences: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let line_texts: Vec<MergedLine> = lines.iter().map(|line| self.text_grouper.merge_line(line)).collect();

        for merged in &line_texts {
            let normalized = merged.text.trim().to_lowercase();
            if !normalized.is_empty() && normalized.len() < 100 {
                *text_occurrences.entry(normalized).or_insert(0) += 1;
            }
        }

        // Section pattern regex
        let _section_pattern = regex::Regex::new(r"^(\d+\.)+\s+[A-Z]").ok();

        let mut last_bbox: Option<BoundingBox> = None;
        let mut last_text: String = String::new();

        for (idx, line) in lines.iter().enumerate() {
            if line.is_empty() {
                continue;
            }

            let merged = &line_texts[idx];
            let text = merged.text.trim();
            if text.is_empty() {
                continue;
            }

            // Get bounding box
            let min_x = line
                .iter()
                .map(|e| e.x)
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(0.0);
            let max_x = line
                .iter()
                .map(|e| e.x)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(page_width);
            let y = line.first().map(|e| e.y).unwrap_or(0.0);

            let bbox = BoundingBox::new(min_x, y, max_x, y + merged.avg_font_size);

            // Deduplication: Check if this block is a duplicate of the previous one
            // (e.g. hidden OCR layer overlapping with visible text)
            if let Some(prev_bbox) = &last_bbox {
                // Check vertical overlap (lines are sorted by Y, so duplicates should be adjacent)
                let overlap_y = prev_bbox.y2.min(bbox.y2) - prev_bbox.y1.max(bbox.y1);
                let min_h = (prev_bbox.y2 - prev_bbox.y1).min(bbox.y2 - bbox.y1);

                if overlap_y > min_h * 0.5 {
                    // Significant vertical overlap (>50%). Check text similarity.
                    // We check for exact match or containment to handle slight OCR variations
                    if text == last_text
                        || (text.len() > 5
                            && (text.contains(&last_text) || last_text.contains(text)))
                    {
                        // tracing::debug!("Skipping duplicate block: '{}'", text);
                        continue;
                    }
                }
            }

            last_bbox = Some(bbox);
            last_text = text.to_string();

            // Detect block type
            let normalized = text.to_lowercase();
            let is_running_header = text_occurrences.get(&normalized).copied().unwrap_or(0) >= 3;

            let block_type = if is_running_header {
                BlockType::PageHeader
            } else {
                BlockType::Text
            };

            let spans = merged
                .spans
                .iter()
                .cloned()
                .map(|mut s| {
                    s.bbox = Some(bbox);
                    s
                })
                .collect::<Vec<_>>();

            let block = Block {
                id: BlockId::with_indices(0, blocks.len()),
                block_type,
                text: text.to_string(),
                bbox,
                page: 0,
                position: blocks.len(),
                level: None,
                spans,
                ..Default::default()
            };

            blocks.push(block);
        }

        blocks
    }

    #[allow(dead_code)]
    fn calculate_header_level(&self, font_size: f32, body_size: f32) -> u8 {
        let ratio = font_size / body_size;
        if ratio >= 2.0 {
            1
        } else if ratio >= 1.5 {
            2
        } else if ratio >= 1.3 {
            3
        } else {
            4
        }
    }

    /// Calculate adaptive region thresholds based on actual content distribution.
    ///
    /// This is a first-principles approach that analyzes the document's
    /// actual layout to determine appropriate thresholds for header/footer/title
    /// detection, instead of using hardcoded magic numbers.
    ///
    /// # Arguments
    /// * `elements` - Text elements to analyze
    ///
    /// Get page dimensions
    fn get_page_dimensions(&self, doc: &LopdfDocument, page_id: ObjectId) -> Result<(f32, f32)> {
        let page_dict = doc
            .get_dictionary(page_id)
            .map_err(|e| PdfError::PdfParse(format!("Failed to get page: {}", e)))?;

        // Try MediaBox
        if let Ok(media_box) = page_dict.get(b"MediaBox") {
            if let Object::Array(arr) = media_box {
                if arr.len() >= 4 {
                    let width = ContentParser::get_number(&arr[2]).unwrap_or(612.0);
                    let height = ContentParser::get_number(&arr[3]).unwrap_or(792.0);
                    return Ok((width, height));
                }
            } else if let Object::Reference(id) = media_box {
                if let Ok(Object::Array(arr)) = doc.get_object(*id) {
                    if arr.len() >= 4 {
                        let width = ContentParser::get_number(&arr[2]).unwrap_or(612.0);
                        let height = ContentParser::get_number(&arr[3]).unwrap_or(792.0);
                        return Ok((width, height));
                    }
                }
            }
        }

        Ok((612.0, 792.0))
    }

    /// Extract a single page
    fn extract_page(
        &self,
        doc: &LopdfDocument,
        page_id: ObjectId,
        page_num: usize,
    ) -> Result<Page> {
        let (page_width, page_height) = self.get_page_dimensions(doc, page_id)?;

        // Get fonts
        let fonts = self.get_page_fonts(doc, page_id).unwrap_or_default();
        debug!("Page {} has {} fonts", page_num, fonts.len());

        // Get content
        let content_bytes = self.get_page_content(doc, page_id)?;

        // Extract text and graphical elements using ContentParser
        let (elements, pdf_lines) = self.content_parser.parse(&content_bytes, &fonts)?;

        // Deduplicate elements (OCR layers)
        let elements = self.deduplicate_elements(elements);

        // Merge fragmented text elements
        let elements = self.merge_text_elements(elements);

        debug!(
            "Page {} has {} text elements and {} graphical lines",
            page_num,
            elements.len(),
            pdf_lines.len()
        );

        // Detect tables using lattice-based line detection
        let tables: Vec<Block> = self
            .lattice_engine
            .detect_tables(&pdf_lines, &elements, page_width, page_height)
            .into_iter()
            .filter(|table| {
                // Exclude tables that are too small (< 50x50 points)
                // This filters out small decorative boxes
                let min_size = 50.0;
                if table.bbox.width() < min_size || table.bbox.height() < min_size {
                    debug!(
                        "Filtered out table: too small ({:.1}x{:.1})",
                        table.bbox.width(),
                        table.bbox.height()
                    );
                    return false;
                }

                // Exclude tables that are too large (> 80% of page)
                // This filters out page borders and full-page elements
                // First principles: tables typically have margins on all sides
                let max_width = page_width * 0.8;
                let max_height = page_height * 0.8;
                if table.bbox.width() > max_width || table.bbox.height() > max_height {
                    debug!(
                        "Filtered out table: too large ({:.1}x{:.1})",
                        table.bbox.width(),
                        table.bbox.height()
                    );
                    return false;
                }

                // Exclude tables that are too close to page edges (likely page borders)
                // First principles: tables are typically centered with margins
                let margin_threshold = 20.0; // 20 points from edge
                if table.bbox.x1 < margin_threshold
                    || table.bbox.y1 < margin_threshold
                    || table.bbox.x2 > page_width - margin_threshold
                    || table.bbox.y2 > page_height - margin_threshold
                {
                    debug!("Filtered out table: too close to page edges");
                    return false;
                }

                // Exclude empty tables (no text content)
                if table.text.trim().is_empty() {
                    debug!("Filtered out table: empty");
                    return false;
                }

                // Exclude tables with very low text density (likely decorative boxes)
                // First principles: tables contain data, not just whitespace
                let text_len = table.text.trim().len();
                let table_area = table.bbox.width() * table.bbox.height();
                let text_density = text_len as f32 / table_area;
                if text_density < 0.0001 {
                    // Less than 1 char per 10000 points²
                    debug!("Filtered out table: low text density ({:.6})", text_density);
                    return false;
                }

                true
            })
            .collect();

        // Filter out text elements that are inside tables
        let mut non_table_elements = Vec::new();
        for elem in &elements {
            let mut inside_table = false;
            for table in &tables {
                // Check if element center is inside table bbox
                let cx = elem.x;
                let cy = elem.y;
                if table.bbox.contains_point(&Point::new(cx, cy)) {
                    inside_table = true;
                    break;
                }
            }
            if !inside_table {
                non_table_elements.push(elem.clone());
            }
        }

        // Safety check: if we filtered everything, maybe the table detection was too aggressive (e.g. page border)
        if non_table_elements.is_empty() && !elements.is_empty() {
            warn!("Table detection filtered all text elements on page {}. Ignoring table detection for text filtering.", page_num);
            non_table_elements = elements;
        }

        // Group into lines (handles two-column layouts) and get column bounding boxes
        let (lines, columns) = self.group_into_lines(non_table_elements, page_width, page_height);
        debug!(
            "Page {} has {} lines, {} columns detected",
            page_num,
            lines.len(),
            columns.len()
        );

        // Convert to blocks
        let mut blocks = self.lines_to_blocks(lines, page_width, page_height);

        // Insert detected tables back into the existing reading order.
        // We intentionally do NOT re-sort `blocks` globally (that can break multi-column reading
        // order), but we also do not want tables to be appended at the end of the page.
        // Instead, place each table at the first position where subsequent blocks appear below
        // the table on the page.
        let mut tables = tables;
        tables.sort_by(|a, b| {
            // Top-to-bottom insertion (higher Y first).
            b.bbox
                .y2
                .partial_cmp(&a.bbox.y2)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for table in tables {
            let table_y = (table.bbox.y1 + table.bbox.y2) * 0.5;
            let mut insert_idx = blocks.len();
            for (idx, blk) in blocks.iter().enumerate() {
                let blk_y = (blk.bbox.y1 + blk.bbox.y2) * 0.5;
                if blk_y < table_y {
                    insert_idx = idx;
                    break;
                }
            }
            blocks.insert(insert_idx, table);
        }

        // NOTE: Do NOT sort blocks here! The reading order has already been established by
        // group_into_lines() -> group_two_column_layout() or group_single_column_layout().
        // Sorting by Y would destroy the correct column-based reading order.

        let char_count: usize = blocks.iter().map(|b| b.text.len()).sum();
        let word_count: usize = blocks
            .iter()
            .map(|b| b.text.split_whitespace().count())
            .sum();

        let mut page = Page::new(page_num, page_width, page_height);
        page.blocks = blocks;
        page.columns = columns; // Set detected columns to prevent LayoutProcessor re-analysis
        page.method = ExtractionMethod::Native;
        page.stats = PageStats {
            text_blocks: page.blocks.len(),
            tables: page
                .blocks
                .iter()
                .filter(|b| b.block_type == BlockType::Table)
                .count(),
            figures: 0,
            headers: page
                .blocks
                .iter()
                .filter(|b| b.block_type == BlockType::SectionHeader)
                .count(),
            code_blocks: 0,
            equations: 0,
            char_count,
            word_count,
            avg_confidence: 1.0,
            ocr_used: false,
            processing_time_ms: 0,
        };

        Ok(page)
    }
}

impl Default for ExtractionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PdfBackend for ExtractionEngine {
    async fn extract(&self, pdf_bytes: &[u8]) -> Result<Document> {
        info!("Extracting PDF with SOTA backend");

        let lopdf_doc = LopdfDocument::load_mem(pdf_bytes)
            .map_err(|e| PdfError::PdfParse(format!("Failed to load PDF: {}", e)))?;

        if lopdf_doc.is_encrypted() {
            return Err(PdfError::PdfParse(
                "PDF is encrypted and password-protected".to_string(),
            ));
        }

        let pages = lopdf_doc.get_pages();
        let page_count = pages.len();
        info!("PDF has {} pages", page_count);

        let max_pages = self.config.max_pages.unwrap_or(page_count);
        let pages_to_process = page_count.min(max_pages);

        let mut document = Document::new();
        document.metadata = DocumentMetadata {
            pdf_version: Some(lopdf_doc.version.clone()),
            ..Default::default()
        };

        for (page_num, page_id) in pages.iter().take(pages_to_process) {
            debug!("Processing page {}", page_num);

            match self.extract_page(&lopdf_doc, *page_id, *page_num as usize) {
                Ok(page) => {
                    document.add_page(page);
                }
                Err(e) => {
                    warn!("Failed to extract page {}: {}", page_num, e);
                }
            }
        }

        info!("Extracted {} pages with SOTA backend", document.pages.len());
        Ok(document)
    }

    fn get_info(&self, pdf_bytes: &[u8]) -> Result<PdfInfo> {
        let lopdf_doc = LopdfDocument::load_mem(pdf_bytes)
            .map_err(|e| PdfError::PdfParse(format!("Failed to load PDF: {}", e)))?;

        if lopdf_doc.is_encrypted() {
            return Err(PdfError::PdfParse(
                "PDF is encrypted and password-protected".to_string(),
            ));
        }

        let pages = lopdf_doc.get_pages();

        Ok(PdfInfo {
            page_count: pages.len(),
            pdf_version: lopdf_doc.version.clone(),
            has_images: false,
            image_count: 0,
            file_size: pdf_bytes.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_line_preserves_style_runs_as_spans() {
        let backend = ExtractionEngine::new();

        let elems = vec![
            TextElement {
                text: "Hello".to_string(),
                x: 10.0,
                y: 700.0,
                font_size: 12.0,
                font_name: "Times-Roman".to_string(),
                is_bold: false,
                is_italic: false,
            },
            TextElement {
                text: "World".to_string(),
                x: 60.0,
                y: 700.0,
                font_size: 12.0,
                font_name: "Times-Bold".to_string(),
                is_bold: true,
                is_italic: false,
            },
        ];

        let merged = backend.text_grouper.merge_line(&elems);
        assert_eq!(merged.text, "Hello World");
        assert!(merged.spans.len() >= 2);
        assert_eq!(merged.spans[0].text, "Hello ");
        assert_eq!(merged.spans[0].style.weight, Some(400));
        assert_eq!(merged.spans[1].text, "World");
        assert_eq!(merged.spans[1].style.weight, Some(700));
    }

    fn make_text_element(text: &str, x: f32, y: f32) -> TextElement {
        TextElement {
            text: text.to_string(),
            x,
            y,
            font_size: 12.0,
            font_name: "Times-Roman".to_string(),
            is_bold: false,
            is_italic: false,
        }
    }

    #[test]
    fn test_deduplicate_removes_exact_duplicates() {
        let backend = ExtractionEngine::new();

        let elements = vec![
            make_text_element("Hello", 10.0, 700.0),
            make_text_element("Hello", 10.0, 700.0), // Exact duplicate
            make_text_element("World", 60.0, 700.0),
        ];

        let deduped = backend.deduplicate_elements(elements);
        assert_eq!(deduped.len(), 2, "Should remove exact duplicates");
    }

    #[test]
    fn test_deduplicate_keeps_near_elements() {
        let backend = ExtractionEngine::new();

        let elements = vec![
            make_text_element("Hello", 10.0, 700.0),
            make_text_element("Hello", 10.5, 700.5), // Near duplicate (within tolerance)
            make_text_element("World", 60.0, 700.0),
        ];

        let deduped = backend.deduplicate_elements(elements);
        // Near duplicates should also be removed
        assert!(deduped.len() <= 2, "Should handle near-duplicates");
    }

    #[test]
    fn test_merge_text_elements_horizontal() {
        let backend = ExtractionEngine::new();

        let elements = vec![
            make_text_element("Hel", 10.0, 700.0),
            make_text_element("lo", 30.0, 700.0), // Same line, close X
            make_text_element("World", 60.0, 700.0), // Same line
        ];

        let merged = backend.merge_text_elements(elements);
        // Should merge into fewer elements
        assert!(merged.len() <= 2, "Should merge horizontally adjacent text");
    }

    #[test]
    fn test_merge_preserves_vertical_separation() {
        let backend = ExtractionEngine::new();

        let elements = vec![
            make_text_element("Line 1", 10.0, 700.0),
            make_text_element("Line 2", 10.0, 680.0), // Different Y
        ];

        let merged = backend.merge_text_elements(elements);
        assert_eq!(
            merged.len(),
            2,
            "Vertically separated text should not merge"
        );
    }

    #[test]
    fn test_empty_elements() {
        let backend = ExtractionEngine::new();

        let empty: Vec<TextElement> = vec![];

        assert!(backend.deduplicate_elements(empty.clone()).is_empty());
        assert!(backend.merge_text_elements(empty).is_empty());
    }
}
