//! Settings-related API handlers.
//!
//! @implements SPEC-032: Ollama/LM Studio provider support - Status API
//! @iteration OODA Loop #5 - Phase 5E.3 + OODA 12

use axum::{extract::State, Json};

use crate::{
    error::ApiError,
    provider_types::{AvailableProvidersResponse, ProviderStatusResponse},
    state::AppState,
};

/// Get current provider status
///
/// Returns detailed information about the currently active LLM provider,
/// embedding provider, and vector storage configuration.
pub async fn get_provider_status(
    State(app_state): State<AppState>,
) -> Result<Json<ProviderStatusResponse>, ApiError> {
    // Create status response from current AppState
    let status = ProviderStatusResponse::from_app_state(&app_state);

    tracing::debug!(
        provider = %status.provider.name,
        embedding_dim = %status.embedding.dimension,
        storage_dim = %status.storage.dimension,
        dimension_mismatch = %status.storage.dimension_mismatch,
        "Provider status requested"
    );

    Ok(Json(status))
}

/// List all available providers
///
/// Returns information about all supported LLM and embedding providers,
/// including their availability status based on environment configuration.
///
/// # Response
///
/// Returns [`AvailableProvidersResponse`] with:
/// - `llm_providers`: List of available LLM providers
/// - `embedding_providers`: List of available embedding providers
/// - `active_llm_provider`: Currently active LLM provider name
/// - `active_embedding_provider`: Currently active embedding provider name
///
/// # Example
///
/// ```json
/// {
///   "llm_providers": [
///     {
///       "id": "openai",
///       "name": "OpenAI",
///       "available": true,
///       "default_models": { "chat_model": "gpt-4o-mini", ... }
///     },
///     ...
///   ],
///   "active_llm_provider": "openai",
///   "active_embedding_provider": "openai"
/// }
/// ```
pub async fn list_available_providers(
    State(app_state): State<AppState>,
) -> Result<Json<AvailableProvidersResponse>, ApiError> {
    let active_llm = app_state.llm_provider.name();
    let active_embedding = app_state.embedding_provider.name();

    let response = AvailableProvidersResponse::build(active_llm, active_embedding);

    tracing::debug!(
        llm_count = response.llm_providers.len(),
        embedding_count = response.embedding_providers.len(),
        active_llm = %active_llm,
        active_embedding = %active_embedding,
        "Available providers listed"
    );

    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_provider_status_structure() {
        // Setup: Create AppState with mock provider
        let app_state = AppState::new_memory(None::<String>);

        // Act: Call handler
        let result = get_provider_status(State(app_state)).await;

        // Assert: Success
        assert!(result.is_ok());

        let Json(status) = result.unwrap();

        // Assert: Response structure
        assert!(!status.provider.name.is_empty());
        assert_eq!(status.provider.provider_type, "llm");
        assert!(!status.embedding.model.is_empty());
        assert!(status.embedding.dimension > 0);
    }

    #[tokio::test]
    async fn test_list_available_providers() {
        // Setup: Create AppState with mock provider
        let app_state = AppState::new_memory(None::<String>);

        // Act: Call handler
        let result = list_available_providers(State(app_state)).await;

        // Assert: Success
        assert!(result.is_ok());

        let Json(response) = result.unwrap();

        // Assert: Has all providers
        assert!(response.llm_providers.len() >= 4); // openai, ollama, lmstudio, mock
        assert!(response.embedding_providers.len() >= 4);

        // Assert: Provider IDs
        let ids: Vec<_> = response
            .llm_providers
            .iter()
            .map(|p| p.id.as_str())
            .collect();
        assert!(ids.contains(&"openai"));
        assert!(ids.contains(&"ollama"));
        assert!(ids.contains(&"lmstudio"));
        assert!(ids.contains(&"mock"));

        // Assert: Mock is always available
        let mock = response
            .llm_providers
            .iter()
            .find(|p| p.id == "mock")
            .unwrap();
        assert!(mock.available);
        assert_eq!(mock.default_models.embedding_dimension, 1536);

        // Assert: LM Studio defaults
        let lmstudio = response
            .llm_providers
            .iter()
            .find(|p| p.id == "lmstudio")
            .unwrap();
        assert_eq!(lmstudio.default_models.chat_model, "gemma-3n-e4b-it");
        assert_eq!(
            lmstudio.default_models.embedding_model,
            "nomic-embed-text-v1.5"
        );
        assert_eq!(lmstudio.default_models.embedding_dimension, 768);
    }
}
