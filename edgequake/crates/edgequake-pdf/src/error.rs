//! Error types for PDF processing operations.

use thiserror::Error;

/// Errors that can occur during PDF processing.
#[derive(Error, Debug)]
pub enum PdfError {
    #[error("PDF parsing error: {0}")]
    PdfParse(String),

    #[error("AI processing error: {0}")]
    AiProcessing(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Image processing error: {0}")]
    ImageProcessing(String),

    #[error("OCR processing error: {0}")]
    OcrProcessing(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Processor error: {0}")]
    Processor(String),

    #[error("Unsupported operation: {0}")]
    Unsupported(String),
}
