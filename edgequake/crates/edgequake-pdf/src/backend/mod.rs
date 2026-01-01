use crate::extractor::PdfInfo;
use crate::schema::Document;
use crate::Result;
use async_trait::async_trait;

/// Trait for PDF extraction backends.
///
/// This trait abstracts the underlying PDF engine (e.g., Pdfium, lopdf, etc.)
/// allowing for swappable backends and easier testing.
#[async_trait]
pub trait PdfBackend: Send + Sync {
    /// Extract the raw document structure from PDF bytes.
    async fn extract(&self, pdf_bytes: &[u8]) -> Result<Document>;

    /// Get metadata/info about the PDF without full extraction.
    fn get_info(&self, pdf_bytes: &[u8]) -> Result<PdfInfo>;
}

// SOTA backend (based on lopdf) removed — lopdf dependency deleted.
// Keep mock backend for tests and development.
// pub mod sota_backend; // removed

/// Mock backend for testing
pub mod mock;
