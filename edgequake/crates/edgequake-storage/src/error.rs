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

    /// Invalid input (e.g. a stale confirm token for an admin repair job).
    #[error("Invalid input: {0}")]
    InvalidInput(String),

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

    /// Backend does not implement the requested capability.
    #[error("Unsupported capability: {0}")]
    UnsupportedCapability(String),

    /// Storage is temporarily unavailable.
    #[error("Storage unavailable: {0}")]
    Unavailable(String),

    /// Storage operation exceeded its deadline.
    #[error("Storage deadline exceeded: {0}")]
    DeadlineExceeded(String),

    /// Storage throttled the operation.
    #[error("Storage rate limited: {0}")]
    RateLimited(String),

    /// Transaction can be retried after serialization/deadlock failure.
    #[error("Serialization retry required: {0}")]
    SerializationRetry(String),

    /// The caller cannot determine whether the operation committed.
    #[error("Storage operation outcome unknown: {0}")]
    UnknownOutcome(String),

    /// Requested tenant or workspace scope is forbidden.
    #[error("Forbidden storage scope: {0}")]
    ForbiddenScope(String),
}

impl From<StorageError> for edgequake_storage_contracts::AccessError {
    fn from(error: StorageError) -> Self {
        use edgequake_storage_contracts::AccessError;
        match error {
            StorageError::NotFound(message) => AccessError::NotFound(message),
            StorageError::AlreadyExists(message) | StorageError::Conflict(message) => {
                AccessError::Conflict(message)
            }
            StorageError::InvalidQuery(message)
            | StorageError::InvalidInput(message)
            | StorageError::InvalidConfig(message) => AccessError::InvalidInput(message),
            StorageError::ForbiddenScope(message) => AccessError::ForbiddenScope(message),
            StorageError::UnsupportedCapability(message) => {
                AccessError::UnsupportedCapability(message)
            }
            StorageError::Unavailable(message) => AccessError::Unavailable(message),
            StorageError::DeadlineExceeded(message) => AccessError::DeadlineExceeded(message),
            StorageError::RateLimited(message) => AccessError::RateLimited(message),
            StorageError::SerializationRetry(message) => AccessError::SerializationRetry(message),
            StorageError::UnknownOutcome(message) | StorageError::Transaction(message) => {
                AccessError::UnknownOutcome(message)
            }
            StorageError::Serialization(message) | StorageError::InvalidData(message) => {
                AccessError::CorruptData(message)
            }
            StorageError::Connection(message) | StorageError::Database(message) => {
                AccessError::Unavailable(message)
            }
            StorageError::Io(error) => AccessError::Unavailable(error.to_string()),
            StorageError::NotInitialized => {
                AccessError::Unavailable("Storage not initialized".into())
            }
        }
    }
}

impl From<edgequake_storage_contracts::AccessError> for StorageError {
    fn from(error: edgequake_storage_contracts::AccessError) -> Self {
        use edgequake_storage_contracts::AccessError;
        match error {
            AccessError::InvalidInput(message) => StorageError::InvalidInput(message),
            AccessError::ForbiddenScope(message) => StorageError::ForbiddenScope(message),
            AccessError::NotFound(message) => StorageError::NotFound(message),
            AccessError::Conflict(message) => StorageError::Conflict(message),
            AccessError::UnsupportedCapability(message) => {
                StorageError::UnsupportedCapability(message)
            }
            AccessError::Unavailable(message) => StorageError::Unavailable(message),
            AccessError::DeadlineExceeded(message) => StorageError::DeadlineExceeded(message),
            AccessError::RateLimited(message) => StorageError::RateLimited(message),
            AccessError::SerializationRetry(message) => StorageError::SerializationRetry(message),
            AccessError::UnknownOutcome(message) => StorageError::UnknownOutcome(message),
            AccessError::CorruptData(message) => StorageError::InvalidData(message),
        }
    }
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
                classify_sqlstate(e.code().as_deref(), e.message(), e.constraint())
            }
            sqlx::Error::PoolTimedOut => {
                StorageError::Unavailable("Connection pool timeout".to_string())
            }
            sqlx::Error::PoolClosed => {
                StorageError::Unavailable("Connection pool closed".to_string())
            }
            sqlx::Error::Io(e) => StorageError::Connection(e.to_string()),
            sqlx::Error::Tls(e) => StorageError::Connection(e.to_string()),
            sqlx::Error::Protocol(e) => StorageError::Connection(e),
            sqlx::Error::WorkerCrashed => {
                StorageError::Unavailable("Database worker crashed".to_string())
            }
            _ => StorageError::Database(err.to_string()),
        }
    }
}

#[cfg(feature = "postgres")]
fn classify_sqlstate(code: Option<&str>, message: &str, constraint: Option<&str>) -> StorageError {
    let message = match constraint {
        Some(name) => format!("{message} (constraint: {name})"),
        None => message.to_string(),
    };
    match code {
        Some("23505") => StorageError::AlreadyExists(message),
        Some("40001" | "40P01") => StorageError::SerializationRetry(message),
        Some("57014") => StorageError::DeadlineExceeded(message),
        Some(code) if code.starts_with("08") => StorageError::Connection(message),
        Some("53300" | "57P01" | "57P02" | "57P03") => StorageError::Unavailable(message),
        _ => StorageError::Database(message),
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

    #[cfg(feature = "postgres")]
    #[test]
    fn sqlstate_unique_violation_maps_to_already_exists() {
        assert!(matches!(
            classify_sqlstate(Some("23505"), "duplicate key", Some("chunks_pkey")),
            StorageError::AlreadyExists(message) if message.contains("chunks_pkey")
        ));
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn sqlstate_retryable_transaction_failures_are_typed() {
        for code in ["40001", "40P01"] {
            assert!(matches!(
                classify_sqlstate(Some(code), "retry transaction", None),
                StorageError::SerializationRetry(_)
            ));
        }
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn sqlstate_query_cancel_maps_to_deadline() {
        assert!(matches!(
            classify_sqlstate(Some("57014"), "canceling statement", None),
            StorageError::DeadlineExceeded(_)
        ));
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn sqlstate_connection_class_maps_to_connection() {
        assert!(matches!(
            classify_sqlstate(Some("08006"), "connection failure", None),
            StorageError::Connection(_)
        ));
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn constraint_name_does_not_override_sqlstate() {
        assert!(matches!(
            classify_sqlstate(Some("23503"), "foreign key failure", Some("looks_unique")),
            StorageError::Database(_)
        ));
    }
}
