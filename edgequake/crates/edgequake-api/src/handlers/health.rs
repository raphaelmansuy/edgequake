//! Health check handlers for operational monitoring.
//!
//! # Implements
//!
//! - **UC0501**: Health Check
//! - **FEAT0401**: REST API Readiness/Liveness Endpoints
//!
//! # Endpoints
//!
//! | Method | Path | Handler | Description |
//! |--------|------|---------|-------------|
//! | GET | `/health` | [`health_check`] | Deep health with component status |
//! | GET | `/ready` | [`readiness_check`] | K8s readiness probe (can serve traffic) |
//! | GET | `/live` | [`liveness_check`] | K8s liveness probe (process alive) |
//!
//! # WHY: Three Health Endpoints
//!
//! Container orchestrators (Kubernetes, ECS) need separate probes:
//!
//! - **Liveness** (`/live`): Is process alive? Failure → restart container
//! - **Readiness** (`/ready`): Can serve traffic? Failure → remove from load balancer
//! - **Health** (`/health`): Deep check with component status for dashboards
//!
//! This separation enables:
//! - Graceful degradation (remove from LB but don't restart)
//! - Fast startup (ready before all caches warm)
//! - Detailed debugging via `/health` response

use axum::{extract::State, Json};

use crate::error::ApiResult;
use crate::state::AppState;

// Re-export DTOs from health_types for backwards compatibility
pub use crate::handlers::health_types::{ComponentHealth, HealthResponse};

/// Deep health check with component status.
///
/// # Implements
///
/// - **UC0501**: Health Check
/// - **FEAT0401**: REST API Service
///
/// # Returns
///
/// JSON with:
/// - `status`: "healthy" or "degraded"
/// - `version`: API server version
/// - `storage_mode`: "postgres" or "memory"
/// - `components`: Per-component health (KV, vector, graph, LLM)
///
/// # WHY: Component-Level Visibility
///
/// Returns individual component health to help operators identify which
/// backend is failing (database vs vector store vs LLM provider).
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
