//! Document processing pipeline.
//!
//! This module provides the processing pipeline architecture:
//! - **Provider**: Loads documents from various sources
//! - **Builder**: Constructs document representation from provider (DEPRECATED - use PdfiumExtractor)
//! - **Processor**: Transforms documents (layout, enhancement, etc.)
//! - **Renderer**: Outputs to various formats (Markdown, JSON, etc.)

// DEPRECATED: builder module depends on pdf_oxide which has been removed
// Use PdfiumExtractor directly for SOTA PDF extraction
// mod builder;

mod llm_enhance;
mod processor;
mod provider;

// pub use builder::{DocumentBuilder, PageBuilder};
pub use llm_enhance::{LlmEnhanceConfig, LlmEnhanceProcessor, LlmEnhanced};
pub use processor::{
    BlockMergeProcessor, LayoutProcessor, PostProcessor, Processor, ProcessorChain,
    TableDetectionProcessor,
};
pub use provider::{ByteProvider, FileProvider, PdfProvider};
