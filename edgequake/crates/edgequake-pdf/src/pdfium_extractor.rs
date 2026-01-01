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

use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info};

use pdfium_render::prelude::*;

use crate::config::PdfConfig;
use crate::error::PdfError;
use crate::layout::LayoutAnalyzer;

use crate::renderers::{MarkdownRenderer, MarkdownStyle, Renderer};
use crate::schema::{Block, BlockId, BlockType, BoundingBox, Document, FontStyle, Page, TextSpan};
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

        Ok(doc)
    }

    /// Extract blocks from a page with character-level word boundary detection.
    fn extract_page_blocks(&self, page: &PdfPage, page_num: usize) -> Result<Vec<Block>> {
        let text_page = page
            .text()
            .map_err(|e| PdfError::PdfParse(format!("Failed to get text page: {:?}", e)))?;

        let page_height = page.height().value;

        // 1. Collect all characters with their properties
        #[derive(Clone)]
        struct CharData {
            text: String,
            left: f32,
            right: f32,
            top: f32,
            bottom: f32,
            height: f32,
            font_style: FontStyle,
        }

        let mut all_chars = Vec::new();
        for char_info in text_page.chars().iter() {
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

            let font_name = char_info.font_name();
            let is_bold = char_info
                .font_weight()
                .map(|w| pdf_font_weight_to_u16(w) >= 600)
                .unwrap_or(false)
                || font_name.to_lowercase().contains("bold");
            let is_italic = char_info.font_is_italic()
                || font_name.to_lowercase().contains("italic")
                || font_name.to_lowercase().contains("oblique");

            let char_left = bounds.left().value;
            let char_right = bounds.right().value;
            let char_top = page_height - bounds.top().value;
            let char_bottom = page_height - bounds.bottom().value;
            let char_height = (char_bottom - char_top).abs();

            all_chars.push(CharData {
                text: char_text,
                left: char_left,
                right: char_right,
                top: char_top,
                bottom: char_bottom,
                height: char_height,
                font_style: FontStyle {
                    family: Some(font_name),
                    size: Some(char_info.scaled_font_size().value),
                    weight: Some(if is_bold { 700 } else { 400 }),
                    italic: is_italic,
                    underline: false,
                    strikethrough: false,
                    superscript: false, // Will be detected later
                    subscript: false,   // Will be detected later
                    color: None,
                    background_color: None,
                },
            });
        }

        if all_chars.is_empty() {
            return Ok(Vec::new());
        }

        // 2. Group characters into lines
        // Sort by top coordinate first
        all_chars.sort_by(|a, b| {
            a.top
                .partial_cmp(&b.top)
                .unwrap()
                .then(a.left.partial_cmp(&b.left).unwrap())
        });

        let mut lines: Vec<Vec<CharData>> = Vec::new();
        for char_data in all_chars {
            let mut found_line = false;
            // Try to find a line that this character belongs to (check last few lines)
            // Use a more relaxed overlap check for lines
            for line in lines.iter_mut().rev().take(10) {
                let l_top = line.iter().map(|c| c.top).fold(f32::MAX, |a, b| a.min(b));
                let l_bottom = line
                    .iter()
                    .map(|c| c.bottom)
                    .fold(f32::MIN, |a, b| a.max(b));
                let l_height = (l_bottom - l_top).max(1.0);

                let overlap = (char_data.bottom.min(l_bottom) - char_data.top.max(l_top)).max(0.0);
                // If overlap is more than 30% of either height, it's the same line
                if overlap > char_data.height * 0.3 || overlap > l_height * 0.3 {
                    line.push(char_data.clone());
                    found_line = true;
                    break;
                }
            }

            if !found_line {
                lines.push(vec![char_data]);
            }
        }

        // 3. Process each line into blocks
        let mut blocks = Vec::new();
        for mut line_chars in lines {
            // Sort characters in line by left coordinate
            line_chars.sort_by(|a, b| a.left.partial_cmp(&b.left).unwrap());

            // Detect columns within the line (large horizontal gaps)
            let mut line_parts = Vec::new();
            let mut current_part = Vec::new();
            let mut prev_right: Option<f32> = None;

            for char_data in line_chars {
                if let Some(pr) = prev_right {
                    // Split if gap is larger than 1.2x char height (typical for columns/tables)
                    let gap = char_data.left - pr;
                    if gap > char_data.height * 1.2 && gap > 5.0 {
                        debug!("Splitting line at gap: {}", gap);
                        line_parts.push(current_part);
                        current_part = Vec::new();
                    }
                }
                prev_right = Some(char_data.right);
                current_part.push(char_data);
            }
            line_parts.push(current_part);

            for part in line_parts {
                if part.is_empty() {
                    continue;
                }

                let mut current_spans = Vec::new();
                let mut span_text = String::new();
                let mut current_style: Option<FontStyle> = None;
                let mut prev_char: Option<&CharData> = None;

                let mut min_x = f32::MAX;
                let mut max_x = f32::MIN;
                let mut min_y = f32::MAX;
                let mut max_y = f32::MIN;

                // Find max height in part for reference
                let _max_h = part.iter().map(|c| c.height).fold(0.0f32, |a, b| a.max(b));

                for char_data in &part {
                    min_x = min_x.min(char_data.left);
                    max_x = max_x.max(char_data.right);
                    min_y = min_y.min(char_data.top);
                    max_y = max_y.max(char_data.bottom);

                    let style = char_data.font_style.clone();

                    // Detect superscript/subscript relative to line
                    /*
                    let line_mid = (min_y + max_y) / 2.0;
                    let is_punct_check = char_data.text.chars().next().map_or(false, |c| c.is_ascii_punctuation());
                    
                    // Only if character is significantly smaller
                    if !is_punct_check && char_data.height < max_h * 0.75 {
                        let char_mid = (char_data.top + char_data.bottom) / 2.0;
                        
                        if char_mid < line_mid - (max_h * 0.15) {
                             style.superscript = true;
                        } else if char_mid > line_mid + (max_h * 0.15) {
                             style.subscript = true;
                        }
                    }
                    */

                    // Handle style changes
                    if let Some(ref cur_style) = current_style {
                        // Check if styles are effectively different (ignoring minor size diffs)
                        let size_diff = (cur_style.size.unwrap_or(0.0) - style.size.unwrap_or(0.0)).abs();
                        
                        // Treat weight >= 600 as bold, < 600 as normal
                        let cur_bold = cur_style.weight.unwrap_or(400) >= 600;
                        let new_bold = style.weight.unwrap_or(400) >= 600;
                        let bold_changed = cur_bold != new_bold;
                        let code_changed = cur_style.looks_like_code() != style.looks_like_code();
                        
                        let style_changed = bold_changed 
                            || cur_style.italic != style.italic
                            || cur_style.superscript != style.superscript
                            || cur_style.subscript != style.subscript
                            || code_changed
                            || size_diff > 1.5; // Allow small size variations

                        if style_changed {
                            debug!("Style changed: {:?} -> {:?}", cur_style, style);
                            if !span_text.is_empty() {
                                current_spans.push(TextSpan {
                                    text: span_text.clone(),
                                    bbox: None,
                                    style: cur_style.clone(),
                                });
                                span_text.clear();
                            }
                            current_style = Some(style.clone());
                        }
                    } else {
                        current_style = Some(style.clone());
                    }

                    // Add space if needed
                    if let Some(pc) = prev_char {
                        let h_dist = char_data.left - pc.right;
                        
                        // Determine threshold based on content
                        let is_punct = char_data.text.chars().next().map_or(false, |c| c.is_ascii_punctuation());
                        let is_code = char_data.font_style.looks_like_code();
                        
                        let threshold = if is_code {
                            char_data.height * 0.8 // Larger gap for monospace
                        } else if is_punct {
                            char_data.height * 1.5 // Require VERY large gap for punctuation
                        } else {
                            char_data.height * 0.35 // Standard gap for text
                        };

                        // Also don't add space if characters overlap or are very close (ligatures)
                        if h_dist > threshold
                            && char_data.text != " "
                            && !span_text.ends_with(' ')
                        {
                            debug!("Inserting space: '{}' - '{}', dist: {}, thresh: {}", pc.text, char_data.text, h_dist, threshold);
                            span_text.push(' ');
                        }
                    }

                    span_text.push_str(&char_data.text);
                    prev_char = Some(char_data);
                }

                if !span_text.is_empty() {
                    current_spans.push(TextSpan {
                        text: span_text,
                        bbox: None,
                        style: current_style.unwrap_or_default(),
                    });
                }

                let block_text = current_spans
                    .iter()
                    .map(|s| s.text.as_str())
                    .collect::<Vec<_>>()
                    .join("");

                if block_text.trim().is_empty() {
                    continue;
                }

                blocks.push(Block {
                    id: BlockId::with_indices(page_num, blocks.len()),
                    block_type: BlockType::Text,
                    bbox: BoundingBox::new(min_x, min_y, max_x, max_y),
                    page: page_num,
                    position: blocks.len(),
                    text: block_text,
                    html: None,
                    spans: current_spans,
                    children: Vec::new(),
                    confidence: 1.0,
                    level: None,
                    source: None,
                    metadata: HashMap::new(),
                });
            }
        }

        Ok(blocks)
    }

    /// Extract text from PDF with proper word boundaries.
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

        let scale = dpi as f32 / 72.0;
        let width = (page.width().value * scale) as i32;
        let height = (page.height().value * scale) as i32;

        let render_config = PdfRenderConfig::new()
            .set_target_width(width)
            .set_maximum_height(height);

        let bitmap = page
            .render_with_config(&render_config)
            .map_err(|e| PdfError::PdfParse(format!("Failed to render page: {:?}", e)))?;

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

        let pdf_version = Some(format!("{:?}", document.version()));

        let mut doc_metadata = DocumentMetadata::default();
        doc_metadata.title = title;
        doc_metadata.author = author;
        doc_metadata.creation_date = creation_date;
        doc_metadata.pdf_version = pdf_version;

        Ok(doc_metadata)
    }
}

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
        let result = PdfiumExtractor::new();
        match result {
            Ok(_) => println!("PdfiumExtractor created successfully"),
            Err(e) => println!("PdfiumExtractor not available: {}", e),
        }
    }
}
