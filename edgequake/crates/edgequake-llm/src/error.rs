//! LLM error types.

use thiserror::Error;

/// Result type for LLM operations.
pub type Result<T> = std::result::Result<T, LlmError>;

/// Errors that can occur in LLM operations.
#[derive(Debug, Error)]
pub enum LlmError {
    /// API error from the provider.
    #[error("API error: {0}")]
    ApiError(String),

    /// Rate limit exceeded.
    #[error("Rate limit exceeded: {0}")]
    RateLimited(String),

    /// Invalid request parameters.
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Authentication error.
    #[error("Authentication error: {0}")]
    AuthError(String),

    /// Token limit exceeded.
    #[error("Token limit exceeded: max {max}, got {got}")]
    TokenLimitExceeded { max: usize, got: usize },

    /// Model not found.
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    /// Network error.
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Provider-specific error.
    #[error("Provider error: {0}")]
    ProviderError(String),

    /// Timeout error.
    #[error("Request timed out")]
    Timeout,

    /// Unknown error.
    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<reqwest::Error> for LlmError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            LlmError::Timeout
        } else if err.is_connect() {
            LlmError::NetworkError(format!("Connection failed: {}", err))
        } else {
            LlmError::NetworkError(err.to_string())
        }
    }
}

impl From<async_openai::error::OpenAIError> for LlmError {
    fn from(err: async_openai::error::OpenAIError) -> Self {
        match err {
            async_openai::error::OpenAIError::ApiError(api_err) => {
                let message = api_err.message.clone();
                if message.contains("rate limit") || message.contains("Rate limit") {
                    LlmError::RateLimited(message)
                } else if message.contains("authentication") || message.contains("invalid_api_key")
                {
                    LlmError::AuthError(message)
                } else if message.contains("model") && message.contains("not found") {
                    LlmError::ModelNotFound(message)
                } else {
                    LlmError::ApiError(message)
                }
            }
            async_openai::error::OpenAIError::Reqwest(req_err) => LlmError::from(req_err),
            async_openai::error::OpenAIError::JSONDeserialize(json_err, _context) => {
                LlmError::SerializationError(json_err)
            }
            _ => LlmError::ProviderError(err.to_string()),
        }
    }
}
