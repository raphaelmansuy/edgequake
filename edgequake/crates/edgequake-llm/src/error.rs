//! LLM error types.
//!
//! # Error Handling Philosophy
//!
//! Errors should be:
//! 1. **Actionable**: Tell the user what to do, not just what went wrong
//! 2. **Specific**: Include relevant context (model name, token counts, etc.)
//! 3. **Recoverable**: Distinguish transient errors (retry) from permanent ones
//!
//! # Common Errors and Solutions
//!
//! | Error | Cause | Solution |
//! |-------|-------|----------|
//! | `AuthError` | Invalid/expired API key | Check `OPENAI_API_KEY` env var |
//! | `RateLimited` | Too many requests | Wait for `retry_after` seconds |
//! | `TokenLimitExceeded` | Input too long | Reduce chunk size or context |
//! | `ModelNotFound` | Invalid model name | Use `gpt-4.1-nano` or `gpt-4o` |
//! | `Timeout` | Network slow | Increase timeout or retry |

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

    /// Feature not supported.
    #[error("Not supported: {0}")]
    NotSupported(String),

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_error_display() {
        let error = LlmError::ApiError("something went wrong".to_string());
        assert_eq!(error.to_string(), "API error: something went wrong");

        let error = LlmError::RateLimited("too many requests".to_string());
        assert_eq!(error.to_string(), "Rate limit exceeded: too many requests");

        let error = LlmError::InvalidRequest("bad params".to_string());
        assert_eq!(error.to_string(), "Invalid request: bad params");
    }

    #[test]
    fn test_llm_error_auth() {
        let error = LlmError::AuthError("invalid key".to_string());
        assert_eq!(error.to_string(), "Authentication error: invalid key");
    }

    #[test]
    fn test_llm_error_token_limit() {
        let error = LlmError::TokenLimitExceeded {
            max: 4096,
            got: 5000,
        };
        assert_eq!(
            error.to_string(),
            "Token limit exceeded: max 4096, got 5000"
        );
    }

    #[test]
    fn test_llm_error_model_not_found() {
        let error = LlmError::ModelNotFound("gpt-5-turbo".to_string());
        assert_eq!(error.to_string(), "Model not found: gpt-5-turbo");
    }

    #[test]
    fn test_llm_error_network() {
        let error = LlmError::NetworkError("connection refused".to_string());
        assert_eq!(error.to_string(), "Network error: connection refused");
    }

    #[test]
    fn test_llm_error_config() {
        let error = LlmError::ConfigError("missing api key".to_string());
        assert_eq!(error.to_string(), "Configuration error: missing api key");
    }

    #[test]
    fn test_llm_error_provider() {
        let error = LlmError::ProviderError("openai specific error".to_string());
        assert_eq!(error.to_string(), "Provider error: openai specific error");
    }

    #[test]
    fn test_llm_error_timeout() {
        let error = LlmError::Timeout;
        assert_eq!(error.to_string(), "Request timed out");
    }

    #[test]
    fn test_llm_error_not_supported() {
        let error = LlmError::NotSupported("function calling".to_string());
        assert_eq!(error.to_string(), "Not supported: function calling");
    }

    #[test]
    fn test_llm_error_unknown() {
        let error = LlmError::Unknown("mystery error".to_string());
        assert_eq!(error.to_string(), "Unknown error: mystery error");
    }

    #[test]
    fn test_llm_error_debug() {
        let error = LlmError::ApiError("test".to_string());
        let debug = format!("{:?}", error);
        assert!(debug.contains("ApiError"));
        assert!(debug.contains("test"));
    }

    #[test]
    fn test_llm_error_from_serde_json() {
        let json_str = "not json at all";
        let json_err: serde_json::Error =
            serde_json::from_str::<serde_json::Value>(json_str).unwrap_err();
        let llm_err: LlmError = json_err.into();
        assert!(matches!(llm_err, LlmError::SerializationError(_)));
    }
}
