//! PDF extraction using lopdf - a pure Rust PDF library.
//!
//! This module provides PDF text extraction without external dependencies,
//! using lopdf for parsing and text extraction.

#![cfg(feature = "lopdf")]

use async_trait::async_trait;
use tracing::{debug, info};

use super::PdfBackend;
use crate::config::PdfConfig;
use crate::error::PdfError;
use crate::extractor::PdfInfo;
use crate::schema::{
    Block, BoundingBox, Document, ExtractionMethod, Page, PageStats,
};
use crate::{DocumentMetadata, Result};

/// Pure Rust PDF backend using lopdf.
pub struct LopdfBackend {
    config: PdfConfig,
}

impl LopdfBackend {
    /// Create a new LopdfBackend.
    pub fn new() -> Self {
        Self::with_config(PdfConfig::default())
    }

    /// Create a LopdfBackend with custom configuration.
    pub fn with_config(config: PdfConfig) -> Self {
        Self { config }
    }

    /// Extract text from a page using lopdf.
    fn extract_page_text(
        &self,
        doc: &lopdf::Document,
        page_num: u32,
    ) -> Result<(String, f32, f32)> {
        // Get page dimensions from MediaBox
        let (width, height) = self.get_page_dimensions(doc, page_num)?;

        // Extract text for this page
        let text = doc
            .extract_text(&[page_num])
            .map_err(|e| PdfError::PdfParse(format!("Failed to extract text from page {}: {}", page_num, e)))?;

        Ok((text, width, height))
    }

    /// Get page dimensions from MediaBox.
    fn get_page_dimensions(&self, doc: &lopdf::Document, page_num: u32) -> Result<(f32, f32)> {
        let pages = doc.get_pages();
        let page_id = pages
            .get(&page_num)
            .ok_or_else(|| PdfError::PdfParse(format!("Page {} not found", page_num)))?;

        // Try to get MediaBox from page or inherit from parent
        if let Ok(page_dict) = doc.get_dictionary(*page_id) {
            if let Ok(media_box) = page_dict.get(b"MediaBox") {
                if let Ok(arr) = media_box.as_array() {
                    if arr.len() >= 4 {
                        let width = Self::object_to_f32(&arr[2]).unwrap_or(612.0);
                        let height = Self::object_to_f32(&arr[3]).unwrap_or(792.0);
                        return Ok((width, height));
                    }
                }
            }
        }

        // Default to US Letter size
        Ok((612.0, 792.0))
    }

    /// Convert a lopdf Object to f32.
    fn object_to_f32(obj: &lopdf::Object) -> Option<f32> {
        match obj {
            lopdf::Object::Integer(i) => Some(*i as f32),
            lopdf::Object::Real(f) => Some(*f as f32),
            _ => None,
        }
    }

    /// Split extracted text into blocks based on paragraph breaks.
    fn text_to_blocks(&self, text: &str, page_width: f32, _page_height: f32) -> Vec<Block> {
        let mut blocks = Vec::new();
        let mut current_y: f32 = 50.0; // Start from top
        let line_height: f32 = 14.0;

        // Split by double newlines for paragraphs
        for paragraph in text.split("\n\n") {
            let paragraph = paragraph.trim();
            if paragraph.is_empty() {
                continue;
            }

            // Count lines in paragraph
            let line_count = paragraph.lines().count();
            let block_height = (line_count as f32) * line_height;

            // Create bounding box (approximate)
            let bbox = BoundingBox::new(50.0, current_y, page_width - 50.0, current_y + block_height);

            let block = Block::text(paragraph.to_string(), bbox);
            blocks.push(block);

            current_y += block_height + 10.0; // Gap between paragraphs
        }

        // If no paragraphs found, treat lines as blocks
        if blocks.is_empty() {
            for (line_idx, line) in text.lines().enumerate() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                let y = 50.0 + (line_idx as f32) * line_height;
                let bbox = BoundingBox::new(50.0, y, page_width - 50.0, y + line_height);

                let block = Block::text(line.to_string(), bbox);
                blocks.push(block);
            }
        }

        blocks
    }
}

impl Default for LopdfBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PdfBackend for LopdfBackend {
    async fn extract(&self, pdf_bytes: &[u8]) -> Result<Document> {
        info!("Extracting PDF with lopdf backend");

        // Load PDF from bytes
        let lopdf_doc = lopdf::Document::load_mem(pdf_bytes)
            .map_err(|e| PdfError::PdfParse(format!("Failed to load PDF: {}", e)))?;

        // Get page count
        let pages = lopdf_doc.get_pages();
        let page_count = pages.len();
        info!("PDF has {} pages", page_count);

        // Respect max_pages config
        let max_pages = self.config.max_pages.unwrap_or(page_count);
        let pages_to_process = page_count.min(max_pages);

        let mut document = Document::new();
        document.metadata = DocumentMetadata {
            pdf_version: Some(lopdf_doc.version.clone()),
            ..Default::default()
        };

        // Process each page
        for page_num in 1..=pages_to_process as u32 {
            debug!("Processing page {}", page_num);

            let (text, width, height) = self.extract_page_text(&lopdf_doc, page_num)?;

            let blocks = self.text_to_blocks(&text, width, height);
            let block_count = blocks.len();

            let mut page = Page::new(page_num as usize, width, height);
            page.blocks = blocks;
            page.method = ExtractionMethod::Native;
            page.stats = PageStats {
                text_blocks: block_count,
                tables: 0,
                figures: 0,
                headers: 0,
                code_blocks: 0,
                equations: 0,
                char_count: text.len(),
                word_count: text.split_whitespace().count(),
                avg_confidence: 1.0,
                ocr_used: false,
                processing_time_ms: 0,
            };

            document.add_page(page);
        }

        info!(
            "Extracted {} pages with lopdf backend",
            document.pages.len()
        );
        Ok(document)
    }

    fn get_info(&self, pdf_bytes: &[u8]) -> Result<PdfInfo> {
        let lopdf_doc = lopdf::Document::load_mem(pdf_bytes)
            .map_err(|e| PdfError::PdfParse(format!("Failed to load PDF: {}", e)))?;

        let pages = lopdf_doc.get_pages();

        Ok(PdfInfo {
            page_count: pages.len(),
            pdf_version: lopdf_doc.version.clone(),
            has_images: false, // TODO: Detect images
            image_count: 0,
            file_size: pdf_bytes.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lopdf_backend_creation() {
        let backend = LopdfBackend::new();
        assert!(backend.config.max_pages.is_none());
    }

    #[tokio::test]
    async fn test_lopdf_backend_with_config() {
        let config = PdfConfig::new().with_max_pages(5);
        let backend = LopdfBackend::with_config(config);
        assert_eq!(backend.config.max_pages, Some(5));
    }
}
