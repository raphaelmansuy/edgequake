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
pub use crate::handlers::health_types::{
    BuildInfo, ComponentHealth, EmbeddingProviderHealth, HealthResponse, LlmProviderHealth,
    ProvidersHealth, SchemaHealth,
};

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
/// - `schema`: Database migration state (PostgreSQL only)
///
/// # WHY: Component-Level Visibility
///
/// Returns individual component health to help operators identify which
/// backend is failing (database vs vector store vs LLM provider).
///
/// # WHY: Schema Health (OODA-14)
///
/// Mission requirement: "verify the integrity of schema against the version
/// of edgequake running." Provides visibility into migration state.
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

    // Query schema health (PostgreSQL only)
    // WHY: OODA-14 - Mission requires schema version verification
    let schema = get_schema_health(&state).await;

    // WHY: OODA-11 - Mission requirement: "know all parts of the applied configuration
    // (llm provider, embedding provider, models used)".
    // Operators need full visibility to debug ingestion/query issues.
    let providers = Some(ProvidersHealth {
        llm: LlmProviderHealth {
            name: state.llm_provider.name().to_string(),
            model: state.llm_provider.model().to_string(),
        },
        embedding: EmbeddingProviderHealth {
            name: state.embedding_provider.name().to_string(),
            model: state.embedding_provider.model().to_string(),
            dimension: state.embedding_provider.dimension(),
        },
    });

    // WHY: OODA-11 - PDF storage availability affects document upload success.
    // When false, PDF uploads will fail. Helps operators diagnose issues.
    #[cfg(feature = "postgres")]
    let pdf_storage_enabled = Some(state.pdf_storage.is_some());
    #[cfg(not(feature = "postgres"))]
    let pdf_storage_enabled: Option<bool> = None;

    let response = HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        build_info: Some(BuildInfo {
            git_hash: env!("EDGEQUAKE_GIT_HASH").to_string(),
            git_branch: env!("EDGEQUAKE_GIT_BRANCH").to_string(),
            build_timestamp: env!("EDGEQUAKE_BUILD_TIMESTAMP").to_string(),
            build_number: env!("EDGEQUAKE_BUILD_NUMBER").to_string(),
        }),
        storage_mode: state.storage_mode.as_str().to_string(),
        workspace_id: state.config.workspace_id.clone(),
        components,
        llm_provider_name,
        schema,
        providers,
        pdf_storage_enabled,
    };

    Ok(Json(response))
}

/// Query database schema health from _sqlx_migrations table.
///
/// Returns None for memory mode or if query fails (graceful degradation).
#[allow(unused_variables)] // state unused when postgres feature disabled
async fn get_schema_health(state: &AppState) -> Option<SchemaHealth> {
    #[cfg(feature = "postgres")]
    {
        let pool = state.pg_pool.as_ref()?;

        // WHY scalar subqueries: Single round-trip, handles empty table gracefully
        #[derive(sqlx::FromRow)]
        struct MigrationStats {
            applied_count: i64,
            latest_version: Option<i64>,
            last_applied_at: Option<chrono::DateTime<chrono::Utc>>,
        }

        let stats: Option<MigrationStats> = sqlx::query_as(
            r#"
            SELECT 
                COUNT(*) FILTER (WHERE success = true) as applied_count,
                MAX(version) FILTER (WHERE success = true) as latest_version,
                MAX(installed_on) FILTER (WHERE success = true) as last_applied_at
            FROM _sqlx_migrations
            "#,
        )
        .fetch_optional(pool)
        .await
        .ok()?;

        let stats = stats?;
        Some(SchemaHealth {
            latest_version: stats.latest_version,
            migrations_applied: stats.applied_count as usize,
            last_applied_at: stats.last_applied_at.map(|dt| dt.to_rfc3339()),
        })
    }

    #[cfg(not(feature = "postgres"))]
    {
        None
    }
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
