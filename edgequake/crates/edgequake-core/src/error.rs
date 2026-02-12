//! Error types for EdgeQuake.
//!
//! This module defines the error hierarchy used throughout the EdgeQuake system.

use thiserror::Error;

/// Main error type for EdgeQuake operations.
#[derive(Error, Debug)]
pub enum Error {
    /// Storage operation failed
    #[error("Storage error: {0}")]
    Storage(#[from] edgequake_storage::StorageError),

    /// LLM operation failed
    #[error("LLM error: {0}")]
    Llm(#[from] edgequake_llm::LlmError),

    /// Pipeline operation failed
    #[cfg(feature = "pipeline")]
    #[error("Pipeline error: {0}")]
    Pipeline(#[from] edgequake_pipeline::PipelineError),

    /// Query operation failed
    #[error("Query error: {0}")]
    Query(#[from] QueryError),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Not found error
    #[error("Not found: {0}")]
    NotFound(String),

    /// Not initialized error
    #[error("Not initialized: {0}")]
    NotInitialized(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl Error {
    /// Create a new not initialized error.
    pub fn not_initialized<S: Into<String>>(msg: S) -> Self {
        Error::NotInitialized(msg.into())
    }

    /// Create a new not found error.
    pub fn not_found<S: Into<String>>(msg: S) -> Self {
        Error::NotFound(msg.into())
    }

    /// Create a new configuration error.
    pub fn config<S: Into<String>>(msg: S) -> Self {
        Error::Config(msg.into())
    }

    /// Create a new validation error.
    pub fn validation<S: Into<String>>(msg: S) -> Self {
        Error::Validation(msg.into())
    }

    /// Create a new internal error.
    pub fn internal<S: Into<String>>(msg: S) -> Self {
        Error::Internal(msg.into())
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_not_initialized() {
        let error = Error::not_initialized("storage layer");
        assert_eq!(error.to_string(), "Not initialized: storage layer");
    }

    #[test]
    fn test_error_config() {
        let error = Error::config("missing api key");
        assert_eq!(error.to_string(), "Configuration error: missing api key");
    }

    #[test]
    fn test_error_validation() {
        let error = Error::validation("invalid document format");
        assert_eq!(
            error.to_string(),
            "Validation error: invalid document format"
        );
    }

    #[test]
    fn test_error_internal() {
        let error = Error::internal("unexpected state");
        assert_eq!(error.to_string(), "Internal error: unexpected state");
    }

    #[test]
    fn test_query_error_invalid_mode() {
        let error = QueryError::InvalidMode("super".to_string());
        assert_eq!(error.to_string(), "Invalid query mode: super");
    }

    #[test]
    fn test_query_error_empty_query() {
        let error = QueryError::EmptyQuery;
        assert_eq!(error.to_string(), "Empty query");
    }

    #[test]
    fn test_query_error_no_results() {
        let error = QueryError::NoResults;
        assert_eq!(error.to_string(), "No results found");
    }

    #[test]
    fn test_query_error_context_retrieval() {
        let error = QueryError::ContextRetrievalFailed("storage unavailable".to_string());
        assert_eq!(
            error.to_string(),
            "Context retrieval failed: storage unavailable"
        );
    }

    #[test]
    fn test_query_error_response_generation() {
        let error = QueryError::ResponseGenerationFailed("llm offline".to_string());
        assert_eq!(error.to_string(), "Response generation failed: llm offline");
    }

    #[test]
    fn test_query_error_timeout() {
        let error = QueryError::Timeout;
        assert_eq!(error.to_string(), "Query timeout");
    }

    #[test]
    fn test_error_debug() {
        let error = Error::config("test");
        let debug = format!("{:?}", error);
        assert!(debug.contains("Config"));
    }

    #[test]
    fn test_query_error_debug() {
        let error = QueryError::EmptyQuery;
        let debug = format!("{:?}", error);
        assert!(debug.contains("EmptyQuery"));
    }
}
