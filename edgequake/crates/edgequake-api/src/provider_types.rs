//! Provider status types for API responses.
//!
//! @implements SPEC-032: Ollama/LM Studio provider support - Status API
//! @iteration OODA Loop #5 - Phase 5E.1 + OODA 12

use serde::{Deserialize, Serialize};

/// Complete provider status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatusResponse {
    pub provider: LLMProviderStatus,
    pub embedding: EmbeddingProviderStatus,
    pub storage: StorageStatus,
    pub metadata: StatusMetadata,
}

/// LLM provider status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMProviderStatus {
    /// Provider name: "ollama", "openai", "lmstudio", "mock"
    pub name: String,

    /// Provider type (always "llm" for LLM providers)
    #[serde(rename = "type")]
    pub provider_type: String,

    /// Connection status
    pub status: ConnectionStatus,

    /// Model name (e.g., "gemma3:12b", "gpt-4o-mini")
    pub model: String,

    /// Base URL for the provider (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,

    /// Provider-specific configuration
    pub config: serde_json::Value,
}

/// Embedding provider status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingProviderStatus {
    /// Provider name
    pub name: String,

    /// Provider type (always "embedding")
    #[serde(rename = "type")]
    pub provider_type: String,

    /// Connection status
    pub status: ConnectionStatus,

    /// Model name (e.g., "embeddinggemma:latest")
    pub model: String,

    /// Embedding dimension (768, 1536, etc.)
    pub dimension: usize,
}

/// Vector storage status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStatus {
    /// Storage type: "memory" or "postgres"
    #[serde(rename = "type")]
    pub storage_type: String,

    /// Storage dimension (must match embedding dimension)
    pub dimension: usize,

    /// Whether storage dimension mismatches provider dimension
    pub dimension_mismatch: bool,

    /// Storage namespace
    pub namespace: String,
}

/// Provider connection status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    /// Provider is responsive
    Connected,

    /// Currently checking provider status
    Connecting,

    /// Provider not reachable
    Disconnected,

    /// Configuration error
    Error,
}

/// Status check metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusMetadata {
    /// ISO 8601 timestamp of status check
    pub checked_at: String,

    /// Server uptime in seconds
    pub uptime_seconds: u64,
}

/// Response for listing available providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableProvidersResponse {
    /// List of available LLM providers
    pub llm_providers: Vec<ProviderInfo>,
    /// List of available embedding providers
    pub embedding_providers: Vec<ProviderInfo>,
    /// Current active LLM provider name
    pub active_llm_provider: String,
    /// Current active embedding provider name
    pub active_embedding_provider: String,
}

/// Information about a single provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    /// Unique provider ID (e.g., "openai", "ollama", "lmstudio", "mock")
    pub id: String,
    /// Human-readable provider name
    pub name: String,
    /// Provider description
    pub description: String,
    /// Whether the provider is available (API key set, service reachable, etc.)
    pub available: bool,
    /// Provider-specific configuration requirements
    pub config_requirements: Vec<ConfigRequirement>,
    /// Default models for this provider
    pub default_models: DefaultModels,
}

/// Configuration requirement for a provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigRequirement {
    /// Environment variable name
    pub env_var: String,
    /// Whether this is required (vs optional)
    pub required: bool,
    /// Description of the requirement
    pub description: String,
    /// Whether this requirement is currently satisfied
    pub satisfied: bool,
}

/// Default models for a provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultModels {
    /// Default chat/LLM model
    pub chat_model: String,
    /// Default embedding model
    pub embedding_model: String,
    /// Default embedding dimension
    pub embedding_dimension: usize,
}

impl AvailableProvidersResponse {
    /// Build from bundled `models.toml` (legacy entry point — prefer [`Self::from_models_config`]).
    pub fn build(active_llm: &str, active_embedding: &str) -> Self {
        Self::from_models_config(
            &crate::state::bundled_models::load_bundled_models_config(),
            active_llm,
            active_embedding,
        )
    }

    /// Build from runtime [`ModelsConfig`] held in [`QueryRuntime`](crate::state::QueryRuntime).
    pub fn from_models_config(
        config: &edgequake_llm::ModelsConfig,
        active_llm: &str,
        active_embedding: &str,
    ) -> Self {
        crate::provider_catalog::build_available_providers_response(
            config,
            active_llm,
            active_embedding,
        )
    }
}

impl ProviderStatusResponse {
    /// Create a new provider status response from AppState
    pub fn from_app_state(app_state: &crate::state::AppState) -> Self {
        use chrono::Utc;

        // Get LLM provider info
        let llm_name = app_state.query.llm_provider.name().to_string();
        let llm_model = app_state.query.llm_provider.model().to_string();

        // Get embedding provider info
        let emb_name = app_state.query.embedding_provider.name().to_string();
        let emb_model = app_state.query.embedding_provider.model().to_string();
        let emb_dim = app_state.query.embedding_provider.dimension();

        // Get storage info
        let storage_dim = app_state.storage.vector_storage.dimension();
        let storage_namespace = app_state.storage.vector_storage.namespace();

        // Detect storage type using storage_mode field
        let storage_type = app_state.storage.mode.as_str();

        // Check dimension mismatch
        let dimension_mismatch = storage_dim != emb_dim;

        // Get uptime
        let uptime = app_state.start_time.elapsed().as_secs();

        // Generate timestamp
        let checked_at = Utc::now().to_rfc3339();

        Self {
            provider: LLMProviderStatus {
                name: llm_name,
                provider_type: "llm".to_string(),
                status: ConnectionStatus::Connected, // MVP: assume connected
                model: llm_model,
                base_url: None, // TODO: Extract from provider config
                config: serde_json::json!({}),
            },
            embedding: EmbeddingProviderStatus {
                name: emb_name,
                provider_type: "embedding".to_string(),
                status: ConnectionStatus::Connected, // MVP: assume connected
                model: emb_model,
                dimension: emb_dim,
            },
            storage: StorageStatus {
                storage_type: storage_type.to_string(),
                dimension: storage_dim,
                dimension_mismatch,
                namespace: storage_namespace.to_string(),
            },
            metadata: StatusMetadata {
                checked_at,
                uptime_seconds: uptime,
            },
        }
    }
}
