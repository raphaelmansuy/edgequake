//! Models API type definitions.
//!
//! DTOs for the models configuration API endpoints.
//!
//! # Implements
//!
//! - **FEAT0470**: Models Configuration API
//! - **FEAT0471**: Provider Capability Exposure
//!
//! # Response Types
//!
//! - [`ModelsListResponse`]: All available providers and models
//! - [`ProviderResponse`]: Provider details with models
//! - [`ModelResponse`]: Individual model card
//! - [`ProviderHealthResponse`]: Provider runtime health status

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Response for listing all models and providers.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModelsListResponse {
    /// All configured providers with their models.
    pub providers: Vec<ProviderResponse>,

    /// Default LLM provider name.
    pub default_llm_provider: String,

    /// Default LLM model name.
    pub default_llm_model: String,

    /// Default embedding provider name.
    pub default_embedding_provider: String,

    /// Default embedding model name.
    pub default_embedding_model: String,
}

/// Provider information with models.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProviderResponse {
    /// Unique provider identifier.
    pub name: String,

    /// Human-readable display name.
    pub display_name: String,

    /// Provider type (openai, ollama, lmstudio, etc.).
    pub provider_type: String,

    /// Whether the provider is enabled in config.
    pub enabled: bool,

    /// Provider priority (lower = higher priority).
    pub priority: u32,

    /// Provider description.
    pub description: String,

    /// All models available from this provider.
    pub models: Vec<ModelResponse>,

    /// Runtime health status (None if not checked).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<ProviderHealthResponse>,
}

/// Individual model information (model card).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModelResponse {
    /// Unique model identifier.
    pub name: String,

    /// Human-readable display name.
    pub display_name: String,

    /// Model type: "llm", "embedding", or "multimodal".
    pub model_type: String,

    /// Model description.
    pub description: String,

    /// Whether the model is deprecated.
    pub deprecated: bool,

    /// Replacement model if deprecated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,

    /// Model capabilities.
    pub capabilities: ModelCapabilitiesResponse,

    /// Cost information.
    pub cost: ModelCostResponse,

    /// Optional tags for categorization.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Model capabilities information.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModelCapabilitiesResponse {
    /// Maximum context length in tokens.
    pub context_length: usize,

    /// Maximum output tokens.
    pub max_output_tokens: usize,

    /// Supports vision/image input.
    pub supports_vision: bool,

    /// Supports function calling.
    pub supports_function_calling: bool,

    /// Supports JSON mode output.
    pub supports_json_mode: bool,

    /// Supports streaming responses.
    pub supports_streaming: bool,

    /// Supports system message.
    pub supports_system_message: bool,

    /// Embedding dimension (0 for non-embedding models).
    pub embedding_dimension: usize,
}

/// Model cost information.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModelCostResponse {
    /// Cost per 1K input tokens (USD).
    pub input_per_1k: f64,

    /// Cost per 1K output tokens (USD).
    pub output_per_1k: f64,

    /// Cost per 1K embedding tokens (USD).
    pub embedding_per_1k: f64,

    /// Cost per image processing unit (USD).
    pub image_per_unit: f64,
}

/// Provider health check response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProviderHealthResponse {
    /// Is the provider reachable.
    pub available: bool,

    /// Response latency in milliseconds.
    pub latency_ms: u64,

    /// Error message if unavailable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Last check timestamp (ISO 8601).
    pub checked_at: String,
}

/// Response for LLM-only models.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LlmModelsResponse {
    /// All LLM models across all providers.
    pub models: Vec<LlmModelItem>,

    /// Default provider name.
    pub default_provider: String,

    /// Default model name.
    pub default_model: String,
}

/// LLM model item with provider info.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LlmModelItem {
    /// Provider name.
    pub provider: String,

    /// Provider display name.
    pub provider_display_name: String,

    /// Model details.
    #[serde(flatten)]
    pub model: ModelResponse,
}

/// Response for embedding-only models.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EmbeddingModelsResponse {
    /// All embedding models across all providers.
    pub models: Vec<EmbeddingModelItem>,

    /// Default provider name.
    pub default_provider: String,

    /// Default model name.
    pub default_model: String,
}

/// Embedding model item with provider info.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EmbeddingModelItem {
    /// Provider name.
    pub provider: String,

    /// Provider display name.
    pub provider_display_name: String,

    /// Embedding dimension.
    pub dimension: usize,

    /// Model details.
    #[serde(flatten)]
    pub model: ModelResponse,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_models_list_response_serialization() {
        let response = ModelsListResponse {
            providers: vec![],
            default_llm_provider: "openai".to_string(),
            default_llm_model: "gpt-4o".to_string(),
            default_embedding_provider: "openai".to_string(),
            default_embedding_model: "text-embedding-3-small".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("openai"));
        assert!(json.contains("gpt-4o"));
    }
}
