//! Axum `FromRef` extractors for AppState runtime bundles (SPEC-017 API-SOLID-I-001).
//!
//! Handlers that only need a subset of [`AppState`] can request `State(StorageRuntime)` (etc.)
//! instead of the full state — ascending-compat: existing `State(AppState)` handlers unchanged.

use std::sync::Arc;

use axum::extract::FromRef;
use edgequake_core::ResourceBudgetConfig;

use super::{
    ApiSecurityConfig, AppConfig, AppState, AuthRuntime, ComplianceRuntime, GraphQueryRuntime,
    PostgresRuntime, QueryRuntime, StorageRuntime, TaskRuntime,
};
use edgequake_auth::extractors::AuthState;

use crate::path_validation::PathValidationConfig;

impl FromRef<AppState> for StorageRuntime {
    fn from_ref(state: &AppState) -> Self {
        state.storage.clone()
    }
}

impl FromRef<AppState> for QueryRuntime {
    fn from_ref(state: &AppState) -> Self {
        state.query.clone()
    }
}

impl FromRef<AppState> for AuthRuntime {
    fn from_ref(state: &AppState) -> Self {
        state.auth.clone()
    }
}

impl FromRef<AppState> for TaskRuntime {
    fn from_ref(state: &AppState) -> Self {
        state.tasks.clone()
    }
}

impl FromRef<AppState> for GraphQueryRuntime {
    fn from_ref(state: &AppState) -> Self {
        GraphQueryRuntime::from_parts(
            Arc::clone(&state.graph_materialize),
            state.resource_budget().clone(),
        )
    }
}

impl FromRef<AppState> for AuthState {
    fn from_ref(state: &AppState) -> Self {
        AuthState {
            jwt_service: Arc::clone(&state.auth.jwt),
            rbac_service: Arc::clone(&state.auth.rbac),
        }
    }
}

/// SPEC-006 budget SSOT — extract without full [`AppState`] (pagination clamps, etc.).
impl FromRef<AppState> for ResourceBudgetConfig {
    fn from_ref(state: &AppState) -> Self {
        state.resource_budget().clone()
    }
}

impl FromRef<AppState> for ComplianceRuntime {
    fn from_ref(state: &AppState) -> Self {
        ComplianceRuntime {
            audit_logger: state.audit_logger.clone(),
        }
    }
}

impl FromRef<AppState> for PathValidationConfig {
    fn from_ref(state: &AppState) -> Self {
        state.path_validation_config.clone()
    }
}

impl FromRef<AppState> for AppConfig {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}

#[cfg(feature = "postgres")]
impl FromRef<AppState> for PostgresRuntime {
    fn from_ref(state: &AppState) -> Self {
        PostgresRuntime {
            pool: state.pg_pool.clone(),
            capabilities: state.postgres_capabilities.clone(),
        }
    }
}

#[cfg(not(feature = "postgres"))]
impl FromRef<AppState> for PostgresRuntime {
    fn from_ref(_state: &AppState) -> Self {
        PostgresRuntime
    }
}

impl FromRef<AppState> for ApiSecurityConfig {
    fn from_ref(state: &AppState) -> Self {
        state.security.clone()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[tokio::test]
    async fn from_ref_clones_storage_bundle() {
        let state = AppState::test_state();
        let storage = StorageRuntime::from_ref(&state);
        assert_eq!(storage.mode, state.storage.mode);
        assert!(Arc::ptr_eq(&storage.kv_storage, &state.storage.kv_storage));
    }

    #[tokio::test]
    async fn from_ref_clones_query_bundle() {
        let state = AppState::test_state();
        let query = QueryRuntime::from_ref(&state);
        assert_eq!(query.llm_provider.name(), state.query.llm_provider.name());
    }

    #[tokio::test]
    async fn from_ref_clones_auth_and_task_bundles() {
        let state = AppState::test_state();
        let auth = AuthRuntime::from_ref(&state);
        assert_eq!(auth.config.auth_enabled, state.auth.config.auth_enabled);
        let tasks = TaskRuntime::from_ref(&state);
        assert!(Arc::ptr_eq(&tasks.queue, &state.tasks.queue));
    }

    #[tokio::test]
    async fn from_ref_resource_budget_matches_app_state() {
        let state = AppState::test_state();
        let budget = ResourceBudgetConfig::from_ref(&state);
        assert_eq!(budget.max_page_size, state.resource_budget().max_page_size);
    }

    #[tokio::test]
    async fn from_ref_clones_graph_query_runtime() {
        let state = AppState::test_state();
        let graph = GraphQueryRuntime::from_ref(&state);
        assert_eq!(
            graph.budget.graph_query_timeout_secs,
            state.resource_budget().graph_query_timeout_secs
        );
        assert!(Arc::ptr_eq(&graph.materialize, &state.graph_materialize));
    }
}
