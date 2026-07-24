//! Safety-limited LLM provider wrapper.
//!
//! This module provides a wrapper around any LLM provider that enforces
//! hard safety limits on token generation and request timeouts.
//!
//! Relocated from edgequake-llm to edgequake-api during the migration
//! to the external edgequake-llm crate (v0.2.1) which does not include
//! this application-level safety layer.

use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use edgequake_llm::{
    ApplicationContext, ChatMessage, CompletionOptions, EmbeddingProvider, LLMProvider,
    LLMResponse, LlmError, ProviderFactory, Result,
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

/// E2E test hook — when set (and `EDGEQUAKE_ALLOW_TEST_PROVIDER_OVERRIDE=1`),
/// workspace pipeline factory reuses seeded mock providers (SPEC-021 worker tests).
#[allow(clippy::type_complexity)]
static TEST_PROVIDER_OVERRIDE: Mutex<Option<(Arc<dyn LLMProvider>, Arc<dyn EmbeddingProvider>)>> =
    Mutex::new(None);

const TEST_PROVIDER_OVERRIDE_ENV: &str = "EDGEQUAKE_ALLOW_TEST_PROVIDER_OVERRIDE";

/// Wire shared mock providers for worker E2E (see `tests/common/mod.rs`).
pub fn set_test_provider_override(
    llm: Arc<dyn LLMProvider>,
    embedding: Arc<dyn EmbeddingProvider>,
) {
    if std::env::var(TEST_PROVIDER_OVERRIDE_ENV).as_deref() != Ok("1") {
        return;
    }
    *TEST_PROVIDER_OVERRIDE
        .lock()
        .expect("test provider override mutex") = Some((llm, embedding));
}

/// Clear E2E provider override (call from `WorkerAppGuard` drop).
pub fn clear_test_provider_override() {
    *TEST_PROVIDER_OVERRIDE
        .lock()
        .expect("test provider override mutex") = None;
}

fn test_provider_override() -> Option<(Arc<dyn LLMProvider>, Arc<dyn EmbeddingProvider>)> {
    TEST_PROVIDER_OVERRIDE
        .lock()
        .expect("test provider override mutex")
        .clone()
}

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

/// Default safe maximum embedding batch count (512).
///
/// Recommended operator default when setting `EDGEQUAKE_EMBEDDING_BATCH_SIZE`
/// for providers without a hard-coded provider clamp (documentation / tests).
///
/// SPEC-083 X-08: this is **not** applied as a silent secondary clamp when the
/// env var is unset — `provider.max_batch_size()` is the live default, and the
/// env (when set) is min'd once in the safety wrapper.
pub const DEFAULT_SAFE_EMBED_BATCH_SIZE: usize = 256;

/// Sentinel meaning "no env override" — wrapper passes through provider batch size.
const EMBED_BATCH_NO_OVERRIDE: usize = usize::MAX;

/// Configuration for safety limits.
#[derive(Debug, Clone)]
pub struct SafetyLimitsConfig {
    /// Maximum tokens to generate per request.
    pub max_tokens: usize,
    /// Request timeout.
    pub timeout: Duration,
    /// Whether to log when limits are enforced.
    pub log_enforcement: bool,
    /// Optional operator cap from `EDGEQUAKE_EMBEDDING_BATCH_SIZE`.
    ///
    /// SPEC-083 X-08 SSOT: when unset (`usize::MAX`), use
    /// `provider.max_batch_size()` alone. When set, clamp once via
    /// `min(provider, env)`.
    pub max_embed_batch_size: usize,
}

impl Default for SafetyLimitsConfig {
    fn default() -> Self {
        Self {
            max_tokens: DEFAULT_MAX_TOKENS,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            log_enforcement: true,
            max_embed_batch_size: Self::env_embed_batch_size(),
        }
    }
}

impl SafetyLimitsConfig {
    /// Read `EDGEQUAKE_EMBEDDING_BATCH_SIZE` (X-08 SSOT). Unset → no override.
    fn env_embed_batch_size() -> usize {
        std::env::var("EDGEQUAKE_EMBEDDING_BATCH_SIZE")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|&n| n > 0)
            .unwrap_or(EMBED_BATCH_NO_OVERRIDE)
    }

    /// Create a new config with custom limits.
    pub fn new(max_tokens: usize, timeout_secs: u64) -> Self {
        Self {
            max_tokens: max_tokens.clamp(1, ABSOLUTE_MAX_TOKENS),
            timeout: Duration::from_secs(
                timeout_secs.clamp(MINIMUM_TIMEOUT_SECS, MAXIMUM_TIMEOUT_SECS),
            ),
            log_enforcement: true,
            max_embed_batch_size: Self::env_embed_batch_size(),
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
            max_embed_batch_size: Self::env_embed_batch_size(),
        }
    }

    /// Create a strict config for testing (low limits).
    pub fn strict() -> Self {
        Self {
            max_tokens: 1024,
            timeout: Duration::from_secs(30),
            log_enforcement: true,
            max_embed_batch_size: DEFAULT_SAFE_EMBED_BATCH_SIZE,
        }
    }

    /// Create a permissive config (high limits).
    pub fn permissive() -> Self {
        Self {
            max_tokens: ABSOLUTE_MAX_TOKENS,
            timeout: Duration::from_secs(MAXIMUM_TIMEOUT_SECS),
            log_enforcement: true,
            max_embed_batch_size: DEFAULT_SAFE_EMBED_BATCH_SIZE,
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
        let _gate = crate::local_inference_gate::acquire_local_inference_permit(self.name()).await;

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
        let _gate = crate::local_inference_gate::acquire_local_inference_permit(self.name()).await;

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
        let _gate = crate::local_inference_gate::acquire_local_inference_permit(self.name()).await;
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
        // SPEC-083 X-08: single clamp — min(provider, EDGEQUAKE_EMBEDDING_BATCH_SIZE)
        // when env is set; otherwise pass through provider.max_batch_size().
        let inner = self.inner.max_batch_size().max(1);
        let cap = self.config.max_embed_batch_size;
        if cap == EMBED_BATCH_NO_OVERRIDE {
            return inner;
        }
        let effective = inner.min(cap.max(1));
        if effective < inner && self.config.log_enforcement {
            tracing::debug!(
                inner_batch_size = inner,
                effective_batch_size = effective,
                "Safety limit: embedding batch size clamped via EDGEQUAKE_EMBEDDING_BATCH_SIZE"
            );
        }
        effective
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let _gate = crate::local_inference_gate::acquire_local_inference_permit(self.name()).await;
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
    let provider = provider_name.to_ascii_lowercase();
    if provider == "vertexai" || provider == "vertex" {
        if !crate::providers::credentials::vertex_auth_configured_sync() {
            return Err(LlmError::ConfigError(
                crate::providers::credentials::vertex_auth_health_error(),
            ));
        }
        return Ok(());
    }

    let (env_var, display_name) = match provider_name {
        "openai" => ("OPENAI_API_KEY", "OpenAI"),
        "anthropic" => ("ANTHROPIC_API_KEY", "Anthropic"),
        "gemini" => ("GEMINI_API_KEY", "Gemini"),
        "mistral" => ("MISTRAL_API_KEY", "Mistral"),
        "xai" => ("XAI_API_KEY", "xAI"),
        "openrouter" => ("OPENROUTER_API_KEY", "OpenRouter"),
        "nvidia" => ("NVIDIA_API_KEY", "NVIDIA NIM"),
        "cohere" => ("COHERE_API_KEY", "Cohere"),
        "jina" => ("JINA_API_KEY", "Jina AI"),
        "huggingface" | "hf" => ("HF_TOKEN", "HuggingFace"),
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

/// Create an LLM provider, using Vertex ADC path when applicable (SPEC-043 §011).
fn create_inner_llm_provider(
    provider_name: &str,
    model: &str,
    ctx: Option<ApplicationContext>,
) -> Result<Arc<dyn LLMProvider>> {
    let provider = provider_name.to_ascii_lowercase();
    if provider == "vertexai" || provider == "vertex" {
        return create_vertex_llm_via_adc(model, ctx);
    }
    match ctx {
        Some(ctx) => ProviderFactory::create_llm_provider_with_context(provider_name, model, ctx),
        None => ProviderFactory::create_llm_provider(provider_name, model),
    }
}

fn create_vertex_llm_via_adc(
    model: &str,
    ctx: Option<ApplicationContext>,
) -> Result<Arc<dyn LLMProvider>> {
    use edgequake_llm::GeminiProvider;

    let actual = model.strip_prefix("vertexai:").unwrap_or(model);
    let handle = tokio::runtime::Handle::try_current().map_err(|_| {
        LlmError::ConfigError(
            "Vertex AI requires a Tokio runtime for Application Default Credentials".to_string(),
        )
    })?;
    let mut provider =
        tokio::task::block_in_place(|| handle.block_on(GeminiProvider::from_env_vertex_ai_adc()))?;
    provider = provider.with_model(actual);
    if let Some(ctx) = ctx {
        provider = provider.with_application_context(ctx);
    }
    Ok(Arc::new(provider))
}

/// Heal Mock → server-runtime provider when Mock is not explicitly allowed.
fn heal_mock_llm_selection<'a>(provider_name: &'a str, model: &'a str) -> (String, String) {
    if crate::provider_visibility::is_mock_provider(provider_name)
        && !crate::provider_visibility::mock_provider_allowed()
    {
        let (healed_model, healed_provider) =
            edgequake_core::Workspace::server_runtime_llm_config();
        tracing::warn!(
            requested_provider = provider_name,
            requested_model = model,
            healed_provider = %healed_provider,
            healed_model = %healed_model,
            "Mock LLM is forbidden in the application — using server runtime provider instead"
        );
        (healed_provider, healed_model)
    } else {
        (provider_name.to_string(), model.to_string())
    }
}

fn heal_mock_embedding_selection(
    provider_name: &str,
    model: &str,
    dimension: usize,
) -> (String, String, usize) {
    if crate::provider_visibility::is_mock_provider(provider_name)
        && !crate::provider_visibility::mock_provider_allowed()
    {
        let (healed_model, healed_provider, healed_dim) =
            edgequake_core::Workspace::server_runtime_embedding_config();
        tracing::warn!(
            requested_provider = provider_name,
            requested_model = model,
            healed_provider = %healed_provider,
            healed_model = %healed_model,
            healed_dimension = healed_dim,
            "Mock embedding is forbidden in the application — using server runtime provider instead"
        );
        (healed_provider, healed_model, healed_dim)
    } else {
        (provider_name.to_string(), model.to_string(), dimension)
    }
}

/// Create a safety-limited LLM provider from workspace configuration.
pub fn create_safe_llm_provider(provider_name: &str, model: &str) -> Result<Arc<dyn LLMProvider>> {
    build_safe_llm_provider(provider_name, model, false)
}

/// Create a safety-limited LLM for **entity extraction** with provider-aware HTTP timeout.
///
/// Local providers (Ollama / LM Studio) get a 900s HTTP budget so the pipeline's
/// 600s per-chunk timeout remains the controlling deadline. Cloud stays at 600s.
pub fn create_safe_extraction_llm_provider(
    provider_name: &str,
    model: &str,
) -> Result<Arc<dyn LLMProvider>> {
    build_safe_llm_provider(provider_name, model, true)
}

fn build_safe_llm_provider(
    provider_name: &str,
    model: &str,
    extraction_profile: bool,
) -> Result<Arc<dyn LLMProvider>> {
    if let Some((llm, _)) = test_provider_override() {
        return Ok(llm);
    }

    let (provider_name, model) = heal_mock_llm_selection(provider_name, model);
    crate::provider_visibility::ensure_non_mock_provider(&provider_name, "LLM")
        .map_err(LlmError::ConfigError)?;
    check_api_key(&provider_name)?;

    // WHY: Same compat guard as create_safe_vision_provider — entity extraction
    // tasks may also carry stale model names from a prior provider session.
    let effective_model = if is_model_provider_mismatch(&provider_name, &model) {
        let corrected = default_model_for_provider(&provider_name);
        tracing::warn!(
            provider = %provider_name,
            requested_model = %model,
            corrected_model = corrected,
            "COMPAT-GUARD: LLM model/provider mismatch — auto-correcting to provider default."
        );
        corrected.to_string()
    } else {
        model
    };

    let config = if extraction_profile {
        SafetyLimitsConfig::from_env_for_extraction(&provider_name)
    } else {
        SafetyLimitsConfig::from_env()
    };

    let inner = create_inner_llm_provider(&provider_name, &effective_model, None)?;

    tracing::info!(
        provider = %provider_name,
        model = %effective_model,
        max_tokens = config.max_tokens,
        timeout_secs = config.timeout.as_secs(),
        extraction_profile,
        is_local = is_slow_local_provider(&provider_name),
        "Creating safety-limited LLM provider"
    );

    Ok(Arc::new(SafetyLimitedProviderWrapper::new(inner, config)))
}

/// Create a safety-limited LLM provider with full application attribution context.
pub fn create_safe_llm_provider_with_context(
    provider_name: &str,
    model: &str,
    ctx: ApplicationContext,
) -> Result<Arc<dyn LLMProvider>> {
    if let Some((llm, _)) = test_provider_override() {
        return Ok(llm);
    }

    let (provider_name, model) = heal_mock_llm_selection(provider_name, model);
    crate::provider_visibility::ensure_non_mock_provider(&provider_name, "LLM")
        .map_err(LlmError::ConfigError)?;
    check_api_key(&provider_name)?;

    let effective_model = if is_model_provider_mismatch(&provider_name, &model) {
        let corrected = default_model_for_provider(&provider_name);
        tracing::warn!(
            provider = %provider_name,
            requested_model = %model,
            corrected_model = corrected,
            "COMPAT-GUARD: LLM model/provider mismatch — auto-correcting to provider default."
        );
        corrected.to_string()
    } else {
        model
    };

    let inner = create_inner_llm_provider(&provider_name, &effective_model, Some(ctx))?;
    let config = SafetyLimitsConfig::from_env();

    Ok(Arc::new(SafetyLimitedProviderWrapper::new(inner, config)))
}

/// Create a safety-limited LLM provider with optional caller-supplied HTTP headers.
///
/// Headers are forwarded to the upstream LLM API call so that B2B / multi-tenant
/// metadata (`x-request-id`, `x-tenant-id`, `x-correlation-id`, `traceparent`,
/// HMAC tokens) flows through from the incoming API request to the LLM provider.
///
/// If `extra_headers` is `None` or empty this is identical to
/// [`create_safe_llm_provider`].
pub fn create_safe_llm_provider_with_headers(
    provider_name: &str,
    model: &str,
    extra_headers: Option<std::collections::HashMap<String, String>>,
) -> Result<Arc<dyn LLMProvider>> {
    if let Some((llm, _)) = test_provider_override() {
        return Ok(llm);
    }

    let (provider_name, model) = heal_mock_llm_selection(provider_name, model);
    crate::provider_visibility::ensure_non_mock_provider(&provider_name, "LLM")
        .map_err(LlmError::ConfigError)?;
    check_api_key(&provider_name)?;

    let effective_model = if is_model_provider_mismatch(&provider_name, &model) {
        let corrected = default_model_for_provider(&provider_name);
        tracing::warn!(
            provider = %provider_name,
            requested_model = %model,
            corrected_model = corrected,
            "COMPAT-GUARD: LLM model/provider mismatch — auto-correcting to provider default."
        );
        corrected.to_string()
    } else {
        model
    };

    let headers = extra_headers.unwrap_or_default();
    let header_count = headers.len();

    let mut ctx = crate::attribution::baseline_application_context();
    ctx.extra_headers.extend(headers);

    let inner = create_inner_llm_provider(&provider_name, &effective_model, Some(ctx))?;

    let config = SafetyLimitsConfig::from_env();

    tracing::info!(
        provider = %provider_name,
        model = %effective_model,
        max_tokens = config.max_tokens,
        timeout_secs = config.timeout.as_secs(),
        extra_header_count = header_count,
        "Creating safety-limited LLM provider with extra headers"
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
    if let Some((_, embedding)) = test_provider_override() {
        return Ok(embedding);
    }

    let (provider_name, model, dimension) =
        heal_mock_embedding_selection(provider_name, model, dimension);
    crate::provider_visibility::ensure_non_mock_provider(&provider_name, "embedding")
        .map_err(LlmError::ConfigError)?;

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
                        .with_embedding_model(&model),
                )
            } else {
                Arc::new(edgequake_llm::OpenAIProvider::new(api_key).with_embedding_model(&model))
            };
            provider
        } else {
            ProviderFactory::create_embedding_provider(&provider_name, &model, dimension)?
        }
    } else {
        ProviderFactory::create_embedding_provider(&provider_name, &model, dimension)?
    };
    let config = SafetyLimitsConfig::from_env();

    tracing::info!(
        provider = %provider_name,
        model = %model,
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

/// Default HTTP safety timeout for entity-extraction LLM calls on local providers.
///
/// Must exceed [`edgequake_pipeline::LOCAL_CHUNK_TIMEOUT_SECS`] (600) so the
/// pipeline chunk timeout remains the controlling deadline.
pub const LOCAL_EXTRACTION_HTTP_TIMEOUT_SECS: u64 = 900;

/// Per-chunk entity-extraction timeout for a provider (env override wins).
pub fn chunk_extraction_timeout_secs(provider_name: &str) -> u64 {
    edgequake_pipeline::PipelineConfig::from_env_for_provider(provider_name)
        .chunk_extraction_timeout_secs
}

/// Max concurrent entity extractions for a provider (env override wins).
pub fn max_concurrent_extractions_for_provider(provider_name: &str) -> usize {
    edgequake_pipeline::PipelineConfig::from_env_for_provider(provider_name)
        .max_concurrent_extractions
}

/// HTTP safety-wrapper timeout for entity extraction LLM calls.
///
/// - Explicit `EDGEQUAKE_LLM_TIMEOUT_SECS` always wins
/// - Local (Ollama/LM Studio): 900s
/// - Cloud: [`DEFAULT_TIMEOUT_SECS`] (600s)
pub fn extraction_http_timeout_secs(provider_name: &str) -> u64 {
    if let Ok(val) = std::env::var("EDGEQUAKE_LLM_TIMEOUT_SECS") {
        if let Ok(n) = val.parse::<u64>() {
            return n.clamp(MINIMUM_TIMEOUT_SECS, MAXIMUM_TIMEOUT_SECS);
        }
    }
    if is_slow_local_provider(provider_name) {
        LOCAL_EXTRACTION_HTTP_TIMEOUT_SECS.clamp(MINIMUM_TIMEOUT_SECS, MAXIMUM_TIMEOUT_SECS)
    } else {
        DEFAULT_TIMEOUT_SECS.clamp(MINIMUM_TIMEOUT_SECS, MAXIMUM_TIMEOUT_SECS)
    }
}

impl SafetyLimitsConfig {
    /// Safety config for entity-extraction LLM calls (provider-aware HTTP timeout).
    pub fn from_env_for_extraction(provider_name: &str) -> Self {
        let mut config = Self::from_env();
        // from_env already applied EDGEQUAKE_LLM_TIMEOUT_SECS when set; only
        // override the default cloud 600s with local 900s when env is absent.
        if std::env::var("EDGEQUAKE_LLM_TIMEOUT_SECS").is_err() {
            config.timeout = Duration::from_secs(extraction_http_timeout_secs(provider_name));
        }
        config
    }
}

/// Returns the recommended default seconds-per-page for the given provider.
///
/// Reads `EDGEQUAKE_PDF_SECS_PER_PAGE` first; falls back to:
/// - Local providers: 30 s / page (conservative for a mid-range GPU)
/// - Cloud providers: 15 s / page (VLM round-trip + postprocess; raised from
///   8 s after figure-heavy arXiv PDFs exhausted the 520 s under-budget)
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
        15
    }
}

/// Extra seconds/page for figure extraction + Pass-B VLM analyze + page PNG render.
///
/// These phases scale with figure density, not just OCR page count. We budget a
/// deterministic per-page overhead (env `EDGEQUAKE_PDF_FIGURE_SECS_PER_PAGE`) so
/// figure-heavy docs are not killed by the OCR-only formula. The stall watchdog
/// is the primary reliability guarantee; this is a backstop absolute budget.
pub fn figure_secs_per_page_for_provider(provider_name: &str) -> u64 {
    if let Ok(val) = std::env::var("EDGEQUAKE_PDF_FIGURE_SECS_PER_PAGE") {
        if let Ok(n) = val.parse::<u64>() {
            return n; // 0 allowed to disable the overhead term
        }
    }
    if is_local_provider(provider_name) {
        20
    } else {
        10
    }
}

/// Compute the outer vision-conversion timeout for the entire PDF.
///
/// Formula:
/// `120 + pages×secs_per_page + pages×figure_secs_per_page`
/// clamped to `VISION_MAX_OUTER_TIMEOUT_SECS`.
///
/// First principles (no flaky adaptivity):
/// - Budget scales with **document size + provider class + figure overhead**.
/// - Never treat unknown page_count as 0 (that collapsed budgets to 120s).
/// - Load/queue pressure must not shrink the budget mid-run.
/// - The stall watchdog (progress-resetting) is the primary hang detector;
///   this absolute budget is a backstop only.
pub fn vision_outer_timeout_secs(provider_name: &str, page_count: usize) -> u64 {
    let pages = effective_page_count_for_vision_budget(page_count);
    let per_page = secs_per_page_for_provider(provider_name);
    let figure_per_page = figure_secs_per_page_for_provider(provider_name);
    let page_budget = per_page.saturating_add(figure_per_page);
    let computed = 120_u64.saturating_add(page_budget.saturating_mul(pages as u64));
    computed.min(VISION_MAX_OUTER_TIMEOUT_SECS)
}

/// When page_count is missing/zero after PDF heal, assume a mid-size doc so
/// explicit Vision does not die at the 120s floor (deterministic, not adaptive).
pub const UNKNOWN_PAGE_COUNT_VISION_BUDGET_ASSUMPTION: usize = 50;

/// Normalize page_count for vision outer-timeout math.
pub fn effective_page_count_for_vision_budget(page_count: usize) -> usize {
    if page_count == 0 {
        UNKNOWN_PAGE_COUNT_VISION_BUDGET_ASSUMPTION
    } else {
        page_count
    }
}

/// Default HTTP timeout for synchronous markdown/text upload processing (cloud/mock).
pub const DEFAULT_SYNC_PROCESSING_TIMEOUT_SECS: u64 = 120;

/// HTTP timeout for synchronous upload when workspace uses Ollama or LM Studio.
///
/// WHY: Local LLM extraction + embedding commonly exceeds 120 s (especially
/// Docker → host Ollama). SPEC-020 Ollama proofs hit 408 at 120 s on v0.12.9.
pub const LOCAL_SYNC_PROCESSING_TIMEOUT_SECS: u64 = 600;

/// Returns `true` for local inference servers that need longer sync upload windows.
pub fn is_slow_local_provider(provider_name: &str) -> bool {
    matches!(
        provider_name.to_ascii_lowercase().as_str(),
        "ollama" | "lmstudio" | "lm-studio" | "lm_studio"
    )
}

/// HTTP-level timeout for synchronous document upload (`async_processing: false`).
///
/// Reads `EDGEQUAKE_SYNC_PROCESSING_TIMEOUT_SECS` first (global override).
/// Falls back to [`LOCAL_SYNC_PROCESSING_TIMEOUT_SECS`] for Ollama/LM Studio,
/// [`DEFAULT_SYNC_PROCESSING_TIMEOUT_SECS`] for cloud and mock providers.
pub fn sync_processing_timeout_secs(provider_name: &str) -> u64 {
    if let Ok(val) = std::env::var("EDGEQUAKE_SYNC_PROCESSING_TIMEOUT_SECS") {
        if let Ok(n) = val.parse::<u64>() {
            return n.max(30);
        }
    }
    if is_slow_local_provider(provider_name) {
        LOCAL_SYNC_PROCESSING_TIMEOUT_SECS
    } else {
        DEFAULT_SYNC_PROCESSING_TIMEOUT_SECS
    }
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

/// Default per-VLM-call timeout for local Pass B figure analyze (classify-only).
pub const LOCAL_PASS_B_VISION_TIMEOUT_SECS: u64 = 90;

/// Per-call vision timeout for multimodal Pass B (figure analyze).
///
/// Distinct from page OCR ([`vision_page_timeout_secs`]): local Pass B uses a
/// shorter default (90s) so one hung Ollama encode cannot burn 10 minutes.
/// Override with `EDGEQUAKE_MM_PASS_B_PAGE_TIMEOUT_SECS`.
pub fn vision_pass_b_timeout_secs(provider_name: &str) -> u64 {
    if let Ok(val) = std::env::var("EDGEQUAKE_MM_PASS_B_PAGE_TIMEOUT_SECS") {
        if let Ok(n) = val.parse::<u64>() {
            return n.max(10);
        }
    }
    // Keep this match local to avoid crate::services ↔ safety_limits cycles.
    let is_local_vlm = matches!(
        provider_name.trim().to_ascii_lowercase().as_str(),
        "ollama" | "lmstudio" | "lm-studio" | "lm_studio"
    );
    if is_local_vlm {
        LOCAL_PASS_B_VISION_TIMEOUT_SECS
    } else {
        vision_page_timeout_secs(provider_name)
    }
}

/// Validate vision provider credentials before pdf2md factory resolution (SPEC-043).
pub fn check_vision_provider_available(provider_name: &str, model: &str) -> Result<()> {
    check_api_key(provider_name)?;
    if is_model_provider_mismatch(provider_name, model) {
        tracing::warn!(
            provider = provider_name,
            requested_model = model,
            corrected_model = default_model_for_provider(provider_name),
            "COMPAT-GUARD: Vision model/provider mismatch — pdf2md will use provider default."
        );
    }
    Ok(())
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
    if let Some((llm, _)) = test_provider_override() {
        return Ok(llm);
    }

    let (provider_name, model) = heal_mock_llm_selection(provider_name, model);
    crate::provider_visibility::ensure_non_mock_provider(&provider_name, "vision LLM")
        .map_err(LlmError::ConfigError)?;
    check_api_key(&provider_name)?;

    // WHY: Guard against stale task data where a model was stored at upload time
    // under one provider (e.g., OpenAI) and is later retried under a different
    // provider (e.g., Ollama). Without this check, Ollama receives "gpt-4.1-nano"
    // and returns 404 Not Found, failing all pages and exhausting all retries.
    //
    // When a clear mismatch is detected (OpenAI model name with non-OpenAI provider),
    // we auto-correct to the provider's default model and log a warning so operators
    // can update stale workspace / task configurations.
    let effective_model = if is_model_provider_mismatch(&provider_name, &model) {
        let corrected = default_model_for_provider(&provider_name);
        tracing::warn!(
            provider = %provider_name,
            requested_model = %model,
            corrected_model = corrected,
            "COMPAT-GUARD: Model/provider mismatch detected — auto-correcting to provider default. \
             This indicates stale task data or misconfigured workspace settings. \
             Update workspace vision_llm_model to a {}-compatible model to suppress this warning.",
            provider_name
        );
        corrected.to_string()
    } else {
        model
    };

    let inner = create_inner_llm_provider(&provider_name, &effective_model, None)?;

    let timeout_secs = vision_page_timeout_secs(&provider_name);
    let config = SafetyLimitsConfig {
        max_tokens: DEFAULT_MAX_TOKENS,
        timeout: Duration::from_secs(timeout_secs),
        log_enforcement: true,
        max_embed_batch_size: SafetyLimitsConfig::env_embed_batch_size(),
    };

    tracing::info!(
        provider = %provider_name,
        model = %effective_model,
        timeout_secs = timeout_secs,
        is_local = is_local_provider(&provider_name),
        "Creating safety-limited VISION LLM provider (provider-aware timeout)"
    );

    Ok(Arc::new(SafetyLimitedProviderWrapper::new(inner, config)))
}

/// Vision provider for multimodal Pass B (figure analyze) with shorter local timeout.
///
/// Pass A page OCR continues to use [`create_safe_vision_provider`] (600s local).
pub fn create_safe_vision_provider_for_pass_b(
    provider_name: &str,
    model: &str,
) -> Result<Arc<dyn LLMProvider>> {
    if let Some((llm, _)) = test_provider_override() {
        return Ok(llm);
    }

    let (provider_name, model) = heal_mock_llm_selection(provider_name, model);
    crate::provider_visibility::ensure_non_mock_provider(&provider_name, "vision LLM")
        .map_err(LlmError::ConfigError)?;
    check_api_key(&provider_name)?;

    let effective_model = if is_model_provider_mismatch(&provider_name, &model) {
        let corrected = default_model_for_provider(&provider_name);
        tracing::warn!(
            provider = %provider_name,
            requested_model = %model,
            corrected_model = corrected,
            "COMPAT-GUARD: Model/provider mismatch detected — auto-correcting to provider default \
             (Pass B vision)."
        );
        corrected.to_string()
    } else {
        model
    };

    let inner = create_inner_llm_provider(&provider_name, &effective_model, None)?;

    let timeout_secs = vision_pass_b_timeout_secs(&provider_name);
    let config = SafetyLimitsConfig {
        max_tokens: DEFAULT_MAX_TOKENS,
        timeout: Duration::from_secs(timeout_secs),
        log_enforcement: true,
        max_embed_batch_size: SafetyLimitsConfig::env_embed_batch_size(),
    };

    tracing::info!(
        provider = %provider_name,
        model = %effective_model,
        timeout_secs = timeout_secs,
        is_local = is_local_provider(&provider_name),
        "Creating safety-limited VISION LLM provider for Pass B (figure analyze)"
    );

    Ok(Arc::new(SafetyLimitedProviderWrapper::new(inner, config)))
}

// ─────────────────────────────────────────────────────────────────────────────

const GATEWAY_MODEL_IDS_ENV: &str = "EDGEQUAKE_ALLOW_GATEWAY_MODEL_IDS";

/// True when slash-separated gateway model IDs (e.g. `deepinfra/minimax-m2.5`) should
/// pass through COMPAT-GUARD without rewrite.
fn gateway_slash_models_allowed() -> bool {
    if std::env::var(GATEWAY_MODEL_IDS_ENV)
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        return true;
    }

    if std::env::var("EDGEQUAKE_CHAT_BASE_URL")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
    {
        return true;
    }

    if let Ok(base_url) = std::env::var("OPENAI_BASE_URL") {
        let trimmed = base_url.trim();
        if !trimmed.is_empty() && !trimmed.contains("api.openai.com") {
            return true;
        }
    }

    false
}

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
            // WHY: slash in model is also valid for gateway routing keys — allow when
            // a custom OpenAI-compatible base URL or explicit gateway flag is set.
            let slash_mismatch = model.contains('/') && !gateway_slash_models_allowed();
            is_local_style_model || slash_mismatch
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

#[cfg(test)]
mod sync_timeout_tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn sync_timeout_ollama_uses_extended_default() {
        std::env::remove_var("EDGEQUAKE_SYNC_PROCESSING_TIMEOUT_SECS");
        assert_eq!(
            sync_processing_timeout_secs("ollama"),
            LOCAL_SYNC_PROCESSING_TIMEOUT_SECS
        );
        assert_eq!(
            sync_processing_timeout_secs("LMStudio"),
            LOCAL_SYNC_PROCESSING_TIMEOUT_SECS
        );
    }

    #[test]
    #[serial]
    fn sync_timeout_mock_uses_cloud_default() {
        std::env::remove_var("EDGEQUAKE_SYNC_PROCESSING_TIMEOUT_SECS");
        assert_eq!(
            sync_processing_timeout_secs("mock"),
            DEFAULT_SYNC_PROCESSING_TIMEOUT_SECS
        );
        assert_eq!(
            sync_processing_timeout_secs("openai"),
            DEFAULT_SYNC_PROCESSING_TIMEOUT_SECS
        );
    }

    #[test]
    #[serial]
    fn sync_timeout_env_override_wins() {
        std::env::set_var("EDGEQUAKE_SYNC_PROCESSING_TIMEOUT_SECS", "900");
        assert_eq!(sync_processing_timeout_secs("ollama"), 900);
        std::env::remove_var("EDGEQUAKE_SYNC_PROCESSING_TIMEOUT_SECS");
    }
}

#[cfg(test)]
mod vision_outer_timeout_tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn unknown_page_count_uses_deterministic_assumption_not_zero() {
        std::env::remove_var("EDGEQUAKE_PDF_SECS_PER_PAGE");
        std::env::remove_var("EDGEQUAKE_PDF_FIGURE_SECS_PER_PAGE");
        let zero_budget = vision_outer_timeout_secs("mistral", 0);
        let assumed =
            vision_outer_timeout_secs("mistral", UNKNOWN_PAGE_COUNT_VISION_BUDGET_ASSUMPTION);
        assert_eq!(zero_budget, assumed);
        // mistral cloud: 120 + 50*(15+10) = 1370 — never the broken 120s floor
        assert_eq!(zero_budget, 120 + (15 + 10) * 50);
        assert!(zero_budget > 120);
    }

    #[test]
    #[serial]
    fn known_page_count_scales_linearly_cloud() {
        std::env::remove_var("EDGEQUAKE_PDF_SECS_PER_PAGE");
        std::env::remove_var("EDGEQUAKE_PDF_FIGURE_SECS_PER_PAGE");
        assert_eq!(
            vision_outer_timeout_secs("mistral", 10),
            120 + (15 + 10) * 10
        );
        assert_eq!(
            vision_outer_timeout_secs("openai", 100),
            120 + (15 + 10) * 100
        );
    }

    #[test]
    #[serial]
    fn known_page_count_scales_local_provider() {
        std::env::remove_var("EDGEQUAKE_PDF_SECS_PER_PAGE");
        std::env::remove_var("EDGEQUAKE_PDF_FIGURE_SECS_PER_PAGE");
        assert_eq!(
            vision_outer_timeout_secs("ollama", 10),
            120 + (30 + 20) * 10
        );
    }

    #[test]
    #[serial]
    fn figure_overhead_can_be_disabled_via_env() {
        std::env::remove_var("EDGEQUAKE_PDF_SECS_PER_PAGE");
        std::env::set_var("EDGEQUAKE_PDF_FIGURE_SECS_PER_PAGE", "0");
        assert_eq!(vision_outer_timeout_secs("mistral", 10), 120 + 15 * 10);
        std::env::remove_var("EDGEQUAKE_PDF_FIGURE_SECS_PER_PAGE");
    }

    #[test]
    fn effective_page_count_zero_maps_to_assumption() {
        assert_eq!(
            effective_page_count_for_vision_budget(0),
            UNKNOWN_PAGE_COUNT_VISION_BUDGET_ASSUMPTION
        );
        assert_eq!(effective_page_count_for_vision_budget(42), 42);
    }
}

#[cfg(test)]
mod issue255_gateway_model_tests {
    use super::*;
    use serial_test::serial;

    fn clear_gateway_env() {
        std::env::remove_var(GATEWAY_MODEL_IDS_ENV);
        std::env::remove_var("EDGEQUAKE_CHAT_BASE_URL");
        std::env::remove_var("OPENAI_BASE_URL");
    }

    #[test]
    #[serial]
    fn issue255_gateway_slash_model_not_rewritten() {
        clear_gateway_env();
        std::env::set_var(GATEWAY_MODEL_IDS_ENV, "1");
        assert!(!is_model_provider_mismatch(
            "openai",
            "deepinfra/minimax-m2.5"
        ));
        clear_gateway_env();
    }

    #[test]
    #[serial]
    fn issue255_local_model_on_openai_still_guarded() {
        clear_gateway_env();
        assert!(is_model_provider_mismatch("openai", "gemma3:latest"));
        clear_gateway_env();
    }
}
