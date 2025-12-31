//! EdgeQuake PDF to Markdown extraction crate.
//!
//! This crate provides functionality to extract text, tables, images, and other
//! content from PDF documents and convert them to structured Markdown using
//! AI enhancement through EdgeQuake's LLM providers.

pub mod error;
pub mod extractor;
pub mod config;

pub use extractor::PdfExtractor;
pub use config::PdfConfig;
pub use error::PdfError;

/// Result type for PDF operations.
pub type Result<T> = std::result::Result<T, PdfError>;