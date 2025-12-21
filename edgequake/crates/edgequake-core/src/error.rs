//! Error types for EdgeQuake.
//!
//! This module defines the error hierarchy used throughout the EdgeQuake system.

use thiserror::Error;

/// Main error type for EdgeQuake operations.
#[derive(Error, Debug)]
pub enum Error {
    /// Storage operation failed
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    /// LLM operation failed
    #[error("LLM error: {0}")]
    Llm(#[from] LlmError),

    /// Pipeline operation failed
    #[error("Pipeline error: {0}")]
    Pipeline(#[from] PipelineError),

    /// Query operation failed
    #[error("Query error: {0}")]
    Query(#[from] QueryError),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Not initialized error
    #[error("Not initialized: {0}")]
    NotInitialized(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl Error {
    /// Create a new not initialized error.
    pub fn not_initialized(msg: &str) -> Self {
        Error::NotInitialized(msg.to_string())
    }

    /// Create a new configuration error.
    pub fn config(msg: &str) -> Self {
        Error::Config(msg.to_string())
    }

    /// Create a new validation error.
    pub fn validation(msg: &str) -> Self {
        Error::Validation(msg.to_string())
    }

    /// Create a new internal error.
    pub fn internal(msg: &str) -> Self {
        Error::Internal(msg.to_string())
    }
}

/// Storage-related errors.
#[derive(Error, Debug)]
pub enum StorageError {
    /// Connection to storage failed
    #[error("Connection failed: {0}")]
    Connection(String),

    /// Record not found
    #[error("Record not found: {0}")]
    NotFound(String),

    /// Record already exists
    #[error("Record already exists: {0}")]
    AlreadyExists(String),

    /// Invalid query
    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    /// Transaction failed
    #[error("Transaction failed: {0}")]
    Transaction(String),

    /// Serialization/deserialization failed
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Database-specific error
    #[error("Database error: {0}")]
    Database(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// LLM-related errors.
#[derive(Error, Debug)]
pub enum LlmError {
    /// Provider not configured
    #[error("Provider not configured: {0}")]
    ProviderNotConfigured(String),

    /// API request failed
    #[error("API request failed: {0}")]
    ApiRequest(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    /// Invalid response from LLM
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Token limit exceeded
    #[error("Token limit exceeded: expected max {max}, got {actual}")]
    TokenLimitExceeded { max: usize, actual: usize },

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Model not found
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    /// Timeout
    #[error("Request timeout: {0}")]
    Timeout(String),

    /// Streaming error
    #[error("Streaming error: {0}")]
    Streaming(String),
}

/// Pipeline processing errors.
#[derive(Error, Debug)]
pub enum PipelineError {
    /// Document is empty
    #[error("Empty document")]
    EmptyDocument,

    /// Document already processed
    #[error("Document already processed: {0}")]
    AlreadyProcessed(String),

    /// Chunking failed
    #[error("Chunking failed: {0}")]
    ChunkingFailed(String),

    /// Entity extraction failed
    #[error("Entity extraction failed: {0}")]
    ExtractionFailed(String),

    /// Merging failed
    #[error("Merging failed: {0}")]
    MergingFailed(String),

    /// Embedding generation failed
    #[error("Embedding generation failed: {0}")]
    EmbeddingFailed(String),

    /// Document processing was cancelled
    #[error("Processing cancelled")]
    Cancelled,

    /// Invalid document state for operation
    #[error("Invalid document state: expected {expected}, got {actual}")]
    InvalidState { expected: String, actual: String },
}

/// Query-related errors.
#[derive(Error, Debug)]
pub enum QueryError {
    /// Invalid query mode
    #[error("Invalid query mode: {0}")]
    InvalidMode(String),

    /// Empty query
    #[error("Empty query")]
    EmptyQuery,

    /// No results found
    #[error("No results found")]
    NoResults,

    /// Context retrieval failed
    #[error("Context retrieval failed: {0}")]
    ContextRetrievalFailed(String),

    /// Response generation failed
    #[error("Response generation failed: {0}")]
    ResponseGenerationFailed(String),

    /// Query timeout
    #[error("Query timeout")]
    Timeout,
}

/// Result type alias for EdgeQuake operations.
pub type Result<T> = std::result::Result<T, Error>;

impl From<serde_json::Error> for StorageError {
    fn from(err: serde_json::Error) -> Self {
        StorageError::Serialization(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Storage(StorageError::Serialization(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::Storage(StorageError::NotFound("doc-123".to_string()));
        assert!(err.to_string().contains("not found"));
        assert!(err.to_string().contains("doc-123"));
    }

    #[test]
    fn test_error_conversion() {
        let storage_err = StorageError::Connection("timeout".to_string());
        let err: Error = storage_err.into();
        assert!(matches!(err, Error::Storage(_)));
    }
}
