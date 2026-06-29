use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::time::Instant;
use uuid::Uuid;

use super::helpers::{
    verify_workspace_tenant_access, CachedStats, STATS_CACHE_TTL, WORKSPACE_STATS_CACHE,
};
use crate::error::ApiError;
use crate::handlers::workspaces_types::*;
use crate::middleware::TenantContext;
use crate::services::document_metadata_scan::load_workspace_metadata_values;
use crate::state::AppState;
use edgequake_core::MetricsTriggerType;

/// Get workspace statistics.
///
/// GET /api/v1/workspaces/{workspace_id}/stats
#[utoipa::path(
    get,
    path = "/api/v1/workspaces/{workspace_id}/stats",
    params(
        ("workspace_id" = Uuid, Path, description = "Workspace ID")
    ),
    responses(
        (status = 200, description = "Workspace statistics", body = WorkspaceStatsResponse),
        (status = 404, description = "Workspace not found"),
    ),
    tags = ["workspaces"]
)]
pub async fn get_workspace_stats(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    tenant_ctx: TenantContext,
) -> Result<Json<WorkspaceStatsResponse>, ApiError> {
    // BR0201: Verify workspace belongs to requesting tenant before serving
    // stats. Do this before the cache so cross-tenant requests never receive
    // cached data for workspaces they do not own.
    verify_workspace_tenant_access(&state, workspace_id, &tenant_ctx).await?;

    // HYBRID APPROACH WITH CACHING: 4-tier performance optimization
    // See: logs/2026-01-26-18-00-storage-architecture-analysis.md
    // See: specs/021-storage-study/06-first-principles/11-ux-zero-documents-root-cause-assessment.md
    //
    // Performance tiers:
    // 0. Cache (<1ms) - FASTEST, 60s TTL
    // 1. PostgreSQL documents table (1-5ms) - primary for document_count (SPEC-021 P5-01)
    // 2. KV storage aggregation (15ms) - fallback for legacy uploads + chunk metrics
    // 3. AGE graph queries (50-200ms) - authoritative for entity/relationship counts

    use std::time::Instant;
    let start = Instant::now();

    // Tier 0: Check cache first (fastest path - <1ms)
    {
        let cache = WORKSPACE_STATS_CACHE.read().await;
        if let Some(cached) = cache.get(&workspace_id) {
            if cached.cached_at.elapsed() < STATS_CACHE_TTL {
                let elapsed = start.elapsed();
                tracing::debug!(
                    workspace_id = %workspace_id,
                    duration_us = elapsed.as_micros(),
                    method = "cache",
                    age_secs = cached.cached_at.elapsed().as_secs(),
                    "Workspace stats retrieved from cache (fastest path)"
                );
                return Ok(Json(cached.stats.clone()));
            }
        }
    }

    // SPEC-021 P-G13: stale-if-error — under ingestion load, prefer last-known
    // stats over timing out (which makes the UI show 0 and triggers false
    // "backend unreachable" banners via React Query transport failures).
    let stale_fallback = {
        let cache = WORKSPACE_STATS_CACHE.read().await;
        cache.get(&workspace_id).map(|c| c.stats.clone())
    };

    const STATS_FETCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(4);

    let fetch_result = tokio::time::timeout(
        STATS_FETCH_TIMEOUT,
        fetch_workspace_stats_uncached(&state, workspace_id, start),
    )
    .await;

    let stats = match fetch_result {
        Ok(Ok(stats)) => stats,
        Ok(Err(e)) => {
            if let Some(mut stale) = stale_fallback {
                stale.stale = true;
                tracing::warn!(
                    workspace_id = %workspace_id,
                    error = %e,
                    "Workspace stats fetch failed — serving stale cache (P-G13)"
                );
                return Ok(Json(stale));
            }
            return Err(e);
        }
        Err(_) => {
            if let Some(mut stale) = stale_fallback {
                stale.stale = true;
                tracing::warn!(
                    workspace_id = %workspace_id,
                    timeout_secs = STATS_FETCH_TIMEOUT.as_secs(),
                    "Workspace stats fetch timed out under load — serving stale cache (P-G13)"
                );
                return Ok(Json(stale));
            }
            return Err(ApiError::Internal(
                "Workspace stats temporarily unavailable — retry shortly".to_string(),
            ));
        }
    };

    // Update cache for next request
    {
        let mut cache = WORKSPACE_STATS_CACHE.write().await;
        cache.insert(
            workspace_id,
            CachedStats {
                stats: stats.clone(),
                cached_at: Instant::now(),
            },
        );
    }

    Ok(Json(stats))
}

/// Fetch workspace stats from storage backends (uncached).
///
/// SPEC-021 P5-01 (fixes UX "0 documents" when relational rows exist):
/// - **document_count / storage_bytes**: `max(postgresql, kv)` — relational primary,
///   KV fallback for legacy uploads that never dual-wrote.
/// - **entity_count / relationship_count / entity_type_count**: AGE graph (always).
/// - **chunk_count / embedding_count**: KV chunk keys for workspace documents.
async fn fetch_workspace_stats_uncached(
    state: &AppState,
    workspace_id: Uuid,
    start: Instant,
) -> Result<WorkspaceStatsResponse, ApiError> {
    let mut stats = try_kv_storage_stats(state, workspace_id).await?;
    let kv_document_count = stats.document_count;
    let kv_storage_bytes = stats.storage_bytes;

    let mut method = "kv_storage";

    if let Some((pg_docs, pg_bytes)) =
        crate::document_read_model::postgres_document_metrics(state, workspace_id).await
    {
        stats.document_count =
            crate::document_read_model::merge_document_count(pg_docs, kv_document_count);
        stats.storage_bytes =
            crate::document_read_model::merge_storage_bytes(pg_bytes, kv_storage_bytes);
        method = if pg_docs >= kv_document_count {
            "postgresql+kv"
        } else {
            "kv+postgresql"
        };

        if pg_docs > 0 && kv_document_count == 0 {
            tracing::info!(
                workspace_id = %workspace_id,
                pg_document_count = pg_docs,
                "SPEC-021: relational documents present but KV metadata missing for workspace"
            );
        }
    }

    let elapsed = start.elapsed();
    tracing::info!(
        workspace_id = %workspace_id,
        duration_ms = elapsed.as_millis(),
        method = method,
        document_count = stats.document_count,
        entity_count = stats.entity_count,
        relationship_count = stats.relationship_count,
        "Workspace stats from hybrid read model (SPEC-021 P5-01)"
    );
    Ok(stats)
}

/// Get stats from KV storage (moderate speed, current source of truth).
///
/// This aggregates document metadata from KV storage and counts chunks.
/// Reliable but slower than PostgreSQL as it requires fetching all metadata
/// and filtering in memory.
async fn try_kv_storage_stats(
    state: &AppState,
    workspace_id: Uuid,
) -> Result<WorkspaceStatsResponse, ApiError> {
    // SPEC-027 phase 10: wsdoc index prefix scan with suffix-scan fallback.
    let metadata_values = load_workspace_metadata_values(
        state.storage.kv_storage.as_ref(),
        &workspace_id.to_string(),
    )
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to get document metadata: {}", e)))?;

    // Aggregate stats from documents belonging to this workspace
    let mut document_count = 0;
    let mut storage_bytes: u64 = 0;
    let mut workspace_doc_ids = Vec::new();
    let mut chunk_count_from_metadata = 0usize;

    for value in metadata_values {
        if let Some(obj) = value.as_object() {
            document_count += 1;

            // Collect document ID for per-doc chunk prefix scan
            if let Some(id) = obj.get("id").and_then(|v| v.as_str()) {
                workspace_doc_ids.push(id.to_string());
            }

            chunk_count_from_metadata += obj
                .get("chunk_count")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize)
                .unwrap_or(0);

            // Sum storage bytes
            if let Some(bytes) = obj.get("file_size_bytes").and_then(|v| v.as_u64()) {
                storage_bytes += bytes;
            }
        }
    }

    // OODA-03: Get entity/relationship counts from Apache AGE graph storage
    // WHY: KV metadata doesn't have entity_count/relationship_count fields.
    // The actual entity/relationship data is stored in the graph, not metadata.
    // This fixes dashboard showing 0 entities despite successful extraction.
    let entity_count = state
        .storage
        .graph_storage
        .node_count_by_workspace(&workspace_id)
        .await
        .unwrap_or(0);

    let relationship_count = state
        .storage
        .graph_storage
        .edge_count_by_workspace(&workspace_id)
        .await
        .unwrap_or(0);

    // Embedding count requires chunk payloads; use per-doc prefix scan (no global keys_like).
    let chunk_count = chunk_count_from_metadata;
    let mut embedding_count = 0;

    for doc_id in &workspace_doc_ids {
        let prefix = format!("{doc_id}-chunk-");
        let doc_chunk_keys = state
            .storage
            .kv_storage
            .keys_with_prefix(&prefix)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to list chunk keys: {}", e)))?;

        if !doc_chunk_keys.is_empty() {
            let chunk_values = state
                .storage
                .kv_storage
                .get_by_ids(&doc_chunk_keys)
                .await
                .map_err(|e| ApiError::Internal(format!("Failed to get chunk data: {}", e)))?;

            for chunk_value in chunk_values {
                if let Some(obj) = chunk_value.as_object() {
                    if obj.get("embedding").is_some() {
                        embedding_count += 1;
                    }
                }
            }
        }
    }

    // Get distinct entity type count from graph storage.
    // WHY: Dashboard EntityTypes KPI was extremely slow — it fetched ALL graph
    // nodes over the wire just to compute unique types. This single aggregate
    // query reduces latency from seconds to milliseconds.
    let entity_type_count = state
        .storage
        .graph_storage
        .distinct_node_type_count_by_workspace(&workspace_id)
        .await
        .unwrap_or(0);

    Ok(WorkspaceStatsResponse {
        workspace_id,
        document_count,
        entity_count,
        relationship_count,
        entity_type_count,
        chunk_count,
        embedding_count,
        storage_bytes,
        stale: false,
    })
}

// ============================================================================
// OODA-22: Metrics History Endpoint
// ============================================================================

/// Get metrics history for a workspace.
///
/// Returns time-series metrics snapshots in reverse chronological order (newest first).
/// Useful for trend analysis, debugging, and monitoring workspace growth.
///
/// ## Query Parameters
///
/// - `limit`: Maximum number of snapshots to return (default: 100, max: 1000)
/// - `offset`: Number of snapshots to skip (default: 0)
///
/// ## Trigger Types
///
/// - `event`: Recorded after document add/delete operations
/// - `scheduled`: Recorded by background hourly task
/// - `manual`: Recorded by admin request
#[utoipa::path(
    get,
    path = "/api/v1/workspaces/{workspace_id}/metrics-history",
    params(
        ("workspace_id" = Uuid, Path, description = "Workspace ID"),
        ("limit" = Option<usize>, Query, description = "Maximum snapshots to return (default: 100)"),
        ("offset" = Option<usize>, Query, description = "Number of snapshots to skip (default: 0)")
    ),
    responses(
        (status = 200, description = "Metrics history", body = MetricsHistoryResponse),
        (status = 404, description = "Workspace not found"),
    ),
    tags = ["workspaces"]
)]
pub async fn get_metrics_history(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    Query(params): Query<MetricsHistoryParams>,
    tenant_ctx: TenantContext,
) -> Result<Json<MetricsHistoryResponse>, ApiError> {
    // BR0201: Verify workspace belongs to requesting tenant
    verify_workspace_tenant_access(&state, workspace_id, &tenant_ctx).await?;

    // Apply defaults and limits
    let limit = params.limit.unwrap_or(100).min(1000);
    let offset = params.offset.unwrap_or(0);

    let snapshots = state
        .workspace_service
        .get_metrics_history(workspace_id, limit, offset)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let response = MetricsHistoryResponse {
        workspace_id,
        count: snapshots.len(),
        offset,
        limit,
        snapshots: snapshots
            .into_iter()
            .map(|s| MetricsSnapshotDTO {
                id: s.id,
                recorded_at: s.recorded_at.to_rfc3339(),
                trigger_type: s.trigger_type.to_string(),
                document_count: s.document_count as i64,
                chunk_count: s.chunk_count as i64,
                entity_count: s.entity_count as i64,
                relationship_count: s.relationship_count as i64,
                embedding_count: s.embedding_count as i64,
                storage_bytes: s.storage_bytes as i64,
            })
            .collect(),
    };

    Ok(Json(response))
}

/// Query parameters for metrics history endpoint.
#[derive(Debug, Deserialize)]
pub struct MetricsHistoryParams {
    /// Maximum number of snapshots to return.
    pub limit: Option<usize>,
    /// Number of snapshots to skip.
    pub offset: Option<usize>,
}

/// Manually trigger a metrics snapshot for a workspace.
///
/// # Implements
///
/// - **FEAT1701**: Workspace Metrics Tracking
///
/// # WHY: Manual Trigger
///
/// Users may want to capture a metrics snapshot at a specific point in time
/// for debugging, auditing, or comparison purposes. This endpoint allows
/// manual triggering without waiting for automatic event-based recording.
///
/// # Use Cases
///
/// - Debug workspace state at a specific moment
/// - Capture baseline before bulk operations
/// - External scheduler integration (cron jobs)
#[utoipa::path(
    post,
    path = "/api/v1/workspaces/{workspace_id}/metrics-snapshot",
    params(
        ("workspace_id" = Uuid, Path, description = "Workspace ID")
    ),
    responses(
        (status = 201, description = "Snapshot created", body = MetricsSnapshotDTO),
        (status = 404, description = "Workspace not found"),
    ),
    tags = ["workspaces"]
)]
pub async fn trigger_metrics_snapshot(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    tenant_ctx: TenantContext,
) -> Result<(StatusCode, Json<MetricsSnapshotDTO>), ApiError> {
    // BR0201: Verify workspace belongs to requesting tenant before recording
    verify_workspace_tenant_access(&state, workspace_id, &tenant_ctx).await?;

    // Record a manual-triggered snapshot
    let snapshot = state
        .workspace_service
        .record_metrics_snapshot(workspace_id, MetricsTriggerType::Manual)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let dto = MetricsSnapshotDTO {
        id: snapshot.id,
        recorded_at: snapshot.recorded_at.to_rfc3339(),
        trigger_type: snapshot.trigger_type.to_string(),
        document_count: snapshot.document_count as i64,
        chunk_count: snapshot.chunk_count as i64,
        entity_count: snapshot.entity_count as i64,
        relationship_count: snapshot.relationship_count as i64,
        embedding_count: snapshot.embedding_count as i64,
        storage_bytes: snapshot.storage_bytes as i64,
    };

    Ok((StatusCode::CREATED, Json(dto)))
}
