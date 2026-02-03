use crate::extractor::PdfInfo;
use crate::progress::ProgressCallback;
use crate::schema::Document;
use crate::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// Trait for PDF extraction backends.
///
/// This trait abstracts the underlying PDF engine (e.g., lopdf, etc.)
/// allowing for swappable backends and easier testing.
///
/// ## Implements
///
/// - [`SPEC-001-upload-pdf`]: PDF extraction backend with progress tracking
/// - [`FEAT0610`]: Page-level progress callbacks during extraction
///
/// ## WHY Two Extract Methods?
///
/// We provide both `extract()` and `extract_with_progress()` because:
/// 1. **Backwards compatibility**: Existing code calling `extract()` works unchanged
/// 2. **Optional progress**: Not all callers need progress tracking
/// 3. **Default implementation**: `extract_with_progress()` falls back to `extract()`
#[async_trait]
pub trait PdfBackend: Send + Sync {
    /// Extract the raw document structure from PDF bytes.
    async fn extract(&self, pdf_bytes: &[u8]) -> Result<Document>;

    /// Extract with progress callbacks for each page.
    ///
    /// This method reports progress during extraction:
    /// - `on_extraction_start` called once at start with total page count
    /// - `on_page_start` called before each page
    /// - `on_page_complete` or `on_page_error` called after each page
    /// - `on_extraction_complete` called at end with success count
    ///
    /// Default implementation ignores the callback and calls `extract()`.
    /// Backends that support progress should override this method.
    ///
    /// # Arguments
    ///
    /// * `pdf_bytes` - Raw PDF file bytes
    /// * `callback` - Progress callback implementation (use `NoopProgress` for no-op)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use std::sync::Arc;
    /// use edgequake_pdf::{NoopProgress, PdfBackend, ExtractionEngine};
    ///
    /// let backend = ExtractionEngine::new();
    /// let callback = Arc::new(NoopProgress);
    /// let doc = backend.extract_with_progress(pdf_bytes, callback).await?;
    /// ```
    async fn extract_with_progress(
        &self,
        pdf_bytes: &[u8],
        callback: Arc<dyn ProgressCallback>,
    ) -> Result<Document> {
        // WHY: Default ignores callback for backwards compatibility.
        // Backends that support progress should override this method.
        let _ = callback;
        self.extract(pdf_bytes).await
    }

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
pub mod glyph_list;
#[cfg(feature = "lopdf")]
pub mod lattice;
pub mod mock;
pub mod spatial;
#[cfg(feature = "lopdf")]
pub mod text_grouping;
#[cfg(feature = "lopdf")]
pub mod truetype_cmap;

#[cfg(feature = "lopdf")]
pub use extraction_engine::ExtractionEngine;
pub use mock::MockBackend;
pub use spatial::{LineRect, LineSpatialIndex};
