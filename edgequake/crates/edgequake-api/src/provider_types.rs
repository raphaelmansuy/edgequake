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
    /// Build the list of available providers based on environment configuration
    pub fn build(active_llm: &str, active_embedding: &str) -> Self {
        let llm_providers = vec![
            ProviderInfo {
                id: "openai".to_string(),
                name: "OpenAI".to_string(),
                description: "OpenAI API (GPT-4o, GPT-5 Nano)".to_string(),
                available: std::env::var("OPENAI_API_KEY").is_ok(),
                config_requirements: vec![ConfigRequirement {
                    env_var: "OPENAI_API_KEY".to_string(),
                    required: true,
                    description: "OpenAI API key".to_string(),
                    satisfied: std::env::var("OPENAI_API_KEY").is_ok(),
                }],
                default_models: DefaultModels {
                    chat_model: "gpt-4.1-nano".to_string(),
                    embedding_model: "text-embedding-3-small".to_string(),
                    embedding_dimension: 1536,
                },
            },
            ProviderInfo {
                id: "anthropic".to_string(),
                name: "Anthropic".to_string(),
                description: "Anthropic Claude (Opus 4.6, Sonnet 4.5, Haiku 4.5)".to_string(),
                available: std::env::var("ANTHROPIC_API_KEY").is_ok(),
                config_requirements: vec![ConfigRequirement {
                    env_var: "ANTHROPIC_API_KEY".to_string(),
                    required: true,
                    description: "Anthropic API key".to_string(),
                    satisfied: std::env::var("ANTHROPIC_API_KEY").is_ok(),
                }],
                default_models: DefaultModels {
                    chat_model: "claude-sonnet-4-5-20250929".to_string(),
                    embedding_model: "".to_string(),
                    embedding_dimension: 0,
                },
            },
            ProviderInfo {
                id: "gemini".to_string(),
                name: "Google Gemini".to_string(),
                description: "Google Gemini (2.5 Pro, 2.5 Flash)".to_string(),
                available: std::env::var("GEMINI_API_KEY").is_ok(),
                config_requirements: vec![ConfigRequirement {
                    env_var: "GEMINI_API_KEY".to_string(),
                    required: true,
                    description: "Google Gemini API key".to_string(),
                    satisfied: std::env::var("GEMINI_API_KEY").is_ok(),
                }],
                default_models: DefaultModels {
                    chat_model: "gemini-2.5-flash".to_string(),
                    embedding_model: "gemini-embedding-001".to_string(),
                    embedding_dimension: 3072,
                },
            },
            ProviderInfo {
                id: "xai".to_string(),
                name: "xAI".to_string(),
                description: "xAI Grok models (Grok-4.1, Grok-3)".to_string(),
                available: std::env::var("XAI_API_KEY").is_ok(),
                config_requirements: vec![ConfigRequirement {
                    env_var: "XAI_API_KEY".to_string(),
                    required: true,
                    description: "xAI API key".to_string(),
                    satisfied: std::env::var("XAI_API_KEY").is_ok(),
                }],
                default_models: DefaultModels {
                    chat_model: "grok-4-1-fast".to_string(),
                    embedding_model: "".to_string(),
                    embedding_dimension: 0,
                },
            },
            ProviderInfo {
                id: "openrouter".to_string(),
                name: "OpenRouter".to_string(),
                description: "OpenRouter - Unified access to 616+ models".to_string(),
                available: std::env::var("OPENROUTER_API_KEY").is_ok(),
                config_requirements: vec![ConfigRequirement {
                    env_var: "OPENROUTER_API_KEY".to_string(),
                    required: true,
                    description: "OpenRouter API key".to_string(),
                    satisfied: std::env::var("OPENROUTER_API_KEY").is_ok(),
                }],
                default_models: DefaultModels {
                    chat_model: "openai/gpt-4o-mini".to_string(),
                    embedding_model: "".to_string(),
                    embedding_dimension: 0,
                },
            },
            ProviderInfo {
                id: "ollama".to_string(),
                name: "Ollama".to_string(),
                description: "Local/remote Ollama instance".to_string(),
                available: true, // Ollama always available with defaults
                config_requirements: vec![
                    ConfigRequirement {
                        env_var: "OLLAMA_HOST".to_string(),
                        required: false,
                        description: "Ollama server URL (default: http://localhost:11434)"
                            .to_string(),
                        // WHY: Always satisfied because Ollama has builtin defaults
                        satisfied: true,
                    },
                    ConfigRequirement {
                        env_var: "OLLAMA_MODEL".to_string(),
                        required: false,
                        description: "Chat model name".to_string(),
                        satisfied: true,
                    },
                ],
                default_models: DefaultModels {
                    chat_model: "gemma3:12b".to_string(),
                    embedding_model: "embeddinggemma:latest".to_string(),
                    embedding_dimension: 768,
                },
            },
            ProviderInfo {
                id: "lmstudio".to_string(),
                name: "LM Studio".to_string(),
                description: "Local LM Studio instance (OpenAI-compatible)".to_string(),
                available: true, // LM Studio always available with defaults
                config_requirements: vec![
                    ConfigRequirement {
                        env_var: "LMSTUDIO_HOST".to_string(),
                        required: false,
                        description: "LM Studio server URL (default: http://localhost:1234)"
                            .to_string(),
                        // WHY: Always satisfied because LM Studio has builtin defaults
                        satisfied: true,
                    },
                    ConfigRequirement {
                        env_var: "LMSTUDIO_MODEL".to_string(),
                        required: false,
                        description: "Chat model name".to_string(),
                        satisfied: true,
                    },
                ],
                default_models: DefaultModels {
                    chat_model: "gemma-3n-e4b-it".to_string(),
                    embedding_model: "nomic-embed-text-v1.5".to_string(),
                    embedding_dimension: 768,
                },
            },
            ProviderInfo {
                id: "azure".to_string(),
                name: "Azure OpenAI".to_string(),
                description: "Azure-hosted OpenAI models (enterprise)".to_string(),
                available: std::env::var("AZURE_OPENAI_API_KEY").is_ok(),
                config_requirements: vec![
                    ConfigRequirement {
                        env_var: "AZURE_OPENAI_API_KEY".to_string(),
                        required: true,
                        description: "Azure OpenAI API key".to_string(),
                        satisfied: std::env::var("AZURE_OPENAI_API_KEY").is_ok(),
                    },
                    ConfigRequirement {
                        env_var: "AZURE_OPENAI_ENDPOINT".to_string(),
                        required: true,
                        description: "Azure OpenAI endpoint URL".to_string(),
                        satisfied: std::env::var("AZURE_OPENAI_ENDPOINT").is_ok(),
                    },
                ],
                default_models: DefaultModels {
                    chat_model: "gpt-4o".to_string(),
                    embedding_model: "text-embedding-3-small".to_string(),
                    embedding_dimension: 1536,
                },
            },
            ProviderInfo {
                id: "mock".to_string(),
                name: "Mock".to_string(),
                description: "Mock provider for testing (no API calls)".to_string(),
                available: true, // Mock is always available
                config_requirements: vec![],
                default_models: DefaultModels {
                    chat_model: "mock-gpt-4".to_string(),
                    embedding_model: "mock-embedding".to_string(),
                    embedding_dimension: 1536,
                },
            },
        ];

        // Embedding providers share the same options
        let embedding_providers = llm_providers.clone();

        Self {
            llm_providers,
            embedding_providers,
            active_llm_provider: active_llm.to_string(),
            active_embedding_provider: active_embedding.to_string(),
        }
    }
}

impl ProviderStatusResponse {
    /// Create a new provider status response from AppState
    pub fn from_app_state(app_state: &crate::state::AppState) -> Self {
        use chrono::Utc;

        // Get LLM provider info
        let llm_name = app_state.llm_provider.name().to_string();
        let llm_model = app_state.llm_provider.model().to_string();

        // Get embedding provider info
        let emb_name = app_state.embedding_provider.name().to_string();
        let emb_model = app_state.embedding_provider.model().to_string();
        let emb_dim = app_state.embedding_provider.dimension();

        // Get storage info
        let storage_dim = app_state.vector_storage.dimension();
        let storage_namespace = app_state.vector_storage.namespace();

        // Detect storage type using storage_mode field
        let storage_type = app_state.storage_mode.as_str();

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
