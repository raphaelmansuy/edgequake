//! Settings-related API handlers.
//!
//! @implements SPEC-032: Ollama/LM Studio provider support - Status API
//! @iteration OODA Loop #5 - Phase 5E.3

use axum::{extract::State, Json};

use crate::{error::ApiError, provider_types::ProviderStatusResponse, state::AppState};

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
}
