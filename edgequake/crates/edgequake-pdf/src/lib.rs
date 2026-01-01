//! EdgeQuake PDF to Markdown extraction crate.
//!
//! This crate provides functionality to extract text, tables, images, and other
//! content from PDF documents and convert them to structured Markdown using
//! AI enhancement through EdgeQuake's LLM providers.
//!
//! # Architecture
//!
//! The crate is organized into several modules:
//!
//! - **schema**: Block-based document representation (Marker-style)
//! - **layout**: Layout detection and reading order algorithms
//! - **processors**: Document processing pipeline
//! - **renderers**: Output format renderers
//! - **config**: Extraction configuration options
//! - **extractor**: Main PDF extraction logic
//! - **backend**: Pluggable PDF extraction backends
//!
//! # Example
//!
//! ```rust,no_run
//! use edgequake_pdf::{PdfExtractor, PdfConfig};
//! use edgequake_llm::providers::mock::MockProvider;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let provider = Arc::new(MockProvider::new());
//!     let extractor = PdfExtractor::new(provider);
//!     
//!     let pdf_bytes = std::fs::read("document.pdf")?;
//!     let markdown = extractor.extract_to_markdown(&pdf_bytes).await?;
//!     
//!     println!("{}", markdown);
//!     Ok(())
//! }
//! ```

pub mod backend;
pub mod config;
pub mod error;
pub mod extractor;
pub mod layout;
pub mod processors;
pub mod renderers;
pub mod schema;
pub mod vision;

pub use backend::PdfBackend;
pub use config::{ExtractionMode, LayoutConfig, OutputFormat, PdfConfig};
pub use error::PdfError;
pub use extractor::{ExtractedImage, ExtractionResult, PageContent, PdfExtractor, PdfInfo};

// Re-export schema types for convenience
pub use schema::{
    Block, BlockId, BlockType, BoundingBox, Document, DocumentMetadata, ExtractionMethod, Page,
    PageStats, Point, Polygon, TocEntry,
};

// Re-export layout types for convenience
pub use layout::{
    ColumnDetector, ColumnLayout, LayoutAnalysis, LayoutAnalyzer, LayoutRegion, PageMargins,
    ReadingOrder, ReadingOrderDetector, RegionType, XYCut, XYCutNode, XYCutParams,
};

// Re-export processor types
pub use processors::{
    BlockMergeProcessor, ByteProvider, FileProvider, LayoutProcessor, LlmEnhanceConfig,
    LlmEnhanceProcessor, LlmEnhanced, PdfProvider, PostProcessor, Processor, ProcessorChain,
};

// Re-export renderer types
pub use renderers::{JsonRenderer, MarkdownRenderer, MarkdownStyle, Renderer};

// Re-export vision types
pub use vision::{ImageFormat, PageImage, VisionCapable, VisionConfig, VisionExtractor};

// Pdfium backend removed from this crate. Use a separate optional crate if needed.

/// Result type for PDF operations.
pub type Result<T> = std::result::Result<T, PdfError>;
