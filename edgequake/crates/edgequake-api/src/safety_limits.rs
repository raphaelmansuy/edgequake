//! Safety-limited LLM provider wrapper.
//!
//! This module provides a wrapper around any LLM provider that enforces
//! hard safety limits on token generation and request timeouts.
//!
//! Relocated from edgequake-llm to edgequake-api during the migration
//! to the external edgequake-llm crate (v0.2.1) which does not include
//! this application-level safety layer.

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;

use edgequake_llm::{
    ChatMessage, CompletionOptions, EmbeddingProvider, LLMProvider, LLMResponse, LlmError,
    ProviderFactory, Result,
};
use futures::stream::BoxStream;

/// Default maximum tokens for generation (8192).
pub const DEFAULT_MAX_TOKENS: usize = 8192;

/// Default request timeout in seconds (600 = 10 minutes).
pub const DEFAULT_TIMEOUT_SECS: u64 = 600;

/// Absolute maximum tokens allowed (32768).
pub const ABSOLUTE_MAX_TOKENS: usize = 32768;

/// Minimum timeout in seconds (10).
pub const MINIMUM_TIMEOUT_SECS: u64 = 10;

/// Maximum timeout in seconds (600 = 10 minutes).
pub const MAXIMUM_TIMEOUT_SECS: u64 = 600;

/// Configuration for safety limits.
#[derive(Debug, Clone)]
pub struct SafetyLimitsConfig {
    /// Maximum tokens to generate per request.
    pub max_tokens: usize,
    /// Request timeout.
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

/// Safety-limited LLM provider wrapper that works with `Arc<dyn LLMProvider>`.
pub struct SafetyLimitedProviderWrapper {
    inner: Arc<dyn LLMProvider>,
    config: SafetyLimitsConfig,
}

/// Normalize local OpenAI-compatible aliases to the supported runtime provider.
fn normalize_provider_name(provider_name: &str) -> String {
    match provider_name.trim().to_lowercase().as_str() {
        "litellm-local" | "litellm" | "openai-compatible" => "openai".to_string(),
        _ => provider_name.trim().to_string(),
    }
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

    fn supports_streaming(&self) -> bool {
        self.inner.supports_streaming()
    }
}

/// Safety-limited embedding provider wrapper that works with `Arc<dyn EmbeddingProvider>`.
pub struct SafetyLimitedEmbeddingProviderWrapper {
    inner: Arc<dyn EmbeddingProvider>,
    config: SafetyLimitsConfig,
    dimension: usize,
}

impl SafetyLimitedEmbeddingProviderWrapper {
    /// Create a new safety-limited embedding provider wrapper.
    pub fn new(
        provider: Arc<dyn EmbeddingProvider>,
        dimension: usize,
        config: SafetyLimitsConfig,
    ) -> Self {
        Self {
            inner: provider,
            config,
            dimension,
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
        self.dimension
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

/// Create a safety-limited LLM provider from workspace configuration.
pub fn create_safe_llm_provider(provider_name: &str, model: &str) -> Result<Arc<dyn LLMProvider>> {
    let runtime_provider_name = normalize_provider_name(provider_name);
    let inner = ProviderFactory::create_llm_provider(&runtime_provider_name, model)?;
    let config = SafetyLimitsConfig::from_env();

    tracing::info!(
        provider = provider_name,
        runtime_provider = runtime_provider_name.as_str(),
        model = model,
        max_tokens = config.max_tokens,
        timeout_secs = config.timeout.as_secs(),
        "Creating safety-limited LLM provider"
    );

    Ok(Arc::new(SafetyLimitedProviderWrapper::new(inner, config)))
}

/// Create a safety-limited embedding provider from workspace configuration.
pub fn create_safe_embedding_provider(
    provider_name: &str,
    model: &str,
    dimension: usize,
) -> Result<Arc<dyn EmbeddingProvider>> {
    let runtime_provider_name = normalize_provider_name(provider_name);
    let inner =
        ProviderFactory::create_embedding_provider(&runtime_provider_name, model, dimension)?;
    let config = SafetyLimitsConfig::from_env();

    tracing::info!(
        provider = provider_name,
        runtime_provider = runtime_provider_name.as_str(),
        model = model,
        dimension = dimension,
        timeout_secs = config.timeout.as_secs(),
        "Creating safety-limited embedding provider"
    );

    Ok(Arc::new(SafetyLimitedEmbeddingProviderWrapper::new(
        inner, dimension, config,
    )))
}

/// Get the default model for a given provider name.
pub fn default_model_for_provider(provider_name: &str) -> &'static str {
    match provider_name.to_lowercase().as_str() {
        "openai" => "gpt-4.1-nano",
        "anthropic" => "claude-sonnet-4-5-20250929",
        "gemini" => "gemini-2.5-flash",
        "xai" => "grok-4-1-fast",
        "openrouter" => "openai/gpt-4o-mini",
        "ollama" => "gemma3:12b",
        "litellm-local" | "litellm" | "openai-compatible" => "qwen3.5-35b-a3b",
        "lmstudio" | "lm-studio" | "lm_studio" => "gemma-3n-e4b-it",
        "minimax" => "MiniMax-M2.7",
        "mock" => "mock-model",
        _ => "gpt-4.1-nano",
    }
}

/// Get the default embedding model for a given provider name.
pub fn default_embedding_model_for_provider(provider_name: &str) -> &'static str {
    match provider_name.to_lowercase().as_str() {
        "openai" => "text-embedding-3-small",
        "ollama" => "embeddinggemma:latest",
        "litellm-local" | "litellm" | "openai-compatible" => "qwen3-embedding-0.6b",
        "lmstudio" | "lm-studio" | "lm_studio" => "nomic-embed-text",
        "mock" => "mock-embedding",
        _ => "text-embedding-3-small",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn litellm_local_uses_qwen_default_model() {
        assert_eq!(
            default_model_for_provider("litellm-local"),
            "qwen3.5-35b-a3b"
        );
    }

    #[test]
    fn litellm_local_normalizes_to_openai_runtime_provider() {
        assert_eq!(normalize_provider_name("litellm-local"), "openai");
    }

    #[test]
    fn litellm_local_uses_qwen_default_embedding_model() {
        assert_eq!(
            default_embedding_model_for_provider("litellm-local"),
            "qwen3-embedding-0.6b"
        );
    }
}
