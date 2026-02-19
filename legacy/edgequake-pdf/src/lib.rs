//! EdgeQuake PDF to Markdown extraction crate.
//!
//! This crate provides functionality to extract text, tables, images, and other
//! content from PDF documents and convert them to structured Markdown using
//! AI enhancement through EdgeQuake's LLM providers.
//!
//! ## Implements
//!
//! - [`FEAT1001`]: PDF to Markdown conversion with structure preservation
//! - [`FEAT1002`]: Table detection and extraction (lattice and stream modes)
//! - [`FEAT1003`]: Multi-column layout detection and reading order
//! - [`FEAT1004`]: Image extraction with optional OCR
//! - [`FEAT1005`]: Formula detection and LaTeX conversion
//! - [`FEAT1006`]: LLM-enhanced content cleaning and formatting
//!
//! ## Enforces
//!
//! - [`BR1001`]: Preserve document structure in output
//! - [`BR1002`]: Handle malformed PDFs gracefully
//! - [`BR1003`]: Maintain reading order accuracy >95%
//! - [`BR1004`]: Table cell alignment preserved
//!
//! ## Use Cases
//!
//! - [`UC1001`]: User uploads PDF for knowledge graph ingestion
//! - [`UC1002`]: System extracts text blocks with bounding boxes
//! - [`UC1003`]: Pipeline converts tables to markdown tables
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
// Intentional clippy suppression:
// - manual_clamp: We use min().max() chains for NaN-safe clamping
// - too_many_arguments: Complex layout functions need many parameters
// - should_implement_trait: BoundingBox::add is semantic, not std::ops::Add
#![allow(clippy::manual_clamp)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::should_implement_trait)]

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
pub mod formula;
pub mod image_ocr;
pub mod layout;
pub mod pipeline;
pub mod processors;
pub mod progress;
pub mod renderers;
pub mod rendering;
pub mod schema;
pub mod vision;

pub use backend::PdfBackend;
pub use config::{ExtractionMode, ImageOcrConfig, LayoutConfig, OutputFormat, PdfConfig};
pub use error::{PageError, PdfError};
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

// Re-export formula detection types
pub use formula::{Formula, FormulaConfig, FormulaDetector, SymbolMap, MATH_SYMBOL_MAP};

// Re-export renderer types
pub use renderers::{JsonRenderer, MarkdownRenderer, MarkdownStyle, Renderer};

// Re-export vision types
pub use vision::{ImageFormat, PageImage, VisionCapable, VisionConfig, VisionExtractor};

// Re-export rendering types
pub use rendering::PageRenderer;

// Re-export image OCR types
pub use image_ocr::{ImageData, ImageOcrCapable, ImageOcrProcessor, ImageOcrResult, ImageType};

// Re-export progress callback types
pub use progress::{CountingProgress, LoggingProgress, NoopProgress, ProgressCallback};

// Re-export pipeline types when pdfium feature is enabled
#[cfg(feature = "pdfium")]
pub use pipeline::{PipelineConfig, PymupdfPipeline};

// Pdfium backend removed from this crate. Use a separate optional crate if needed.

/// Result type for PDF operations.
pub type Result<T> = std::result::Result<T, PdfError>;
