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

use axum::{extract::State, http::StatusCode, Json};
use std::sync::Arc;

use crate::error::ApiResult;
use crate::state::AppState;

// Re-export DTOs from health_types for backwards compatibility
pub use crate::handlers::health_types::{
    ApiCapabilities, BuildInfo, ComponentHealth, EmbeddingProviderHealth, HealthResponse,
    IngestionHealthSnapshot, LlmProviderHealth, MigrationHealthSnapshot,
    ObservabilityHealthSnapshot, OperationalHealth, ProvidersHealth, QueryEngineHealthSnapshot,
    ReadModelHealthSnapshot, SchemaHealth, SourceIdsIndexHealth, StorageHealthSnapshot,
    TaskQueueHealthSnapshot,
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
    // SPEC-021 P-G13: bounded pings — never block on pool acquire during ingestion bursts.
    let (kv_ok, vector_ok, graph_ok) = super::health_probes::probe_storage_components(
        Arc::clone(&state.storage.kv_storage),
        Arc::clone(&state.storage.vector_storage),
        Arc::clone(&state.storage.graph_storage),
    )
    .await;
    let components = ComponentHealth {
        kv_storage: kv_ok,
        vector_storage: vector_ok,
        graph_storage: graph_ok,
        llm_provider: true, // Assume available, actual check would require API call
    };

    // Get the LLM provider name from the configured provider
    let llm_provider_name = Some(state.query.llm_provider.name().to_string());

    // Query schema health (PostgreSQL only)
    // WHY: OODA-14 - Mission requires schema version verification
    let schema = get_schema_health(&state).await;
    let storage_degraded = !kv_ok || !vector_ok || !graph_ok;

    let operational = build_operational_health(&state).await;
    let queue_overloaded = operational.as_ref().is_some_and(|op| {
        crate::task_queue_pressure::health_degraded_by_queue(op.task_queue.pending)
    });

    let status = if schema
        .as_ref()
        .and_then(|s| s.source_ids_indexes.as_ref())
        .is_some_and(|m| !m.ready)
        || storage_degraded
        || queue_overloaded
    {
        "degraded"
    } else {
        "healthy"
    };

    // WHY: OODA-11 - Mission requirement: "know all parts of the applied configuration
    // (llm provider, embedding provider, models used)".
    // Operators need full visibility to debug ingestion/query issues.
    let providers = Some(ProvidersHealth {
        llm: LlmProviderHealth {
            name: state.query.llm_provider.name().to_string(),
            model: state.query.llm_provider.model().to_string(),
        },
        embedding: EmbeddingProviderHealth {
            name: state.query.embedding_provider.name().to_string(),
            model: state.query.embedding_provider.model().to_string(),
            dimension: state.query.embedding_provider.dimension(),
        },
    });

    // WHY: OODA-11 - PDF storage availability affects document upload success.
    // When false, PDF uploads will fail. Helps operators diagnose issues.
    #[cfg(feature = "postgres")]
    let pdf_storage_enabled = Some(state.storage.pdf_storage.is_some());
    #[cfg(not(feature = "postgres"))]
    let pdf_storage_enabled: Option<bool> = None;

    #[cfg(feature = "postgres")]
    let identity_policy = crate::services::identity_storage::IdentityPolicy::resolve(
        &state.security,
        state.pg_pool.is_some(),
    );
    #[cfg(not(feature = "postgres"))]
    let identity_policy =
        crate::services::identity_storage::IdentityPolicy::resolve(&state.security, false);

    let response = HealthResponse {
        status: status.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        build_info: Some(BuildInfo {
            git_hash: env!("EDGEQUAKE_GIT_HASH").to_string(),
            git_branch: env!("EDGEQUAKE_GIT_BRANCH").to_string(),
            build_timestamp: env!("EDGEQUAKE_BUILD_TIMESTAMP").to_string(),
            build_number: env!("EDGEQUAKE_BUILD_NUMBER").to_string(),
        }),
        storage_mode: state.storage.mode.as_str().to_string(),
        workspace_id: state.config.workspace_id.clone(),
        components,
        llm_provider_name,
        schema,
        providers,
        pdf_storage_enabled,
        operational,
        capabilities: Some(ApiCapabilities {
            openapi_url: "/api-docs/openapi.json".to_string(),
            asyncapi_url: "/api-docs/asyncapi.json".to_string(),
            swagger_ui_url: "/swagger-ui".to_string(),
            admin_api_prefix: "/api/v1/admin".to_string(),
            shared_conversations_prefix: "/api/v1/shared".to_string(),
            jobs_v2_prefix: "/api/v2/workspaces/{workspace_id}/jobs".to_string(),
            jobs_v2_catalog: "/api/v2/workspaces/{workspace_id}/jobs/catalog".to_string(),
            auth_identity_ssot: Some(identity_policy.identity_backend_label().to_string()),
            auth_enabled: Some(state.auth.config.auth_enabled),
            dev_mode: Some(state.auth.config.dev_mode),
            kv_identity_mirror_configured: Some(state.security.kv_identity_mirror),
            kv_identity_mirror_effective: Some(identity_policy.kv_mirror),
            auth_mechanisms: Some(state.auth.oidc_config.resolved_auth_mechanisms()),
            oauth2_oidc_builtin: Some(state.auth.oidc_config.is_runtime_builtin()),
            auth_kv_harness_active: Some(!identity_policy.pg_primary),
            external_sso_pattern: if state.auth.oidc_config.is_runtime_builtin() {
                Some("builtin-oidc".to_string())
            } else {
                Some(edgequake_auth::EXTERNAL_SSO_PATTERN.to_string())
            },
        }),
        attribution: Some(crate::attribution::health_attribution_summary()),
    };

    Ok(Json(response))
}

async fn build_operational_health(state: &AppState) -> Option<OperationalHealth> {
    use crate::task_queue_pressure::{assess_queue_pressure, publish_queue_observability};
    use edgequake_observability::{log_format_label, ObservabilityConfig};
    use edgequake_query::{
        fusion::{mix_fusion_mode_from_env, mix_fusion_mode_label},
        hybrid_merge::{hybrid_fusion_mode_from_env, hybrid_fusion_mode_label},
    };
    use edgequake_storage::{
        community_refresh_debounce_secs, pending_community_refresh_workspaces,
    };

    let task_stats = state
        .tasks
        .storage
        .get_statistics(edgequake_tasks::storage::TaskFilter::default())
        .await
        .ok()?;

    let obs_cfg = ObservabilityConfig::from_env();
    let engine = &state.query.engine_impl;

    #[cfg(feature = "postgres")]
    let relational_backfill_enabled = state.pg_pool.is_some();
    #[cfg(not(feature = "postgres"))]
    let relational_backfill_enabled = false;

    let pressure = assess_queue_pressure(task_stats.pending);
    publish_queue_observability(
        task_stats.pending,
        task_stats.processing,
        task_stats.failed,
        &pressure,
    );

    let community_scheduled = pending_community_refresh_workspaces().await as u64;

    Some(OperationalHealth {
        task_queue: TaskQueueHealthSnapshot {
            pending: task_stats.pending,
            processing: task_stats.processing,
            failed: task_stats.failed,
            pressure: pressure.level.as_str().to_string(),
            pending_warn_threshold: pressure.pending_warn_threshold,
            pending_critical_threshold: pressure.pending_critical_threshold,
            operator_action: pressure.operator_action.clone(),
        },
        query_engine: QueryEngineHealthSnapshot {
            default_mode: engine.config().default_mode.to_string(),
            reranker_configured: engine.has_reranker(),
            community_refresh_debounce_secs: community_refresh_debounce_secs(),
            hybrid_fusion: hybrid_fusion_mode_label(hybrid_fusion_mode_from_env()).to_string(),
            mix_fusion: mix_fusion_mode_label(mix_fusion_mode_from_env()).to_string(),
            community_refresh_scheduled_workspaces: community_scheduled,
        },
        observability: ObservabilityHealthSnapshot {
            log_format: log_format_label(obs_cfg.log_format).to_string(),
            otel_enabled: obs_cfg.otel_enabled,
        },
        read_model: ReadModelHealthSnapshot {
            merge_strategy: crate::document_read_model::MERGE_STRATEGY.to_string(),
            relational_backfill_enabled,
            entity_count_graph_reconcile: true,
        },
        migration: build_migration_health_snapshot(state),
        ingestion: IngestionHealthSnapshot {
            execution_model: "worker_queue".to_string(),
            persist_ssot: "IngestionPersister".to_string(),
            duplicate_reingest_enabled: true,
        },
        storage: build_storage_health_snapshot(state),
    })
}

#[cfg(feature = "postgres")]
fn build_storage_health_snapshot(state: &AppState) -> StorageHealthSnapshot {
    let mut snap = StorageHealthSnapshot {
        chunk_text_ssot: "kv".to_string(),
        vector_metadata_ref: "content_ref".to_string(),
        chunk_kv_in_persister: true,
        vector_storage_mode: None,
        document_id_generator: None,
        age_rls_enabled: None,
        age_copy_loader_enabled: None,
    };
    if let Some(caps) = state.postgres_capabilities.as_ref() {
        snap.vector_storage_mode = Some(caps.vector_storage_mode.as_str().to_string());
        snap.document_id_generator = Some(caps.document_id_generator.as_str().to_string());
        snap.age_rls_enabled = Some(caps.age_rls_effective);
        snap.age_copy_loader_enabled = Some(caps.age_copy_loader_effective);
    }
    snap
}

#[cfg(not(feature = "postgres"))]
fn build_storage_health_snapshot(_state: &AppState) -> StorageHealthSnapshot {
    StorageHealthSnapshot {
        chunk_text_ssot: "kv".to_string(),
        vector_metadata_ref: "content_ref".to_string(),
        chunk_kv_in_persister: true,
        vector_storage_mode: None,
        document_id_generator: None,
        age_rls_enabled: None,
        age_copy_loader_enabled: None,
    }
}

#[cfg(feature = "postgres")]
fn build_migration_health_snapshot(state: &AppState) -> Option<MigrationHealthSnapshot> {
    let report = state.migration_bootstrap.as_ref()?;
    Some(MigrationHealthSnapshot {
        latest_version: report.latest_version,
        source_ids_indexes_ready: report.migration_038.indexes_ready,
        pgvector_extversion: report.migration_042.extversion_after.clone(),
        pgvector_shipped_version: report.migration_042.shipped_extversion.clone(),
        pgvector_iterative_scan_capable: report.migration_042.iterative_scan_capable,
        age_extversion: report.migration_043.extversion_after.clone(),
        age_shipped_version: report.migration_043.shipped_extversion.clone(),
        ready_for_traffic: crate::state::migration_bootstrap::is_ready_for_traffic(
            &state.migration_bootstrap,
        ),
    })
}

#[cfg(not(feature = "postgres"))]
fn build_migration_health_snapshot(_state: &AppState) -> Option<MigrationHealthSnapshot> {
    None
}

/// Query database schema health from _sqlx_migrations table.
///
/// Returns None for memory mode or if query fails (graceful degradation).
#[allow(unused_variables)] // state unused when postgres feature disabled
async fn get_schema_health(state: &AppState) -> Option<SchemaHealth> {
    #[cfg(feature = "postgres")]
    {
        let pool = state.pg_pool.as_ref()?;

        // WHY: Global ops table — see `services/health_schema.rs` (no tenant RLS).
        let stats = crate::services::health_schema::fetch_sqlx_migration_stats(pool).await?;

        let source_ids_indexes = state.migration_bootstrap.as_ref().map(|report| {
            let m = &report.migration_038;
            crate::handlers::health_types::SourceIdsIndexHealth {
                ready: m.indexes_ready,
                graphs_checked: m.graphs_checked,
                indexes_repaired_at_bootstrap: m.indexes_repaired_inline,
                deferred_large_graphs: if m.deferred_large_graphs.is_empty() {
                    None
                } else {
                    Some(m.deferred_large_graphs.clone())
                },
                missing_indexes: if m.missing_indexes.is_empty() {
                    None
                } else {
                    Some(m.missing_indexes.clone())
                },
                operator_action: m.operator_action.clone(),
            }
        });

        Some(SchemaHealth {
            latest_version: stats.latest_version,
            migrations_applied: stats.applied_count as usize,
            last_applied_at: stats.last_applied_at.map(|dt| dt.to_rfc3339()),
            source_ids_indexes,
        })
    }

    #[cfg(not(feature = "postgres"))]
    {
        None
    }
}

/// Readiness check (for Kubernetes).
///
/// Returns 503 when migration 038 indexes are required but missing on a large graph
/// (AGE present, indexes not ready). Prevents routing traffic to slow-prefix nodes.
#[utoipa::path(
    get,
    path = "/ready",
    tag = "Health",
    responses(
        (status = 200, description = "Service is ready"),
        (status = 503, description = "Migration 038 indexes pending — not ready for traffic")
    )
)]
pub async fn readiness_check(State(state): State<AppState>) -> StatusCode {
    #[cfg(feature = "postgres")]
    {
        if crate::state::migration_bootstrap::is_ready_for_traffic(&state.migration_bootstrap) {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        }
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = state;
        StatusCode::OK
    }
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
