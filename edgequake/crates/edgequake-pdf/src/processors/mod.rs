//! Document processing pipeline.
//!
//! This module provides a modular processing pipeline architecture:
//! - **Processor**: Core trait for document transformation
//! - **ProcessorChain**: Composes multiple processors
//!
//! ## Module Organization (Single Responsibility)
//!
//! - `font_analysis`: Font size statistics (FontAnalyzer)
//! - `heading_classifier`: Geometric heading detection (HeadingClassifier)
//! - `layout_processing`: Layout, margins, block merging
//! - `structure_detection`: Headers, captions, lists, code blocks
//! - `table_detection`: Table detection and reconstruction
//! - `text_cleanup`: Text normalization, OCR fixes, hyphenation
//! - `llm_enhance`: LLM-based document enhancement
//! - `image_processor`: Image extraction and OCR preparation
//! - `stats`: Document statistics

// DEPRECATED: builder module depends on pdf_oxide which has been removed.
// mod builder;

mod font_analysis;
mod heading_classifier;
#[cfg(feature = "lopdf")]
mod image_processor;
mod layout_processing;
mod llm_enhance;
mod processor;
mod provider;
mod stats;
mod structure_detection;
mod table_detection;
mod text_cleanup;

#[cfg(test)]
mod test_helpers;

// Re-exports for public API
pub use font_analysis::FontAnalyzer;
pub use heading_classifier::HeadingClassifier;
pub use llm_enhance::{LlmEnhanceConfig, LlmEnhanceProcessor, LlmEnhanced};
pub use provider::{ByteProvider, FileProvider, PdfProvider};
pub use stats::DocumentStats;

// Core processor trait and chain
pub use processor::{Processor, ProcessorChain, SectionPatternProcessor, StyleDetectionProcessor};

// Layout processors
pub use layout_processing::{
    BlockMergeProcessor, LayoutProcessor, MarginFilterProcessor, SectionNumberMergeProcessor,
};

// Structure detection processors
pub use structure_detection::{
    CaptionDetectionProcessor, CodeBlockDetectionProcessor, HeaderDetectionProcessor,
    ListDetectionProcessor,
};

// Table detection processors
pub use table_detection::{TableDetectionProcessor, TextTableReconstructionProcessor};

// Text cleanup processors
pub use text_cleanup::{GarbledTextFilterProcessor, HyphenContinuationProcessor, PostProcessor};

// Image extraction processor
#[cfg(feature = "lopdf")]
pub use image_processor::{ImageExtractionProcessor, ImagePageStats};
