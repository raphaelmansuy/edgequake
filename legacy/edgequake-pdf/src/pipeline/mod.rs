//! PDF processing pipelines.
//!
//! This module provides high-level pipelines for PDF to Markdown conversion.
//!
//! ## Pipelines
//!
//! - **PymupdfPipeline**: Pure Rust pipeline using PDFium for character extraction
//!   and pymupdf4llm-inspired algorithms for layout analysis.

#[cfg(feature = "pdfium")]
mod pymupdf_pipeline;

#[cfg(feature = "pdfium")]
pub use pymupdf_pipeline::{PipelineConfig, PymupdfPipeline};
