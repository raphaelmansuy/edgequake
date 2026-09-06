//! Authz errors (fail-closed).

use thiserror::Error;

pub type AuthzResult<T> = Result<T, AuthzError>;

#[derive(Debug, Error)]
pub enum AuthzError {
    #[error("policy evaluation failed: {0}")]
    Policy(String),
    #[error("cedar schema/compile error: {0}")]
    Cedar(String),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("invalid principal")]
    InvalidPrincipal,
    #[error("feature disabled")]
    Disabled,
}
