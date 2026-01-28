//! Safety-limited LLM provider wrapper.
//!
//! This module provides a wrapper around any LLM provider that enforces
//! hard safety limits on token generation and request timeouts.
//!
//! ## Purpose
//!
//! Ensures system stability and security by preventing:
//! - Runaway token generation (infinite loops in LLM)
//! - Hung requests that never complete
//! - Resource exhaustion from unbounded API calls
//!
//! ## Implements
//!
//! - **FEAT0777**: Safety limits for LLM calls
//! - **BR0777**: Hard max_tokens limit enforcement
//! - **BR0778**: Request timeout enforcement
//!
//! ## Usage
//!
//! ```rust,ignore
//! use edgequake_llm::{SafetyLimitedProvider, SafetyLimitsConfig, OllamaProvider};
//!
//! let ollama = OllamaProvider::from_env()?;
//! let config = SafetyLimitsConfig::default();
//! let safe_provider = SafetyLimitedProvider::new(ollama, config);
//!
//! // All calls now have hard limits enforced
//! let response = safe_provider.complete("Hello").await?;
//! ```

use async_trait::async_trait;
use std::time::Duration;

use crate::error::{LlmError, Result};
use crate::traits::{ChatMessage, CompletionOptions, EmbeddingProvider, LLMProvider, LLMResponse};
use futures::stream::BoxStream;

/// Default maximum tokens for generation (8192).
///
/// This is a safe default that works for most models:
/// - OpenAI GPT-4o supports up to 16K output
/// - Ollama models vary (4K-32K depending on model)
/// - 8192 is a reasonable middle ground
pub const DEFAULT_MAX_TOKENS: usize = 8192;

/// Default request timeout in seconds (600 = 10 minutes).
///
/// 10 minutes is long enough for:
/// - Complex entity extraction from large documents (100KB+)
/// - Scientific papers with many entities and relationships
/// - Long document summarization
/// - Multi-turn conversations
///
/// But short enough to catch:
/// - Hung connections
/// - Infinite generation loops
/// - Network failures
///
/// **WHY 600 seconds**: Testing showed 124KB scientific papers timeout at 120s
/// during entity extraction. The circuit breaker correctly trips after 3 timeouts,
/// but the underlying issue is insufficient timeout for large documents.
pub const DEFAULT_TIMEOUT_SECS: u64 = 600;

/// Absolute maximum tokens allowed (32768).
///
/// This is a hard cap that cannot be exceeded even if configured higher.
/// Prevents accidental configuration errors from causing system issues.
pub const ABSOLUTE_MAX_TOKENS: usize = 32768;

/// Minimum timeout in seconds (10).
///
/// Prevents configuration errors that would cause immediate timeouts.
pub const MINIMUM_TIMEOUT_SECS: u64 = 10;

/// Maximum timeout in seconds (600 = 10 minutes).
///
/// Prevents runaway requests from consuming resources for too long.
pub const MAXIMUM_TIMEOUT_SECS: u64 = 600;

/// Configuration for safety limits.
#[derive(Debug, Clone)]
pub struct SafetyLimitsConfig {
    /// Maximum tokens to generate per request.
    ///
    /// This limit is enforced even if the underlying provider or
    /// CompletionOptions specifies a higher value.
    pub max_tokens: usize,

    /// Request timeout.
    ///
    /// If a request takes longer than this, it will be cancelled.
    pub timeout: Duration,

    /// Whether to log when limits are enforced.
    pub log_enforcement: bool,
}

impl Default for SafetyLimitsConfig {
    fn default() -> Self {
        Self {
            max_tokens: DEFAULT_MAX_TOKENS,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            log_enforcement: true,
        }
    }
}

impl SafetyLimitsConfig {
    /// Create a new config with custom limits.
    ///
    /// Values are clamped to safe ranges:
    /// - max_tokens: 1 to ABSOLUTE_MAX_TOKENS
    /// - timeout: MINIMUM_TIMEOUT_SECS to MAXIMUM_TIMEOUT_SECS
    pub fn new(max_tokens: usize, timeout_secs: u64) -> Self {
        Self {
            max_tokens: max_tokens.clamp(1, ABSOLUTE_MAX_TOKENS),
            timeout: Duration::from_secs(
                timeout_secs.clamp(MINIMUM_TIMEOUT_SECS, MAXIMUM_TIMEOUT_SECS),
            ),
            log_enforcement: true,
        }
    }

    /// Create config from environment variables.
    ///
    /// Environment variables:
    /// - `EDGEQUAKE_LLM_MAX_TOKENS`: Maximum tokens (default: 8192)
    /// - `EDGEQUAKE_LLM_TIMEOUT_SECS`: Timeout in seconds (default: 120)
    pub fn from_env() -> Self {
        let max_tokens = std::env::var("EDGEQUAKE_LLM_MAX_TOKENS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_MAX_TOKENS)
            .clamp(1, ABSOLUTE_MAX_TOKENS);

        let timeout_secs = std::env::var("EDGEQUAKE_LLM_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_TIMEOUT_SECS)
            .clamp(MINIMUM_TIMEOUT_SECS, MAXIMUM_TIMEOUT_SECS);

        Self {
            max_tokens,
            timeout: Duration::from_secs(timeout_secs),
            log_enforcement: true,
        }
    }

    /// Create a strict config for testing (low limits).
    pub fn strict() -> Self {
        Self {
            max_tokens: 1024,
            timeout: Duration::from_secs(30),
            log_enforcement: true,
        }
    }

    /// Create a permissive config (high limits).
    pub fn permissive() -> Self {
        Self {
            max_tokens: ABSOLUTE_MAX_TOKENS,
            timeout: Duration::from_secs(MAXIMUM_TIMEOUT_SECS),
            log_enforcement: true,
        }
    }

    /// Disable enforcement logging.
    pub fn without_logging(mut self) -> Self {
        self.log_enforcement = false;
        self
    }
}

/// Safety-limited LLM provider wrapper.
///
/// Wraps any LLM provider and enforces hard limits on:
/// - Maximum tokens generated per request
/// - Request timeout
///
/// All limits are enforced regardless of what CompletionOptions specifies.
pub struct SafetyLimitedProvider<P> {
    inner: P,
    config: SafetyLimitsConfig,
}

impl<P> SafetyLimitedProvider<P> {
    /// Create a new safety-limited provider wrapper.
    pub fn new(provider: P, config: SafetyLimitsConfig) -> Self {
        Self {
            inner: provider,
            config,
        }
    }

    /// Create with default safety limits.
    pub fn with_defaults(provider: P) -> Self {
        Self::new(provider, SafetyLimitsConfig::default())
    }

    /// Create with limits from environment variables.
    pub fn from_env(provider: P) -> Self {
        Self::new(provider, SafetyLimitsConfig::from_env())
    }

    /// Get a reference to the inner provider.
    pub fn inner(&self) -> &P {
        &self.inner
    }

    /// Get the safety limits config.
    pub fn config(&self) -> &SafetyLimitsConfig {
        &self.config
    }

    /// Apply max_tokens limit to options.
    ///
    /// Returns new options with max_tokens clamped to config limit.
    fn apply_token_limit(&self, options: &CompletionOptions) -> CompletionOptions {
        let mut opts = options.clone();

        let requested = opts.max_tokens.unwrap_or(self.config.max_tokens);
        let effective = requested.min(self.config.max_tokens);

        if requested != effective && self.config.log_enforcement {
            tracing::warn!(
                requested_tokens = requested,
                enforced_tokens = effective,
                "Safety limit: max_tokens clamped to configured limit"
            );
        }

        opts.max_tokens = Some(effective);
        opts
    }
}

#[async_trait]
impl<P: LLMProvider + Send + Sync> LLMProvider for SafetyLimitedProvider<P> {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn model(&self) -> &str {
        self.inner.model()
    }

    fn max_context_length(&self) -> usize {
        self.inner.max_context_length()
    }

    async fn complete(&self, prompt: &str) -> Result<LLMResponse> {
        let options = CompletionOptions {
            max_tokens: Some(self.config.max_tokens),
            ..Default::default()
        };

        self.complete_with_options(prompt, &options).await
    }

    async fn complete_with_options(
        &self,
        prompt: &str,
        options: &CompletionOptions,
    ) -> Result<LLMResponse> {
        let safe_options = self.apply_token_limit(options);

        // Apply timeout
        let result = tokio::time::timeout(
            self.config.timeout,
            self.inner.complete_with_options(prompt, &safe_options),
        )
        .await;

        match result {
            Ok(inner_result) => inner_result,
            Err(_elapsed) => {
                if self.config.log_enforcement {
                    tracing::error!(
                        timeout_secs = self.config.timeout.as_secs(),
                        "Safety limit: LLM request timed out"
                    );
                }
                Err(LlmError::Timeout)
            }
        }
    }

    async fn chat(
        &self,
        messages: &[ChatMessage],
        options: Option<&CompletionOptions>,
    ) -> Result<LLMResponse> {
        let default_options = CompletionOptions {
            max_tokens: Some(self.config.max_tokens),
            ..Default::default()
        };

        let safe_options = match options {
            Some(opts) => self.apply_token_limit(opts),
            None => default_options,
        };

        // Apply timeout
        let result = tokio::time::timeout(
            self.config.timeout,
            self.inner.chat(messages, Some(&safe_options)),
        )
        .await;

        match result {
            Ok(inner_result) => inner_result,
            Err(_elapsed) => {
                if self.config.log_enforcement {
                    tracing::error!(
                        timeout_secs = self.config.timeout.as_secs(),
                        message_count = messages.len(),
                        "Safety limit: LLM chat request timed out"
                    );
                }
                Err(LlmError::Timeout)
            }
        }
    }

    async fn stream(&self, prompt: &str) -> Result<BoxStream<'static, Result<String>>> {
        // For streaming, we can't easily enforce token limits on the response
        // But we can apply timeout to the initial connection
        let result = tokio::time::timeout(self.config.timeout, self.inner.stream(prompt)).await;

        match result {
            Ok(inner_result) => inner_result,
            Err(_elapsed) => {
                if self.config.log_enforcement {
                    tracing::error!(
                        timeout_secs = self.config.timeout.as_secs(),
                        "Safety limit: LLM stream request timed out"
                    );
                }
                Err(LlmError::Timeout)
            }
        }
    }

    async fn stream_with_options(
        &self,
        prompt: &str,
        options: &CompletionOptions,
    ) -> Result<BoxStream<'static, Result<String>>> {
        let safe_options = self.apply_token_limit(options);

        let result = tokio::time::timeout(
            self.config.timeout,
            self.inner.stream_with_options(prompt, &safe_options),
        )
        .await;

        match result {
            Ok(inner_result) => inner_result,
            Err(_elapsed) => {
                if self.config.log_enforcement {
                    tracing::error!(
                        timeout_secs = self.config.timeout.as_secs(),
                        "Safety limit: LLM stream request timed out"
                    );
                }
                Err(LlmError::Timeout)
            }
        }
    }

    fn supports_streaming(&self) -> bool {
        self.inner.supports_streaming()
    }
}

/// Safety-limited embedding provider wrapper.
///
/// Applies timeout to embedding requests.
pub struct SafetyLimitedEmbeddingProvider<P> {
    inner: P,
    config: SafetyLimitsConfig,
}

impl<P> SafetyLimitedEmbeddingProvider<P> {
    /// Create a new safety-limited embedding provider wrapper.
    pub fn new(provider: P, config: SafetyLimitsConfig) -> Self {
        Self {
            inner: provider,
            config,
        }
    }

    /// Create with default safety limits.
    pub fn with_defaults(provider: P) -> Self {
        Self::new(provider, SafetyLimitsConfig::default())
    }

    /// Get a reference to the inner provider.
    pub fn inner(&self) -> &P {
        &self.inner
    }
}

#[async_trait]
impl<P: EmbeddingProvider + Send + Sync> EmbeddingProvider for SafetyLimitedEmbeddingProvider<P> {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn model(&self) -> &str {
        self.inner.model()
    }

    fn dimension(&self) -> usize {
        self.inner.dimension()
    }

    fn max_tokens(&self) -> usize {
        self.inner.max_tokens()
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let result = tokio::time::timeout(self.config.timeout, self.inner.embed(texts)).await;

        match result {
            Ok(inner_result) => inner_result,
            Err(_elapsed) => {
                if self.config.log_enforcement {
                    tracing::error!(
                        timeout_secs = self.config.timeout.as_secs(),
                        text_count = texts.len(),
                        "Safety limit: Embedding request timed out"
                    );
                }
                Err(LlmError::Timeout)
            }
        }
    }
}

// ============================================================================
// Arc-based wrapper variants for use with factory
// ============================================================================

use std::sync::Arc;

/// Safety-limited LLM provider wrapper that works with `Arc<dyn LLMProvider>`.
///
/// This variant is used by ProviderFactory when creating workspace-specific providers.
pub struct SafetyLimitedProviderWrapper {
    inner: Arc<dyn LLMProvider>,
    config: SafetyLimitsConfig,
}

impl SafetyLimitedProviderWrapper {
    /// Create a new safety-limited provider wrapper.
    pub fn new(provider: Arc<dyn LLMProvider>, config: SafetyLimitsConfig) -> Self {
        Self {
            inner: provider,
            config,
        }
    }

    /// Apply max_tokens limit to options.
    fn apply_token_limit(&self, options: &CompletionOptions) -> CompletionOptions {
        let mut opts = options.clone();

        let requested = opts.max_tokens.unwrap_or(self.config.max_tokens);
        let effective = requested.min(self.config.max_tokens);

        if requested != effective && self.config.log_enforcement {
            tracing::warn!(
                requested_tokens = requested,
                enforced_tokens = effective,
                "Safety limit: max_tokens clamped to configured limit"
            );
        }

        opts.max_tokens = Some(effective);
        opts
    }
}

#[async_trait]
impl LLMProvider for SafetyLimitedProviderWrapper {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn model(&self) -> &str {
        self.inner.model()
    }

    fn max_context_length(&self) -> usize {
        self.inner.max_context_length()
    }

    async fn complete(&self, prompt: &str) -> Result<LLMResponse> {
        let options = CompletionOptions {
            max_tokens: Some(self.config.max_tokens),
            ..Default::default()
        };

        self.complete_with_options(prompt, &options).await
    }

    async fn complete_with_options(
        &self,
        prompt: &str,
        options: &CompletionOptions,
    ) -> Result<LLMResponse> {
        let safe_options = self.apply_token_limit(options);

        let result = tokio::time::timeout(
            self.config.timeout,
            self.inner.complete_with_options(prompt, &safe_options),
        )
        .await;

        match result {
            Ok(inner_result) => inner_result,
            Err(_elapsed) => {
                if self.config.log_enforcement {
                    tracing::error!(
                        timeout_secs = self.config.timeout.as_secs(),
                        "Safety limit: LLM request timed out"
                    );
                }
                Err(LlmError::Timeout)
            }
        }
    }

    async fn chat(
        &self,
        messages: &[ChatMessage],
        options: Option<&CompletionOptions>,
    ) -> Result<LLMResponse> {
        let default_options = CompletionOptions {
            max_tokens: Some(self.config.max_tokens),
            ..Default::default()
        };

        let safe_options = match options {
            Some(opts) => self.apply_token_limit(opts),
            None => default_options,
        };

        let result = tokio::time::timeout(
            self.config.timeout,
            self.inner.chat(messages, Some(&safe_options)),
        )
        .await;

        match result {
            Ok(inner_result) => inner_result,
            Err(_elapsed) => {
                if self.config.log_enforcement {
                    tracing::error!(
                        timeout_secs = self.config.timeout.as_secs(),
                        message_count = messages.len(),
                        "Safety limit: LLM chat request timed out"
                    );
                }
                Err(LlmError::Timeout)
            }
        }
    }

    async fn stream(&self, prompt: &str) -> Result<BoxStream<'static, Result<String>>> {
        let result = tokio::time::timeout(self.config.timeout, self.inner.stream(prompt)).await;

        match result {
            Ok(inner_result) => inner_result,
            Err(_elapsed) => {
                if self.config.log_enforcement {
                    tracing::error!(
                        timeout_secs = self.config.timeout.as_secs(),
                        "Safety limit: LLM stream request timed out"
                    );
                }
                Err(LlmError::Timeout)
            }
        }
    }

    async fn stream_with_options(
        &self,
        prompt: &str,
        options: &CompletionOptions,
    ) -> Result<BoxStream<'static, Result<String>>> {
        let safe_options = self.apply_token_limit(options);

        let result = tokio::time::timeout(
            self.config.timeout,
            self.inner.stream_with_options(prompt, &safe_options),
        )
        .await;

        match result {
            Ok(inner_result) => inner_result,
            Err(_elapsed) => {
                if self.config.log_enforcement {
                    tracing::error!(
                        timeout_secs = self.config.timeout.as_secs(),
                        "Safety limit: LLM stream request timed out"
                    );
                }
                Err(LlmError::Timeout)
            }
        }
    }

    fn supports_streaming(&self) -> bool {
        self.inner.supports_streaming()
    }
}

/// Safety-limited embedding provider wrapper that works with `Arc<dyn EmbeddingProvider>`.
pub struct SafetyLimitedEmbeddingProviderWrapper {
    inner: Arc<dyn EmbeddingProvider>,
    config: SafetyLimitsConfig,
}

impl SafetyLimitedEmbeddingProviderWrapper {
    /// Create a new safety-limited embedding provider wrapper.
    pub fn new(provider: Arc<dyn EmbeddingProvider>, config: SafetyLimitsConfig) -> Self {
        Self {
            inner: provider,
            config,
        }
    }
}

#[async_trait]
impl EmbeddingProvider for SafetyLimitedEmbeddingProviderWrapper {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn model(&self) -> &str {
        self.inner.model()
    }

    fn dimension(&self) -> usize {
        self.inner.dimension()
    }

    fn max_tokens(&self) -> usize {
        self.inner.max_tokens()
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let result = tokio::time::timeout(self.config.timeout, self.inner.embed(texts)).await;

        match result {
            Ok(inner_result) => inner_result,
            Err(_elapsed) => {
                if self.config.log_enforcement {
                    tracing::error!(
                        timeout_secs = self.config.timeout.as_secs(),
                        text_count = texts.len(),
                        "Safety limit: Embedding request timed out"
                    );
                }
                Err(LlmError::Timeout)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockProvider;

    #[test]
    fn test_config_defaults() {
        let config = SafetyLimitsConfig::default();
        assert_eq!(config.max_tokens, DEFAULT_MAX_TOKENS);
        assert_eq!(config.timeout.as_secs(), DEFAULT_TIMEOUT_SECS);
        assert!(config.log_enforcement);
    }

    #[test]
    fn test_config_clamping() {
        // Test max_tokens clamping
        let config = SafetyLimitsConfig::new(100_000, 60);
        assert_eq!(config.max_tokens, ABSOLUTE_MAX_TOKENS);

        let config = SafetyLimitsConfig::new(0, 60);
        assert_eq!(config.max_tokens, 1);

        // Test timeout clamping
        let config = SafetyLimitsConfig::new(1000, 5);
        assert_eq!(config.timeout.as_secs(), MINIMUM_TIMEOUT_SECS);

        let config = SafetyLimitsConfig::new(1000, 10000);
        assert_eq!(config.timeout.as_secs(), MAXIMUM_TIMEOUT_SECS);
    }

    #[test]
    fn test_config_from_env() {
        // Save current env
        let saved_max = std::env::var("EDGEQUAKE_LLM_MAX_TOKENS").ok();
        let saved_timeout = std::env::var("EDGEQUAKE_LLM_TIMEOUT_SECS").ok();

        // Test with custom values
        std::env::set_var("EDGEQUAKE_LLM_MAX_TOKENS", "4096");
        std::env::set_var("EDGEQUAKE_LLM_TIMEOUT_SECS", "60");

        let config = SafetyLimitsConfig::from_env();
        assert_eq!(config.max_tokens, 4096);
        assert_eq!(config.timeout.as_secs(), 60);

        // Restore env
        match saved_max {
            Some(v) => std::env::set_var("EDGEQUAKE_LLM_MAX_TOKENS", v),
            None => std::env::remove_var("EDGEQUAKE_LLM_MAX_TOKENS"),
        }
        match saved_timeout {
            Some(v) => std::env::set_var("EDGEQUAKE_LLM_TIMEOUT_SECS", v),
            None => std::env::remove_var("EDGEQUAKE_LLM_TIMEOUT_SECS"),
        }
    }

    #[test]
    fn test_token_limit_enforcement() {
        let mock = MockProvider::new();
        let config = SafetyLimitsConfig::new(1000, 60).without_logging();
        let provider = SafetyLimitedProvider::new(mock, config);

        // Test with no max_tokens specified
        let opts = CompletionOptions::default();
        let safe_opts = provider.apply_token_limit(&opts);
        assert_eq!(safe_opts.max_tokens, Some(1000));

        // Test with lower max_tokens - should keep lower value
        let opts = CompletionOptions {
            max_tokens: Some(500),
            ..Default::default()
        };
        let safe_opts = provider.apply_token_limit(&opts);
        assert_eq!(safe_opts.max_tokens, Some(500));

        // Test with higher max_tokens - should clamp
        let opts = CompletionOptions {
            max_tokens: Some(5000),
            ..Default::default()
        };
        let safe_opts = provider.apply_token_limit(&opts);
        assert_eq!(safe_opts.max_tokens, Some(1000));
    }

    #[tokio::test]
    async fn test_provider_delegation() {
        let mock = MockProvider::new();
        let provider = SafetyLimitedProvider::with_defaults(mock);

        assert_eq!(provider.name(), "mock");
        // MockProvider's model and context length are configurable, just verify they're reasonable
        assert!(!provider.model().is_empty());
        assert!(provider.max_context_length() > 0);
        assert!(provider.supports_streaming());
    }

    #[tokio::test]
    async fn test_complete_with_limits() {
        let mock = MockProvider::new();
        let config = SafetyLimitsConfig::new(1000, 60).without_logging();
        let provider = SafetyLimitedProvider::new(mock, config);

        let result = provider.complete("Hello").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_chat_with_limits() {
        let mock = MockProvider::new();
        let config = SafetyLimitsConfig::new(1000, 60).without_logging();
        let provider = SafetyLimitedProvider::new(mock, config);

        let messages = vec![ChatMessage::user("Hello")];
        let result = provider.chat(&messages, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_embedding_with_timeout() {
        let mock = MockProvider::new();
        let config = SafetyLimitsConfig::new(1000, 60).without_logging();
        let provider = SafetyLimitedEmbeddingProvider::new(mock, config);

        assert_eq!(provider.name(), "mock");
        assert_eq!(provider.dimension(), 1536);

        let texts = vec!["Hello".to_string()];
        let result = provider.embed(&texts).await;
        assert!(result.is_ok());
    }
}
