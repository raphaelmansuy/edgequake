//! PDF page rendering for vision mode using pdftoppm (poppler-utils).
//!
//! This module provides functionality to render PDF pages as images
//! for processing with vision LLMs. Uses the external `pdftoppm` command
//! from poppler-utils for reliable, high-quality page rendering.
//!
//! ## Implements
//!
//! - **FEAT1025**: PDF page to image rendering
//! - **FEAT1026**: Configurable DPI and format
//!
//! ## Enforces
//!
//! - **BR1025**: Maximum resolution of 2048px to prevent memory issues
//! - **BR1026**: Support PNG and JPEG formats only
//!
//! ## Use Cases
//!
//! - **UC1025**: Vision LLM needs page images for OCR/extraction
//! - **UC1026**: Scanned PDF requires image rendering
//!
//! ## Dependencies
//!
//! Requires `pdftoppm` from poppler-utils to be installed:
//! - macOS: `brew install poppler`
//! - Ubuntu/Debian: `apt-get install poppler-utils`
//! - Alpine: `apk add poppler-utils`

use crate::error::PdfError;
use crate::vision::{ImageFormat, PageImage};
use crate::Result;

#[cfg(feature = "vision")]
use std::fs;
#[cfg(feature = "vision")]
use std::io::Write;
#[cfg(feature = "vision")]
use std::process::Command;
#[cfg(feature = "vision")]
use tempfile::{NamedTempFile, TempDir};
#[cfg(feature = "vision")]
use tracing::{debug, info};

/// Render PDF pages to images for vision LLM processing.
///
/// Uses pdftoppm (poppler-utils) to render PDF pages at configurable DPI and format.
///
/// # Example
///
/// ```rust,no_run
/// use edgequake_pdf::rendering::PageRenderer;
/// use edgequake_pdf::vision::ImageFormat;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let renderer = PageRenderer::new()?
///     .with_dpi(150)
///     .with_format(ImageFormat::Png);
///
/// let pdf_bytes = std::fs::read("document.pdf")?;
/// let images = renderer.render_pages(&pdf_bytes)?;
///
/// println!("Rendered {} pages", images.len());
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "vision")]
pub struct PageRenderer {
    dpi: u32,
    format: ImageFormat,
}

#[cfg(feature = "vision")]
impl PageRenderer {
    /// Create a new page renderer.
    ///
    /// This verifies that pdftoppm is available on the system.
    ///
    /// # Errors
    ///
    /// Returns `PdfError::Rendering` if pdftoppm is not installed.
    pub fn new() -> Result<Self> {
        info!("Initializing PageRenderer with pdftoppm");

        // Verify pdftoppm is available
        let output = Command::new("pdftoppm").arg("-v").output().map_err(|e| {
            PdfError::Rendering(format!(
                "pdftoppm not found: {}. \
                     Install poppler-utils: brew install poppler (macOS) or \
                     apt-get install poppler-utils (Linux)",
                e
            ))
        })?;

        // pdftoppm -v outputs to stderr, check if command ran
        if !output.status.success() && output.stderr.is_empty() {
            return Err(PdfError::Rendering(
                "pdftoppm failed version check. Ensure poppler-utils is properly installed.".into(),
            ));
        }

        let version = String::from_utf8_lossy(&output.stderr);
        debug!(
            "pdftoppm version: {}",
            version.lines().next().unwrap_or("unknown")
        );

        Ok(Self {
            dpi: 150, // Default DPI for vision mode (good balance of quality/size)
            format: ImageFormat::Png,
        })
    }

    /// Set the DPI for rendering.
    ///
    /// Higher DPI produces larger, more detailed images but increases memory usage
    /// and LLM processing cost. Default is 150 DPI.
    ///
    /// # Recommended Values
    ///
    /// - 72 DPI: Screen resolution, fast but low quality
    /// - 150 DPI: Good balance (default)
    /// - 300 DPI: Print quality, large files
    pub fn with_dpi(mut self, dpi: u32) -> Self {
        self.dpi = dpi;
        self
    }

    /// Set the output image format.
    ///
    /// # Supported Formats
    ///
    /// - `ImageFormat::Png`: Lossless, larger files (recommended)
    /// - `ImageFormat::Jpeg`: Lossy, smaller files
    /// - `ImageFormat::WebP`: Not supported by pdftoppm
    pub fn with_format(mut self, format: ImageFormat) -> Self {
        self.format = format;
        self
    }

    /// Render all pages to images.
    ///
    /// # Arguments
    ///
    /// * `pdf_bytes` - Raw PDF file bytes
    ///
    /// # Returns
    ///
    /// A vector of `PageImage` objects, one per page.
    ///
    /// # Errors
    ///
    /// - `PdfError::Rendering` if PDF cannot be rendered
    /// - `PdfError::Unsupported` if format is not supported (e.g., WebP)
    pub fn render_pages(&self, pdf_bytes: &[u8]) -> Result<Vec<PageImage>> {
        info!(
            "Rendering PDF pages (dpi: {}, format: {:?})",
            self.dpi, self.format
        );

        // Validate format
        if matches!(self.format, ImageFormat::WebP) {
            return Err(PdfError::Unsupported(
                "WebP format is not supported by pdftoppm. Use PNG or JPEG.".into(),
            ));
        }

        // Create temp file for PDF
        let mut temp_pdf = NamedTempFile::new().map_err(|e| {
            PdfError::Rendering(format!("Failed to create temp file for PDF: {}", e))
        })?;
        temp_pdf
            .write_all(pdf_bytes)
            .map_err(|e| PdfError::Rendering(format!("Failed to write PDF to temp file: {}", e)))?;
        temp_pdf
            .flush()
            .map_err(|e| PdfError::Rendering(format!("Failed to flush PDF temp file: {}", e)))?;

        // Create temp directory for output images
        let temp_dir = TempDir::new()
            .map_err(|e| PdfError::Rendering(format!("Failed to create temp directory: {}", e)))?;

        let output_prefix = temp_dir.path().join("page");

        // Build pdftoppm command
        let format_flag = match self.format {
            ImageFormat::Png => "-png",
            ImageFormat::Jpeg => "-jpeg",
            ImageFormat::WebP => unreachable!(), // Validated above
        };

        debug!(
            "Running: pdftoppm {} -r {} {} {}",
            format_flag,
            self.dpi,
            temp_pdf.path().display(),
            output_prefix.display()
        );

        // Run pdftoppm
        let output = Command::new("pdftoppm")
            .arg(format_flag)
            .arg("-r")
            .arg(self.dpi.to_string())
            .arg(temp_pdf.path())
            .arg(&output_prefix)
            .output()
            .map_err(|e| PdfError::Rendering(format!("pdftoppm execution failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PdfError::Rendering(format!(
                "pdftoppm failed (exit code {:?}): {}",
                output.status.code(),
                stderr
            )));
        }

        // Read generated images
        // pdftoppm creates files like: page-1.png, page-2.png, ...
        // or page-01.png, page-02.png, ... depending on page count
        let mut images = Vec::new();

        // Find all generated image files
        let ext = match self.format {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpg",
            ImageFormat::WebP => unreachable!(),
        };

        // Collect and sort image files
        let mut image_files: Vec<_> = fs::read_dir(temp_dir.path())
            .map_err(|e| PdfError::Rendering(format!("Failed to read temp directory: {}", e)))?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().map_or(false, |e| e == ext))
            .collect();

        // Sort by filename to ensure correct page order
        image_files.sort_by(|a, b| a.path().cmp(&b.path()));

        for (page_index, entry) in image_files.iter().enumerate() {
            let path = entry.path();
            debug!("Reading rendered page: {}", path.display());

            let image_data = fs::read(&path).map_err(|e| {
                PdfError::Rendering(format!(
                    "Failed to read rendered page {}: {}",
                    page_index + 1,
                    e
                ))
            })?;

            // Get image dimensions using image crate
            let img = image::load_from_memory(&image_data).map_err(|e| {
                PdfError::Rendering(format!(
                    "Failed to decode image for page {}: {}",
                    page_index + 1,
                    e
                ))
            })?;

            images.push(
                PageImage::new(image_data, img.width(), img.height(), self.format)
                    .with_page(page_index)
                    .with_dpi(self.dpi),
            );

            debug!(
                "Page {} rendered: {}x{} ({} bytes)",
                page_index + 1,
                img.width(),
                img.height(),
                images[page_index].data.len()
            );
        }

        if images.is_empty() {
            return Err(PdfError::Rendering(
                "No pages were rendered. PDF may be empty or corrupted.".into(),
            ));
        }

        info!("Successfully rendered {} pages", images.len());
        Ok(images)
    }

    /// Render a single page to an image.
    ///
    /// # Arguments
    ///
    /// * `pdf_bytes` - Raw PDF file bytes
    /// * `page_number` - Zero-indexed page number
    ///
    /// # Errors
    ///
    /// - `PdfError::Rendering` if PDF cannot be rendered or page doesn't exist
    pub fn render_page(&self, pdf_bytes: &[u8], page_number: usize) -> Result<PageImage> {
        info!(
            "Rendering page {} (dpi: {}, format: {:?})",
            page_number + 1,
            self.dpi,
            self.format
        );

        // For single page, still render all and extract
        // (pdftoppm doesn't efficiently support single page rendering)
        let images = self.render_pages(pdf_bytes)?;
        let total_pages = images.len();

        images
            .into_iter()
            .find(|img| img.page == page_number)
            .ok_or_else(|| {
                PdfError::Rendering(format!(
                    "Page {} not found (document has {} pages)",
                    page_number + 1,
                    total_pages
                ))
            })
    }
}

// Stub implementation when vision feature is disabled
#[cfg(not(feature = "vision"))]
pub struct PageRenderer;

#[cfg(not(feature = "vision"))]
impl PageRenderer {
    /// Create a new page renderer.
    ///
    /// # Errors
    ///
    /// Always returns an error when the `vision` feature is not enabled.
    pub fn new() -> Result<Self> {
        Err(PdfError::Unsupported(
            "Vision mode requires the 'vision' feature flag. \
             Recompile edgequake-pdf with --features vision"
                .into(),
        ))
    }

    /// Stub for render_pages when vision feature is disabled.
    #[allow(unused_variables)]
    pub fn render_pages(&self, pdf_bytes: &[u8]) -> Result<Vec<PageImage>> {
        Err(PdfError::Unsupported(
            "Vision mode requires the 'vision' feature flag".into(),
        ))
    }

    /// Stub for render_page when vision feature is disabled.
    #[allow(unused_variables)]
    pub fn render_page(&self, pdf_bytes: &[u8], page_number: usize) -> Result<PageImage> {
        Err(PdfError::Unsupported(
            "Vision mode requires the 'vision' feature flag".into(),
        ))
    }

    /// Stub for with_dpi when vision feature is disabled.
    #[allow(unused_variables)]
    pub fn with_dpi(self, dpi: u32) -> Self {
        self
    }

    /// Stub for with_format when vision feature is disabled.
    #[allow(unused_variables)]
    pub fn with_format(self, format: ImageFormat) -> Self {
        self
    }
}

#[cfg(test)]
#[cfg(feature = "vision")]
mod tests {
    use super::*;

    #[test]
    fn test_page_renderer_creation() {
        // This test requires pdftoppm to be installed
        let result = PageRenderer::new();
        if result.is_err() {
            println!("pdftoppm not available, skipping test. Install poppler-utils.");
            return;
        }

        let renderer = result.unwrap();
        assert_eq!(renderer.dpi, 150);
        assert!(matches!(renderer.format, ImageFormat::Png));
    }

    #[test]
    fn test_page_renderer_builder() {
        let result = PageRenderer::new();
        if result.is_err() {
            println!("pdftoppm not available, skipping test");
            return;
        }

        let renderer = result.unwrap().with_dpi(300).with_format(ImageFormat::Jpeg);

        assert_eq!(renderer.dpi, 300);
        assert!(matches!(renderer.format, ImageFormat::Jpeg));
    }

    #[test]
    fn test_webp_not_supported() {
        let result = PageRenderer::new();
        if result.is_err() {
            println!("pdftoppm not available, skipping test");
            return;
        }

        let renderer = result.unwrap().with_format(ImageFormat::WebP);

        // Create a minimal PDF
        let pdf_bytes = b"%PDF-1.4\n1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n3 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R>>endobj\nxref\n0 4\n0000000000 65535 f \n0000000009 00000 n \n0000000052 00000 n \n0000000101 00000 n \ntrailer<</Size 4/Root 1 0 R>>\nstartxref\n171\n%%EOF";

        let result = renderer.render_pages(pdf_bytes);
        assert!(result.is_err());
        if let Err(PdfError::Unsupported(msg)) = result {
            assert!(msg.contains("WebP"));
        }
    }
}

#[cfg(test)]
#[cfg(not(feature = "vision"))]
mod stub_tests {
    use super::*;

    #[test]
    fn test_stub_returns_error() {
        let result = PageRenderer::new();
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, PdfError::Unsupported(_)));
        }
    }
}
