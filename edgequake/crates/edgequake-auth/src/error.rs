//! Authentication error types.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Authentication and authorization result type.
pub type AuthResult<T> = Result<T, AuthError>;

/// Authentication and authorization errors.
#[derive(Debug, Error)]
pub enum AuthError {
    /// Invalid credentials (wrong username/password).
    #[error("Invalid credentials")]
    InvalidCredentials,

    /// Token has expired.
    #[error("Token expired")]
    TokenExpired,

    /// Token is invalid or malformed.
    #[error("Invalid token: {reason}")]
    InvalidToken { reason: String },

    /// Missing authentication token.
    #[error("Missing authentication token")]
    MissingToken,

    /// Missing authentication (no token provided).
    #[error("Authentication required")]
    MissingAuth,

    /// Invalid Authorization header format.
    #[error("Invalid Authorization header: {reason}")]
    InvalidAuthorizationHeader { reason: String },

    /// User does not have required permission.
    #[error("Permission denied: {required_permission}")]
    Forbidden { required_permission: String },

    /// Missing API key.
    #[error("Missing API key")]
    MissingApiKey,

    /// User not found.
    #[error("User not found")]
    UserNotFound,

    /// User account is inactive.
    #[error("Account is inactive")]
    AccountInactive,

    /// API key not found or invalid.
    #[error("Invalid API key")]
    InvalidApiKey,

    /// API key has expired.
    #[error("API key expired")]
    ApiKeyExpired,

    /// Insufficient scope for API key.
    #[error("Insufficient scope: requires {required}")]
    InsufficientScope { required: String },

    /// Rate limit exceeded.
    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    /// Too many login attempts.
    #[error("Too many login attempts, account locked")]
    AccountLocked,

    /// Weak password.
    #[error("Password too weak: {reason}")]
    WeakPassword { reason: String },

    /// Password hashing error.
    #[error("Password hashing failed: {reason}")]
    PasswordHashingFailed { reason: String },

    /// Token generation failed.
    #[error("Token generation failed: {reason}")]
    TokenGenerationFailed { reason: String },

    /// Password hashing error.
    #[error("Password hashing error: {0}")]
    PasswordError(String),

    /// JWT encoding/decoding error.
    #[error("JWT error: {0}")]
    JwtError(String),

    /// Database error.
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(String),

    /// Tenant not found (multi-tenant mode).
    #[error("Tenant not found")]
    TenantNotFound,

    /// Workspace not found (multi-tenant mode).
    #[error("Workspace not found")]
    WorkspaceNotFound,

    /// User not member of tenant.
    #[error("Not a member of this tenant")]
    NotTenantMember,

    /// Tenant limit exceeded.
    #[error("Tenant limit exceeded: {limit}")]
    TenantLimitExceeded { limit: String },
}

impl AuthError {
    /// Get the HTTP status code for this error.
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidCredentials => StatusCode::UNAUTHORIZED,
            Self::TokenExpired => StatusCode::UNAUTHORIZED,
            Self::InvalidToken { .. } => StatusCode::UNAUTHORIZED,
            Self::MissingToken => StatusCode::UNAUTHORIZED,
            Self::MissingAuth => StatusCode::UNAUTHORIZED,
            Self::InvalidAuthorizationHeader { .. } => StatusCode::BAD_REQUEST,
            Self::Forbidden { .. } => StatusCode::FORBIDDEN,
            Self::MissingApiKey => StatusCode::UNAUTHORIZED,
            Self::UserNotFound => StatusCode::NOT_FOUND,
            Self::AccountInactive => StatusCode::FORBIDDEN,
            Self::InvalidApiKey => StatusCode::UNAUTHORIZED,
            Self::ApiKeyExpired => StatusCode::UNAUTHORIZED,
            Self::InsufficientScope { .. } => StatusCode::FORBIDDEN,
            Self::RateLimitExceeded => StatusCode::TOO_MANY_REQUESTS,
            Self::AccountLocked => StatusCode::FORBIDDEN,
            Self::WeakPassword { .. } => StatusCode::BAD_REQUEST,
            Self::PasswordHashingFailed { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::TokenGenerationFailed { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::PasswordError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::JwtError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::TenantNotFound => StatusCode::NOT_FOUND,
            Self::WorkspaceNotFound => StatusCode::NOT_FOUND,
            Self::NotTenantMember => StatusCode::FORBIDDEN,
            Self::TenantLimitExceeded { .. } => StatusCode::PAYMENT_REQUIRED,
        }
    }

    /// Get the error code for API responses.
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::InvalidCredentials => "invalid_credentials",
            Self::TokenExpired => "token_expired",
            Self::InvalidToken { .. } => "invalid_token",
            Self::MissingToken => "missing_token",
            Self::MissingAuth => "missing_auth",
            Self::InvalidAuthorizationHeader { .. } => "invalid_authorization_header",
            Self::Forbidden { .. } => "forbidden",
            Self::MissingApiKey => "missing_api_key",
            Self::UserNotFound => "user_not_found",
            Self::AccountInactive => "account_inactive",
            Self::InvalidApiKey => "invalid_api_key",
            Self::ApiKeyExpired => "api_key_expired",
            Self::InsufficientScope { .. } => "insufficient_scope",
            Self::RateLimitExceeded => "rate_limit_exceeded",
            Self::AccountLocked => "account_locked",
            Self::WeakPassword { .. } => "weak_password",
            Self::PasswordHashingFailed { .. } => "password_hashing_failed",
            Self::TokenGenerationFailed { .. } => "token_generation_failed",
            Self::PasswordError(_) => "password_error",
            Self::JwtError(_) => "jwt_error",
            Self::DatabaseError(_) => "database_error",
            Self::Internal(_) => "internal_error",
            Self::TenantNotFound => "tenant_not_found",
            Self::WorkspaceNotFound => "workspace_not_found",
            Self::NotTenantMember => "not_tenant_member",
            Self::TenantLimitExceeded { .. } => "tenant_limit_exceeded",
        }
    }
}

/// API error response format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthErrorResponse {
    /// Error code.
    pub error: String,

    /// Human-readable error message.
    pub message: String,

    /// Additional details (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = AuthErrorResponse {
            error: self.error_code().to_string(),
            message: self.to_string(),
            details: None,
        };

        (status, axum::Json(body)).into_response()
    }
}

impl From<jsonwebtoken::errors::Error> for AuthError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        use jsonwebtoken::errors::ErrorKind;
        match err.kind() {
            ErrorKind::ExpiredSignature => Self::TokenExpired,
            _ => Self::InvalidToken {
                reason: err.to_string(),
            },
        }
    }
}

impl From<argon2::password_hash::Error> for AuthError {
    fn from(err: argon2::password_hash::Error) -> Self {
        Self::PasswordError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(
            AuthError::InvalidCredentials.status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            AuthError::Forbidden {
                required_permission: "test".to_string()
            }
            .status_code(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(AuthError::UserNotFound.status_code(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(
            AuthError::InvalidCredentials.error_code(),
            "invalid_credentials"
        );
        assert_eq!(AuthError::TokenExpired.error_code(), "token_expired");
    }
}
