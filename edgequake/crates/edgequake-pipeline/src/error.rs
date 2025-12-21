//! Pipeline error types.

use thiserror::Error;

/// Result type for pipeline operations.
pub type Result<T> = std::result::Result<T, PipelineError>;

/// Errors that can occur during pipeline processing.
#[derive(Debug, Error)]
pub enum PipelineError {
    /// Error during document processing.
    #[error("Document processing error: {0}")]
    DocumentError(String),

    /// Error during chunking.
    #[error("Chunking error: {0}")]
    ChunkingError(String),

    /// Error during entity extraction.
    #[error("Entity extraction error: {0}")]
    ExtractionError(String),

    /// Error during embedding generation.
    #[error("Embedding error: {0}")]
    EmbeddingError(String),

    /// Error during graph operations.
    #[error("Graph error: {0}")]
    GraphError(String),

    /// Storage error.
    #[error("Storage error: {0}")]
    StorageError(#[from] edgequake_storage::error::StorageError),

    /// LLM error.
    #[error("LLM error: {0}")]
    LlmError(#[from] edgequake_llm::error::LlmError),

    /// Invalid configuration.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Document not found.
    #[error("Document not found: {0}")]
    NotFound(String),

    /// Invalid document format.
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
}
