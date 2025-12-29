//! Health check handlers.

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::ApiResult;
use crate::state::AppState;

/// Health check response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    /// Service status.
    pub status: String,

    /// Service version.
    pub version: String,

    /// Storage mode: "memory" or "postgresql".
    pub storage_mode: String,

    /// Workspace ID.
    pub workspace_id: String,

    /// Component health.
    pub components: ComponentHealth,

    /// LLM provider name (e.g., "openai", "mock", "ollama").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_provider_name: Option<String>,
}

/// Component health status.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ComponentHealth {
    /// KV storage status.
    pub kv_storage: bool,

    /// Vector storage status.
    pub vector_storage: bool,

    /// Graph storage status.
    pub graph_storage: bool,

    /// LLM provider status.
    pub llm_provider: bool,
}

/// Health check endpoint.
#[utoipa::path(
    get,
    path = "/health",
    tag = "Health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    )
)]
pub async fn health_check(State(state): State<AppState>) -> ApiResult<Json<HealthResponse>> {
    let components = ComponentHealth {
        kv_storage: state.kv_storage.count().await.is_ok(),
        vector_storage: state.vector_storage.count().await.is_ok(),
        graph_storage: state.graph_storage.node_count().await.is_ok(),
        llm_provider: true, // Assume available, actual check would require API call
    };

    // Get the LLM provider name from the configured provider
    let llm_provider_name = Some(state.llm_provider.name().to_string());

    let response = HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        storage_mode: state.storage_mode.as_str().to_string(),
        workspace_id: state.config.workspace_id.clone(),
        components,
        llm_provider_name,
    };

    Ok(Json(response))
}

/// Readiness check (for Kubernetes).
#[utoipa::path(
    get,
    path = "/ready",
    tag = "Health",
    responses(
        (status = 200, description = "Service is ready")
    )
)]
pub async fn readiness_check() -> &'static str {
    "OK"
}

/// Liveness check (for Kubernetes).
#[utoipa::path(
    get,
    path = "/live",
    tag = "Health",
    responses(
        (status = 200, description = "Service is alive")
    )
)]
pub async fn liveness_check() -> &'static str {
    "OK"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let state = AppState::test_state();
        let result = health_check(State(state)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert_eq!(response.status, "healthy");
        assert_eq!(response.storage_mode, "memory"); // test_state uses memory
    }
}
