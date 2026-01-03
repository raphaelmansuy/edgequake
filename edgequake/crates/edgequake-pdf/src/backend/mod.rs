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

#[cfg(feature = "lopdf")]
pub mod block_builder;
#[cfg(feature = "lopdf")]
pub mod column_detection;
#[cfg(feature = "lopdf")]
pub mod content_parser;
#[cfg(feature = "lopdf")]
pub mod element_processing;
pub mod elements;
#[cfg(feature = "lopdf")]
pub mod encodings;
#[cfg(feature = "lopdf")]
pub mod extraction_engine;
#[cfg(feature = "lopdf")]
pub mod font_handling;
#[cfg(feature = "lopdf")]
pub mod lattice;
pub mod mock;
pub mod spatial;
#[cfg(feature = "lopdf")]
pub mod text_grouping;

pub use mock::MockBackend;
pub use spatial::{LineRect, LineSpatialIndex};
#[cfg(feature = "lopdf")]
pub use extraction_engine::ExtractionEngine;
