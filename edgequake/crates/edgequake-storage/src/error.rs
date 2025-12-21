//! Storage error types.

use thiserror::Error;

/// Storage operation errors.
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

    /// Storage not initialized
    #[error("Storage not initialized")]
    NotInitialized,

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

impl From<serde_json::Error> for StorageError {
    fn from(err: serde_json::Error) -> Self {
        StorageError::Serialization(err.to_string())
    }
}

/// Result type for storage operations.
pub type Result<T> = std::result::Result<T, StorageError>;
