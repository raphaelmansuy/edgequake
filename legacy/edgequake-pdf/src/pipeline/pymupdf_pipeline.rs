//! PyMuPDF4LLM-inspired PDF to Markdown pipeline.
//!
//! This module provides a high-level API for converting PDF documents to
//! Markdown text using the same approach as pymupdf4llm but in pure Rust.
//!
//! ## Pipeline Overview
//!
//! ```text
//! PDF → PDFium → RawChars → Spans → Lines → Blocks → Markdown
//! ```
//!
//! ## Features
//!
//! - **Character-level extraction** via PDFium with accurate bounding boxes
//! - **Font style detection** for bold, italic, monospace
//! - **Header detection** based on font size analysis
//! - **Code block detection** from monospace fonts
//! - **List detection** from bullet/number patterns
//!
//! ## Example
//!
//! ```rust,ignore
//! use edgequake_pdf::pipeline::PymupdfPipeline;
//!
//! let pipeline = PymupdfPipeline::new()?;
//! let markdown = pipeline.convert_file("document.pdf")?;
//! println!("{}", markdown);
//! ```

#[cfg(feature = "pdfium")]
use crate::backend::PdfiumExtractor;
use crate::error::PdfError;
use crate::layout::page_filter::{filter_headers_footers, HeaderFooterConfig};
use crate::layout::{GroupingParams, MarkdownConfig, MarkdownRenderer, TextBlock, TextGrouper};

/// Configuration for the pymupdf4llm-inspired pipeline.
#[derive(Debug, Clone, Default)]
pub struct PipelineConfig {
    /// Text grouping parameters
    pub grouping: GroupingParams,
    /// Markdown rendering configuration
    pub markdown: MarkdownConfig,
    /// Body text font size (for header detection)
    /// If None, will be auto-detected from most common font size
    pub body_font_size: Option<f32>,
}

/// High-level pipeline for PDF to Markdown conversion.
///
/// This uses PDFium for accurate character extraction and applies
/// pymupdf4llm-inspired algorithms for layout analysis.
#[cfg(feature = "pdfium")]
pub struct PymupdfPipeline {
    extractor: PdfiumExtractor,
    config: PipelineConfig,
}

#[cfg(feature = "pdfium")]
impl PymupdfPipeline {
    /// Create a new pipeline with default configuration.
    ///
    /// Requires `PDFIUM_DYNAMIC_LIB_PATH` environment variable to be set,
    /// or libpdfium to be in a standard system location.
    pub fn new() -> Result<Self, PdfError> {
        let extractor = PdfiumExtractor::new()?;
        Ok(Self {
            extractor,
            config: PipelineConfig::default(),
        })
    }

    /// Create a pipeline with explicit library path.
    pub fn with_library_path<P: AsRef<std::path::Path>>(path: P) -> Result<Self, PdfError> {
        let extractor = PdfiumExtractor::with_library_path(path)?;
        Ok(Self {
            extractor,
            config: PipelineConfig::default(),
        })
    }

    /// Create a pipeline with custom configuration.
    pub fn with_config(config: PipelineConfig) -> Result<Self, PdfError> {
        let extractor = PdfiumExtractor::new()?;
        Ok(Self { extractor, config })
    }

    /// Convert a PDF file to Markdown.
    pub fn convert_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<String, PdfError> {
        // Extract characters
        let chars = self.extractor.extract_chars_from_file(path.as_ref())?;

        // Run the pipeline
        self.process_chars(&chars)
    }

    /// Convert PDF bytes to Markdown.
    pub fn convert_bytes(&self, bytes: &[u8]) -> Result<String, PdfError> {
        // Extract characters
        let chars = self.extractor.extract_chars_from_bytes(bytes)?;

        // Run the pipeline
        self.process_chars(&chars)
    }

    /// Process extracted characters into Markdown.
    fn process_chars(
        &self,
        chars: &[crate::backend::elements::RawChar],
    ) -> Result<String, PdfError> {
        // Group chars → spans → lines → blocks
        let grouper = TextGrouper::with_params(self.config.grouping.clone());
        let blocks = grouper.group(chars);

        // OODA-12: Split blocks at bullet/list item lines BEFORE classification
        // WHY: Block grouping merges bullet items with preceding text. Splitting first
        // ensures each bullet becomes its own block that can be classified as ListItem.
        let mut blocks = grouper.split_at_bullet_lines(blocks);

        // Detect body font size if not specified
        let body_size = self
            .config
            .body_font_size
            .unwrap_or_else(|| detect_body_font_size(&blocks));

        // Classify blocks (headers, code, lists, footnotes, etc.)
        // OODA-09: Use page-aware classification for footnote detection
        grouper.classify_blocks_page_aware(&mut blocks, body_size);

        // OODA-12: Merge consecutive header blocks (title continuation)
        // WHY: Paper titles can wrap across lines, creating separate blocks.
        // This merges them into a single header block.
        let blocks = grouper.merge_title_blocks(blocks);

        // OODA-10: Split blocks where headers were merged with following paragraphs
        // WHY: Block merging can group header lines with paragraph content when they're
        // close together. This splits them so headers are rendered correctly.
        let blocks = grouper.split_header_blocks(blocks);

        // OODA-11: Filter repeated page headers and footers
        // WHY: Running headers ("Journal Name") and page numbers add noise.
        // Estimate page_height from block coordinates per page, then filter.
        let blocks = filter_page_headers_footers(&blocks);

        // Render to Markdown
        let renderer = MarkdownRenderer::with_config(self.config.markdown.clone());
        Ok(renderer.render(&blocks))
    }

    /// Get detailed extraction results (blocks with metadata).
    pub fn extract_blocks<P: AsRef<std::path::Path>>(
        &self,
        path: P,
    ) -> Result<Vec<TextBlock>, PdfError> {
        let chars = self.extractor.extract_chars_from_file(path.as_ref())?;

        let grouper = TextGrouper::with_params(self.config.grouping.clone());
        let blocks = grouper.group(&chars);

        // OODA-12: Split blocks at bullet/list item lines BEFORE classification
        let mut blocks = grouper.split_at_bullet_lines(blocks);

        let body_size = self
            .config
            .body_font_size
            .unwrap_or_else(|| detect_body_font_size(&blocks));

        grouper.classify_blocks(&mut blocks, body_size);

        // OODA-12: Merge consecutive header blocks (title continuation)
        let blocks = grouper.merge_title_blocks(blocks);

        // OODA-10: Split headers merged with paragraphs
        let blocks = grouper.split_header_blocks(blocks);

        // OODA-11: Filter repeated page headers and footers
        let blocks = filter_page_headers_footers(&blocks);

        Ok(blocks)
    }
}

/// Detect the most common (body) font size from blocks.
///
/// OODA-30: Enhanced with outlier filtering and finer-grained binning.
/// This uses the pymupdf4llm approach: the most frequent font size
/// (weighted by text length) is assumed to be body text.
///
/// Improvements over basic approach:
/// - Filter extreme outliers (< 4pt or > 36pt) that are likely metadata or display text
/// - Use half-point binning (round to nearest 0.5pt) for better discrimination
/// - Ignore whitespace-only spans
fn detect_body_font_size(blocks: &[TextBlock]) -> f32 {
    use std::collections::HashMap;

    // OODA-30: Use half-point bins for finer discrimination
    // WHY: Some documents have 9.5pt body and 10pt headers - integer binning
    // merges them, making header detection unreliable.
    let mut size_counts: HashMap<i32, usize> = HashMap::new();

    for block in blocks {
        for line in &block.lines {
            for span in &line.spans {
                // OODA-30: Skip tiny/huge outliers
                // WHY: PDFs contain metadata chars at 0-3pt and display text at 40-200pt.
                // These pollute body font detection. 4-36pt covers all reasonable body text.
                if span.font_size < 4.0 || span.font_size > 36.0 {
                    continue;
                }

                // OODA-30: Skip whitespace-only spans
                if span.text.trim().is_empty() {
                    continue;
                }

                // OODA-30: Half-point binning (multiply by 2, round, use as key)
                let size_key = (span.font_size * 2.0).round() as i32;
                let text_len = span.text.trim().len();
                *size_counts.entry(size_key).or_insert(0) += text_len;
            }
        }
    }

    // Find most common font size (convert back from half-point bin)
    size_counts
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(size_key, _)| size_key as f32 / 2.0)
        .unwrap_or(12.0) // Default fallback
}

/// OODA-11: Filter repeated page headers and footers from blocks.
/// Estimates page_height per page from block coordinates.
fn filter_page_headers_footers(blocks: &[TextBlock]) -> Vec<TextBlock> {
    if blocks.is_empty() {
        return blocks.to_vec();
    }

    // Estimate page_height from max y coordinate per page + margin
    let mut max_y_per_page: std::collections::HashMap<usize, f32> =
        std::collections::HashMap::new();
    for block in blocks {
        let entry = max_y_per_page.entry(block.page_num).or_insert(0.0_f32);
        *entry = entry.max(block.y1);
    }

    // Use the global max as estimated page_height (most pages share the same size)
    let estimated_page_height = max_y_per_page.values().copied().fold(0.0_f32, f32::max) + 72.0; // Add 1 inch margin

    let config = HeaderFooterConfig::default();
    filter_headers_footers(blocks, estimated_page_height, &config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{BlockType, Span};

    #[test]
    fn test_detect_body_font_size() {
        // Create blocks with mixed font sizes
        // Body text (12pt) should be most common
        let blocks = vec![
            TextBlock {
                lines: vec![crate::layout::Line {
                    spans: vec![Span {
                        text: "This is a header".to_string(),
                        font_size: 24.0,
                        x0: 0.0,
                        y0: 0.0,
                        x1: 200.0,
                        y1: 24.0,
                        font_name: None,
                        page_num: 0,
                        font_is_bold: None,
                        font_is_italic: None,
                        font_is_monospace: None,
                    }],
                    x0: 0.0,
                    y0: 0.0,
                    x1: 200.0,
                    y1: 24.0,
                    page_num: 0,
                }],
                x0: 0.0,
                y0: 0.0,
                x1: 200.0,
                y1: 24.0,
                page_num: 0,
                block_type: BlockType::Paragraph,
            },
            TextBlock {
                lines: vec![crate::layout::Line {
                    spans: vec![Span {
                        text: "This is body text that is much longer than the header because it contains more content and should therefore be detected as the body font size".to_string(),
                        font_size: 12.0,
                        x0: 0.0,
                        y0: 0.0,
                        x1: 500.0,
                        y1: 12.0,
                        font_name: None,
                        page_num: 0,
                        font_is_bold: None,
                        font_is_italic: None,
                        font_is_monospace: None,
                    }],
                    x0: 0.0,
                    y0: 0.0,
                    x1: 500.0,
                    y1: 12.0,
                    page_num: 0,
                }],
                x0: 0.0,
                y0: 0.0,
                x1: 500.0,
                y1: 12.0,
                page_num: 0,
                block_type: BlockType::Paragraph,
            },
        ];

        let body_size = detect_body_font_size(&blocks);
        assert_eq!(body_size, 12.0);
    }

    /// OODA-30: Test outlier filtering in body font detection
    #[test]
    fn test_detect_body_font_size_with_outliers() {
        // Create blocks with outlier sizes (tiny metadata, huge display text)
        // Body text at 10pt should still be detected correctly
        let make_block = |text: &str, font_size: f32| TextBlock {
            lines: vec![crate::layout::Line {
                spans: vec![Span {
                    text: text.to_string(),
                    font_size,
                    x0: 0.0,
                    y0: 0.0,
                    x1: 200.0,
                    y1: font_size,
                    font_name: None,
                    page_num: 0,
                    font_is_bold: None,
                    font_is_italic: None,
                    font_is_monospace: None,
                }],
                x0: 0.0,
                y0: 0.0,
                x1: 200.0,
                y1: font_size,
                page_num: 0,
            }],
            x0: 0.0,
            y0: 0.0,
            x1: 200.0,
            y1: font_size,
            page_num: 0,
            block_type: BlockType::Paragraph,
        };

        let blocks = vec![
            make_block("x", 2.0),  // Tiny outlier - should be filtered
            make_block("This is the main body text with many words to ensure it dominates the count for detection purposes", 10.0),
            make_block("HUGE DISPLAY", 50.0),  // Huge outlier - should be filtered
            make_block("Another paragraph of body text that reinforces the ten point font", 10.0),
        ];

        let body_size = detect_body_font_size(&blocks);
        assert_eq!(body_size, 10.0, "Should detect 10pt body despite outliers");
    }

    /// OODA-30: Test half-point binning
    #[test]
    fn test_detect_body_font_size_half_point() {
        let make_block = |text: &str, font_size: f32| TextBlock {
            lines: vec![crate::layout::Line {
                spans: vec![Span {
                    text: text.to_string(),
                    font_size,
                    x0: 0.0,
                    y0: 0.0,
                    x1: 200.0,
                    y1: font_size,
                    font_name: None,
                    page_num: 0,
                    font_is_bold: None,
                    font_is_italic: None,
                    font_is_monospace: None,
                }],
                x0: 0.0,
                y0: 0.0,
                x1: 200.0,
                y1: font_size,
                page_num: 0,
            }],
            x0: 0.0,
            y0: 0.0,
            x1: 200.0,
            y1: font_size,
            page_num: 0,
            block_type: BlockType::Paragraph,
        };

        // 9.5pt body text should be distinguished from 10pt header
        let blocks = vec![
            make_block("Header text which is shorter", 10.0),
            make_block("This is body text at nine point five which is much longer than header to ensure correct detection as body font", 9.5),
            make_block("More body text at nine point five confirming the pattern for the detection algorithm to pick up", 9.5),
        ];

        let body_size = detect_body_font_size(&blocks);
        assert_eq!(
            body_size, 9.5,
            "Should detect 9.5pt body with half-point binning"
        );
    }
}
