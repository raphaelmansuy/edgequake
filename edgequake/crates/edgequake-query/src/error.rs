//! Query error types.

use thiserror::Error;

/// Result type for query operations.
pub type Result<T> = std::result::Result<T, QueryError>;

/// Errors that can occur during query processing.
#[derive(Debug, Error)]
pub enum QueryError {
    /// Invalid query.
    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    /// No results found.
    #[error("No results found for query")]
    NoResults,

    /// Context limit exceeded.
    #[error("Context limit exceeded: max {max} tokens, got {got}")]
    ContextLimitExceeded { max: usize, got: usize },

    /// Storage error.
    #[error("Storage error: {0}")]
    StorageError(#[from] edgequake_storage::error::StorageError),

    /// LLM error.
    #[error("LLM error: {0}")]
    LlmError(#[from] edgequake_llm::error::LlmError),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Timeout during query processing.
    #[error("Query timed out after {0}ms")]
    Timeout(u64),

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(String),
}
