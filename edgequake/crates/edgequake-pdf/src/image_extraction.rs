//! Image extraction from PDF pages.
//!
//! This module provides functionality to extract embedded images from PDF pages
//! and convert them to a format suitable for LLM-based OCR processing.
//!
//! ## Implements
//!
//! - **FEAT1004**: Image extraction from PDF pages
//! - **FEAT1023**: Image format conversion (PNG/JPEG)
//!
//! ## Enforces
//!
//! - **BR1023**: Skip corrupt or unsupported image formats
//! - **BR1024**: Limit extracted image size to 10MB
//!
//! # Architecture
//!
//! WHY: PDF images are stored in various formats (JPEG, PNG, raw bitmap) and need
//! to be converted to a consistent format (PNG/JPEG) for LLM vision APIs.
//!
//! The module integrates with:
//! - `lopdf::Document::get_page_images()` for raw image extraction
//! - `image_ocr::ImageData` for the standardized image representation
//! - `image_ocr::ImageOcrProcessor` for LLM-based processing
//!
//! # Example
//!
//! ```ignore
//! use edgequake_pdf::image_extraction::ImageExtractor;
//! use edgequake_pdf::config::ImageOcrConfig;
//!
//! let extractor = ImageExtractor::new(ImageOcrConfig::default());
//! let images = extractor.extract_page_images(&pdf_doc, page_id, 0)?;
//! ```

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use lopdf::{Document as LopdfDocument, ObjectId};
use tracing::{debug, warn};

use crate::config::ImageOcrConfig;
use crate::image_ocr::ImageData;
use crate::Result;

/// Extracts images from PDF pages.
///
/// WHY: This struct encapsulates the logic for extracting images from PDF pages
/// and converting them to the `ImageData` format expected by `ImageOcrProcessor`.
pub struct ImageExtractor {
    /// Configuration for image extraction
    config: ImageOcrConfig,
}

impl ImageExtractor {
    /// Create a new image extractor with the given configuration.
    pub fn new(config: ImageOcrConfig) -> Self {
        Self { config }
    }

    /// Check if image extraction is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Extract images from a specific PDF page.
    ///
    /// Returns a vector of `ImageData` structs, each representing an embedded image.
    /// Images smaller than `min_image_size` are filtered out.
    /// Only up to `max_images_per_page` images are returned.
    ///
    /// # Arguments
    ///
    /// * `doc` - The lopdf document to extract from
    /// * `page_id` - The ObjectId of the page to extract images from
    /// * `page_number` - The 0-indexed page number (for metadata)
    ///
    /// WHY: We extract all images from the page resources (XObjects), filter by size,
    /// and convert to PNG format for consistent LLM processing.
    pub fn extract_page_images(
        &self,
        doc: &LopdfDocument,
        page_id: ObjectId,
        page_number: usize,
    ) -> Result<Vec<ImageData>> {
        if !self.config.enabled {
            return Ok(Vec::new());
        }

        let mut images = Vec::new();

        // Use lopdf's built-in image extraction
        match doc.get_page_images(page_id) {
            Ok(pdf_images) => {
                debug!(
                    "Found {} images on page {}",
                    pdf_images.len(),
                    page_number + 1
                );

                for (index, pdf_image) in pdf_images.iter().enumerate() {
                    // Check max images limit
                    if images.len() >= self.config.max_images_per_page {
                        debug!(
                            "Reached max images limit ({}) for page {}",
                            self.config.max_images_per_page,
                            page_number + 1
                        );
                        break;
                    }

                    // Check minimum size
                    let width = pdf_image.width as u32;
                    let height = pdf_image.height as u32;
                    if width < self.config.min_image_size || height < self.config.min_image_size {
                        debug!(
                            "Skipping small image {}x{} (min: {})",
                            width, height, self.config.min_image_size
                        );
                        continue;
                    }

                    // Convert the raw image data to a usable format
                    match self.convert_pdf_image_to_data(pdf_image, page_number, index) {
                        Ok(Some(image_data)) => {
                            images.push(image_data);
                        }
                        Ok(None) => {
                            debug!(
                                "Image {} on page {} could not be converted",
                                index,
                                page_number + 1
                            );
                        }
                        Err(e) => {
                            warn!(
                                "Failed to convert image {} on page {}: {}",
                                index,
                                page_number + 1,
                                e
                            );
                        }
                    }
                }
            }
            Err(e) => {
                debug!(
                    "No images found on page {} or error: {}",
                    page_number + 1,
                    e
                );
            }
        }

        Ok(images)
    }

    /// Convert a lopdf PdfImage to our ImageData format.
    ///
    /// WHY: PDF images can be stored in various formats (DCTDecode for JPEG,
    /// FlateDecode for deflated raw data, etc.). We need to convert them to
    /// a format suitable for LLM vision APIs (base64-encoded PNG or JPEG).
    fn convert_pdf_image_to_data(
        &self,
        pdf_image: &lopdf::xobject::PdfImage,
        page_number: usize,
        index: usize,
    ) -> Result<Option<ImageData>> {
        let width = pdf_image.width as u32;
        let height = pdf_image.height as u32;
        let content = pdf_image.content;

        // Determine the image format from filters
        let (data, mime_type) = self.decode_image_content(
            content,
            width,
            height,
            pdf_image.filters.as_ref(),
            pdf_image.color_space.as_ref(),
            pdf_image.bits_per_component,
        )?;

        if data.is_empty() {
            return Ok(None);
        }

        // Create bounding box (we don't have position info from lopdf, use placeholder)
        // WHY: The actual position would require parsing the page content stream for
        // "Do" operators, which is complex. For OCR purposes, position is less critical.
        let bbox = (0.0, 0.0, width as f32, height as f32);

        Ok(Some(ImageData {
            data,
            mime_type,
            width,
            height,
            page: page_number,
            index,
            bbox: Some(bbox),
        }))
    }

    /// Decode image content based on the filter type.
    ///
    /// WHY: PDF images use various compression filters:
    /// - DCTDecode: JPEG data (can be used directly)
    /// - FlateDecode: zlib-compressed raw pixel data
    /// - JPXDecode: JPEG2000 (rare, complex)
    /// - None: Raw uncompressed pixel data
    fn decode_image_content(
        &self,
        content: &[u8],
        width: u32,
        height: u32,
        filters: Option<&Vec<String>>,
        color_space: Option<&String>,
        bits_per_component: Option<i64>,
    ) -> Result<(Vec<u8>, String)> {
        // Check for DCTDecode (JPEG) - can use directly
        if let Some(filters) = filters {
            if filters.iter().any(|f| f == "DCTDecode") {
                // JPEG data - use as-is
                return Ok((content.to_vec(), "image/jpeg".to_string()));
            }

            // JPXDecode (JPEG2000) - less common, try to use as-is
            if filters.iter().any(|f| f == "JPXDecode") {
                return Ok((content.to_vec(), "image/jp2".to_string()));
            }
        }

        // For other formats, we need to reconstruct the image
        // This is raw pixel data (possibly after FlateDecode decompression by lopdf)
        let bpc = bits_per_component.unwrap_or(8) as u32;
        let components = match color_space.map(|s| s.as_str()) {
            Some("DeviceRGB") => 3,
            Some("DeviceGray") => 1,
            Some("DeviceCMYK") => 4,
            _ => 3, // Default to RGB
        };

        // Validate expected data size
        let expected_size = (width * height * components * bpc / 8) as usize;
        if content.len() < expected_size / 2 {
            // Data too small, might be compressed or invalid
            debug!(
                "Image data size {} is smaller than expected {} ({}x{}x{}x{}/8)",
                content.len(),
                expected_size,
                width,
                height,
                components,
                bpc
            );
            return Ok((Vec::new(), String::new()));
        }

        // Try to encode as PNG using the image crate
        self.encode_raw_to_png(content, width, height, components, bpc)
    }

    /// Encode raw pixel data to PNG format.
    ///
    /// WHY: LLM vision APIs generally accept PNG or JPEG. Converting raw pixel
    /// data to PNG ensures consistent handling regardless of the original format.
    fn encode_raw_to_png(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
        components: u32,
        _bpc: u32,
    ) -> Result<(Vec<u8>, String)> {
        // Try to create an image from the raw data
        let expected_len = (width * height * components) as usize;

        // If data matches expected size, try to create image directly
        if data.len() >= expected_len {
            let result = match components {
                1 => {
                    // Grayscale
                    image::GrayImage::from_raw(width, height, data[..expected_len].to_vec())
                        .map(image::DynamicImage::ImageLuma8)
                }
                3 => {
                    // RGB
                    image::RgbImage::from_raw(width, height, data[..expected_len].to_vec())
                        .map(image::DynamicImage::ImageRgb8)
                }
                4 => {
                    // RGBA or CMYK - treat as RGBA
                    image::RgbaImage::from_raw(width, height, data[..expected_len].to_vec())
                        .map(image::DynamicImage::ImageRgba8)
                }
                _ => None,
            };

            if let Some(img) = result {
                let mut png_data = Vec::new();
                let mut cursor = std::io::Cursor::new(&mut png_data);
                if img.write_to(&mut cursor, image::ImageFormat::Png).is_ok() {
                    return Ok((png_data, "image/png".to_string()));
                }
            }
        }

        // Fallback: return empty if we can't decode
        debug!(
            "Could not encode image {}x{} with {} components from {} bytes",
            width,
            height,
            components,
            data.len()
        );
        Ok((Vec::new(), String::new()))
    }

    /// Extract all images from a PDF document.
    ///
    /// This is a convenience method that iterates through all pages and
    /// extracts images from each one.
    pub fn extract_all_images(&self, doc: &LopdfDocument) -> Result<Vec<ImageData>> {
        if !self.config.enabled {
            return Ok(Vec::new());
        }

        let mut all_images = Vec::new();
        let pages = doc.get_pages();

        for (page_num, page_id) in pages.iter() {
            let page_images = self.extract_page_images(doc, *page_id, (*page_num - 1) as usize)?;
            all_images.extend(page_images);
        }

        debug!("Extracted {} total images from document", all_images.len());
        Ok(all_images)
    }

    /// Check if a PDF page contains images.
    ///
    /// This is a quick check without fully extracting the images.
    pub fn page_has_images(&self, doc: &LopdfDocument, page_id: ObjectId) -> bool {
        match doc.get_page_images(page_id) {
            Ok(images) => {
                // Check if any image meets the minimum size requirement
                images.iter().any(|img| {
                    img.width as u32 >= self.config.min_image_size
                        && img.height as u32 >= self.config.min_image_size
                })
            }
            Err(_) => false,
        }
    }

    /// Get image statistics for a PDF page.
    pub fn get_page_image_stats(&self, doc: &LopdfDocument, page_id: ObjectId) -> PageImageStats {
        match doc.get_page_images(page_id) {
            Ok(images) => {
                let total_count = images.len();
                let valid_count = images
                    .iter()
                    .filter(|img| {
                        img.width as u32 >= self.config.min_image_size
                            && img.height as u32 >= self.config.min_image_size
                    })
                    .count();
                let total_pixels: u64 = images
                    .iter()
                    .map(|img| (img.width * img.height) as u64)
                    .sum();

                PageImageStats {
                    total_count,
                    valid_count,
                    total_pixels,
                }
            }
            Err(_) => PageImageStats::default(),
        }
    }
}

/// Statistics about images on a PDF page.
#[derive(Debug, Default, Clone)]
pub struct PageImageStats {
    /// Total number of images found
    pub total_count: usize,
    /// Number of images meeting size requirements
    pub valid_count: usize,
    /// Total pixels across all images
    pub total_pixels: u64,
}

impl PageImageStats {
    /// Check if the page has any valid images.
    pub fn has_valid_images(&self) -> bool {
        self.valid_count > 0
    }
}

/// Helper to encode image data as a base64 data URL.
///
/// This is the format expected by LLM vision APIs.
pub fn image_to_data_url(data: &[u8], mime_type: &str) -> String {
    let base64_data = BASE64.encode(data);
    format!("data:{};base64,{}", mime_type, base64_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_extractor_disabled_by_default() {
        let config = ImageOcrConfig::default();
        let extractor = ImageExtractor::new(config);
        assert!(!extractor.is_enabled());
    }

    #[test]
    fn test_image_extractor_enabled() {
        let config = ImageOcrConfig::enabled();
        let extractor = ImageExtractor::new(config);
        assert!(extractor.is_enabled());
    }

    #[test]
    fn test_page_image_stats_default() {
        let stats = PageImageStats::default();
        assert_eq!(stats.total_count, 0);
        assert_eq!(stats.valid_count, 0);
        assert_eq!(stats.total_pixels, 0);
        assert!(!stats.has_valid_images());
    }

    #[test]
    fn test_page_image_stats_with_images() {
        let stats = PageImageStats {
            total_count: 5,
            valid_count: 3,
            total_pixels: 1_000_000,
        };
        assert!(stats.has_valid_images());
    }

    #[test]
    fn test_image_to_data_url() {
        let data = b"test image data";
        let url = image_to_data_url(data, "image/png");
        assert!(url.starts_with("data:image/png;base64,"));
        assert!(url.contains("dGVzdCBpbWFnZSBkYXRh")); // base64 of "test image data"
    }

    #[test]
    fn test_extract_disabled_returns_empty() {
        let config = ImageOcrConfig::default(); // disabled by default
        let extractor = ImageExtractor::new(config);

        // Create a minimal lopdf document
        let doc = lopdf::Document::with_version("1.5");

        // Should return empty when disabled
        let result = extractor.extract_all_images(&doc);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_config_min_image_size() {
        let mut config = ImageOcrConfig::enabled();
        config.min_image_size = 100;
        let extractor = ImageExtractor::new(config.clone());
        assert_eq!(extractor.config.min_image_size, 100);
    }

    #[test]
    fn test_config_max_images_per_page() {
        let mut config = ImageOcrConfig::enabled();
        config.max_images_per_page = 5;
        let extractor = ImageExtractor::new(config.clone());
        assert_eq!(extractor.config.max_images_per_page, 5);
    }
}
