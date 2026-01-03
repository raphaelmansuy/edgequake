use crate::extractor::PdfInfo;
use crate::schema::Document;
use crate::Result;
use async_trait::async_trait;

/// Trait for PDF extraction backends.
///
/// This trait abstracts the underlying PDF engine (e.g., lopdf, etc.)
/// allowing for swappable backends and easier testing.
#[async_trait]
pub trait PdfBackend: Send + Sync {
    /// Extract the raw document structure from PDF bytes.
    async fn extract(&self, pdf_bytes: &[u8]) -> Result<Document>;

    /// Get metadata/info about the PDF without full extraction.
    fn get_info(&self, pdf_bytes: &[u8]) -> Result<PdfInfo>;
}

pub mod elements;
#[cfg(feature = "lopdf")]
pub mod encodings;
#[cfg(feature = "lopdf")]
pub mod lattice;
pub mod mock;
#[cfg(feature = "lopdf")]
pub mod sota_backend;
#[cfg(feature = "lopdf")]
pub mod text_grouping;

pub use mock::MockBackend;
#[cfg(feature = "lopdf")]
pub use sota_backend::SotaBackend;
