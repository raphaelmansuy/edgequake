//! Error types for PDF processing operations.
//!
//! # Error Recovery Model
//!
//! PDF extraction uses a **graceful degradation** approach:
//! - Individual page failures don't stop document extraction
//! - Recoverable errors allow partial results with warnings
//! - Non-recoverable errors (e.g., encrypted PDF) fail fast
//!
//! ## WHY: Graceful Degradation
//! Real-world PDFs often have malformed pages, unsupported encodings, or
//! corrupt objects. Failing the entire document for one bad page loses
//! valuable content. Instead, we extract what we can and track errors.

use thiserror::Error;

/// Errors that can occur during PDF processing.
#[derive(Error, Debug, Clone)]
pub enum PdfError {
    #[error("PDF parsing error: {0}")]
    PdfParse(String),

    #[error("AI processing error: {0}")]
    AiProcessing(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Image processing error: {0}")]
    ImageProcessing(String),

    #[error("OCR processing error: {0}")]
    OcrProcessing(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Processor error: {0}")]
    Processor(String),

    #[error("Unsupported operation: {0}")]
    Unsupported(String),

    #[error("Page extraction failed: page {page}: {message}")]
    PageExtraction { page: usize, message: String },

    #[error("Encrypted PDF: password required")]
    Encrypted,

    #[error("Font decoding error: {0}")]
    FontDecoding(String),
}

impl From<std::io::Error> for PdfError {
    fn from(err: std::io::Error) -> Self {
        PdfError::Io(err.to_string())
    }
}

impl PdfError {
    /// Returns `true` if extraction can continue despite this error.
    ///
    /// # WHY: Recovery Classification
    /// Some errors are recoverable (one bad page) while others are fatal
    /// (encrypted PDF, corrupt file header). This method enables the
    /// extraction engine to make continue/abort decisions.
    ///
    /// Recoverable:
    /// - Individual page extraction failures
    /// - Font decoding issues (fallback to raw bytes)
    /// - Image processing failures
    /// - OCR failures
    ///
    /// Non-recoverable:
    /// - Encrypted PDFs
    /// - PDF parsing failures (corrupt structure)
    /// - IO errors (file not readable)
    /// - Configuration errors
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            PdfError::PageExtraction { .. }
                | PdfError::FontDecoding(_)
                | PdfError::ImageProcessing(_)
                | PdfError::OcrProcessing(_)
                | PdfError::Processor(_)
        )
    }

    /// Creates a page extraction error.
    pub fn page_error(page: usize, message: impl Into<String>) -> Self {
        PdfError::PageExtraction {
            page,
            message: message.into(),
        }
    }
}

/// Error for a specific page during extraction.
///
/// Used to track which pages failed and why, while allowing
/// extraction to continue for other pages.
#[derive(Debug, Clone)]
pub struct PageError {
    /// The page number that failed (1-indexed for user display)
    pub page: usize,
    /// The error that occurred
    pub error: PdfError,
    /// Whether this page was partially extracted before failure
    pub partial_content: Option<String>,
}

impl PageError {
    /// Create a new page error.
    pub fn new(page: usize, error: PdfError) -> Self {
        Self {
            page,
            error,
            partial_content: None,
        }
    }

    /// Create a page error with partial content that was recovered.
    pub fn with_partial(page: usize, error: PdfError, content: String) -> Self {
        Self {
            page,
            error,
            partial_content: Some(content),
        }
    }
}

impl std::fmt::Display for PageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Page {}: {}", self.page, self.error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recoverable_errors() {
        assert!(PdfError::page_error(1, "bad encoding").is_recoverable());
        assert!(PdfError::FontDecoding("unknown font".into()).is_recoverable());
        assert!(PdfError::ImageProcessing("corrupt image".into()).is_recoverable());
        assert!(PdfError::OcrProcessing("tesseract failed".into()).is_recoverable());
        assert!(PdfError::Processor("chain error".into()).is_recoverable());
    }

    #[test]
    fn test_non_recoverable_errors() {
        assert!(!PdfError::Encrypted.is_recoverable());
        assert!(!PdfError::PdfParse("corrupt header".into()).is_recoverable());
        assert!(!PdfError::Io("file not found".into()).is_recoverable());
        assert!(!PdfError::Config("invalid config".into()).is_recoverable());
    }

    #[test]
    fn test_page_error_display() {
        let err = PageError::new(5, PdfError::FontDecoding("CID mapping missing".into()));
        assert!(err.to_string().contains("Page 5"));
        assert!(err.to_string().contains("CID mapping"));
    }

    #[test]
    fn test_page_error_with_partial() {
        let err = PageError::with_partial(
            3,
            PdfError::FontDecoding("partial decode".into()),
            "Some text recovered".into(),
        );
        assert_eq!(err.page, 3);
        assert!(err.partial_content.is_some());
        assert_eq!(err.partial_content.unwrap(), "Some text recovered");
    }
}
