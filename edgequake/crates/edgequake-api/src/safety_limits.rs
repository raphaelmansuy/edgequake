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

/// Default maximum tokens for generation (16384).
///
/// WHY 16384: Entity extraction prompts generate structured JSON that can contain
/// 100+ entities with descriptions. At an average of ~100 tokens per entity, a
/// moderately complex chunk produces 10 000+ output tokens. 8 192 was too
/// conservative and caused JSON-EOF truncation errors on attempt 3 of 3.
/// 16 384 matches the `max_tokens` the LLM extractor already requests.
pub const DEFAULT_MAX_TOKENS: usize = 16384;

/// Default request timeout in seconds (600 = 10 minutes).
pub const DEFAULT_TIMEOUT_SECS: u64 = 600;

/// Absolute maximum tokens allowed (65536).
///
/// WHY 65536: Allows operators to configure larger budgets for very dense documents
/// via `EDGEQUAKE_LLM_MAX_TOKENS`. The previous cap of 32 768 prevented legitimate
/// extraction of large entity lists from complex technical PDFs.
pub const ABSOLUTE_MAX_TOKENS: usize = 65536;

/// Minimum timeout in seconds (10).
pub const MINIMUM_TIMEOUT_SECS: u64 = 10;

/// Maximum timeout in seconds (3600 = 1 hour).
///
/// WHY 3600: The previous cap of 600 s (10 min) was appropriate for cloud
/// APIs (OpenAI, Anthropic) but too restrictive for local LLMs running on
/// consumer hardware (Ollama on a single GPU can take 5–10 minutes per large
/// chunk).  Raising to 1 hour lets operators set
/// `EDGEQUAKE_LLM_TIMEOUT_SECS=1800` without hitting an invisible wall.
/// The real per-chunk safeguard is `EDGEQUAKE_CHUNK_TIMEOUT_SECS` in the
/// pipeline layer; this is the HTTP-level safety backstop.
pub const MAXIMUM_TIMEOUT_SECS: u64 = 3600;

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

    fn max_batch_size(&self) -> usize {
        self.inner.max_batch_size()
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

/// Validate that the required API key environment variable is set and non-empty for the
/// given provider, returning a clear `ConfigError` before attempting to build the client.
fn check_api_key(provider_name: &str) -> Result<()> {
    let (env_var, display_name) = match provider_name {
        "openai" => ("OPENAI_API_KEY", "OpenAI"),
        "anthropic" => ("ANTHROPIC_API_KEY", "Anthropic"),
        "gemini" => ("GEMINI_API_KEY", "Gemini"),
        "mistral" => ("MISTRAL_API_KEY", "Mistral"),
        "xai" => ("XAI_API_KEY", "xAI"),
        "openrouter" => ("OPENROUTER_API_KEY", "OpenRouter"),
        _ => return Ok(()), // Local / key-less providers (ollama, lmstudio, mock, etc.)
    };
    let key_present = std::env::var(env_var)
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    if !key_present {
        return Err(LlmError::ConfigError(format!(
            "{env_var} is not set. To use the {display_name} provider, \
             set the environment variable and restart the server. \
             Alternatively, select the Ollama provider which runs locally."
        )));
    }
    Ok(())
}

/// Create a safety-limited LLM provider from workspace configuration.
pub fn create_safe_llm_provider(provider_name: &str, model: &str) -> Result<Arc<dyn LLMProvider>> {
    check_api_key(provider_name)?;

    // WHY: Same compat guard as create_safe_vision_provider — entity extraction
    // tasks may also carry stale model names from a prior provider session.
    let effective_model = if is_model_provider_mismatch(provider_name, model) {
        let corrected = default_model_for_provider(provider_name);
        tracing::warn!(
            provider = provider_name,
            requested_model = model,
            corrected_model = corrected,
            "COMPAT-GUARD: LLM model/provider mismatch — auto-correcting to provider default."
        );
        corrected
    } else {
        model
    };

    let inner = ProviderFactory::create_llm_provider(provider_name, effective_model)?;
    let config = SafetyLimitsConfig::from_env();

    tracing::info!(
        provider = provider_name,
        model = effective_model,
        max_tokens = config.max_tokens,
        timeout_secs = config.timeout.as_secs(),
        "Creating safety-limited LLM provider"
    );

    Ok(Arc::new(SafetyLimitedProviderWrapper::new(inner, config)))
}

/// Create a safety-limited embedding provider from workspace configuration.
///
/// FIX #163: When the provider is OpenAI-compatible, checks `EDGEQUAKE_EMBEDDING_BASE_URL`
/// and `EDGEQUAKE_EMBEDDING_API_KEY` before falling back to standard env vars.
pub fn create_safe_embedding_provider(
    provider_name: &str,
    model: &str,
    dimension: usize,
) -> Result<Arc<dyn EmbeddingProvider>> {
    // FIX #163: If embedding-specific env vars are set and provider is openai-compatible,
    // create the provider with dedicated credentials.
    let is_openai_compatible = matches!(
        provider_name.to_ascii_lowercase().as_str(),
        "openai" | "openai-compatible" | "openai_compatible"
    );

    let inner = if is_openai_compatible {
        let embed_base_url = std::env::var("EDGEQUAKE_EMBEDDING_BASE_URL").ok();
        let embed_api_key = std::env::var("EDGEQUAKE_EMBEDDING_API_KEY").ok();

        if embed_base_url.is_some() || embed_api_key.is_some() {
            let api_key = embed_api_key
                .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                .unwrap_or_default();
            let base_url = embed_base_url.or_else(|| std::env::var("OPENAI_BASE_URL").ok());

            let provider: Arc<dyn EmbeddingProvider> = if let Some(base_url) = base_url {
                Arc::new(
                    edgequake_llm::OpenAIProvider::compatible(api_key, base_url)
                        .with_embedding_model(model),
                )
            } else {
                Arc::new(edgequake_llm::OpenAIProvider::new(api_key).with_embedding_model(model))
            };
            provider
        } else {
            ProviderFactory::create_embedding_provider(provider_name, model, dimension)?
        }
    } else {
        ProviderFactory::create_embedding_provider(provider_name, model, dimension)?
    };
    let config = SafetyLimitsConfig::from_env();

    tracing::info!(
        provider = provider_name,
        model = model,
        dimension = dimension,
        timeout_secs = config.timeout.as_secs(),
        "Creating safety-limited embedding provider"
    );

    Ok(Arc::new(SafetyLimitedEmbeddingProviderWrapper::new(
        inner, config,
    )))
}

// ─────────────────────────────────────────────────────────────────────────────
// Vision / PDF provider helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Maximum allowed outer PDF-vision conversion timeout (24 hours).
///
/// This is a sanity upper-bound only. Vision extraction for very large documents
/// (1 000+ pages) with local models can legitimately take hours.
pub const VISION_MAX_OUTER_TIMEOUT_SECS: u64 = 86_400;

/// Returns `true` when `provider_name` refers to a local, in-process inference
/// server (Ollama, LM Studio, …) rather than a cloud API.
///
/// Local providers are memory-bound rather than network-bound, so they need
/// longer per-page timeouts and lower concurrency.
pub fn is_local_provider(provider_name: &str) -> bool {
    matches!(
        provider_name.to_ascii_lowercase().as_str(),
        "ollama" | "lmstudio" | "lm-studio" | "lm_studio" | "mock"
    )
}

/// Returns the recommended default seconds-per-page for the given provider.
///
/// Reads `EDGEQUAKE_PDF_SECS_PER_PAGE` first; falls back to:
/// - Local providers: 30 s / page (conservative for a mid-range GPU)
/// - Cloud providers:  8 s / page
pub fn secs_per_page_for_provider(provider_name: &str) -> u64 {
    if let Ok(val) = std::env::var("EDGEQUAKE_PDF_SECS_PER_PAGE") {
        if let Ok(n) = val.parse::<u64>() {
            // Enforce a floor of 5 s to prevent accidentally tiny timeouts.
            return n.max(5);
        }
    }
    if is_local_provider(provider_name) {
        30
    } else {
        8
    }
}

/// Compute the outer vision-conversion timeout for the entire PDF.
///
/// Formula: `120 + (page_count × secs_per_page_for_provider(provider))`
/// clamped to `VISION_MAX_OUTER_TIMEOUT_SECS`.
pub fn vision_outer_timeout_secs(provider_name: &str, page_count: usize) -> u64 {
    let per_page = secs_per_page_for_provider(provider_name);
    let computed = 120_u64.saturating_add(per_page.saturating_mul(page_count as u64));
    computed.min(VISION_MAX_OUTER_TIMEOUT_SECS)
}

/// Returns the per-page LLM call timeout for vision/OCR requests.
///
/// Reads `EDGEQUAKE_VISION_PAGE_TIMEOUT_SECS` first; falls back to:
/// - Local providers: 600 s per page (no hard upper cap applied here)
/// - Cloud providers: 120 s per page
///
/// Unlike `create_safe_llm_provider`, this value is NOT clamped to
/// `MAXIMUM_TIMEOUT_SECS` so that local providers can handle slow pages.
pub fn vision_page_timeout_secs(provider_name: &str) -> u64 {
    if let Ok(val) = std::env::var("EDGEQUAKE_VISION_PAGE_TIMEOUT_SECS") {
        if let Ok(n) = val.parse::<u64>() {
            return n.max(10);
        }
    }
    if is_local_provider(provider_name) {
        600
    } else {
        120
    }
}

/// Create a safety-limited LLM provider suitable for **vision/PDF OCR** calls.
///
/// Unlike [`create_safe_llm_provider`] (which caps timeouts at `MAXIMUM_TIMEOUT_SECS`),
/// this function derives the per-page timeout from [`vision_page_timeout_secs`] so that
/// local providers (Ollama, LM Studio) are not artificially cut off mid-page.
///
/// # Usage
/// ```ignore
/// let provider = create_safe_vision_provider("ollama", "glm-ocr:latest")?;
/// ```
pub fn create_safe_vision_provider(
    provider_name: &str,
    model: &str,
) -> Result<Arc<dyn LLMProvider>> {
    check_api_key(provider_name)?;

    // WHY: Guard against stale task data where a model was stored at upload time
    // under one provider (e.g., OpenAI) and is later retried under a different
    // provider (e.g., Ollama). Without this check, Ollama receives "gpt-4.1-nano"
    // and returns 404 Not Found, failing all pages and exhausting all retries.
    //
    // When a clear mismatch is detected (OpenAI model name with non-OpenAI provider),
    // we auto-correct to the provider's default model and log a warning so operators
    // can update stale workspace / task configurations.
    let effective_model = if is_model_provider_mismatch(provider_name, model) {
        let corrected = default_model_for_provider(provider_name);
        tracing::warn!(
            provider = provider_name,
            requested_model = model,
            corrected_model = corrected,
            "COMPAT-GUARD: Model/provider mismatch detected — auto-correcting to provider default. \
             This indicates stale task data or misconfigured workspace settings. \
             Update workspace vision_llm_model to a {}-compatible model to suppress this warning.",
            provider_name
        );
        corrected
    } else {
        model
    };

    let inner = ProviderFactory::create_llm_provider(provider_name, effective_model)?;

    let timeout_secs = vision_page_timeout_secs(provider_name);
    let config = SafetyLimitsConfig {
        max_tokens: DEFAULT_MAX_TOKENS,
        timeout: Duration::from_secs(timeout_secs),
        log_enforcement: true,
    };

    tracing::info!(
        provider = provider_name,
        model = effective_model,
        timeout_secs = timeout_secs,
        is_local = is_local_provider(provider_name),
        "Creating safety-limited VISION LLM provider (provider-aware timeout)"
    );

    Ok(Arc::new(SafetyLimitedProviderWrapper::new(inner, config)))
}

// ─────────────────────────────────────────────────────────────────────────────

/// Detect whether a model name is clearly incompatible with the given provider.
///
/// WHY: Stale task data or misconfigured workspaces can store a model name that
/// was valid for a different provider (e.g., "gpt-4.1-nano" stored when OpenAI
/// was active, then retried with Ollama). We detect the most common cases to
/// auto-correct rather than fail with a confusing 404 from the local provider.
///
/// This is intentionally conservative: we only flag clear cross-provider names
/// (OpenAI model naming conventions used with non-OpenAI providers) to avoid
/// false positives on valid custom model names.
pub fn is_model_provider_mismatch(provider_name: &str, model: &str) -> bool {
    if model.is_empty() {
        return false;
    }

    let provider = provider_name.to_lowercase();
    let model = model.to_lowercase();

    // OpenAI model patterns: gpt-*, o1-*, o3-*, o4-*, text-embedding-*
    let is_openai_model = model.starts_with("gpt-")
        || model.starts_with("o1-")
        || model.starts_with("o3-")
        || model.starts_with("o4-")
        || model.starts_with("text-embedding-");
    // Anthropic model patterns: claude-*
    let is_anthropic_model = model.starts_with("claude-");
    // Gemini model patterns: gemini-*
    let is_gemini_model = model.starts_with("gemini-") || model.starts_with("text-embedding-004");
    // Mistral model patterns: mistral-*, magistral-*, pixtral-*, codestral-*, devstral-*, ministral-*
    let is_mistral_model = model.starts_with("mistral-")
        || model.starts_with("magistral-")
        || model.starts_with("pixtral-")
        || model.starts_with("codestral-")
        || model.starts_with("devstral-")
        || model.starts_with("ministral-");
    // Common local/self-hosted model patterns.
    let is_local_style_model = model.contains(':')
        || model.starts_with("gemma")
        || model.starts_with("llama")
        || model.starts_with("qwen")
        || model.starts_with("mistral")
        || model.starts_with("phi")
        || model.starts_with("deepseek")
        || model.starts_with("glm")
        || model.starts_with("minicpm");

    match provider.as_str() {
        "ollama" | "lmstudio" | "lm-studio" | "lm_studio" => {
            // Local providers cannot run cloud-hosted models.
            is_openai_model || is_anthropic_model || is_gemini_model
        }
        "openai" | "anthropic" | "gemini" | "xai" | "minimax" => {
            // Cloud providers should not inherit self-hosted model names.
            is_local_style_model || model.contains('/')
        }
        "mistral" => {
            // Mismatch when using a model from a different cloud or a purely local namespace.
            // WHY: is_local_style_model includes model.starts_with("mistral") (for Ollama's
            // bare "mistral" / "mistral:latest" tags). We must subtract the Mistral La
            // Plateforme alias set (is_mistral_model) to avoid falsely flagging cloud model
            // names like "mistral-small-latest" that also match the prefix.
            (is_openai_model || is_anthropic_model || is_gemini_model || is_local_style_model)
                && !is_mistral_model
        }
        _ => false,
    }
}

/// Get the default model for a given provider name.
pub fn default_model_for_provider(provider_name: &str) -> &'static str {
    match provider_name.to_lowercase().as_str() {
        "openai" => "gpt-4.1-nano",
        "anthropic" => "claude-sonnet-4-5-20250929",
        "gemini" => "gemini-2.5-flash",
        "xai" => "grok-4-1-fast",
        "openrouter" => "openai/gpt-4o-mini",
        "mistral" => "mistral-small-latest",
        "ollama" => "gemma4:latest",
        "lmstudio" | "lm-studio" | "lm_studio" => "gemma-3n-e4b-it",
        "minimax" => "MiniMax-M2.7",
        "mock" => "mock-model",
        _ => "gpt-4.1-nano",
    }
}
