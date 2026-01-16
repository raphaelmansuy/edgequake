//! Provider Resolution Error Types
//!
//! This module defines the error types used by the `WorkspaceProviderResolver`
//! to communicate provider resolution failures with clear semantics.
//!
//! ## Design Principles
//!
//! 1. **Actionable Errors**: Each error variant includes information the caller
//!    needs to take corrective action or provide helpful user feedback.
//!
//! 2. **API Key Detection**: The `is_api_key_error` flag helps callers provide
//!    specific guidance about configuration issues.
//!
//! 3. **Structured Logging**: Error types support structured logging with
//!    relevant context fields.
//!
//! @implements OODA-226: Provider resolution error types for unified handling

use thiserror::Error;

/// Errors that can occur during provider resolution.
///
/// These errors represent the different failure modes when attempting to
/// create LLM or embedding providers for a workspace.
///
/// ## Usage
///
/// ```rust,ignore
/// use edgequake_api::providers::ProviderResolutionError;
///
/// fn handle_error(err: ProviderResolutionError) {
///     if err.is_api_key_error() {
///         // Provide specific guidance about API key configuration
///     }
/// }
/// ```
#[derive(Debug, Error)]
pub enum ProviderResolutionError {
    /// The requested workspace was not found in the database.
    #[error("Workspace not found: {workspace_id}")]
    WorkspaceNotFound {
        /// The workspace ID that was not found
        workspace_id: String,
    },

    /// Failed to create the requested provider.
    ///
    /// This can happen due to:
    /// - Missing API keys (e.g., OPENAI_API_KEY not set)
    /// - Invalid model names
    /// - Provider service unavailable
    #[error("Provider creation failed for {provider}/{model}: {reason}")]
    ProviderCreationFailed {
        /// The provider name (e.g., "openai", "ollama")
        provider: String,
        /// The model name (e.g., "gpt-4o-mini", "gemma3:12b")
        model: String,
        /// The reason for the failure
        reason: String,
        /// True if this is an API key configuration issue
        is_api_key_error: bool,
    },

    /// The workspace ID format is invalid (not a valid UUID).
    #[error("Invalid workspace ID format: {0}")]
    InvalidWorkspaceId(String),

    /// The workspace service returned an error.
    #[error("Workspace service error: {0}")]
    WorkspaceServiceError(String),

    /// The provider name is empty or invalid.
    #[error("Invalid provider name: {0}")]
    InvalidProviderName(String),
}

impl ProviderResolutionError {
    /// Check if this error is due to a missing or invalid API key.
    ///
    /// Callers can use this to provide specific guidance about
    /// setting environment variables like `OPENAI_API_KEY`.
    pub fn is_api_key_error(&self) -> bool {
        match self {
            Self::ProviderCreationFailed {
                is_api_key_error, ..
            } => *is_api_key_error,
            _ => false,
        }
    }

    /// Get the provider name if this is a provider creation error.
    pub fn provider_name(&self) -> Option<&str> {
        match self {
            Self::ProviderCreationFailed { provider, .. } => Some(provider),
            _ => None,
        }
    }

    /// Get the model name if this is a provider creation error.
    pub fn model_name(&self) -> Option<&str> {
        match self {
            Self::ProviderCreationFailed { model, .. } => Some(model),
            _ => None,
        }
    }

    /// Create an API key error for a specific provider.
    ///
    /// This is a convenience constructor for the common case where
    /// an API key environment variable is not set.
    pub fn api_key_missing(provider: &str, model: &str, env_var: &str) -> Self {
        Self::ProviderCreationFailed {
            provider: provider.to_string(),
            model: model.to_string(),
            reason: format!("{} environment variable is not set", env_var),
            is_api_key_error: true,
        }
    }

    /// Create a provider creation error from a string error.
    ///
    /// This method automatically detects API key errors by checking
    /// for common patterns in the error message.
    pub fn from_creation_error(provider: &str, model: &str, error: &str) -> Self {
        let error_lower = error.to_lowercase();
        let is_api_key_error = error_lower.contains("api_key")
            || error_lower.contains("api key")
            || error_lower.contains("openai_api_key")
            || error_lower.contains("anthropic_api_key")
            || error_lower.contains("authentication")
            || error_lower.contains("unauthorized");

        Self::ProviderCreationFailed {
            provider: provider.to_string(),
            model: model.to_string(),
            reason: error.to_string(),
            is_api_key_error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_detection() {
        let err = ProviderResolutionError::from_creation_error(
            "openai",
            "gpt-4o-mini",
            "OPENAI_API_KEY environment variable not set",
        );
        assert!(err.is_api_key_error());
        assert_eq!(err.provider_name(), Some("openai"));
        assert_eq!(err.model_name(), Some("gpt-4o-mini"));
    }

    #[test]
    fn test_non_api_key_error() {
        let err = ProviderResolutionError::from_creation_error(
            "ollama",
            "gemma3:12b",
            "Connection refused",
        );
        assert!(!err.is_api_key_error());
    }

    #[test]
    fn test_api_key_missing_constructor() {
        let err =
            ProviderResolutionError::api_key_missing("openai", "gpt-4o-mini", "OPENAI_API_KEY");
        assert!(err.is_api_key_error());
        assert!(err.to_string().contains("OPENAI_API_KEY"));
    }

    #[test]
    fn test_workspace_not_found() {
        let err = ProviderResolutionError::WorkspaceNotFound {
            workspace_id: "test-123".to_string(),
        };
        assert!(!err.is_api_key_error());
        assert!(err.to_string().contains("test-123"));
    }
}
