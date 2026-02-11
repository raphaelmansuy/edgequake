//! Error types for the EdgeQuake SDK.

use std::time::Duration;
use thiserror::Error;

/// All errors that can occur when using the EdgeQuake SDK.
#[derive(Error, Debug)]
pub enum Error {
    /// 400 Bad Request
    #[error("Bad request: {message}")]
    BadRequest {
        message: String,
        code: Option<String>,
        details: Option<serde_json::Value>,
    },

    /// 401 Unauthorized
    #[error("Unauthorized: {message}")]
    Unauthorized { message: String },

    /// 403 Forbidden
    #[error("Forbidden: {message}")]
    Forbidden { message: String },

    /// 404 Not Found
    #[error("Not found: {message}")]
    NotFound { message: String },

    /// 409 Conflict
    #[error("Conflict: {message}")]
    Conflict { message: String },

    /// 422 Unprocessable Entity
    #[error("Validation error: {message}")]
    Validation {
        message: String,
        details: Option<serde_json::Value>,
    },

    /// 429 Rate Limited
    #[error("Rate limited (retry after {retry_after:?})")]
    RateLimited {
        message: String,
        retry_after: Option<Duration>,
    },

    /// 500+ Server Error
    #[error("Server error ({status}): {message}")]
    Server {
        status: u16,
        message: String,
        code: Option<String>,
    },

    /// Network/transport error
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// JSON serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// URL parsing error
    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Timeout
    #[error("Timeout after {duration:?} waiting for {operation}")]
    Timeout {
        operation: String,
        duration: Duration,
    },
}

/// Convenience Result alias.
pub type Result<T> = std::result::Result<T, Error>;

/// Server error response body.
#[derive(Debug, Clone, serde::Deserialize)]
pub(crate) struct ErrorResponse {
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub details: Option<serde_json::Value>,
}

impl Error {
    /// Convert an HTTP response into the appropriate error variant.
    pub(crate) async fn from_response(resp: reqwest::Response) -> Self {
        let status = resp.status().as_u16();
        let retry_after = resp
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .map(Duration::from_secs);

        let body = resp.json::<ErrorResponse>().await.ok();
        let message = body
            .as_ref()
            .map(|b| b.message.clone())
            .unwrap_or_else(|| format!("HTTP {status}"));
        let code = body.as_ref().map(|b| b.code.clone());
        let details = body.as_ref().and_then(|b| b.details.clone());

        match status {
            400 => Error::BadRequest {
                message,
                code,
                details,
            },
            401 => Error::Unauthorized { message },
            403 => Error::Forbidden { message },
            404 => Error::NotFound { message },
            409 => Error::Conflict { message },
            422 => Error::Validation {
                message,
                details,
            },
            429 => Error::RateLimited {
                message,
                retry_after,
            },
            _ => Error::Server {
                status,
                message,
                code,
            },
        }
    }

    /// Whether this error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Error::RateLimited { .. }
                | Error::Server {
                    status: 500 | 502 | 503 | 504,
                    ..
                }
                | Error::Network(_)
        )
    }

    /// HTTP status code if this is an API error.
    pub fn status_code(&self) -> Option<u16> {
        match self {
            Error::BadRequest { .. } => Some(400),
            Error::Unauthorized { .. } => Some(401),
            Error::Forbidden { .. } => Some(403),
            Error::NotFound { .. } => Some(404),
            Error::Conflict { .. } => Some(409),
            Error::Validation { .. } => Some(422),
            Error::RateLimited { .. } => Some(429),
            Error::Server { status, .. } => Some(*status),
            _ => None,
        }
    }
}
