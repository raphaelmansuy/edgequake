//! Pipeline error types.
//!
//! @implements SPEC-001/Issue-8: Comprehensive error handling for extraction

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

    /// Extraction timeout error.
    ///
    /// @implements SPEC-001/Issue-8: Timeout handling
    ///
    /// WHY: LLM calls can hang indefinitely. This error indicates the
    /// extraction exceeded the configured timeout and was aborted.
    #[error("Extraction timeout after {timeout_secs}s for chunk {chunk_index}: {message}")]
    ExtractionTimeout {
        /// Chunk index that timed out.
        chunk_index: usize,
        /// Configured timeout in seconds.
        timeout_secs: u64,
        /// Additional context message.
        message: String,
    },

    /// Retry limit exhausted.
    ///
    /// @implements SPEC-001/Issue-8: Retry limit handling
    ///
    /// WHY: After N retry attempts, we stop retrying to prevent infinite loops.
    /// This error provides visibility into how many retries were attempted.
    #[error("Extraction failed after {attempts} retries for chunk {chunk_index}: {message}")]
    RetryExhausted {
        /// Chunk index that failed.
        chunk_index: usize,
        /// Number of attempts made.
        attempts: u32,
        /// Last error message.
        message: String,
    },

    /// Circuit breaker open - LLM provider is failing.
    ///
    /// @implements SPEC-001/Issue-8: Circuit breaker pattern
    ///
    /// WHY: When the LLM provider is having issues (rate limits, outages),
    /// we should stop hammering it and fail fast. The circuit breaker
    /// opens after too many consecutive failures.
    #[error("Circuit breaker open: LLM provider is unavailable. {failures} consecutive failures. Retry after {retry_after_secs}s")]
    CircuitBreakerOpen {
        /// Number of consecutive failures.
        failures: u32,
        /// Seconds until next retry allowed.
        retry_after_secs: u64,
    },

    /// Document validation error.
    ///
    /// @implements SPEC-001/Issue-13: Comprehensive edge case handling
    ///
    /// WHY: Documents must be validated before processing to catch edge cases
    /// early and provide clear error messages to users.
    #[error("Validation error: {0}")]
    Validation(String),
}
