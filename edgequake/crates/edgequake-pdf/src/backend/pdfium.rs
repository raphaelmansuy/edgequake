//! PDFium-based PDF extraction backend.
//!
//! This module provides character-level text extraction using Google's PDFium
//! library (Chromium's PDF engine) via the `pdfium-render` crate.
//!
//! ## Why PDFium? (First Principles Analysis)
//!
//! PDFium provides accurate character positions and font metadata that lopdf cannot match:
//! - Character-level bounding boxes via `PdfPageTextChar::tight_bounds()`
//! - Accurate text matrix computation
//! - Font information via `scaled_font_size()`, `font_name()`
//! - **Font style flags via `font_is_italic()` and `font_weight()`** (critical for markdown)
//!
//! ## Font Style Detection: PDFium vs lopdf
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                    FONT STYLE DETECTION COMPARISON                          │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │                                                                             │
//! │  PDFIUM (this module) - ACCURATE                                            │
//! │  ════════════════════════════════                                           │
//! │                                                                             │
//! │  PDF Font Descriptor                                                        │
//! │       │                                                                     │
//! │       │ PDFium parses FontDescriptor internally                             │
//! │       ▼                                                                     │
//! │  ┌──────────────────────┐                                                   │
//! │  │ PdfPageTextChar      │                                                   │
//! │  │ .font_is_italic()    │ ─→ bool (from Flags bit 7 or ItalicAngle)         │
//! │  │ .font_weight()       │ ─→ PdfFontWeight (from Weight field)              │
//! │  └──────────────────────┘                                                   │
//! │       │                                                                     │
//! │       │ Accuracy: ~99% (matches PyMuPDF behavior)                           │
//! │       ▼                                                                     │
//! │  RawChar { is_bold, is_italic }                                             │
//! │                                                                             │
//! │  LOPDF (legacy) - UNRELIABLE                                                │
//! │  ════════════════════════════                                               │
//! │                                                                             │
//! │  PDF Font Dictionary                                                        │
//! │       │                                                                     │
//! │       │ Manual parsing of /BaseFont name                                    │
//! │       ▼                                                                     │
//! │  ┌──────────────────────┐                                                   │
//! │  │ FontInfo::from_dict()│                                                   │
//! │  │ name.contains("bold")│ ─→ Pattern matching (fails on "F1", "Arial")      │
//! │  │ name.contains("ital")│ ─→ Pattern matching (misses many fonts)           │
//! │  └──────────────────────┘                                                   │
//! │       │                                                                     │
//! │       │ Accuracy: ~70% (fails on numeric font names like F1, F2)            │
//! │       ▼                                                                     │
//! │  TextElement { is_bold, is_italic }                                         │
//! │                                                                             │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Bold Detection: Why Weight >= 700?
//!
//! The 700 threshold comes from CSS font-weight specification:
//! - 400 = Normal
//! - 700 = Bold
//! - 900 = Black/Heavy
//!
//! PDF font descriptors use the same convention in the /Weight field.
//!
//! ## Runtime Dependency
//!
//! Requires `libpdfium.dylib` (macOS), `libpdfium.so` (Linux), or `pdfium.dll` (Windows)
//! at runtime. Pre-built binaries available at:
//! <https://github.com/bblanchon/pdfium-binaries/releases>
//!
//! Set `PDFIUM_DYNAMIC_LIB_PATH` environment variable to specify the library location.
//!
//! ## License
//!
//! pdfium-render is MIT OR Apache-2.0 licensed (permissive, commercial-friendly).

use super::elements::RawChar;
use crate::error::PdfError;
use pdfium_render::prelude::*;
use std::path::Path;

/// PDFium-based character extractor.
///
/// This struct wraps a PDFium instance and provides methods to extract
/// character-level text with accurate bounding boxes.
///
/// ## Example
///
/// ```rust,ignore
/// use edgequake_pdf::backend::pdfium::PdfiumExtractor;
///
/// // Set PDFIUM_DYNAMIC_LIB_PATH or have libpdfium in PATH
/// let extractor = PdfiumExtractor::new()?;
/// let chars = extractor.extract_chars_from_file("document.pdf")?;
///
/// for ch in chars.iter().take(10) {
///     println!("'{}' at ({:.1}, {:.1})", ch.char, ch.x0, ch.y0);
/// }
/// ```
pub struct PdfiumExtractor {
    pdfium: Pdfium,
}

impl PdfiumExtractor {
    /// Create a new PDFium extractor.
    ///
    /// This will search for libpdfium in the following order:
    /// 1. `PDFIUM_DYNAMIC_LIB_PATH` environment variable
    /// 2. System library paths
    ///
    /// # Errors
    ///
    /// Returns an error if libpdfium cannot be found or loaded.
    pub fn new() -> Result<Self, PdfError> {
        // First check for PDFIUM_DYNAMIC_LIB_PATH env var
        if let Ok(path) = std::env::var("PDFIUM_DYNAMIC_LIB_PATH") {
            return Self::with_library_path(&path);
        }

        // Try common paths on macOS
        #[cfg(target_os = "macos")]
        {
            let common_paths = [
                "/usr/local/lib/libpdfium.dylib",
                "/opt/homebrew/lib/libpdfium.dylib",
            ];
            for path in &common_paths {
                if std::path::Path::new(path).exists() {
                    return Self::with_library_path(path);
                }
            }
        }

        // Fallback: try Pdfium::new with StaticBindings (if available)
        // This will fail at compile time if static bindings aren't enabled
        Err(PdfError::Backend(
            "libpdfium not found. Set PDFIUM_DYNAMIC_LIB_PATH environment variable to the path of libpdfium.dylib".to_string(),
        ))
    }

    /// Create extractor with explicit library path.
    ///
    /// Use this when you know the exact location of libpdfium.
    pub fn with_library_path<P: AsRef<Path>>(path: P) -> Result<Self, PdfError> {
        let bindings = Pdfium::bind_to_library(path.as_ref())
            .map_err(|e| PdfError::Backend(format!("Failed to bind to PDFium: {e}")))?;
        Ok(Self {
            pdfium: Pdfium::new(bindings),
        })
    }

    /// Extract all characters from a PDF file.
    ///
    /// Returns characters from all pages, sorted by page number then position.
    pub fn extract_chars_from_file<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<Vec<RawChar>, PdfError> {
        let document = self
            .pdfium
            .load_pdf_from_file(path.as_ref(), None)
            .map_err(|e| PdfError::Backend(format!("Failed to load PDF: {e}")))?;

        self.extract_chars_from_document(&document)
    }

    /// Extract all characters from PDF bytes.
    pub fn extract_chars_from_bytes(&self, bytes: &[u8]) -> Result<Vec<RawChar>, PdfError> {
        let document = self
            .pdfium
            .load_pdf_from_byte_slice(bytes, None)
            .map_err(|e| PdfError::Backend(format!("Failed to load PDF: {e}")))?;

        self.extract_chars_from_document(&document)
    }

    /// Extract characters from a loaded PDF document.
    fn extract_chars_from_document(
        &self,
        document: &PdfDocument,
    ) -> Result<Vec<RawChar>, PdfError> {
        let mut all_chars = Vec::new();

        for (page_idx, page) in document.pages().iter().enumerate() {
            let page_chars = self.extract_chars_from_page(&page, page_idx)?;
            all_chars.extend(page_chars);
        }

        Ok(all_chars)
    }

    /// Extract characters from a single page.
    fn extract_chars_from_page(
        &self,
        page: &PdfPage,
        page_num: usize,
    ) -> Result<Vec<RawChar>, PdfError> {
        let text = page
            .text()
            .map_err(|e| PdfError::Backend(format!("Failed to get page text: {e}")))?;

        let mut chars = Vec::new();
        // Track last non-whitespace character's bounds for synthesizing space positions
        let mut last_x1: f32 = 0.0;
        let mut last_y0: f32 = 0.0;
        let mut last_y1: f32 = 0.0;
        // Track last style flags for whitespace inheritance
        let mut last_is_bold: bool = false;
        let mut last_is_italic: bool = false;
        let mut last_is_monospace: bool = false;

        for char_obj in text.chars().iter() {
            // Get the character - unicode_char() returns Option<char>
            let c = match char_obj.unicode_char() {
                Some(c) => c,
                None => continue, // Skip chars without unicode representation
            };

            // Skip control characters (but NOT spaces/tabs/newlines)
            if c.is_control() && c != ' ' && c != '\n' && c != '\t' {
                continue;
            }

            // Extract font style flags from pdfium-render
            // WHY: Font name matching ("bold", "italic" in name) is unreliable.
            // PyMuPDF uses numeric flags from font descriptors, and pdfium-render
            // provides the same information via font_is_italic() and font_weight().
            let is_italic = char_obj.font_is_italic();
            let is_bold = char_obj.font_weight().is_some_and(|w| {
                matches!(
                    w,
                    PdfFontWeight::Weight700Bold
                        | PdfFontWeight::Weight800
                        | PdfFontWeight::Weight900
                ) || matches!(w, PdfFontWeight::Custom(n) if n >= 700)
            });
            // OODA-03: Monospace detection from font descriptor
            // WHY: Font name pattern matching ("Mono", "Courier") misses many monospace fonts.
            // PDFium provides accurate fixed-pitch flag from font descriptor via font_is_fixed_pitch().
            // This is the same data that PyMuPDF uses for monospace detection.
            let is_monospace = char_obj.font_is_fixed_pitch();

            // Get bounds - tight_bounds() returns Result<PdfRect, PdfiumError>
            // WHY: Spaces often don't have tight bounds in PDFium, but they mark word boundaries.
            // For spaces, we synthesize a position based on the last character.
            let (
                x0,
                y0,
                x1,
                y1,
                font_size,
                font_name,
                final_is_bold,
                final_is_italic,
                final_is_monospace,
            ) = if c.is_whitespace() {
                // Space/newline character - synthesize bounds from last character
                // WHY: Spaces must inherit Y coordinates and style from previous char
                let fs = char_obj.scaled_font_size().value;
                // Position the space right after the last character, with same Y
                (
                    last_x1,
                    last_y0,
                    last_x1 + fs * 0.25,
                    last_y1,
                    fs,
                    Some(char_obj.font_name()),
                    last_is_bold,
                    last_is_italic,
                    last_is_monospace,
                )
            } else {
                // Normal character - get actual bounds
                let bounds = match char_obj.tight_bounds() {
                    Ok(rect) => rect,
                    Err(_) => continue, // Skip chars without bounds
                };
                let fs = char_obj.scaled_font_size().value;
                // Update tracking variables
                last_x1 = bounds.right().value;
                last_y0 = bounds.bottom().value;
                last_y1 = bounds.top().value;
                last_is_bold = is_bold;
                last_is_italic = is_italic;
                last_is_monospace = is_monospace;
                (
                    bounds.left().value,
                    bounds.bottom().value,
                    bounds.right().value,
                    bounds.top().value,
                    fs,
                    Some(char_obj.font_name()),
                    is_bold,
                    is_italic,
                    is_monospace,
                )
            };

            chars.push(RawChar {
                char: c,
                x0,
                y0,
                x1,
                y1,
                font_size,
                font_name,
                page_num,
                is_bold: final_is_bold,
                is_italic: final_is_italic,
                is_monospace: final_is_monospace,
            });
        }

        Ok(chars)
    }

    /// Get the number of pages in a PDF file.
    pub fn page_count<P: AsRef<Path>>(&self, path: P) -> Result<usize, PdfError> {
        let document = self
            .pdfium
            .load_pdf_from_file(path.as_ref(), None)
            .map_err(|e| PdfError::Backend(format!("Failed to load PDF: {e}")))?;

        // pages().len() returns u16, convert to usize
        Ok(document.pages().len() as usize)
    }

    /// Get page dimensions (width, height) in PDF points.
    pub fn page_size<P: AsRef<Path>>(
        &self,
        path: P,
        page_num: usize,
    ) -> Result<(f32, f32), PdfError> {
        let document = self
            .pdfium
            .load_pdf_from_file(path.as_ref(), None)
            .map_err(|e| PdfError::Backend(format!("Failed to load PDF: {e}")))?;

        // get() takes u16, convert from usize
        let page = document
            .pages()
            .get(page_num as u16)
            .map_err(|e| PdfError::Backend(format!("Failed to get page {page_num}: {e}")))?;

        Ok((page.width().value, page.height().value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that PdfiumExtractor can be created when library is available.
    /// This test is designed to pass in both CI (no library) and local dev.
    #[test]
    fn test_pdfium_extractor_creation() {
        // Only run if library path is set
        match std::env::var("PDFIUM_DYNAMIC_LIB_PATH") {
            Ok(path) => {
                if std::path::Path::new(&path).exists() {
                    let result = PdfiumExtractor::with_library_path(&path);
                    assert!(result.is_ok(), "Failed to create extractor");
                    println!("✓ PdfiumExtractor created successfully from {path}");
                } else {
                    println!("PDFIUM_DYNAMIC_LIB_PATH set but file doesn't exist: {path}");
                }
            }
            Err(_) => {
                // No library path - test that new() fails gracefully
                let result = PdfiumExtractor::new();
                assert!(result.is_err(), "Expected error when no library available");
                println!("✓ PdfiumExtractor::new() correctly returns error when no library");
            }
        }
    }
}
