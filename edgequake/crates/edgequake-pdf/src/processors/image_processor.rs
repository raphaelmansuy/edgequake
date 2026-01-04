//! Image extraction processor for PDF documents.
//!
//! This processor extracts embedded images from PDF pages and adds them
//! as Figure/Picture blocks with image data in metadata for later
//! LLM-based OCR processing.
//!
//! # Why as a Processor?
//!
//! Image extraction is separated from the main extraction backend because:
//! 1. It's an optional feature that may not be needed for all use cases
//! 2. It has performance implications (image extraction and encoding is expensive)
//! 3. It keeps the core backend focused on text extraction
//! 4. Configuration (enabled/disabled) can be applied independently

use crate::config::ImageOcrConfig;
use crate::image_extraction::ImageExtractor;
use crate::image_ocr::ImageData;
use crate::schema::{Block, BlockType, BoundingBox, Document};
use crate::Result;
use base64::Engine;
use lopdf::Document as LopdfDocument;
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Processor that extracts images from PDF pages and adds them as blocks.
///
/// This processor should run early in the pipeline, after basic text extraction
/// but before LLM enhancement, so that LlmEnhanceProcessor can describe the images.
pub struct ImageExtractionProcessor {
    config: ImageOcrConfig,
    extractor: ImageExtractor,
}

impl ImageExtractionProcessor {
    /// Create a new image extraction processor with the given configuration.
    pub fn new(config: ImageOcrConfig) -> Self {
        let extractor = ImageExtractor::new(config.clone());
        Self { config, extractor }
    }

    /// Create with default configuration (disabled by default).
    pub fn with_defaults() -> Self {
        Self::new(ImageOcrConfig::default())
    }

    /// Check if image extraction is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Process a document, extracting images from each page and adding them as blocks.
    ///
    /// This requires access to the original PDF bytes to extract images.
    ///
    /// # Arguments
    /// * `document` - The document to process (will be modified)
    /// * `pdf_bytes` - The original PDF file bytes
    ///
    /// # Returns
    /// Ok(()) on success, Err on failure
    pub fn process_document(&self, document: &mut Document, pdf_bytes: &[u8]) -> Result<()> {
        if !self.config.enabled {
            debug!("Image extraction is disabled, skipping");
            return Ok(());
        }

        info!("Extracting images from PDF document");

        // Load the PDF document
        let lopdf_doc = match LopdfDocument::load_mem(pdf_bytes) {
            Ok(doc) => doc,
            Err(e) => {
                warn!("Failed to load PDF for image extraction: {}", e);
                return Ok(()); // Don't fail the whole extraction
            }
        };

        // Process each page
        let page_ids: Vec<_> = lopdf_doc.get_pages().into_iter().collect();
        let mut total_images = 0;

        for page in &mut document.pages {
            let page_num = page.number;

            // Find the page_id for this page number
            if let Some((_page_num, page_id)) =
                page_ids.iter().find(|(n, _)| *n as usize == page_num)
            {
                match self
                    .extractor
                    .extract_page_images(&lopdf_doc, *page_id, page_num)
                {
                    Ok(images) => {
                        let image_count = images.len();
                        if image_count > 0 {
                            debug!("Extracted {} images from page {}", image_count, page_num);

                            // Convert images to blocks
                            for (idx, image_data) in images.into_iter().enumerate() {
                                let block = self.create_image_block(&image_data, page_num, idx);
                                page.blocks.push(block);
                            }

                            total_images += image_count;
                            page.stats.figures = image_count;
                        }
                    }
                    Err(e) => {
                        warn!("Failed to extract images from page {}: {:?}", page_num, e);
                        // Continue with other pages
                    }
                }
            }
        }

        info!(
            "Extracted {} total images from {} pages",
            total_images,
            document.pages.len()
        );

        Ok(())
    }

    /// Create a block for an extracted image.
    ///
    /// The image data is stored in metadata as base64-encoded bytes,
    /// along with dimensions and other properties needed for LLM OCR.
    fn create_image_block(&self, image_data: &ImageData, page_num: usize, index: usize) -> Block {
        let mut metadata = HashMap::new();

        // Store image data as base64
        let image_base64 = base64::engine::general_purpose::STANDARD.encode(&image_data.data);
        metadata.insert(
            "image_data".to_string(),
            serde_json::Value::String(image_base64),
        );

        // Store MIME type
        metadata.insert(
            "image_mime_type".to_string(),
            serde_json::Value::String(image_data.mime_type.clone()),
        );

        // Store dimensions
        metadata.insert(
            "image_width".to_string(),
            serde_json::Value::Number(image_data.width.into()),
        );
        metadata.insert(
            "image_height".to_string(),
            serde_json::Value::Number(image_data.height.into()),
        );

        // Store page and index
        metadata.insert(
            "image_page".to_string(),
            serde_json::Value::Number(page_num.into()),
        );
        metadata.insert(
            "image_index".to_string(),
            serde_json::Value::Number(index.into()),
        );

        // Store bounding box if available
        if let Some((x1, y1, x2, y2)) = image_data.bbox {
            metadata.insert(
                "image_bbox".to_string(),
                serde_json::json!([x1, y1, x2, y2]),
            );
        }

        // Calculate bounding box
        let bbox = if let Some((x1, y1, x2, y2)) = image_data.bbox {
            BoundingBox::new(x1, y1, x2, y2)
        } else {
            // Use image dimensions as bbox (assuming origin at 0,0)
            BoundingBox::new(0.0, 0.0, image_data.width as f32, image_data.height as f32)
        };

        let mut block = Block::new(BlockType::Figure, bbox);
        block.page = page_num;
        block.position = index;
        block.text = String::new(); // Will be filled by LlmEnhanceProcessor
        block.confidence = 0.9;
        block.metadata = metadata;
        block
    }

    /// Get statistics about image extraction for a page.
    pub fn get_page_stats(&self, pdf_bytes: &[u8], page_num: usize) -> Option<ImagePageStats> {
        let lopdf_doc = LopdfDocument::load_mem(pdf_bytes).ok()?;
        let page_ids: Vec<_> = lopdf_doc.get_pages().into_iter().collect();

        if let Some((_, page_id)) = page_ids.iter().find(|(n, _)| *n as usize == page_num) {
            let stats = self.extractor.get_page_image_stats(&lopdf_doc, *page_id);
            Some(ImagePageStats {
                total_images: stats.total_count,
                valid_images: stats.valid_count,
                total_pixels: stats.total_pixels,
            })
        } else {
            None
        }
    }
}

/// Statistics about images on a page.
#[derive(Debug, Clone)]
pub struct ImagePageStats {
    /// Total number of images found.
    pub total_images: usize,
    /// Number of valid/processable images.
    pub valid_images: usize,
    /// Total pixels across all images.
    pub total_pixels: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::Page;

    #[test]
    fn test_processor_disabled_by_default() {
        let processor = ImageExtractionProcessor::with_defaults();
        assert!(!processor.is_enabled());
    }

    #[test]
    fn test_processor_enabled_with_config() {
        let mut config = ImageOcrConfig::default();
        config.enabled = true;
        let processor = ImageExtractionProcessor::new(config);
        assert!(processor.is_enabled());
    }

    #[test]
    fn test_create_image_block() {
        let mut config = ImageOcrConfig::default();
        config.enabled = true;
        let processor = ImageExtractionProcessor::new(config);

        // Create test image data
        let image_data = ImageData {
            data: vec![0x89, 0x50, 0x4E, 0x47], // PNG header
            mime_type: "image/png".to_string(),
            width: 100,
            height: 100,
            page: 1,
            index: 0,
            bbox: Some((10.0, 20.0, 110.0, 120.0)),
        };

        let block = processor.create_image_block(&image_data, 1, 0);

        assert_eq!(block.block_type, BlockType::Figure);
        assert!(block.text.is_empty());
        assert!(block.metadata.contains_key("image_data"));
        assert!(block.metadata.contains_key("image_mime_type"));
        assert!(block.metadata.contains_key("image_width"));
        assert!(block.metadata.contains_key("image_height"));
        assert_eq!(block.bbox.x1, 10.0);
        assert_eq!(block.bbox.y1, 20.0);
    }

    #[test]
    fn test_create_image_block_without_bbox() {
        let mut config = ImageOcrConfig::default();
        config.enabled = true;
        let processor = ImageExtractionProcessor::new(config);

        let image_data = ImageData {
            data: vec![0xFF, 0xD8, 0xFF], // JPEG header
            mime_type: "image/jpeg".to_string(),
            width: 200,
            height: 150,
            page: 2,
            index: 1,
            bbox: None,
        };

        let block = processor.create_image_block(&image_data, 2, 1);

        // Should use image dimensions as bbox
        assert_eq!(block.bbox.x1, 0.0);
        assert_eq!(block.bbox.y1, 0.0);
        assert_eq!(block.bbox.x2, 200.0);
        assert_eq!(block.bbox.y2, 150.0);
    }

    #[test]
    fn test_disabled_processor_skips_extraction() {
        let processor = ImageExtractionProcessor::with_defaults();

        let mut document = Document::default();
        let page = Page::new(1, 612.0, 792.0);
        document.pages.push(page);

        // Should succeed without doing anything
        let result = processor.process_document(&mut document, &[]);
        assert!(result.is_ok());
        assert_eq!(document.pages[0].blocks.len(), 0);
    }
}
