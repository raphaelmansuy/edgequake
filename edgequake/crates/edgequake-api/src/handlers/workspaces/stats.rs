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
    //
    // Performance tiers:
    // 0. Cache (<1ms) - FASTEST, 60s TTL
    // 1. PostgreSQL documents table (1-5ms) - Fast but currently empty
    // 2. KV storage aggregation (15ms) - Moderate, current data source
    // 3. AGE graph queries (50-200ms) - Slowest, last resort

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

    // Cache miss - fetch from storage
    let stats = fetch_workspace_stats_uncached(&state, workspace_id, start).await?;

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
/// FIX-ISSUE-81: Always use KV storage + Apache AGE graph as the single
/// source of truth. The previous PostgreSQL-first fallback short-circuited
/// when `document_count > 0` (e.g. 1 PDF in PostgreSQL), returning stale
/// entity/relationship counts (0) from empty PostgreSQL tables while the
/// accurate data lived in KV + AGE.
///
/// KV storage holds ALL documents (text, markdown, file, PDF), and AGE
/// graph holds ALL entities and relationships — making them authoritative.
async fn fetch_workspace_stats_uncached(
    state: &AppState,
    workspace_id: Uuid,
    start: Instant,
) -> Result<WorkspaceStatsResponse, ApiError> {
    // ALWAYS use KV storage for document count (source of truth for ALL doc types)
    // ALWAYS use AGE graph for entity/relationship counts (source of truth)
    // This eliminates the PostgreSQL fallback that caused the KPI mismatch (Issue #81)
    let stats = try_kv_storage_stats(state, workspace_id).await?;
    let elapsed = start.elapsed();
    tracing::info!(
        workspace_id = %workspace_id,
        duration_ms = elapsed.as_millis(),
        method = "kv_storage",
        document_count = stats.document_count,
        entity_count = stats.entity_count,
        relationship_count = stats.relationship_count,
        "FIX-ISSUE-81: Workspace stats from KV+AGE (authoritative source)"
    );
    Ok(stats)
}

/// Try to get stats from PostgreSQL documents table.
///
/// NOTE (FIX-ISSUE-81): This function is no longer called in the hot path.
/// It is retained for future use when Phase 2 dual-write is fully complete
/// and all upload paths populate the PostgreSQL `documents` table.
/// At that point, it can be re-enabled as an optimization layer.
#[allow(dead_code)]
async fn try_postgres_stats(
    state: &AppState,
    workspace_id: Uuid,
) -> Result<WorkspaceStatsResponse, ApiError> {
    // WHY: Call workspace_service which has access to PgPool
    // This uses the existing service layer with optimized SQL queries
    let stats = state
        .workspace_service
        .get_workspace_stats(workspace_id)
        .await
        .map_err(|e| ApiError::Internal(format!("PostgreSQL stats query failed: {}", e)))?;

    Ok(WorkspaceStatsResponse {
        workspace_id: stats.workspace_id,
        document_count: stats.document_count,
        entity_count: stats.entity_count,
        relationship_count: stats.relationship_count,
        entity_type_count: 0, // PostgreSQL path doesn't have this yet; will be overridden by graph query
        chunk_count: stats.chunk_count,
        embedding_count: stats.embedding_count,
        storage_bytes: stats.storage_bytes as u64,
    })
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
    // Get all keys from KV storage
    let all_keys = state
        .kv_storage
        .keys()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get KV storage keys: {}", e)))?;

    // Filter metadata keys
    let metadata_keys: Vec<String> = all_keys
        .iter()
        .filter(|k| k.ends_with("-metadata"))
        .cloned()
        .collect();

    // Get all metadata values
    let metadata_values = state
        .kv_storage
        .get_by_ids(&metadata_keys)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get document metadata: {}", e)))?;

    // Aggregate stats from documents belonging to this workspace
    let mut document_count = 0;
    let mut storage_bytes: u64 = 0;
    let mut workspace_doc_ids = Vec::new();

    for value in metadata_values {
        if let Some(obj) = value.as_object() {
            // Check if document belongs to this workspace
            let doc_workspace_id = obj
                .get("workspace_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok());

            if doc_workspace_id == Some(workspace_id) {
                document_count += 1;

                // Collect document ID for chunk counting
                if let Some(id) = obj.get("id").and_then(|v| v.as_str()) {
                    workspace_doc_ids.push(id.to_string());
                }

                // Sum storage bytes
                if let Some(bytes) = obj.get("file_size_bytes").and_then(|v| v.as_u64()) {
                    storage_bytes += bytes;
                }
            }
        }
    }

    // OODA-03: Get entity/relationship counts from Apache AGE graph storage
    // WHY: KV metadata doesn't have entity_count/relationship_count fields.
    // The actual entity/relationship data is stored in the graph, not metadata.
    // This fixes dashboard showing 0 entities despite successful extraction.
    let entity_count = state
        .graph_storage
        .node_count_by_workspace(&workspace_id)
        .await
        .unwrap_or(0);

    let relationship_count = state
        .graph_storage
        .edge_count_by_workspace(&workspace_id)
        .await
        .unwrap_or(0);

    // Count chunks and embeddings for this workspace's documents
    let mut chunk_count = 0;
    let mut embedding_count = 0;

    for doc_id in &workspace_doc_ids {
        // Count chunk keys for this document
        let doc_chunk_keys: Vec<String> = all_keys
            .iter()
            .filter(|k| k.starts_with(&format!("{}-chunk-", doc_id)))
            .cloned()
            .collect();

        chunk_count += doc_chunk_keys.len();

        // Get chunk data to check for embeddings
        if !doc_chunk_keys.is_empty() {
            let chunk_values = state
                .kv_storage
                .get_by_ids(&doc_chunk_keys)
                .await
                .map_err(|e| ApiError::Internal(format!("Failed to get chunk data: {}", e)))?;

            // Count chunks that have embeddings
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
