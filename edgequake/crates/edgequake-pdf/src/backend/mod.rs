//! PDF extraction backends.
//!
//! This module provides the PdfiumBackend implementing the [`PdfBackend`] trait.
//!
//! ## Architecture (IT31: lopdf removed)
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                        Single Backend Pipeline                              │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │                                                                             │
//! │  ✅ PdfiumBackend (feature = "pdfium")                                      │
//! │     - Uses Chromium's PDFium engine                                         │
//! │     - Font descriptor flags for accurate bold/italic                        │
//! │     - Character-level bounding boxes                                        │
//! │     - Active maintenance (Google-backed)                                    │
//! │                                                                             │
//! │  🧪 MockBackend                                                             │
//! │     - Returns empty documents                                               │
//! │     - For unit tests only                                                   │
//! │                                                                             │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use edgequake_pdf::backend::{PdfBackend, PdfiumBackend};
//!
//! let backend = PdfiumBackend::new()?;
//! let document = backend.extract(&pdf_bytes).await?;
//! ```

use crate::extractor::PdfInfo;
use crate::progress::ProgressCallback;
use crate::schema::Document;
use crate::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// Trait for PDF extraction backends.
///
/// Abstracts the PDF engine, enabling PdfiumBackend (production) and MockBackend (tests).
///
/// ## Implements
///
/// - [`SPEC-001-upload-pdf`]: PDF extraction backend with progress tracking
/// - [`FEAT0610`]: Page-level progress callbacks during extraction
#[async_trait]
pub trait PdfBackend: Send + Sync {
    /// Extract the raw document structure from PDF bytes.
    async fn extract(&self, pdf_bytes: &[u8]) -> Result<Document>;

    /// Extract with progress callbacks for each page.
    ///
    /// Default implementation ignores the callback and calls `extract()`.
    /// Backends that support progress should override this method.
    async fn extract_with_progress(
        &self,
        pdf_bytes: &[u8],
        callback: Arc<dyn ProgressCallback>,
    ) -> Result<Document> {
        let _ = callback;
        self.extract(pdf_bytes).await
    }

    /// Get metadata/info about the PDF without full extraction.
    fn get_info(&self, pdf_bytes: &[u8]) -> Result<PdfInfo>;
}

pub mod elements;
pub mod mock;
pub mod spatial;

#[cfg(feature = "pdfium")]
pub mod pdfium;
#[cfg(feature = "pdfium")]
pub mod pdfium_backend;

pub use elements::RawChar;
pub use mock::MockBackend;
pub use spatial::{LineRect, LineSpatialIndex};

#[cfg(feature = "pdfium")]
pub use pdfium::ExtractedImageData;
#[cfg(feature = "pdfium")]
pub use pdfium::PdfiumExtractor;
#[cfg(feature = "pdfium")]
pub use pdfium_backend::PdfiumBackend;
