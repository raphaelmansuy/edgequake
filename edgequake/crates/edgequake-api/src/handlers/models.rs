//! Models API handlers for configuration and capability discovery.
//!
//! # Implements
//!
//! - **FEAT0470**: Models Configuration API
//! - **FEAT0471**: Provider Capability Exposure
//! - **UC2070**: Query available LLM/embedding models
//!
//! # Endpoints
//!
//! | Method | Path | Handler | Description |
//! |--------|------|---------|-------------|
//! | GET | `/api/models` | [`list_models`] | List all providers and models |
//! | GET | `/api/models/llm` | [`list_llm_models`] | List LLM models only |
//! | GET | `/api/models/embedding` | [`list_embedding_models`] | List embedding models only |
//! | GET | `/api/models/{provider}` | [`get_provider`] | Get provider details |
//! | GET | `/api/models/{provider}/{model}` | [`get_model`] | Get specific model card |
//! | GET | `/api/models/health` | [`check_providers_health`] | Check provider availability |
//!
//! # WHY: Models Configuration API
//!
//! The frontend needs to know:
//! - Which LLM providers are available
//! - What models each provider supports
//! - Model capabilities (vision, function calling, context length)
//! - Cost per token for billing estimates
//! - Which providers are currently reachable
//!
//! This enables the WebUI to display intelligent model selection with
//! capability tooltips and cost estimates.

use axum::{extract::State, Json};
use edgequake_llm::model_config::ProviderType;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

// Re-export DTOs from models_types
pub use crate::handlers::models_types::{
    EmbeddingModelItem, EmbeddingModelsResponse, LlmModelItem, LlmModelsResponse,
    ModelCapabilitiesResponse, ModelCostResponse, ModelResponse, ModelsListResponse,
    ProviderHealthResponse, ProviderResponse,
};

/// Convert a ModelCard to a ModelResponse DTO.
fn model_card_to_response(card: &edgequake_llm::ModelCard) -> ModelResponse {
    ModelResponse {
        name: card.name.clone(),
        display_name: card.display_name.clone(),
        model_type: card.model_type.to_string(),
        description: card.description.clone(),
        deprecated: card.deprecated,
        replacement: card.replacement.clone(),
        capabilities: ModelCapabilitiesResponse {
            context_length: card.capabilities.context_length,
            max_output_tokens: card.capabilities.max_output_tokens,
            supports_vision: card.capabilities.supports_vision,
            supports_function_calling: card.capabilities.supports_function_calling,
            supports_json_mode: card.capabilities.supports_json_mode,
            supports_streaming: card.capabilities.supports_streaming,
            supports_system_message: card.capabilities.supports_system_message,
            embedding_dimension: card.capabilities.embedding_dimension,
        },
        cost: ModelCostResponse {
            input_per_1k: card.cost.input_per_1k,
            output_per_1k: card.cost.output_per_1k,
            embedding_per_1k: card.cost.embedding_per_1k,
            image_per_unit: card.cost.image_per_unit,
        },
        tags: card.tags.clone(),
    }
}

/// Convert a ProviderConfig to a ProviderResponse DTO.
fn provider_to_response(provider: &edgequake_llm::ProviderConfig) -> ProviderResponse {
    ProviderResponse {
        name: provider.name.clone(),
        display_name: provider.display_name.clone(),
        provider_type: provider.provider_type.to_string(),
        enabled: provider.enabled,
        priority: provider.priority,
        description: provider.description.clone(),
        models: provider.models.iter().map(model_card_to_response).collect(),
        health: None, // Set separately during health check
    }
}

/// List all configured providers and models.
///
/// # Implements
///
/// - **FEAT0470**: Models Configuration API
/// - **UC2070**: Query available models
///
/// # Returns
///
/// All providers with their models and default selections.
///
/// # WHY: Single Endpoint for Model Discovery
///
/// Returns everything the frontend needs in one call:
/// - All providers with their models
/// - Default selections for LLM and embedding
/// - Enables smart dropdown population with grouping
#[utoipa::path(
    get,
    path = "/api/models",
    tag = "Models",
    responses(
        (status = 200, description = "List of all providers and models", body = ModelsListResponse)
    )
)]
pub async fn list_models(State(state): State<AppState>) -> ApiResult<Json<ModelsListResponse>> {
    let config = &*state.models_config;

    let providers: Vec<ProviderResponse> =
        config.providers.iter().map(provider_to_response).collect();

    Ok(Json(ModelsListResponse {
        providers,
        default_llm_provider: config.defaults.llm_provider.clone(),
        default_llm_model: config.defaults.llm_model.clone(),
        default_embedding_provider: config.defaults.embedding_provider.clone(),
        default_embedding_model: config.defaults.embedding_model.clone(),
    }))
}

/// List LLM models only.
///
/// # Implements
///
/// - **FEAT0470**: Models Configuration API
///
/// # Returns
///
/// All LLM and multimodal models across all enabled providers.
#[utoipa::path(
    get,
    path = "/api/models/llm",
    tag = "Models",
    responses(
        (status = 200, description = "List of LLM models", body = LlmModelsResponse)
    )
)]
pub async fn list_llm_models(State(state): State<AppState>) -> ApiResult<Json<LlmModelsResponse>> {
    let config = &*state.models_config;

    let models: Vec<LlmModelItem> = config
        .all_llm_models()
        .into_iter()
        .map(|(provider, model)| LlmModelItem {
            provider: provider.name.clone(),
            provider_display_name: provider.display_name.clone(),
            model: model_card_to_response(model),
        })
        .collect();

    Ok(Json(LlmModelsResponse {
        models,
        default_provider: config.defaults.llm_provider.clone(),
        default_model: config.defaults.llm_model.clone(),
    }))
}

/// List embedding models only.
///
/// # Implements
///
/// - **FEAT0470**: Models Configuration API
///
/// # Returns
///
/// All embedding and multimodal models across all enabled providers.
#[utoipa::path(
    get,
    path = "/api/models/embedding",
    tag = "Models",
    responses(
        (status = 200, description = "List of embedding models", body = EmbeddingModelsResponse)
    )
)]
pub async fn list_embedding_models(
    State(state): State<AppState>,
) -> ApiResult<Json<EmbeddingModelsResponse>> {
    let config = &*state.models_config;

    let models: Vec<EmbeddingModelItem> = config
        .all_embedding_models()
        .into_iter()
        .map(|(provider, model)| EmbeddingModelItem {
            provider: provider.name.clone(),
            provider_display_name: provider.display_name.clone(),
            dimension: model.capabilities.embedding_dimension,
            model: model_card_to_response(model),
        })
        .collect();

    Ok(Json(EmbeddingModelsResponse {
        models,
        default_provider: config.defaults.embedding_provider.clone(),
        default_model: config.defaults.embedding_model.clone(),
    }))
}

/// Get a specific provider by name.
///
/// # Implements
///
/// - **FEAT0470**: Models Configuration API
///
/// # Path Parameters
///
/// - `provider`: Provider name (e.g., "openai", "ollama")
///
/// # Returns
///
/// Provider details with all its models, or 404 if not found.
#[utoipa::path(
    get,
    path = "/api/models/{provider}",
    tag = "Models",
    params(
        ("provider" = String, Path, description = "Provider name")
    ),
    responses(
        (status = 200, description = "Provider details", body = ProviderResponse),
        (status = 404, description = "Provider not found")
    )
)]
pub async fn get_provider(
    State(state): State<AppState>,
    axum::extract::Path(provider_name): axum::extract::Path<String>,
) -> ApiResult<Json<ProviderResponse>> {
    let config = &*state.models_config;

    let provider = config
        .get_provider(&provider_name)
        .ok_or_else(|| ApiError::NotFound(format!("Provider '{}' not found", provider_name)))?;

    Ok(Json(provider_to_response(provider)))
}

/// Get a specific model by provider and model name.
///
/// # Implements
///
/// - **FEAT0470**: Models Configuration API
///
/// # Path Parameters
///
/// - `provider`: Provider name (e.g., "openai")
/// - `model`: Model name (e.g., "gpt-4o")
///
/// # Returns
///
/// Model card with capabilities and cost, or 404 if not found.
#[utoipa::path(
    get,
    path = "/api/models/{provider}/{model}",
    tag = "Models",
    params(
        ("provider" = String, Path, description = "Provider name"),
        ("model" = String, Path, description = "Model name")
    ),
    responses(
        (status = 200, description = "Model details", body = ModelResponse),
        (status = 404, description = "Model not found")
    )
)]
pub async fn get_model(
    State(state): State<AppState>,
    axum::extract::Path((provider_name, model_name)): axum::extract::Path<(String, String)>,
) -> ApiResult<Json<ModelResponse>> {
    let config = &*state.models_config;

    let model = config
        .get_model(&provider_name, &model_name)
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "Model '{}' not found in provider '{}'",
                model_name, provider_name
            ))
        })?;

    Ok(Json(model_card_to_response(model)))
}

/// Check health of all enabled providers.
///
/// # Implements
///
/// - **FEAT0471**: Provider Capability Exposure
///
/// # Returns
///
/// All providers with their current health status.
///
/// # WHY: Runtime Health Checks
///
/// Configuration says what providers *should* be available.
/// Health checks confirm what *is* actually reachable.
/// This helps users understand why a model might not work.
#[utoipa::path(
    get,
    path = "/api/models/health",
    tag = "Models",
    responses(
        (status = 200, description = "Provider health status", body = Vec<ProviderResponse>)
    )
)]
pub async fn check_providers_health(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<ProviderResponse>>> {
    let config = &*state.models_config;
    let now = chrono::Utc::now().to_rfc3339();

    let mut providers: Vec<ProviderResponse> = Vec::new();

    for provider_config in &config.providers {
        if !provider_config.enabled {
            continue;
        }

        let mut provider_response = provider_to_response(provider_config);

        // Perform health check based on provider type
        let health = check_provider_health(provider_config, &now).await;
        provider_response.health = Some(health);

        providers.push(provider_response);
    }

    Ok(Json(providers))
}

/// Check health of a single provider.
///
/// Local providers (Ollama, LM Studio) are checked via TCP connection.
/// Cloud providers are verified by checking their API key environment variable.
/// Mock provider is always available.
async fn check_provider_health(
    provider: &edgequake_llm::ProviderConfig,
    checked_at: &str,
) -> ProviderHealthResponse {
    use std::time::Instant;

    match provider.provider_type {
        ProviderType::Mock => ProviderHealthResponse {
            available: true,
            latency_ms: 0,
            error: None,
            checked_at: checked_at.to_string(),
        },
        ProviderType::Ollama | ProviderType::LMStudio => {
            // Local providers: TCP connection check
            let start = Instant::now();
            let default_url = if provider.provider_type == ProviderType::Ollama {
                "http://localhost:11434"
            } else {
                "http://localhost:1234"
            };
            let base_url = provider.base_url.as_deref().unwrap_or(default_url);
            let host_port = base_url
                .strip_prefix("http://")
                .unwrap_or(base_url)
                .strip_prefix("https://")
                .unwrap_or(base_url);

            match std::net::TcpStream::connect_timeout(
                &host_port
                    .parse()
                    .unwrap_or_else(|_| "127.0.0.1:11434".parse().unwrap()),
                std::time::Duration::from_secs(2),
            ) {
                Ok(_) => ProviderHealthResponse {
                    available: true,
                    latency_ms: start.elapsed().as_millis() as u64,
                    error: None,
                    checked_at: checked_at.to_string(),
                },
                Err(e) => ProviderHealthResponse {
                    available: false,
                    latency_ms: start.elapsed().as_millis() as u64,
                    error: Some(format!("Connection failed: {}", e)),
                    checked_at: checked_at.to_string(),
                },
            }
        }
        _ => {
            // Cloud providers: check if API key is configured
            let api_key_set = provider
                .api_key_env
                .as_deref()
                .map(|env| !env.is_empty() && std::env::var(env).is_ok())
                .unwrap_or(false);

            ProviderHealthResponse {
                available: api_key_set,
                latency_ms: 0,
                error: if api_key_set {
                    None
                } else {
                    let env_hint = provider
                        .api_key_env
                        .as_deref()
                        .filter(|s| !s.is_empty())
                        .unwrap_or("API key");
                    Some(format!("{} not configured", env_hint))
                },
                checked_at: checked_at.to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_card_to_response() {
        let card = edgequake_llm::ModelCard {
            name: "test-model".to_string(),
            display_name: "Test Model".to_string(),
            model_type: edgequake_llm::ModelType::Llm,
            description: "A test model".to_string(),
            deprecated: false,
            replacement: None,
            capabilities: edgequake_llm::ModelCapabilities {
                context_length: 4096,
                max_output_tokens: 1024,
                supports_vision: false,
                supports_function_calling: true,
                ..Default::default()
            },
            cost: edgequake_llm::ModelCost {
                input_per_1k: 0.001,
                output_per_1k: 0.002,
                ..Default::default()
            },
            ..Default::default()
        };

        let response = model_card_to_response(&card);
        assert_eq!(response.name, "test-model");
        assert_eq!(response.capabilities.context_length, 4096);
        assert!(response.capabilities.supports_function_calling);
        assert!(!response.capabilities.supports_vision);
    }
}
