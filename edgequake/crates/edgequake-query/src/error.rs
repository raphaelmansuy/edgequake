//! Query error types.
//!
//! ## WHY: Separate Error Variants for User vs. System Failures
//!
//! - `InvalidQuery` / `ConfigError` — client-fixable; return 4xx.
//! - `StorageError` / `LlmError` / `Internal` — server-side; return 5xx.
//! - `NoResults` — expected outcome, not a hard error; callers may
//!   decide to return an empty answer instead of propagating.
//! - `Timeout` — enables per-query deadline enforcement without
//!   collapsing all failures into a generic Internal variant.
//!
//! Using `#[from]` on `StorageError` and `LlmError` lets `?` propagate
//! infrastructure errors without manual mapping at every call site.

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_query_display() {
        let e = QueryError::InvalidQuery("empty string".into());
        assert_eq!(e.to_string(), "Invalid query: empty string");
    }

    #[test]
    fn test_no_results_display() {
        let e = QueryError::NoResults;
        assert_eq!(e.to_string(), "No results found for query");
    }

    #[test]
    fn test_context_limit_exceeded_display() {
        let e = QueryError::ContextLimitExceeded { max: 4000, got: 5500 };
        assert_eq!(e.to_string(), "Context limit exceeded: max 4000 tokens, got 5500");
    }

    #[test]
    fn test_timeout_display() {
        let e = QueryError::Timeout(3000);
        assert_eq!(e.to_string(), "Query timed out after 3000ms");
    }

    #[test]
    fn test_internal_display() {
        let e = QueryError::Internal("unexpected state".into());
        assert_eq!(e.to_string(), "Internal error: unexpected state");
    }

    #[test]
    fn test_config_error_display() {
        let e = QueryError::ConfigError("missing model".into());
        assert_eq!(e.to_string(), "Configuration error: missing model");
    }
}
