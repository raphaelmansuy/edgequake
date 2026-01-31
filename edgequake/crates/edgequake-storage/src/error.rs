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

    /// Conflict detected (duplicate, constraint violation)
    #[error("Conflict: {0}")]
    Conflict(String),

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

    /// Invalid data
    #[error("Invalid data: {0}")]
    InvalidData(String),
}

impl From<serde_json::Error> for StorageError {
    fn from(err: serde_json::Error) -> Self {
        StorageError::Serialization(err.to_string())
    }
}

#[cfg(feature = "postgres")]
impl From<sqlx::Error> for StorageError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => StorageError::NotFound("Row not found".to_string()),
            sqlx::Error::Database(e) => {
                // Check for unique constraint violations (duplicate keys)
                if let Some(constraint) = e.constraint() {
                    if constraint.contains("unique") || constraint.contains("pkey") {
                        return StorageError::AlreadyExists(format!(
                            "Constraint violation: {}",
                            constraint
                        ));
                    }
                }
                StorageError::Database(e.to_string())
            }
            sqlx::Error::PoolTimedOut => {
                StorageError::Connection("Connection pool timeout".to_string())
            }
            sqlx::Error::PoolClosed => {
                StorageError::Connection("Connection pool closed".to_string())
            }
            _ => StorageError::Database(err.to_string()),
        }
    }
}

/// Result type for storage operations.
pub type Result<T> = std::result::Result<T, StorageError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_error_connection() {
        let error = StorageError::Connection("refused".to_string());
        assert_eq!(error.to_string(), "Connection failed: refused");
    }

    #[test]
    fn test_storage_error_not_found() {
        let error = StorageError::NotFound("doc-123".to_string());
        assert_eq!(error.to_string(), "Record not found: doc-123");
    }

    #[test]
    fn test_storage_error_already_exists() {
        let error = StorageError::AlreadyExists("entity-456".to_string());
        assert_eq!(error.to_string(), "Record already exists: entity-456");
    }

    #[test]
    fn test_storage_error_invalid_query() {
        let error = StorageError::InvalidQuery("syntax error".to_string());
        assert_eq!(error.to_string(), "Invalid query: syntax error");
    }

    #[test]
    fn test_storage_error_transaction() {
        let error = StorageError::Transaction("rollback".to_string());
        assert_eq!(error.to_string(), "Transaction failed: rollback");
    }

    #[test]
    fn test_storage_error_serialization() {
        let error = StorageError::Serialization("invalid json".to_string());
        assert_eq!(error.to_string(), "Serialization error: invalid json");
    }

    #[test]
    fn test_storage_error_database() {
        let error = StorageError::Database("constraint violation".to_string());
        assert_eq!(error.to_string(), "Database error: constraint violation");
    }

    #[test]
    fn test_storage_error_not_initialized() {
        let error = StorageError::NotInitialized;
        assert_eq!(error.to_string(), "Storage not initialized");
    }

    #[test]
    fn test_storage_error_invalid_config() {
        let error = StorageError::InvalidConfig("missing host".to_string());
        assert_eq!(error.to_string(), "Invalid configuration: missing host");
    }

    #[test]
    fn test_storage_error_from_serde_json() {
        let json_err: serde_json::Error =
            serde_json::from_str::<serde_json::Value>("not json").unwrap_err();
        let storage_err: StorageError = json_err.into();
        assert!(matches!(storage_err, StorageError::Serialization(_)));
    }

    #[test]
    fn test_storage_error_debug() {
        let error = StorageError::NotInitialized;
        let debug = format!("{:?}", error);
        assert!(debug.contains("NotInitialized"));
    }
}
