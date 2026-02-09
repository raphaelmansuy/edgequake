//! LLM provider factory for environment-based selection.
//!
//! @implements SPEC-032: Ollama/LM Studio provider support
//! @implements FEAT0024: Multi-provider LLM factory implementation
//! @implements SPEC-033: Hybrid provider mode (separate LLM and embedding providers)
//!
//! # WHY This Factory Exists (Two Separate Provider Paths)
//!
//! EdgeQuake uses LLM providers in TWO independent codepaths:
//!
//! ```text
//!  ┌─────────────────────────────────────────────────────────────────┐
//!  │                  Provider Selection Points                      │
//!  ├─────────────────────────────────────────────────────────────────┤
//!  │                                                                 │
//!  │  1. SERVER DEFAULT PIPELINE (set once at startup)               │
//!  │     ┌──────────┐    from_env()     ┌───────────────────┐       │
//!  │     │  Server   │ ──────────────►  │  Default Pipeline │       │
//!  │     │  Startup  │  auto-detect     │  (doc extraction) │       │
//!  │     └──────────┘                   └───────────────────┘       │
//!  │     Used for: background document ingestion when workspace     │
//!  │     providers cannot be created (API key missing, etc.)        │
//!  │                                                                 │
//!  │  2. PER-REQUEST PROVIDER (resolved per chat query)             │
//!  │     ┌──────────┐    resolver       ┌───────────────────┐       │
//!  │     │  Chat     │ ──────────────►  │  Query LLM        │       │
//!  │     │  Request  │  user selection  │  (answer gen)      │       │
//!  │     └──────────┘                   └───────────────────┘       │
//!  │     Priority: request param > workspace config > server default│
//!  │                                                                 │
//!  │  3. WORKSPACE PIPELINE (resolved per document task)            │
//!  │     ┌──────────┐    create_safe*   ┌───────────────────┐       │
//!  │     │  Worker   │ ──────────────►  │  Workspace Pipeline│      │
//!  │     │  Task     │  ws config       │  (doc extraction)  │      │
//!  │     └──────────┘                   └───────────────────┘       │
//!  │     Uses workspace.llm_provider + workspace.llm_model          │
//!  │     Falls back to (1) if creation fails                        │
//!  └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Environment Variables
//!
//! ## Provider Selection
//!
//! - `EDGEQUAKE_LLM_PROVIDER`: Override LLM provider selection (openai|ollama|lmstudio|mock)
//! - `EDGEQUAKE_EMBEDDING_PROVIDER`: Override embedding provider (enables hybrid mode)
//!
//! ## Hybrid Mode (SPEC-033)
//!
//! When `EDGEQUAKE_EMBEDDING_PROVIDER` is set, you can use different providers for LLM
//! and embeddings. This is useful when:
//! - Your OpenAI account has LLM quota but not embedding quota
//! - You want cost savings (free local embeddings with cloud LLM)
//! - You prefer local embedding privacy with cloud LLM quality
//!
//! Example hybrid configuration:
//! ```bash
//! export EDGEQUAKE_LLM_PROVIDER=openai
//! export EDGEQUAKE_EMBEDDING_PROVIDER=ollama
//! export OPENAI_API_KEY=sk-...
//! export OLLAMA_HOST=http://localhost:11434
//! ```
//!
//! ## Provider-Specific Configuration
//!
//! See individual provider documentation for configuration variables:
//! - OpenAI: `OPENAI_API_KEY`, `OPENAI_BASE_URL`
//! - Ollama: `OLLAMA_HOST`, `OLLAMA_MODEL`, `OLLAMA_EMBEDDING_MODEL`
//! - LM Studio: `LMSTUDIO_HOST`, `LMSTUDIO_MODEL`, `LMSTUDIO_EMBEDDING_MODEL`
//!
//! # Auto-Detection Priority
//!
//! WHY Ollama wins over OpenAI: EdgeQuake is designed for local-first development.
//! The `make dev` workflow starts Ollama by default. If you want OpenAI as the
//! server-level default, set `EDGEQUAKE_LLM_PROVIDER=openai` explicitly.
//!
//! ```text
//!  EDGEQUAKE_LLM_PROVIDER set?
//!       │
//!       ├── YES ──► Use that provider (explicit override)
//!       │
//!       └── NO  ──► Auto-detect:
//!                    ├── OLLAMA_HOST or OLLAMA_MODEL? ──► Ollama
//!                    ├── LMSTUDIO_HOST or LMSTUDIO_MODEL? ──► LM Studio
//!                    ├── OPENAI_API_KEY (non-empty)? ──► OpenAI
//!                    └── Nothing? ──► Mock
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use edgequake_llm::ProviderFactory;
//!
//! // Auto-detect from environment
//! let (llm, embedding) = ProviderFactory::from_env()?;
//!
//! // Explicit provider selection
//! std::env::set_var("EDGEQUAKE_LLM_PROVIDER", "ollama");
//! let (llm, embedding) = ProviderFactory::from_env()?;
//!
//! // Hybrid mode: OpenAI for LLM, Ollama for embeddings
//! std::env::set_var("EDGEQUAKE_LLM_PROVIDER", "openai");
//! std::env::set_var("EDGEQUAKE_EMBEDDING_PROVIDER", "ollama");
//! let (llm, embedding) = ProviderFactory::from_env()?;
//! assert_eq!(llm.name(), "openai");
//! assert_eq!(embedding.name(), "ollama");
//! ```

use std::sync::Arc;

use crate::error::{LlmError, Result};
use crate::providers::lmstudio::LMStudioProvider;
use crate::traits::{EmbeddingProvider, LLMProvider};
use crate::{MockProvider, OllamaProvider, OpenAIProvider};

/// Supported provider types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderType {
    /// OpenAI provider (cloud API)
    OpenAI,
    /// Ollama provider (local models)
    Ollama,
    /// LM Studio provider (OpenAI-compatible local API)
    LMStudio,
    /// Mock provider (testing only)
    Mock,
}

impl ProviderType {
    /// Parse provider type from string (case-insensitive)
    ///
    /// # Examples
    ///
    /// ```
    /// use edgequake_llm::ProviderType;
    ///
    /// assert_eq!(ProviderType::parse("openai"), Some(ProviderType::OpenAI));
    /// assert_eq!(ProviderType::parse("OLLAMA"), Some(ProviderType::Ollama));
    /// assert_eq!(ProviderType::parse("lm-studio"), Some(ProviderType::LMStudio));
    /// ```
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Some(Self::OpenAI),
            "ollama" => Some(Self::Ollama),
            "lmstudio" | "lm-studio" | "lm_studio" => Some(Self::LMStudio),
            "mock" => Some(Self::Mock),
            _ => None,
        }
    }
}

/// Provider factory for creating LLM and embedding providers.
///
/// Provides environment-based auto-detection and explicit provider selection.
pub struct ProviderFactory;

impl ProviderFactory {
    /// Auto-detect and create providers from environment.
    ///
    /// # Priority
    ///
    /// 1. `EDGEQUAKE_LLM_PROVIDER` environment variable (explicit selection)
    /// 2. Auto-detect: OLLAMA_HOST → LMSTUDIO_HOST → OPENAI_API_KEY → Mock
    ///
    /// # Returns
    ///
    /// Returns a tuple of (LLMProvider, EmbeddingProvider). In most cases,
    /// the same provider implementation is used for both.
    ///
    /// # Errors
    ///
    /// Returns error if required configuration for selected provider is missing.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// std::env::set_var("OLLAMA_HOST", "http://localhost:11434");
    /// let (llm, embedding) = ProviderFactory::from_env()?;
    /// assert_eq!(llm.name(), "ollama");
    /// ```
    pub fn from_env() -> Result<(Arc<dyn LLMProvider>, Arc<dyn EmbeddingProvider>)> {
        // SPEC-033: Check for hybrid mode (separate embedding provider)
        let embedding_provider_override = std::env::var("EDGEQUAKE_EMBEDDING_PROVIDER").ok();

        // Determine LLM provider type
        let llm_provider_type = if let Ok(provider_str) = std::env::var("EDGEQUAKE_LLM_PROVIDER") {
            ProviderType::parse(&provider_str).ok_or_else(|| {
                LlmError::ConfigError(format!(
                    "Unknown LLM provider type: {}. Valid options: openai, ollama, lmstudio, mock",
                    provider_str
                ))
            })?
        } else {
            // Auto-detect based on environment.
            // WHY Ollama first: EdgeQuake is local-first. `make dev` always starts Ollama.
            // This means the SERVER DEFAULT is Ollama. Per-query and per-workspace providers
            // are resolved separately (see WorkspaceProviderResolver and processor.rs).
            //
            //   Priority: Ollama → LM Studio → OpenAI → Mock
            //
            // If you see unexpected Ollama usage in logs while expecting OpenAI, either:
            //   (a) Set EDGEQUAKE_LLM_PROVIDER=openai to override auto-detection, or
            //   (b) The workspace pipeline fell back to server default (check CRITICAL logs)
            let has_ollama =
                std::env::var("OLLAMA_HOST").is_ok() || std::env::var("OLLAMA_MODEL").is_ok();
            let has_openai = std::env::var("OPENAI_API_KEY")
                .map(|k| !k.is_empty() && k != "test-key")
                .unwrap_or(false);

            if has_ollama {
                // WHY warn: Users often have both Ollama running AND OPENAI_API_KEY set.
                // When auto-detect picks Ollama, background pipeline tasks use Ollama
                // even though the user expects OpenAI. This warning makes it visible.
                if has_openai {
                    tracing::warn!(
                        "Auto-detect: Ollama selected as SERVER DEFAULT (OLLAMA_HOST/OLLAMA_MODEL found). \
                         OPENAI_API_KEY is also set but unused for server default. \
                         Workspace-specific pipelines and per-query overrides still use OpenAI when configured. \
                         To force OpenAI as server default, set EDGEQUAKE_LLM_PROVIDER=openai"
                    );
                }
                ProviderType::Ollama
            } else if std::env::var("LMSTUDIO_HOST").is_ok()
                || std::env::var("LMSTUDIO_MODEL").is_ok()
            {
                ProviderType::LMStudio
            } else if has_openai {
                ProviderType::OpenAI
            } else {
                ProviderType::Mock
            }
        };

        // SPEC-033: If embedding provider override specified, create hybrid configuration
        if let Some(embedding_str) = embedding_provider_override {
            let embedding_type = ProviderType::parse(&embedding_str).ok_or_else(|| {
                LlmError::ConfigError(format!(
                    "Unknown embedding provider type: {}. Valid options: openai, ollama, lmstudio, mock",
                    embedding_str
                ))
            })?;

            tracing::info!(
                llm_provider = ?llm_provider_type,
                embedding_provider = ?embedding_type,
                "Creating hybrid provider configuration (SPEC-033)"
            );

            return Self::create_hybrid(llm_provider_type, embedding_type);
        }

        // Standard mode: use same provider for both
        Self::create(llm_provider_type)
    }

    /// Create hybrid provider configuration (SPEC-033).
    ///
    /// Uses different providers for LLM and embedding operations.
    /// Useful when:
    /// - OpenAI has LLM quota but not embedding quota
    /// - Cost optimization (free local embeddings)
    /// - Privacy (local embeddings, cloud LLM)
    fn create_hybrid(
        llm_type: ProviderType,
        embedding_type: ProviderType,
    ) -> Result<(Arc<dyn LLMProvider>, Arc<dyn EmbeddingProvider>)> {
        // Create LLM provider
        let llm_provider: Arc<dyn LLMProvider> = match llm_type {
            ProviderType::OpenAI => {
                let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| {
                    LlmError::ConfigError(
                        "OPENAI_API_KEY not set for OpenAI LLM provider".to_string(),
                    )
                })?;
                Arc::new(OpenAIProvider::new(api_key))
            }
            ProviderType::Ollama => Arc::new(OllamaProvider::from_env()?),
            ProviderType::LMStudio => Arc::new(LMStudioProvider::from_env()?),
            ProviderType::Mock => Arc::new(MockProvider::new()),
        };

        // Create embedding provider (separate from LLM)
        let embedding_provider: Arc<dyn EmbeddingProvider> = match embedding_type {
            ProviderType::OpenAI => {
                let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| {
                    LlmError::ConfigError(
                        "OPENAI_API_KEY not set for OpenAI embedding provider".to_string(),
                    )
                })?;
                Arc::new(OpenAIProvider::new(api_key))
            }
            ProviderType::Ollama => Arc::new(OllamaProvider::from_env()?),
            ProviderType::LMStudio => Arc::new(LMStudioProvider::from_env()?),
            ProviderType::Mock => Arc::new(MockProvider::new()),
        };

        tracing::info!(
            llm_name = llm_provider.name(),
            embedding_name = embedding_provider.name(),
            embedding_dimension = embedding_provider.dimension(),
            "Hybrid providers initialized"
        );

        Ok((llm_provider, embedding_provider))
    }

    /// Create specific provider type.
    ///
    /// # Arguments
    ///
    /// * `provider_type` - The type of provider to create
    ///
    /// # Returns
    ///
    /// Returns a tuple of (LLMProvider, EmbeddingProvider).
    ///
    /// # Errors
    ///
    /// Returns error if required configuration for the provider is missing.
    pub fn create(
        provider_type: ProviderType,
    ) -> Result<(Arc<dyn LLMProvider>, Arc<dyn EmbeddingProvider>)> {
        match provider_type {
            ProviderType::OpenAI => Self::create_openai(),
            ProviderType::Ollama => Self::create_ollama(),
            ProviderType::LMStudio => Self::create_lmstudio(),
            ProviderType::Mock => Ok(Self::create_mock()),
        }
    }

    /// Create OpenAI provider from environment.
    ///
    /// Reads `OPENAI_API_KEY` environment variable.
    fn create_openai() -> Result<(Arc<dyn LLMProvider>, Arc<dyn EmbeddingProvider>)> {
        let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| {
            LlmError::ConfigError("OPENAI_API_KEY not set for OpenAI provider".to_string())
        })?;

        if api_key.is_empty() || api_key == "test-key" {
            return Err(LlmError::ConfigError(
                "OPENAI_API_KEY is empty or invalid".to_string(),
            ));
        }

        let provider = Arc::new(OpenAIProvider::new(api_key));
        Ok((provider.clone(), provider))
    }

    /// Create Ollama provider from environment.
    ///
    /// Uses OllamaProvider::from_env() which reads:
    /// - OLLAMA_HOST
    /// - OLLAMA_MODEL
    /// - OLLAMA_EMBEDDING_MODEL
    fn create_ollama() -> Result<(Arc<dyn LLMProvider>, Arc<dyn EmbeddingProvider>)> {
        let provider = Arc::new(OllamaProvider::from_env()?);
        Ok((provider.clone(), provider))
    }

    /// Create LM Studio provider from environment.
    ///
    /// Uses the dedicated LMStudioProvider which reads:
    /// - `LMSTUDIO_HOST`: LM Studio server URL (default: http://localhost:1234)
    /// - `LMSTUDIO_MODEL`: Chat model name (default: gemma2-9b-it)
    /// - `LMSTUDIO_EMBEDDING_MODEL`: Embedding model (default: nomic-embed-text-v1.5)
    /// - `LMSTUDIO_EMBEDDING_DIM`: Embedding dimension (default: 768)
    fn create_lmstudio() -> Result<(Arc<dyn LLMProvider>, Arc<dyn EmbeddingProvider>)> {
        let provider = Arc::new(LMStudioProvider::from_env()?);
        Ok((provider.clone(), provider))
    }

    /// Create mock provider for testing.
    ///
    /// Always returns deterministic responses.
    fn create_mock() -> (Arc<dyn LLMProvider>, Arc<dyn EmbeddingProvider>) {
        let provider = Arc::new(MockProvider::new());
        (provider.clone(), provider)
    }

    /// Get embedding dimension for current provider configuration.
    ///
    /// Useful for configuring vector storage with the correct dimension.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// std::env::set_var("EDGEQUAKE_LLM_PROVIDER", "ollama");
    /// let dim = ProviderFactory::embedding_dimension()?;
    /// assert_eq!(dim, 768); // embeddinggemma dimension
    /// ```
    pub fn embedding_dimension() -> Result<usize> {
        let (_, embedding_provider) = Self::from_env()?;
        Ok(embedding_provider.dimension())
    }

    /// Create an embedding provider from workspace configuration.
    ///
    /// This is used to create workspace-specific embedding providers for query execution.
    /// The provider is configured with the workspace's embedding model and dimension.
    ///
    /// @implements SPEC-032: Workspace-specific embedding in query process
    ///
    /// # Arguments
    ///
    /// * `provider_name` - Provider type (e.g., "openai", "ollama", "lmstudio", "mock")
    /// * `model` - Embedding model name (e.g., "text-embedding-3-small", "embeddinggemma:latest")
    /// * `dimension` - Embedding dimension (e.g., 1536, 768)
    ///
    /// # Returns
    ///
    /// Returns an `Arc<dyn EmbeddingProvider>` configured for the workspace.
    ///
    /// # Errors
    ///
    /// Returns error if the provider type is unknown or required configuration is missing.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let provider = ProviderFactory::create_embedding_provider(
    ///     "ollama",
    ///     "embeddinggemma:latest",
    ///     768,
    /// )?;
    /// assert_eq!(provider.dimension(), 768);
    /// ```
    pub fn create_embedding_provider(
        provider_name: &str,
        model: &str,
        dimension: usize,
    ) -> Result<Arc<dyn EmbeddingProvider>> {
        let provider_type = ProviderType::parse(provider_name).ok_or_else(|| {
            LlmError::ConfigError(format!(
                "Unknown embedding provider: {}. Valid: openai, ollama, lmstudio, mock",
                provider_name
            ))
        })?;

        match provider_type {
            ProviderType::OpenAI => {
                let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| {
                    LlmError::ConfigError(
                        "OPENAI_API_KEY required for OpenAI embedding provider. Set the environment variable or select a different provider (ollama, lmstudio, mock)".to_string(),
                    )
                })?;

                // Validate API key is not empty
                if api_key.is_empty() || api_key == "test-key" {
                    return Err(LlmError::ConfigError(
                        "OPENAI_API_KEY is empty or invalid. Provide a valid API key from https://platform.openai.com/account/api-keys or select a different provider (ollama, lmstudio, mock)".to_string(),
                    ));
                }

                // OpenAI provider with specific embedding model and dimension
                // @implements SPEC-032/OODA-227: Pass dimension to provider (respect workspace config)
                // WHY: Use workspace-specified dimension instead of auto-detection to avoid dimension mismatches
                let provider = OpenAIProvider::new(api_key)
                    .with_embedding_model_and_dimension(model, dimension);
                Ok(Arc::new(provider))
            }
            ProviderType::Ollama => {
                // Ollama provider with specific embedding model and dimension
                // @implements SPEC-032/OODA-227: Pass dimension to provider
                let host = std::env::var("OLLAMA_HOST")
                    .unwrap_or_else(|_| "http://localhost:11434".to_string());
                let provider = OllamaProvider::builder()
                    .host(&host)
                    .embedding_model(model)
                    .embedding_dimension(dimension)
                    .build()?;
                Ok(Arc::new(provider))
            }
            ProviderType::LMStudio => {
                // LM Studio provider with specific embedding model and dimension
                // @implements SPEC-032/OODA-227: Pass dimension to provider
                let host = std::env::var("LMSTUDIO_HOST")
                    .unwrap_or_else(|_| "http://localhost:1234".to_string());
                let provider = LMStudioProvider::builder()
                    .host(&host)
                    .embedding_model(model)
                    .embedding_dimension(dimension)
                    .build()?;
                Ok(Arc::new(provider))
            }
            ProviderType::Mock => {
                // Mock provider uses default dimension (1536)
                // WHY: Mock is for testing, dimension doesn't affect test behavior
                Ok(Arc::new(MockProvider::new()))
            }
        }
    }

    /// Create an LLM provider from workspace configuration.
    ///
    /// This is used to create workspace-specific LLM providers for ingestion/extraction.
    /// The provider is configured with the workspace's LLM model.
    ///
    /// @implements SPEC-032: Workspace-specific LLM in ingestion process
    ///
    /// # Arguments
    ///
    /// * `provider_name` - Provider type (e.g., "openai", "ollama", "lmstudio", "mock")
    /// * `model` - LLM model name (e.g., "gpt-4o-mini", "gemma3:12b")
    ///
    /// # Returns
    ///
    /// Returns an `Arc<dyn LLMProvider>` configured for the workspace.
    ///
    /// # Errors
    ///
    /// Returns error if the provider type is unknown or required configuration is missing.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let provider = ProviderFactory::create_llm_provider(
    ///     "ollama",
    ///     "gemma3:12b",
    /// )?;
    /// assert_eq!(provider.model(), "gemma3:12b");
    /// ```
    pub fn create_llm_provider(provider_name: &str, model: &str) -> Result<Arc<dyn LLMProvider>> {
        let provider_type = ProviderType::parse(provider_name).ok_or_else(|| {
            LlmError::ConfigError(format!(
                "Unknown LLM provider: {}. Valid: openai, ollama, lmstudio, mock",
                provider_name
            ))
        })?;

        match provider_type {
            ProviderType::OpenAI => {
                let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| {
                    LlmError::ConfigError(
                        "OPENAI_API_KEY required for OpenAI LLM provider. Set the environment variable or select a different provider (ollama, lmstudio, mock)".to_string(),
                    )
                })?;

                // Validate API key is not empty
                if api_key.is_empty() || api_key == "test-key" {
                    return Err(LlmError::ConfigError(
                        "OPENAI_API_KEY is empty or invalid. Provide a valid API key from https://platform.openai.com/account/api-keys or select a different provider (ollama, lmstudio, mock)".to_string(),
                    ));
                }

                // OpenAI provider with specific model
                let provider = OpenAIProvider::new(api_key).with_model(model);
                Ok(Arc::new(provider))
            }
            ProviderType::Ollama => {
                // Ollama provider with specific model
                let host = std::env::var("OLLAMA_HOST")
                    .unwrap_or_else(|_| "http://localhost:11434".to_string());
                let provider = OllamaProvider::builder().host(&host).model(model).build()?;
                Ok(Arc::new(provider))
            }
            ProviderType::LMStudio => {
                // LM Studio provider with specific model
                let host = std::env::var("LMSTUDIO_HOST")
                    .unwrap_or_else(|_| "http://localhost:1234".to_string());
                let provider = LMStudioProvider::builder()
                    .host(&host)
                    .model(model)
                    .build()?;
                Ok(Arc::new(provider))
            }
            ProviderType::Mock => {
                // Mock provider ignores model and uses defaults
                Ok(Arc::new(MockProvider::new()))
            }
        }
    }

    /// Get the default LLM model for a provider.
    ///
    /// This is used when only provider name is specified without model.
    /// Returns the provider's default model name.
    ///
    /// @implements SPEC-032: Default model resolution for provider-only requests
    ///
    /// # Arguments
    ///
    /// * `provider_name` - Provider type (e.g., "openai", "ollama", "lmstudio", "mock")
    ///
    /// # Returns
    ///
    /// Returns the default model name for the provider.
    ///
    /// # Examples
    ///
    /// ```
    /// use edgequake_llm::ProviderFactory;
    ///
    /// assert_eq!(ProviderFactory::default_model_for_provider("ollama"), "gemma3:12b");
    /// assert_eq!(ProviderFactory::default_model_for_provider("openai"), "gpt-4o-mini");
    /// ```
    pub fn default_model_for_provider(provider_name: &str) -> &'static str {
        match provider_name.to_lowercase().as_str() {
            "openai" => "gpt-4o-mini",
            "ollama" => "gemma3:12b",
            "lmstudio" | "lm-studio" | "lm_studio" => "gemma2-9b-it",
            "mock" => "mock-model",
            _ => "gpt-4o-mini", // Fallback to a reasonable default
        }
    }

    /// Create a safety-limited LLM provider from workspace configuration.
    ///
    /// This wraps the provider with safety limits (max_tokens, timeout) to prevent
    /// excessive LLM usage and runaway generation. Used for background document
    /// ingestion when workspace providers could not be created (API key missing, etc.).
    ///
    /// @implements SPEC-032: Safe fallback provider for ingestion
    /// @implements FEAT0781: Safety limits for LLM calls
    /// @implements BR0777: Hard max_tokens limit enforcement
    /// @implements BR0778: Request timeout enforcement
    ///
    /// # Arguments
    ///
    /// * `provider_name` - Provider type (e.g., "openai", "ollama", "lmstudio", "mock")
    /// * `model` - LLM model name (e.g., "gpt-4o-mini", "gemma3:12b")
    ///
    /// # Returns
    ///
    /// Returns an `Arc<dyn LLMProvider>` wrapped with safety limits.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let provider = ProviderFactory::create_safe_llm_provider(
    ///     "ollama",
    ///     "gemma3:12b",
    /// )?;
    /// // All calls now have max_tokens and timeout enforced
    /// let response = provider.complete("Hello").await?;
    /// ```
    pub fn create_safe_llm_provider(
        provider_name: &str,
        model: &str,
    ) -> Result<Arc<dyn LLMProvider>> {
        use crate::safety_limits::{SafetyLimitedProviderWrapper, SafetyLimitsConfig};

        let inner = Self::create_llm_provider(provider_name, model)?;
        let config = SafetyLimitsConfig::from_env();

        tracing::info!(
            provider = provider_name,
            model = model,
            max_tokens = config.max_tokens,
            timeout_secs = config.timeout.as_secs(),
            "Creating safety-limited LLM provider"
        );

        // Note: We need to unwrap the Arc to wrap with SafetyLimitedProvider
        // Since we can't easily wrap Arc<dyn Trait>, we create a new wrapper struct
        Ok(Arc::new(SafetyLimitedProviderWrapper::new(inner, config)))
    }

    /// Create a safety-limited embedding provider from workspace configuration.
    ///
    /// This wraps the provider with timeout to prevent hung requests.
    ///
    /// @implements BR0778: Request timeout enforcement for embeddings
    ///
    /// # Arguments
    ///
    /// * `provider_name` - Provider type (e.g., "openai", "ollama", "lmstudio", "mock")
    /// * `model` - Embedding model name
    /// * `dimension` - Embedding dimension
    ///
    /// # Returns
    ///
    /// Returns an `Arc<dyn EmbeddingProvider>` wrapped with safety limits.
    pub fn create_safe_embedding_provider(
        provider_name: &str,
        model: &str,
        dimension: usize,
    ) -> Result<Arc<dyn EmbeddingProvider>> {
        use crate::safety_limits::{SafetyLimitedEmbeddingProviderWrapper, SafetyLimitsConfig};

        let inner = Self::create_embedding_provider(provider_name, model, dimension)?;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn test_provider_type_parsing() {
        assert_eq!(ProviderType::parse("openai"), Some(ProviderType::OpenAI));
        assert_eq!(ProviderType::parse("OLLAMA"), Some(ProviderType::Ollama));
        assert_eq!(
            ProviderType::parse("lmstudio"),
            Some(ProviderType::LMStudio)
        );
        assert_eq!(
            ProviderType::parse("lm-studio"),
            Some(ProviderType::LMStudio)
        );
        assert_eq!(
            ProviderType::parse("lm_studio"),
            Some(ProviderType::LMStudio)
        );
        assert_eq!(ProviderType::parse("mock"), Some(ProviderType::Mock));
        assert_eq!(ProviderType::parse("invalid"), None);
        assert_eq!(ProviderType::parse(""), None);
    }

    #[test]
    fn test_mock_creation() {
        let (llm, embedding) = ProviderFactory::create_mock();
        assert_eq!(llm.name(), "mock");
        assert_eq!(embedding.name(), "mock");
        assert_eq!(embedding.dimension(), 1536);
    }

    #[test]
    fn test_explicit_mock_creation() {
        let (llm, embedding) = ProviderFactory::create(ProviderType::Mock).unwrap();
        assert_eq!(llm.name(), "mock");
        assert_eq!(embedding.dimension(), 1536);
    }

    #[test]
    #[serial]
    fn test_from_env_fallback_to_mock() {
        // Clear all provider environment variables
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("OLLAMA_HOST");
        std::env::remove_var("OLLAMA_MODEL");
        std::env::remove_var("LMSTUDIO_HOST");
        std::env::remove_var("LMSTUDIO_MODEL");

        let (llm, _) = ProviderFactory::from_env().unwrap();
        assert_eq!(llm.name(), "mock");
    }

    #[test]
    #[serial]
    fn test_explicit_provider_env() {
        // Clean up first to avoid interference from other tests
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
        std::env::remove_var("OLLAMA_HOST");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("LMSTUDIO_HOST");

        std::env::set_var("EDGEQUAKE_LLM_PROVIDER", "mock");
        let (llm, _) = ProviderFactory::from_env().unwrap();
        assert_eq!(llm.name(), "mock");

        // Clean up after
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
    }

    #[test]
    #[serial]
    fn test_lmstudio_auto_detection() {
        // Clean up first to avoid interference from other tests
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
        std::env::remove_var("OLLAMA_HOST");
        std::env::remove_var("OLLAMA_MODEL");
        std::env::remove_var("OPENAI_API_KEY");

        // Set LM Studio environment
        std::env::set_var("LMSTUDIO_HOST", "http://localhost:1234");
        let (llm, embedding) = ProviderFactory::from_env().unwrap();
        assert_eq!(llm.name(), "lmstudio");
        assert_eq!(embedding.name(), "lmstudio");

        // Clean up after
        std::env::remove_var("LMSTUDIO_HOST");
    }

    #[test]
    #[serial]
    fn test_lmstudio_model_detection() {
        // Clean up first to avoid interference from other tests
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
        std::env::remove_var("OLLAMA_HOST");
        std::env::remove_var("OLLAMA_MODEL");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("LMSTUDIO_HOST");

        // Set LM Studio model only
        std::env::set_var("LMSTUDIO_MODEL", "mistral-7b");
        let (llm, _) = ProviderFactory::from_env().unwrap();
        assert_eq!(llm.name(), "lmstudio");

        // Clean up after
        std::env::remove_var("LMSTUDIO_MODEL");
    }

    #[test]
    fn test_explicit_lmstudio_creation() {
        let (llm, embedding) = ProviderFactory::create(ProviderType::LMStudio).unwrap();
        assert_eq!(llm.name(), "lmstudio");
        assert_eq!(embedding.name(), "lmstudio");
        // Default LM Studio embedding dimension is 768 (nomic-embed-text-v1.5)
        assert_eq!(embedding.dimension(), 768);
    }

    #[test]
    #[serial]
    fn test_invalid_provider_env() {
        // Clean up first to avoid interference from other tests
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
        std::env::remove_var("EDGEQUAKE_EMBEDDING_PROVIDER");
        std::env::remove_var("OLLAMA_HOST");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("LMSTUDIO_HOST");

        std::env::set_var("EDGEQUAKE_LLM_PROVIDER", "invalid_provider");
        let result = ProviderFactory::from_env();
        assert!(result.is_err());
        if let Err(e) = result {
            // SPEC-033: Error message now says "LLM provider" for clarity in hybrid mode
            assert!(e.to_string().contains("Unknown LLM provider type"));
        }

        // Clean up after
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
    }

    #[test]
    #[serial]
    fn test_openai_creation_requires_api_key() {
        // Clean up first to avoid interference from other tests
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
        std::env::remove_var("OLLAMA_HOST");
        std::env::remove_var("LMSTUDIO_HOST");

        let result = ProviderFactory::create(ProviderType::OpenAI);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("OPENAI_API_KEY not set"));
        }
    }

    #[test]
    #[serial]
    fn test_embedding_dimension_detection() {
        // Clean up first to avoid interference from other tests
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
        std::env::remove_var("OLLAMA_HOST");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("LMSTUDIO_HOST");

        std::env::set_var("EDGEQUAKE_LLM_PROVIDER", "mock");
        let dim = ProviderFactory::embedding_dimension().unwrap();
        assert_eq!(dim, 1536);

        // Clean up after
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
    }

    #[test]
    #[serial]
    fn test_provider_priority_ollama_over_lmstudio() {
        // Clean up first
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
        std::env::remove_var("EDGEQUAKE_EMBEDDING_PROVIDER");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("LMSTUDIO_HOST");
        std::env::remove_var("LMSTUDIO_MODEL");

        // Set both Ollama and LM Studio - Ollama should win
        std::env::set_var("OLLAMA_HOST", "http://localhost:11434");
        std::env::set_var("LMSTUDIO_HOST", "http://localhost:1234");

        let (llm, _) = ProviderFactory::from_env().unwrap();
        assert_eq!(llm.name(), "ollama");

        // Clean up after
        std::env::remove_var("OLLAMA_HOST");
        std::env::remove_var("LMSTUDIO_HOST");
    }

    /// Test SPEC-033: Hybrid provider mode with separate LLM and embedding providers.
    #[test]
    #[serial]
    fn test_hybrid_mode_separate_providers() {
        // Clean up first
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
        std::env::remove_var("EDGEQUAKE_EMBEDDING_PROVIDER");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("OLLAMA_HOST");
        std::env::remove_var("LMSTUDIO_HOST");

        // Set up hybrid mode: mock LLM + mock embedding (different providers simulated)
        std::env::set_var("EDGEQUAKE_LLM_PROVIDER", "mock");
        std::env::set_var("EDGEQUAKE_EMBEDDING_PROVIDER", "mock");

        let (llm, embedding) = ProviderFactory::from_env().unwrap();

        // Both should be mock providers in test mode
        assert_eq!(llm.name(), "mock");
        assert_eq!(embedding.name(), "mock");

        // Clean up after
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
        std::env::remove_var("EDGEQUAKE_EMBEDDING_PROVIDER");
    }

    /// Test SPEC-033: Invalid embedding provider should fail gracefully.
    #[test]
    #[serial]
    fn test_hybrid_mode_invalid_embedding_provider() {
        // Clean up first
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
        std::env::remove_var("EDGEQUAKE_EMBEDDING_PROVIDER");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("OLLAMA_HOST");

        // Set up hybrid mode with invalid embedding provider
        std::env::set_var("EDGEQUAKE_LLM_PROVIDER", "mock");
        std::env::set_var("EDGEQUAKE_EMBEDDING_PROVIDER", "invalid_embedding");

        let result = ProviderFactory::from_env();
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Unknown embedding provider type"));
        }

        // Clean up after
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
        std::env::remove_var("EDGEQUAKE_EMBEDDING_PROVIDER");
    }
}
