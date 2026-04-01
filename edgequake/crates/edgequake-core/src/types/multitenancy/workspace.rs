//! Workspace type, model configuration constants, and builder methods.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ============================================================================
// Model Configuration Constants (SPEC-032)
// ============================================================================
// These defaults are only used as a safe fallback when environment variables
// are not set. Production/runtime defaults should come from the corresponding
// EDGEQUAKE_DEFAULT_* and EDGEQUAKE_VISION_* environment variables.
//
// To use OpenAI or other providers, set environment variables:
//   - EDGEQUAKE_DEFAULT_LLM_PROVIDER=openai
//   - EDGEQUAKE_DEFAULT_LLM_MODEL=gpt-4o-mini
//   - EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER=openai
//   - EDGEQUAKE_DEFAULT_EMBEDDING_MODEL=text-embedding-3-small
//   - EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION=1536

/// Default LLM model (Ollama gemma3:12b - 128K context, vision support).
pub const DEFAULT_LLM_MODEL: &str = "gemma3:12b";

/// Default LLM provider.
pub const DEFAULT_LLM_PROVIDER: &str = "ollama";

/// Default embedding model (Ollama embeddinggemma - 768 dimensions, 2K context).
/// Synced with models.toml [defaults] section.
pub const DEFAULT_EMBEDDING_MODEL: &str = "embeddinggemma";

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
/// - Workspace B: Ollama gemma3:12b + embeddinggemma:latest (768 dims)
///
/// ## Model ID Format
///
/// Models are identified by `provider/model_name` format:
/// - `"ollama/gemma3:12b"` - Ollama with Gemma 3 12B
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
}

impl Workspace {
    /// Create a new workspace with default model configuration.
    ///
    /// Uses server defaults from environment variables if set:
    /// - `EDGEQUAKE_DEFAULT_LLM_MODEL`
    /// - `EDGEQUAKE_DEFAULT_LLM_PROVIDER`
    /// - `EDGEQUAKE_DEFAULT_EMBEDDING_MODEL`
    /// - `EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER`
    /// - `EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION`
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
        }
    }

    /// Get default LLM configuration from environment.
    ///
    /// Returns (model, provider) tuple.
    pub fn default_llm_config() -> (String, String) {
        let model = std::env::var("EDGEQUAKE_DEFAULT_LLM_MODEL")
            .unwrap_or_else(|_| DEFAULT_LLM_MODEL.to_string());

        let provider = std::env::var("EDGEQUAKE_DEFAULT_LLM_PROVIDER")
            .unwrap_or_else(|_| DEFAULT_LLM_PROVIDER.to_string());

        (model, provider)
    }

    /// Get default embedding configuration from environment.
    ///
    /// Returns (model, provider, dimension) tuple.
    pub fn default_embedding_config() -> (String, String, usize) {
        let model = std::env::var("EDGEQUAKE_DEFAULT_EMBEDDING_MODEL")
            .unwrap_or_else(|_| DEFAULT_EMBEDDING_MODEL.to_string());

        let provider = std::env::var("EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER")
            .unwrap_or_else(|_| Self::detect_provider_from_model(&model));

        let dimension = std::env::var("EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION")
            .and_then(|s| s.parse().map_err(|_| std::env::VarError::NotPresent))
            .unwrap_or_else(|_| Self::detect_dimension_from_model(&model));

        (model, provider, dimension)
    }

    /// Get default vision configuration from environment.
    ///
    /// Returns `(provider, model)` when `EDGEQUAKE_VISION_MODEL` is set.
    /// If the provider is not set explicitly, it is inferred from the model
    /// name so OpenAI-compatible aliases keep their runtime provider name.
    pub fn default_vision_config() -> (Option<String>, Option<String>) {
        let model = std::env::var("EDGEQUAKE_VISION_MODEL")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let provider = model.as_deref().map(|model| {
            std::env::var("EDGEQUAKE_VISION_PROVIDER")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| Self::detect_provider_from_model(model))
        });

        match (provider, model) {
            (Some(provider), Some(model)) => (Some(provider), Some(model)),
            (Some(provider), None) => (Some(provider), None),
            _ => (None, None),
        }
    }

    /// Auto-detect provider from model name conventions.
    ///
    /// # Examples
    ///
    /// - "text-embedding-3-small" → "openai"
    /// - "gemma3:12b" → "ollama" (colon indicates Ollama tag format)
    /// - "qwen3.5-35b-a3b" → "litellm-local" (OpenAI-compatible LiteLLM alias)
    /// - "gemma2-9b-it" → "lmstudio"
    pub fn detect_provider_from_model(model: &str) -> String {
        if model.starts_with("text-embedding") || model.starts_with("ada") {
            "openai".to_string()
        } else if model.contains(':') {
            // Ollama uses "model:tag" format
            "ollama".to_string()
        } else if model.starts_with("qwen3") {
            "litellm-local".to_string()
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
    /// | qwen3-embedding-0.6b | 1024 |
    pub fn detect_dimension_from_model(model: &str) -> usize {
        match model {
            "text-embedding-3-small" | "text-embedding-ada-002" => 1536,
            "text-embedding-3-large" => 3072,
            "embeddinggemma:latest" | "nomic-embed-text" | "nomic-embed-text:latest" => 768,
            "mxbai-embed-large" | "mxbai-embed-large:latest" => 1024,
            _ if model.starts_with("qwen3-embedding") => 1024,
            _ if model.contains("768") => 768,
            _ if model.contains("1024") => 1024,
            _ if model.contains("3072") => 3072,
            _ => DEFAULT_EMBEDDING_DIMENSION, // Safe default
        }
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
