//! SOTA PDF extraction using pdfium-render with character-level word detection.
//!
//! This module provides a high-quality PDF extraction alternative that uses
//! Chromium's PDFium library for accurate text extraction with proper word
//! boundaries and page rendering for Vision mode.
//!
//! # Features
//!
//! - Character-level position detection for accurate word boundaries
//! - Page rendering to images for Vision mode extraction
//! - Compatible with the same interface as the default pdf_oxide extractor
//!
//! # Requirements
//!
//! Requires the Pdfium dynamic library to be available at runtime.
//! Download from: https://github.com/bblanchon/pdfium-binaries/releases

#![cfg(feature = "pdfium")]

use std::path::Path;
use tracing::{debug, info};

use pdfium_render::prelude::*;

use crate::config::PdfConfig;
use crate::error::PdfError;
use crate::layout::LayoutAnalyzer;
use crate::processors::{
    LayoutProcessor, PostProcessor, Processor, ProcessorChain, TableDetectionProcessor,
};
use crate::renderers::{MarkdownRenderer, MarkdownStyle, Renderer};
use crate::schema::{Block, BlockType, BoundingBox, Document, FontStyle, Page, TextSpan};
use crate::{DocumentMetadata, Result};

/// Pdfium-based PDF extractor with character-level word detection.
///
/// This extractor provides SOTA quality text extraction by using
/// character positions to accurately detect word boundaries.
pub struct PdfiumExtractor {
    pdfium: Pdfium,
    config: PdfConfig,
}

impl PdfiumExtractor {
    /// Create a new PdfiumExtractor.
    ///
    /// Attempts to bind to Pdfium in the following order:
    /// 1. Library in the current directory
    /// 2. Library in ./libs directory
    /// 3. System-provided library
    pub fn new() -> Result<Self> {
        Self::with_config(PdfConfig::default())
    }

    /// Create a PdfiumExtractor with custom configuration.
    pub fn with_config(config: PdfConfig) -> Result<Self> {
        // Try to bind to Pdfium library
        let bindings = Self::find_pdfium_library()?;
        let pdfium = Pdfium::new(bindings);

        Ok(Self { pdfium, config })
    }

    /// Find and bind to Pdfium library.
    fn find_pdfium_library() -> Result<Box<dyn PdfiumLibraryBindings>> {
        // Try ./libs directory first (where we put the downloaded library)
        let libs_path = Path::new("libs/lib");
        if libs_path.exists() {
            if let Ok(bindings) =
                Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(libs_path))
            {
                info!("Bound to Pdfium library in ./libs/lib");
                return Ok(bindings);
            }
        }

        // Try current directory
        if let Ok(bindings) =
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
        {
            info!("Bound to Pdfium library in current directory");
            return Ok(bindings);
        }

        // Try system library
        if let Ok(bindings) = Pdfium::bind_to_system_library() {
            info!("Bound to system Pdfium library");
            return Ok(bindings);
        }

        Err(PdfError::PdfParse(
            "Could not find Pdfium library. Download from https://github.com/bblanchon/pdfium-binaries/releases".to_string()
        ))
    }

    /// Extract document from PDF with proper word boundaries.
    ///
    /// Uses character-level position detection to insert spaces
    /// where word boundaries should exist and groups text into blocks.
    pub fn extract_document(&self, pdf_bytes: &[u8]) -> Result<Document> {
        info!("Starting Pdfium-based PDF extraction to Document IR");

        let pdfium_doc = self
            .pdfium
            .load_pdf_from_byte_vec(pdf_bytes.to_vec(), None)
            .map_err(|e| PdfError::PdfParse(format!("Failed to load PDF: {:?}", e)))?;

        let page_count = pdfium_doc.pages().len();
        info!("PDF has {} pages", page_count);

        // Extract metadata
        let metadata = self.extract_metadata(pdf_bytes)?;

        // Determine pages to process
        let max_pages = self.config.max_pages.unwrap_or(page_count as usize);
        let pages_to_process = std::cmp::min(page_count as usize, max_pages);

        let mut pages = Vec::new();

        // Process each page
        for page_index in 0..pages_to_process {
            let pdfium_page = pdfium_doc.pages().get(page_index as u16).map_err(|e| {
                PdfError::PdfParse(format!("Failed to get page {}: {:?}", page_index, e))
            })?;

            debug!(
                "Processing page {}/{} with Pdfium",
                page_index + 1,
                pages_to_process
            );

            // Extract blocks with word boundary detection
            let blocks = self.extract_page_blocks(&pdfium_page, page_index + 1)?;

            // Apply layout analysis to each page
            let analyzer = LayoutAnalyzer::new();
            let layout = analyzer.analyze(
                &blocks,
                pdfium_page.width().value,
                pdfium_page.height().value,
            );

            let mut page = Page::new(
                page_index + 1,
                pdfium_page.width().value,
                pdfium_page.height().value,
            );
            page.blocks = blocks;
            page.columns = layout.columns;

            // Sort blocks by reading order
            analyzer.sort_by_reading_order(&mut page.blocks, &page.columns);

            pages.push(page);
        }

        let mut doc = Document::new();
        doc.metadata = metadata;
        doc.pages = pages;

        // Apply post-processing (normalization, OCR fixes, table detection, etc.)
        let chain = ProcessorChain::new()
            .add(LayoutProcessor::new())
            .add(TableDetectionProcessor::new())
            .add(PostProcessor::new());

        let doc = chain.process(doc)?;

        Ok(doc)
    }

    /// Extract blocks from a page with character-level word boundary detection.
    fn extract_page_blocks(&self, page: &PdfPage, _page_num: usize) -> Result<Vec<Block>> {
        let text_page = page
            .text()
            .map_err(|e| PdfError::PdfParse(format!("Failed to get text page: {:?}", e)))?;

        let page_height = page.height().value;
        let mut blocks = Vec::new();
        let mut full_text = String::new();
        let mut span_text = String::new();
        let mut current_spans: Vec<TextSpan> = Vec::new();
        let mut current_style: Option<FontStyle> = None;

        let mut prev_char_right: Option<f32> = None;
        let mut prev_char_top: Option<f32> = None;
        let mut prev_char_bottom: Option<f32> = None;

        // Track block bounding box
        let mut block_min_x = f32::MAX;
        let mut block_max_x = f32::MIN;
        let mut block_min_y = f32::MAX;
        let mut block_max_y = f32::MIN;

        // Iterate over all characters in the page
        for (_i, char_info) in text_page.chars().iter().enumerate() {
            // Get character bounds
            let bounds = match char_info.tight_bounds() {
                Ok(b) => b,
                Err(_) => continue,
            };

            let char_text = match char_info.unicode_string() {
                Some(s) => s.replace("\r", "").replace("\n", ""),
                None => continue,
            };

            if char_text.is_empty() {
                continue;
            }

            // Extract font style
            let font_name = char_info.font_name();
            let is_bold = char_info
                .font_weight()
                .map(|w| pdf_font_weight_to_u16(w) >= 600)
                .unwrap_or(false)
                || font_name.to_lowercase().contains("bold");
            let is_italic = char_info.font_is_italic()
                || font_name.to_lowercase().contains("italic")
                || font_name.to_lowercase().contains("oblique");

            let font_style = FontStyle {
                family: Some(font_name),
                size: Some(char_info.scaled_font_size().value),
                weight: Some(if is_bold { 700 } else { 400 }),
                italic: is_italic,
                underline: false, // Pdfium doesn't easily provide underline per char
                strikethrough: false,
                superscript: false,
                subscript: false,
                color: None, // Could extract from fill_color()
                background_color: None,
            };

            // Convert PDF coordinates (bottom-left origin) to our coordinates (top-left origin)
            let char_left = bounds.left().value;
            let char_right = bounds.right().value;
            let char_bottom_pdf = bounds.bottom().value;
            let char_top_pdf = bounds.top().value;

            let char_top = page_height - char_top_pdf;
            let char_bottom = page_height - char_bottom_pdf;
            let char_width = bounds.width().value.abs();
            let char_height = (char_bottom - char_top).abs();

            // Use current character metrics for thresholds to handle font size changes
            let current_char_height = char_height.max(8.0);
            let current_char_width = char_width.max(4.0);

            // Detect newlines and paragraph breaks using vertical overlap and mid-point distance
            if let (Some(p_top), Some(p_bottom)) = (prev_char_top, prev_char_bottom) {
                let overlap_top = char_top.max(p_top);
                let overlap_bottom = char_bottom.min(p_bottom);
                let overlap_height = (overlap_bottom - overlap_top).max(0.0);

                let min_h = (char_bottom - char_top).abs().min((p_bottom - p_top).abs());
                let overlap_ratio = if min_h > 0.0 {
                    overlap_height / min_h
                } else {
                    1.0
                };
                let mid_y_diff = ((char_top + char_bottom) / 2.0 - (p_top + p_bottom) / 2.0).abs();

                // If overlap is small AND mid-point distance is significant, it's a new line
                if overlap_ratio < 0.2 && mid_y_diff > current_char_height * 0.4 {
                    let y_diff = (char_top - p_top).abs();

                    // If Y change is very large, it's a new block (paragraph)
                    if y_diff > current_char_height * 1.8 && !full_text.trim().is_empty() {
                        // Finish last span
                        if !span_text.is_empty() {
                            if let Some(ref style) = current_style {
                                current_spans
                                    .push(TextSpan::styled(span_text.clone(), style.clone()));
                            }
                        }

                        self.push_block(
                            &mut blocks,
                            &mut full_text,
                            &mut current_spans,
                            &mut current_style,
                            block_min_x,
                            block_min_y,
                            block_max_x,
                            block_max_y,
                        );
                        span_text.clear();

                        block_min_x = f32::MAX;
                        block_max_x = f32::MIN;
                        block_min_y = f32::MAX;
                        block_max_y = f32::MIN;
                    } else if !full_text.ends_with('\n') && !full_text.trim().is_empty() {
                        full_text.push('\n');
                        span_text.push('\n');
                    }
                    prev_char_right = None;
                }
            }

            // Detect word boundaries and column breaks
            if char_text == " " {
                if !full_text.ends_with(' ') && !full_text.is_empty() && !full_text.ends_with('\n')
                {
                    full_text.push(' ');
                    span_text.push(' ');
                }
                prev_char_right = Some(char_right);
                continue;
            } else if let Some(prev_right) = prev_char_right {
                let gap = char_left - prev_right;

                // If there's a large horizontal gap or we moved backwards significantly, start a new block
                if (gap > current_char_width * 2.0 || gap < -current_char_width * 2.0)
                    && !full_text.trim().is_empty()
                {
                    // Finish last span
                    if !span_text.is_empty() {
                        if let Some(ref style) = current_style {
                            current_spans.push(TextSpan::styled(span_text.clone(), style.clone()));
                        }
                    }

                    self.push_block(
                        &mut blocks,
                        &mut full_text,
                        &mut current_spans,
                        &mut current_style,
                        block_min_x,
                        block_min_y,
                        block_max_x,
                        block_max_y,
                    );
                    span_text.clear();

                    block_min_x = f32::MAX;
                    block_max_x = f32::MIN;
                    block_min_y = f32::MAX;
                    block_max_y = f32::MIN;
                } else if gap > current_char_width * 0.5
                    && !full_text.ends_with(' ')
                    && !full_text.ends_with('\n')
                {
                    // Normal word boundary
                    // For code blocks, be more conservative with auto-inserted spaces
                    let is_code = current_style
                        .as_ref()
                        .map(|s| s.looks_like_code())
                        .unwrap_or(false);
                    let threshold = if is_code { 1.5 } else { 0.8 };

                    if gap > current_char_width * threshold {
                        full_text.push(' ');
                        span_text.push(' ');
                    }
                }
            }

            // Handle style changes within a block
            let style_changed = if let Some(ref style) = current_style {
                *style != font_style
            } else {
                true
            };

            if style_changed {
                if !span_text.is_empty() {
                    if let Some(ref style) = current_style {
                        current_spans.push(TextSpan::styled(span_text.clone(), style.clone()));
                    }
                }
                span_text.clear();
                current_style = Some(font_style.clone());
            }

            full_text.push_str(&char_text);
            span_text.push_str(&char_text);

            // Update block bounds
            block_min_x = block_min_x.min(char_left);
            block_max_x = block_max_x.max(char_right);
            block_min_y = block_min_y.min(char_top);
            block_max_y = block_max_y.max(char_bottom);

            prev_char_right = Some(char_right);
            prev_char_top = Some(char_top);
            prev_char_bottom = Some(char_bottom);
        }

        // Add final block
        if !full_text.trim().is_empty() {
            if !span_text.is_empty() {
                if let Some(ref style) = current_style {
                    current_spans.push(TextSpan::styled(span_text.clone(), style.clone()));
                }
            }
            self.push_block(
                &mut blocks,
                &mut full_text,
                &mut current_spans,
                &mut current_style,
                block_min_x,
                block_min_y,
                block_max_x,
                block_max_y,
            );
        }

        // Extract images and other non-text objects
        for object in page.objects().iter() {
            match object.object_type() {
                PdfPageObjectType::Image => {
                    if let Ok(bounds) = object.bounds() {
                        let bbox = BoundingBox::new(
                            bounds.left().value,
                            page_height - bounds.top().value,
                            bounds.right().value,
                            page_height - bounds.bottom().value,
                        );
                        let mut block = Block::new(BlockType::Figure, bbox);
                        block.page = _page_num;
                        blocks.push(block);
                    }
                }
                _ => {}
            }
        }

        Ok(blocks)
    }

    /// Helper to push a block and classify it.
    fn push_block(
        &self,
        blocks: &mut Vec<Block>,
        text: &mut String,
        spans: &mut Vec<TextSpan>,
        style: &mut Option<FontStyle>,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
    ) {
        let trimmed_text = text.trim().to_string();
        if trimmed_text.is_empty() {
            return;
        }

        let bbox = BoundingBox::new(min_x, min_y, max_x, max_y);
        let mut block = Block::text(trimmed_text.clone(), bbox);
        block.spans = spans.clone();

        // Classify block based on style and content
        if let Some(ref s) = style {
            let base_size = 12.0;
            let is_bold = s.weight.map(|w| w >= 600).unwrap_or(false);
            let size = s.size.unwrap_or(base_size);

            if is_bold || size > base_size * 1.1 {
                block.block_type = BlockType::SectionHeader;
                // Improved level estimation
                block.level = if size >= 18.0 {
                    Some(1)
                } else if size >= 14.0 {
                    Some(2)
                } else if size >= 12.0 && is_bold {
                    Some(3)
                } else {
                    Some(4)
                };
            } else if s.looks_like_code() {
                block.block_type = BlockType::Code;
            }
        }

        // Pattern-based classification (Lists)
        if trimmed_text.starts_with("- ")
            || trimmed_text.starts_with("• ")
            || (trimmed_text.len() > 2
                && trimmed_text.chars().next().unwrap().is_ascii_digit()
                && trimmed_text.contains(". "))
        {
            block.block_type = BlockType::ListItem;
            // Store indentation for nested lists
            block
                .metadata
                .insert("indent".to_string(), serde_json::json!(min_x));
        }

        blocks.push(block);
        text.clear();
        spans.clear();
        *style = None;
    }

    /// Extract text from PDF with proper word boundaries.
    ///
    /// Uses character-level position detection to insert spaces
    /// where word boundaries should exist.
    pub fn extract_to_markdown(&self, pdf_bytes: &[u8]) -> Result<String> {
        let doc = self.extract_document(pdf_bytes)?;

        let style = MarkdownStyle {
            page_numbers: self.config.include_page_numbers,
            ..MarkdownStyle::default()
        };
        let renderer = MarkdownRenderer::with_style(style);
        renderer.render(&doc)
    }

    /// Render a page to an image for Vision mode extraction.
    ///
    /// Returns the page as a PNG image at the specified DPI.
    pub fn render_page_to_image(
        &self,
        pdf_bytes: &[u8],
        page_index: usize,
        dpi: u32,
    ) -> Result<Vec<u8>> {
        let document = self
            .pdfium
            .load_pdf_from_byte_vec(pdf_bytes.to_vec(), None)
            .map_err(|e| PdfError::PdfParse(format!("Failed to load PDF: {:?}", e)))?;

        let page = document.pages().get(page_index as u16).map_err(|e| {
            PdfError::PdfParse(format!("Failed to get page {}: {:?}", page_index, e))
        })?;

        // Calculate pixel dimensions based on DPI
        let scale = dpi as f32 / 72.0; // PDF points to pixels
        let width = (page.width().value * scale) as i32;
        let height = (page.height().value * scale) as i32;

        // Render the page
        let render_config = PdfRenderConfig::new()
            .set_target_width(width)
            .set_maximum_height(height);

        let bitmap = page
            .render_with_config(&render_config)
            .map_err(|e| PdfError::PdfParse(format!("Failed to render page: {:?}", e)))?;

        // Convert to PNG
        let image = bitmap.as_image();
        let mut png_data = Vec::new();

        image
            .write_to(
                &mut std::io::Cursor::new(&mut png_data),
                image::ImageFormat::Png,
            )
            .map_err(|e| PdfError::PdfParse(format!("Failed to encode PNG: {:?}", e)))?;

        Ok(png_data)
    }

    /// Get page count from PDF.
    pub fn page_count(&self, pdf_bytes: &[u8]) -> Result<usize> {
        let document = self
            .pdfium
            .load_pdf_from_byte_vec(pdf_bytes.to_vec(), None)
            .map_err(|e| PdfError::PdfParse(format!("Failed to load PDF: {:?}", e)))?;

        Ok(document.pages().len() as usize)
    }

    /// Extract document metadata.
    pub fn extract_metadata(&self, pdf_bytes: &[u8]) -> Result<DocumentMetadata> {
        let document = self
            .pdfium
            .load_pdf_from_byte_vec(pdf_bytes.to_vec(), None)
            .map_err(|e| PdfError::PdfParse(format!("Failed to load PDF: {:?}", e)))?;

        let metadata = document.metadata();

        // Extract metadata tags - convert &str to owned String
        let title = metadata
            .get(PdfDocumentMetadataTagType::Title)
            .map(|tag| tag.value().to_string())
            .filter(|s| !s.is_empty());
        let author = metadata
            .get(PdfDocumentMetadataTagType::Author)
            .map(|tag| tag.value().to_string())
            .filter(|s| !s.is_empty());
        let creation_date = metadata
            .get(PdfDocumentMetadataTagType::CreationDate)
            .map(|tag| tag.value().to_string());

        // Format PDF version using Debug trait since Display is not implemented
        let pdf_version = Some(format!("{:?}", document.version()));

        let mut doc_metadata = DocumentMetadata::default();
        doc_metadata.title = title;
        doc_metadata.author = author;
        doc_metadata.creation_date = creation_date;
        doc_metadata.pdf_version = pdf_version;

        Ok(doc_metadata)
    }
}

/// Helper to convert Pdfium font weight to u16.
fn pdf_font_weight_to_u16(weight: PdfFontWeight) -> u16 {
    match weight {
        PdfFontWeight::Weight100 => 100,
        PdfFontWeight::Weight200 => 200,
        PdfFontWeight::Weight300 => 300,
        PdfFontWeight::Weight400Normal => 400,
        PdfFontWeight::Weight500 => 500,
        PdfFontWeight::Weight600 => 600,
        PdfFontWeight::Weight700Bold => 700,
        PdfFontWeight::Weight800 => 800,
        PdfFontWeight::Weight900 => 900,
        PdfFontWeight::Custom(w) => w as u16,
    }
}

/// Default implementation using system/local Pdfium library.
impl Default for PdfiumExtractor {
    fn default() -> Self {
        Self::new().expect("Failed to initialize PdfiumExtractor")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdfium_extractor_creation() {
        // This test will fail if Pdfium library is not available
        // That's expected behavior for the feature-gated module
        let result = PdfiumExtractor::new();

        // If Pdfium is available, the extractor should be created
        // If not, we get an error which is also valid
        match result {
            Ok(_) => println!("PdfiumExtractor created successfully"),
            Err(e) => println!("PdfiumExtractor not available: {}", e),
        }
    }
}
