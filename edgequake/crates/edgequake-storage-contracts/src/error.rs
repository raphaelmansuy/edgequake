//! Stable, provider-independent data-access failures.

use thiserror::Error;

/// Error classes shared by relational, graph, and vector providers.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AccessError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Forbidden scope: {0}")]
    ForbiddenScope(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("Unsupported capability: {0}")]
    UnsupportedCapability(String),
    #[error("Provider unavailable: {0}")]
    Unavailable(String),
    #[error("Deadline exceeded: {0}")]
    DeadlineExceeded(String),
    #[error("Rate limited: {0}")]
    RateLimited(String),
    #[error("Serialization retry required: {0}")]
    SerializationRetry(String),
    #[error("Operation outcome unknown: {0}")]
    UnknownOutcome(String),
    #[error("Corrupt data: {0}")]
    CorruptData(String),
}

impl AccessError {
    /// Whether replay may be appropriate when the operation is idempotent.
    pub const fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Unavailable(_)
                | Self::DeadlineExceeded(_)
                | Self::RateLimited(_)
                | Self::SerializationRetry(_)
                | Self::UnknownOutcome(_)
        )
    }
}

pub type AccessResult<T> = Result<T, AccessError>;
