//! Workspace type, model configuration constants, and builder methods.

use edgequake_pdf::PdfParserBackend;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::env::{
    first_non_empty_env_var, non_empty_env_var, EMBEDDING_DIMENSION_ALIASES,
    EMBEDDING_MODEL_ALIASES, EMBEDDING_PROVIDER_ALIASES, LLM_MODEL_ALIASES, LLM_PROVIDER_ALIASES,
};

// ============================================================================
// Model Configuration Constants (SPEC-032)
// ============================================================================
// These defaults MUST match models.toml [defaults] section.
// Ollama is used by default for both LLM and embedding to enable
// development without requiring API keys.
//
// Runtime configuration – environment variable resolution order
// (each step falls back to the next if the variable is absent):
//
//   LLM provider  : EDGEQUAKE_DEFAULT_LLM_PROVIDER
//                   → EDGEQUAKE_LLM_PROVIDER
//                   → "ollama" (constant below)
//   LLM model     : EDGEQUAKE_DEFAULT_LLM_MODEL
//                   → EDGEQUAKE_LLM_MODEL
//                   → sensible default for the resolved provider
//   Embedding provider: EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER
//                       → EDGEQUAKE_EMBEDDING_PROVIDER
//                       → "ollama" (constant below)
//   Embedding model   : EDGEQUAKE_DEFAULT_EMBEDDING_MODEL
//                       → EDGEQUAKE_EMBEDDING_MODEL
//                       → sensible default for the resolved provider
//   Embedding dim     : EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION
//                       → auto-detected from embedding model name
//
// Example – OpenAI in Docker / Portainer (issue #147):
//   EDGEQUAKE_LLM_PROVIDER=openai
//   OPENAI_API_KEY=sk-…
//   # Workspace will be created with gpt-4o-mini / text-embedding-3-small
//   # No need to override the model unless you want a specific one.

/// Default LLM model (Ollama gemma4:latest - 128K context, vision support).
pub const DEFAULT_LLM_MODEL: &str = "gemma4:latest";

/// Default LLM provider.
pub const DEFAULT_LLM_PROVIDER: &str = "ollama";

/// Default embedding model (Ollama embeddinggemma:latest - 768 dimensions, 2K context).
/// Synced with models.toml [defaults] section.
pub const DEFAULT_EMBEDDING_MODEL: &str = "embeddinggemma:latest";

/// Default embedding provider.
/// Synced with models.toml [defaults] section.
pub const DEFAULT_EMBEDDING_PROVIDER: &str = "ollama";

/// Default embedding dimension (Ollama embeddinggemma).
/// Synced with models.toml [defaults] section.
pub const DEFAULT_EMBEDDING_DIMENSION: usize = 768;

/// A document workspace within a tenant (knowledge base).
///
/// ## Per-Workspace Model Configuration (SPEC-032)
///
/// Each workspace has its own LLM and embedding configuration:
/// - LLM: Used for entity extraction, summarization, knowledge graph generation
/// - Embedding: Used for vector search on documents and queries
///
/// Different workspaces can use different models, allowing:
/// - Workspace A: OpenAI GPT-4o + text-embedding-3-small (1536 dims)
/// - Workspace B: Ollama gemma4:latest + embeddinggemma:latest (768 dims)
///
/// ## Model ID Format
///
/// Models are identified by `provider/model_name` format:
/// - `"ollama/gemma4:latest"` - Ollama with Gemma 4
/// - `"openai/gpt-4o-mini"` - OpenAI GPT-4o Mini
/// - `"lmstudio/gemma-3n-e4b-it"` - LM Studio local model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    /// Unique workspace identifier.
    pub workspace_id: Uuid,
    /// Owning tenant ID.
    pub tenant_id: Uuid,
    /// Human-readable name.
    pub name: String,
    /// URL-safe slug (unique within tenant).
    pub slug: String,
    /// Optional description.
    pub description: Option<String>,
    /// Whether the workspace is active.
    pub is_active: bool,
    /// Creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last update timestamp.
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Custom metadata including quotas.
    pub metadata: HashMap<String, serde_json::Value>,

    // === LLM Configuration (SPEC-032) ===
    /// LLM model name (e.g., "gemma3:12b", "gpt-4o-mini").
    /// Used for knowledge graph generation, summarization, entity extraction.
    /// Note: Query-time LLM can be different (user's choice in UI).
    pub llm_model: String,

    /// LLM provider (e.g., "ollama", "openai", "lmstudio").
    /// Determines which API to call for LLM completions during ingestion.
    pub llm_provider: String,

    // === Embedding Configuration (SPEC-032) ===
    /// Embedding model name (e.g., "text-embedding-3-small", "embeddinggemma:latest").
    /// Used for both document ingestion and query embedding generation.
    /// MUST be consistent: query embeddings must use same model as stored vectors.
    pub embedding_model: String,

    /// Embedding provider (e.g., "openai", "ollama", "lmstudio").
    /// Determines which API to call for embedding generation.
    pub embedding_provider: String,

    /// Embedding dimension (e.g., 1536 for OpenAI, 768 for Ollama).
    /// Must match the stored vector dimensions in this workspace.
    pub embedding_dimension: usize,

    // === Vision LLM Configuration (SPEC-040) ===
    /// Vision LLM provider for PDF → Markdown extraction (e.g., "openai", "ollama").
    /// When set, overrides the per-request vision_provider in PDF uploads.
    /// If None, falls back to per-request value or server default ("openai").
    pub vision_llm_provider: Option<String>,

    /// Vision LLM model for PDF page image extraction (e.g., "gpt-4o", "gemma3:latest").
    /// When set, overrides the per-request vision_model in PDF uploads.
    /// If None, uses the default for the configured vision provider.
    pub vision_llm_model: Option<String>,

    /// Default PDF parser backend for this workspace.
    /// None falls back to the environment and then Vision.
    pub pdf_parser_backend: Option<PdfParserBackend>,
}

impl Workspace {
    /// Create a new workspace with default model configuration.
    ///
    /// Reads server defaults from environment variables; see the crate-level
    /// constants block for the full resolution order (issue #147).  The key
    /// change vs earlier versions: `EDGEQUAKE_LLM_PROVIDER` (used by the
    /// provider factory) is now also accepted as a fallback so that single-env
    /// deployments (Docker, Portainer) work without duplicating the provider
    /// name in a separate `EDGEQUAKE_DEFAULT_LLM_PROVIDER` variable.
    pub fn new(tenant_id: Uuid, name: impl Into<String>, slug: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        let (llm_model, llm_provider) = Self::default_llm_config();
        let (embedding_model, embedding_provider, embedding_dimension) =
            Self::default_embedding_config();

        Self {
            workspace_id: Uuid::new_v4(),
            tenant_id,
            name: name.into(),
            slug: slug.into(),
            description: None,
            is_active: true,
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
            llm_model,
            llm_provider,
            embedding_model,
            embedding_provider,
            embedding_dimension,
            vision_llm_provider: None,
            vision_llm_model: None,
            pdf_parser_backend: None,
        }
    }

    /// Resolve the effective PDF parser backend for this workspace.
    pub fn resolved_pdf_parser_backend(&self) -> PdfParserBackend {
        self.pdf_parser_backend
            .or_else(PdfParserBackend::from_env)
            .unwrap_or_default()
    }

    /// Get default LLM configuration from environment.
    ///
    /// Returns `(model, provider)`.  Resolution order (first non-empty wins):
    ///
    /// | Priority | Provider variable             | Model variable             |
    /// |----------|-------------------------------|----------------------------|
    /// | 1        | `EDGEQUAKE_DEFAULT_LLM_PROVIDER` | `EDGEQUAKE_DEFAULT_LLM_MODEL` |
    /// | 2        | `EDGEQUAKE_LLM_PROVIDER`      | `EDGEQUAKE_LLM_MODEL`      |
    /// | 3        | `"ollama"` (constant)         | sensible default for provider |
    ///
    /// When only `EDGEQUAKE_LLM_PROVIDER` is set (typical single-env deployment
    /// with Docker / Portainer), the workspace is initialised with a sensible
    /// model for that provider instead of the hard-coded Ollama default.
    pub fn default_llm_config() -> (String, String) {
        // Resolve provider first so the model default can depend on it.
        // WHY: Docker compose may inject empty-string env vars (e.g.
        // `${EDGEQUAKE_LLM_PROVIDER:-}`) even when the variable is not set on
        // the host. `std::env::var` returns Ok("") for those, so we must
        // explicitly filter out empty strings before falling back to the next
        // candidate in the resolution chain.
        let provider = non_empty_env_var("EDGEQUAKE_DEFAULT_LLM_PROVIDER")
            .or_else(|| {
                first_non_empty_env_var(&[
                    "EDGEQUAKE_LLM_PROVIDER",
                    LLM_PROVIDER_ALIASES[0],
                    LLM_PROVIDER_ALIASES[1],
                ])
            })
            .unwrap_or_else(|| DEFAULT_LLM_PROVIDER.to_string());

        let model = non_empty_env_var("EDGEQUAKE_DEFAULT_LLM_MODEL")
            .or_else(|| {
                first_non_empty_env_var(&[
                    "EDGEQUAKE_LLM_MODEL",
                    LLM_MODEL_ALIASES[0],
                    LLM_MODEL_ALIASES[1],
                ])
            })
            .unwrap_or_else(|| Self::default_model_for_provider(&provider));

        (model, provider)
    }

    /// Get default embedding configuration from environment.
    ///
    /// Returns `(model, provider, dimension)`.  Resolution order mirrors
    /// [`Self::default_llm_config`]: explicit `DEFAULT_*` vars take priority,
    /// then `EDGEQUAKE_EMBEDDING_PROVIDER / MODEL` as a single-env fallback.
    pub fn default_embedding_config() -> (String, String, usize) {
        // Resolve provider first so the model default can depend on it.
        // WHY: Same empty-string guard as in default_llm_config — Docker
        // compose expansion of `${VAR:-}` produces Ok("") not Err, so we
        // must filter those out before falling back to the next candidate.
        let provider = non_empty_env_var("EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER")
            .or_else(|| {
                first_non_empty_env_var(&[
                    "EDGEQUAKE_EMBEDDING_PROVIDER",
                    EMBEDDING_PROVIDER_ALIASES[0],
                ])
            })
            .unwrap_or_else(|| DEFAULT_EMBEDDING_PROVIDER.to_string());

        let model = non_empty_env_var("EDGEQUAKE_DEFAULT_EMBEDDING_MODEL")
            .or_else(|| {
                first_non_empty_env_var(&["EDGEQUAKE_EMBEDDING_MODEL", EMBEDDING_MODEL_ALIASES[0]])
            })
            .unwrap_or_else(|| Self::default_embedding_model_for_provider(&provider));

        // WHY: When the provider/model switches (for example from Ollama to OpenAI),
        // a stale dimension env var like 768 must not override a known model-specific
        // dimension such as 1536 for text-embedding-3-small.
        let detected_dimension = Self::detect_dimension_from_model(&model);
        let known_model_dimension = Self::known_embedding_dimension(&model);

        let dimension = first_non_empty_env_var(&[
            "EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION",
            "EDGEQUAKE_EMBEDDING_DIMENSION",
            EMBEDDING_DIMENSION_ALIASES[0],
        ])
        .and_then(|s| s.parse().ok())
        .filter(|dim| known_model_dimension.is_none() || *dim == detected_dimension)
        .unwrap_or(detected_dimension);

        (model, provider, dimension)
    }

    /// Return the recommended LLM model name for a given provider.
    ///
    /// Called when the user has not explicitly configured a model name, so
    /// that the workspace is initialised with a sensible default that is
    /// actually compatible with the chosen provider.
    pub fn default_model_for_provider(provider: &str) -> String {
        match provider {
            "openai" => "gpt-5-mini".to_string(),
            "anthropic" => "claude-sonnet-4-20250514".to_string(),
            "gemini" => "gemini-2.5-flash".to_string(),
            "mistral" => "mistral-small-latest".to_string(),
            "xai" => "grok-3-mini".to_string(),
            "lmstudio" | "openai-compatible" => "local-model".to_string(),
            // ollama and everything else: use the compiled-in Ollama default.
            _ => DEFAULT_LLM_MODEL.to_string(),
        }
    }

    /// Return the recommended embedding model for a given provider.
    ///
    /// Called when the user has not explicitly configured an embedding model.
    pub fn default_embedding_model_for_provider(provider: &str) -> String {
        match provider {
            "openai" | "openai-compatible" => "text-embedding-3-small".to_string(),
            "lmstudio" => "nomic-embed-text".to_string(),
            // Mistral native embedding — 1024 dimensions, optimised for retrieval.
            "mistral" => "mistral-embed".to_string(),
            // ollama and everything else: use the compiled-in Ollama default.
            _ => DEFAULT_EMBEDDING_MODEL.to_string(),
        }
    }

    /// Auto-detect provider from model name conventions.
    ///
    /// # Examples
    ///
    /// - "text-embedding-3-small" → "openai"
    /// - "gemma3:12b" → "ollama" (colon indicates Ollama tag format)
    /// - "gemma2-9b-it" → "lmstudio"
    pub fn detect_provider_from_model(model: &str) -> String {
        if model.starts_with("text-embedding") || model.starts_with("ada") {
            "openai".to_string()
        } else if model.contains(':') {
            // Ollama uses "model:tag" format
            "ollama".to_string()
        } else if model.starts_with("gemma") || model.starts_with("llama") {
            "lmstudio".to_string()
        } else {
            // Default fallback to openai
            "openai".to_string()
        }
    }

    /// Auto-detect embedding dimension from known model names.
    ///
    /// # Known Models
    ///
    /// | Model | Dimension |
    /// |-------|-----------|
    /// | text-embedding-3-small | 1536 |
    /// | text-embedding-3-large | 3072 |
    /// | text-embedding-ada-002 | 1536 |
    /// | embeddinggemma:latest | 768 |
    /// | nomic-embed-text | 768 |
    /// | mxbai-embed-large | 1024 |
    pub fn known_embedding_dimension(model: &str) -> Option<usize> {
        match model {
            "text-embedding-3-small" | "text-embedding-ada-002" => Some(1536),
            "text-embedding-3-large" => Some(3072),
            "embeddinggemma:latest" | "nomic-embed-text" | "nomic-embed-text:latest" => Some(768),
            // Mistral embed returns 1024-dimensional vectors.
            "mistral-embed" | "mistral-embed-2312" | "codestral-embed" | "codestral-embed-2505" => {
                Some(1024)
            }
            "mxbai-embed-large" | "mxbai-embed-large:latest" => Some(1024),
            _ if model.contains("768") => Some(768),
            _ if model.contains("1024") => Some(1024),
            _ if model.contains("3072") => Some(3072),
            _ => None,
        }
    }

    pub fn detect_dimension_from_model(model: &str) -> usize {
        Self::known_embedding_dimension(model).unwrap_or(DEFAULT_EMBEDDING_DIMENSION)
    }

    /// Set the description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set max documents quota.
    pub fn with_max_documents(mut self, max: usize) -> Self {
        self.metadata
            .insert("max_documents".to_string(), serde_json::json!(max));
        self
    }

    /// Get max documents quota.
    pub fn max_documents(&self) -> Option<usize> {
        self.metadata
            .get("max_documents")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
    }

    // === Embedding Configuration Builder Methods (SPEC-032) ===

    /// Set the embedding model and auto-detect provider/dimension.
    ///
    /// # Example
    ///
    /// ```
    /// use edgequake_core::Workspace;
    /// use uuid::Uuid;
    ///
    /// let workspace = Workspace::new(Uuid::new_v4(), "My Workspace", "my-workspace")
    ///     .with_embedding_model("embeddinggemma:latest");
    ///
    /// assert_eq!(workspace.embedding_model, "embeddinggemma:latest");
    /// assert_eq!(workspace.embedding_provider, "ollama");
    /// assert_eq!(workspace.embedding_dimension, 768);
    /// ```
    pub fn with_embedding_model(mut self, model: impl Into<String>) -> Self {
        let model = model.into();
        self.embedding_provider = Self::detect_provider_from_model(&model);
        self.embedding_dimension = Self::detect_dimension_from_model(&model);
        self.embedding_model = model;
        self
    }

    /// Set the embedding provider explicitly.
    pub fn with_embedding_provider(mut self, provider: impl Into<String>) -> Self {
        self.embedding_provider = provider.into();
        self
    }

    /// Set the embedding dimension explicitly.
    ///
    /// Use this when auto-detection doesn't work for custom models.
    pub fn with_embedding_dimension(mut self, dimension: usize) -> Self {
        self.embedding_dimension = dimension;
        self
    }

    /// Set complete embedding configuration.
    ///
    /// # Arguments
    ///
    /// * `model` - Embedding model name
    /// * `provider` - Provider name (openai, ollama, lmstudio)
    /// * `dimension` - Vector dimension
    pub fn with_embedding_config(
        mut self,
        model: impl Into<String>,
        provider: impl Into<String>,
        dimension: usize,
    ) -> Self {
        self.embedding_model = model.into();
        self.embedding_provider = provider.into();
        self.embedding_dimension = dimension;
        self
    }

    // === LLM Configuration Builder Methods (SPEC-032) ===

    /// Set the LLM model and auto-detect provider.
    ///
    /// # Example
    ///
    /// ```
    /// use edgequake_core::Workspace;
    /// use uuid::Uuid;
    ///
    /// let workspace = Workspace::new(Uuid::new_v4(), "My Workspace", "my-workspace")
    ///     .with_llm_model("gemma3:12b");
    ///
    /// assert_eq!(workspace.llm_model, "gemma3:12b");
    /// assert_eq!(workspace.llm_provider, "ollama");
    /// ```
    pub fn with_llm_model(mut self, model: impl Into<String>) -> Self {
        let model = model.into();
        self.llm_provider = Self::detect_provider_from_model(&model);
        self.llm_model = model;
        self
    }

    /// Set the LLM provider explicitly.
    pub fn with_llm_provider(mut self, provider: impl Into<String>) -> Self {
        self.llm_provider = provider.into();
        self
    }

    /// Set complete LLM configuration.
    ///
    /// # Arguments
    ///
    /// * `model` - LLM model name
    /// * `provider` - Provider name (openai, ollama, lmstudio)
    pub fn with_llm_config(
        mut self,
        model: impl Into<String>,
        provider: impl Into<String>,
    ) -> Self {
        self.llm_model = model.into();
        self.llm_provider = provider.into();
        self
    }

    // === Full Model ID Methods (SPEC-032) ===

    /// Get fully qualified LLM model ID in `provider/model` format.
    ///
    /// # Example
    ///
    /// ```
    /// use edgequake_core::Workspace;
    /// use uuid::Uuid;
    ///
    /// let workspace = Workspace::new(Uuid::new_v4(), "Test", "test")
    ///     .with_llm_config("gemma3:12b", "ollama");
    ///
    /// assert_eq!(workspace.llm_full_id(), "ollama/gemma3:12b");
    /// ```
    pub fn llm_full_id(&self) -> String {
        format!("{}/{}", self.llm_provider, self.llm_model)
    }

    /// Get fully qualified embedding model ID in `provider/model` format.
    ///
    /// # Example
    ///
    /// ```
    /// use edgequake_core::Workspace;
    /// use uuid::Uuid;
    ///
    /// let workspace = Workspace::new(Uuid::new_v4(), "Test", "test")
    ///     .with_embedding_config("text-embedding-3-small", "openai", 1536);
    ///
    /// assert_eq!(workspace.embedding_full_id(), "openai/text-embedding-3-small");
    /// ```
    pub fn embedding_full_id(&self) -> String {
        format!("{}/{}", self.embedding_provider, self.embedding_model)
    }

    /// Parse a full model ID into (provider, model) tuple.
    ///
    /// # Arguments
    ///
    /// * `full_id` - Model ID in `provider/model` format (e.g., "ollama/gemma3:12b")
    ///
    /// # Returns
    ///
    /// `Some((provider, model))` if valid format, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```
    /// use edgequake_core::Workspace;
    ///
    /// assert_eq!(
    ///     Workspace::parse_model_id("ollama/gemma3:12b"),
    ///     Some(("ollama".to_string(), "gemma3:12b".to_string()))
    /// );
    ///
    /// assert_eq!(Workspace::parse_model_id("invalid"), None);
    /// ```
    pub fn parse_model_id(full_id: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = full_id.splitn(2, '/').collect();
        if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
            Some((parts[0].to_string(), parts[1].to_string()))
        } else {
            None
        }
    }

    // === Vision Configuration Builder Methods (SPEC-040) ===

    /// Set the vision LLM provider for PDF-to-Markdown conversion.
    pub fn with_vision_provider(mut self, provider: impl Into<String>) -> Self {
        self.vision_llm_provider = Some(provider.into());
        self
    }

    /// Set the vision LLM model for PDF-to-Markdown conversion.
    pub fn with_vision_model(mut self, model: impl Into<String>) -> Self {
        self.vision_llm_model = Some(model.into());
        self
    }
}

// ============================================================================
// Tests
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    // WHY: Tests that read/write process-level env vars must not run in
    // parallel.  A single `static Mutex` provides a lightweight serial gate
    // without pulling in external crates.
    static ENV_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn lock_env_tests() -> std::sync::MutexGuard<'static, ()> {
        ENV_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    // ── default_model_for_provider ─────────────────────────────────────────

    #[test]
    fn test_default_model_openai() {
        assert_eq!(
            Workspace::default_model_for_provider("openai"),
            "gpt-5-mini"
        );
    }

    #[test]
    fn test_default_model_ollama() {
        assert_eq!(
            Workspace::default_model_for_provider("ollama"),
            DEFAULT_LLM_MODEL
        );
    }

    #[test]
    fn test_default_model_unknown_falls_back_to_ollama_default() {
        assert_eq!(
            Workspace::default_model_for_provider("unknown-provider"),
            DEFAULT_LLM_MODEL
        );
    }

    // ── default_embedding_model_for_provider ──────────────────────────────

    #[test]
    fn test_default_embedding_model_openai() {
        assert_eq!(
            Workspace::default_embedding_model_for_provider("openai"),
            "text-embedding-3-small"
        );
    }

    #[test]
    fn test_default_embedding_model_ollama() {
        assert_eq!(
            Workspace::default_embedding_model_for_provider("ollama"),
            DEFAULT_EMBEDDING_MODEL
        );
    }

    // ── default_llm_config env-var resolution (issue #147) ────────────────

    #[test]
    fn test_llm_config_reads_edgequake_llm_provider_as_fallback() {
        let _guard = lock_env_tests();
        // Simulate a Portainer / Docker deployment where only the factory var is set.
        // EDGEQUAKE_DEFAULT_LLM_PROVIDER is NOT set; EDGEQUAKE_LLM_PROVIDER IS set.
        // The workspace must honour it and pick a sensible model for OpenAI.
        let key_provider = "EDGEQUAKE_LLM_PROVIDER";
        let key_model = "EDGEQUAKE_LLM_MODEL";
        let key_default_provider = "EDGEQUAKE_DEFAULT_LLM_PROVIDER";
        let key_default_model = "EDGEQUAKE_DEFAULT_LLM_MODEL";

        // Remove both DEFAULT vars to test fallback path.
        std::env::remove_var(key_default_provider);
        std::env::remove_var(key_default_model);
        std::env::set_var(key_provider, "openai");
        std::env::remove_var(key_model);

        let (model, provider) = Workspace::default_llm_config();

        // Restore environment.
        std::env::remove_var(key_provider);

        assert_eq!(provider, "openai");
        assert_eq!(
            model, "gpt-5-mini",
            "Should pick the sensible OpenAI default when no model is explicitly set"
        );
    }

    #[test]
    fn test_llm_config_default_vars_take_priority_over_llm_provider() {
        let _guard = lock_env_tests();
        // EDGEQUAKE_DEFAULT_LLM_PROVIDER takes priority over EDGEQUAKE_LLM_PROVIDER.
        let key_default_provider = "EDGEQUAKE_DEFAULT_LLM_PROVIDER";
        let key_default_model = "EDGEQUAKE_DEFAULT_LLM_MODEL";
        let key_provider = "EDGEQUAKE_LLM_PROVIDER";

        std::env::set_var(key_default_provider, "anthropic");
        std::env::set_var(key_default_model, "claude-3-opus-20240229");
        std::env::set_var(key_provider, "openai"); // should be ignored

        let (model, provider) = Workspace::default_llm_config();

        std::env::remove_var(key_default_provider);
        std::env::remove_var(key_default_model);
        std::env::remove_var(key_provider);

        assert_eq!(provider, "anthropic");
        assert_eq!(model, "claude-3-opus-20240229");
    }

    #[test]
    fn test_llm_config_constant_fallback_when_no_env_set() {
        let _guard = lock_env_tests();
        let key_default_provider = "EDGEQUAKE_DEFAULT_LLM_PROVIDER";
        let key_default_model = "EDGEQUAKE_DEFAULT_LLM_MODEL";
        let key_provider = "EDGEQUAKE_LLM_PROVIDER";
        let key_model = "EDGEQUAKE_LLM_MODEL";

        std::env::remove_var(key_default_provider);
        std::env::remove_var(key_default_model);
        std::env::remove_var(key_provider);
        std::env::remove_var(key_model);

        let (model, provider) = Workspace::default_llm_config();

        assert_eq!(provider, DEFAULT_LLM_PROVIDER);
        assert_eq!(model, DEFAULT_LLM_MODEL);
    }

    // ── default_embedding_config env-var resolution ────────────────────────

    #[test]
    fn test_embedding_config_reads_edgequake_embedding_provider_as_fallback() {
        let _guard = lock_env_tests();
        let key_provider = "EDGEQUAKE_EMBEDDING_PROVIDER";
        let key_default_provider = "EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER";
        let key_default_model = "EDGEQUAKE_DEFAULT_EMBEDDING_MODEL";
        let key_model = "EDGEQUAKE_EMBEDDING_MODEL";

        std::env::remove_var(key_default_provider);
        std::env::remove_var(key_default_model);
        std::env::set_var(key_provider, "openai");
        std::env::remove_var(key_model);

        let (model, provider, dim) = Workspace::default_embedding_config();

        std::env::remove_var(key_provider);

        assert_eq!(provider, "openai");
        assert_eq!(model, "text-embedding-3-small");
        assert_eq!(dim, 1536);
    }

    /// Regression test: Docker Compose expands `${VAR:-}` to empty string when
    /// the variable is not set on the host.  `std::env::var` returns `Ok("")`
    /// for those values.  The provider resolution chain MUST treat empty strings
    /// as absent and fall through to the next candidate / hard-coded default.
    #[test]
    fn test_embedding_config_ignores_empty_string_env_vars() {
        let _guard = lock_env_tests();
        let key_default_provider = "EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER";
        let key_provider = "EDGEQUAKE_EMBEDDING_PROVIDER";
        let key_default_model = "EDGEQUAKE_DEFAULT_EMBEDDING_MODEL";
        let key_model = "EDGEQUAKE_EMBEDDING_MODEL";

        // Simulate Docker Compose injecting empty strings (${VAR:-})
        std::env::set_var(key_default_provider, "");
        std::env::set_var(key_provider, "");
        std::env::set_var(key_default_model, "");
        std::env::set_var(key_model, "");

        let (model, provider, _dim) = Workspace::default_embedding_config();

        std::env::remove_var(key_default_provider);
        std::env::remove_var(key_provider);
        std::env::remove_var(key_default_model);
        std::env::remove_var(key_model);

        // Must fall back to the hard-coded Ollama defaults, NOT use empty string
        assert_eq!(
            provider, DEFAULT_EMBEDDING_PROVIDER,
            "empty env var must not override the default provider"
        );
        assert_eq!(
            model, DEFAULT_EMBEDDING_MODEL,
            "empty env var must not override the default model"
        );
    }

    /// Same empty-string guard for LLM config.
    #[test]
    fn test_llm_config_ignores_empty_string_env_vars() {
        let _guard = lock_env_tests();
        let key_default_provider = "EDGEQUAKE_DEFAULT_LLM_PROVIDER";
        let key_provider = "EDGEQUAKE_LLM_PROVIDER";
        let key_default_model = "EDGEQUAKE_DEFAULT_LLM_MODEL";
        let key_model = "EDGEQUAKE_LLM_MODEL";

        std::env::set_var(key_default_provider, "");
        std::env::set_var(key_provider, "");
        std::env::set_var(key_default_model, "");
        std::env::set_var(key_model, "");

        let (model, provider) = Workspace::default_llm_config();

        std::env::remove_var(key_default_provider);
        std::env::remove_var(key_provider);
        std::env::remove_var(key_default_model);
        std::env::remove_var(key_model);

        assert_eq!(
            provider, DEFAULT_LLM_PROVIDER,
            "empty env var must not override the default provider"
        );
        assert_eq!(
            model, DEFAULT_LLM_MODEL,
            "empty env var must not override the default model"
        );
    }

    #[test]
    fn test_llm_config_supports_light_rag_style_aliases() {
        let _guard = lock_env_tests();

        std::env::remove_var("EDGEQUAKE_DEFAULT_LLM_PROVIDER");
        std::env::remove_var("EDGEQUAKE_DEFAULT_LLM_MODEL");
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
        std::env::remove_var("EDGEQUAKE_LLM_MODEL");
        std::env::set_var("MODEL_PROVIDER", "openai");
        std::env::set_var("CHAT_MODEL", "gpt-5-nano");

        let (model, provider) = Workspace::default_llm_config();

        std::env::remove_var("MODEL_PROVIDER");
        std::env::remove_var("CHAT_MODEL");

        assert_eq!(provider, "openai");
        assert_eq!(model, "gpt-5-nano");
    }

    #[test]
    fn test_embedding_config_supports_compatibility_aliases() {
        let _guard = lock_env_tests();

        std::env::remove_var("EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER");
        std::env::remove_var("EDGEQUAKE_DEFAULT_EMBEDDING_MODEL");
        std::env::remove_var("EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION");
        std::env::remove_var("EDGEQUAKE_EMBEDDING_PROVIDER");
        std::env::remove_var("EDGEQUAKE_EMBEDDING_MODEL");
        std::env::remove_var("EDGEQUAKE_EMBEDDING_DIMENSION");
        std::env::set_var("EMBEDDING_PROVIDER", "openai");
        std::env::set_var("EMBEDDING_MODEL", "text-embedding-3-small");
        std::env::set_var("EMBEDDING_DIMENSION", "1536");

        let (model, provider, dimension) = Workspace::default_embedding_config();

        std::env::remove_var("EMBEDDING_PROVIDER");
        std::env::remove_var("EMBEDDING_MODEL");
        std::env::remove_var("EMBEDDING_DIMENSION");

        assert_eq!(provider, "openai");
        assert_eq!(model, "text-embedding-3-small");
        assert_eq!(dimension, 1536);
    }
}
