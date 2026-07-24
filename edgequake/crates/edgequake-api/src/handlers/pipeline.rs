//! Pipeline status and control handlers (Phase 3).
//!
//! ## Implements
//!
//! - **FEAT0550**: Pipeline status retrieval with progress info
//! - **FEAT0551**: Pipeline cancellation for long-running jobs
//! - **FEAT0552**: History message retrieval for debugging
//!
//! ## Use Cases
//!
//! - **UC2150**: User checks current pipeline processing status
//! - **UC2151**: User cancels stuck or unwanted pipeline job
//! - **UC2152**: User reviews pipeline history for troubleshooting
//!
//! ## Enforces
//!
//! - **BR0550**: Pipeline status must include task statistics
//! - **BR0551**: Cancellation must be graceful with cleanup
//! - **BR0552**: History messages must be time-ordered

use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::services::sync_doc_cancelled_for_task;
use crate::services::task_cancel::apply_cancel_all_active;
use crate::state::AppState;
use std::sync::Arc;

// Re-export DTOs from pipeline_types for backwards compatibility
pub use crate::handlers::ingestion_types::PipelineActivityResponse;
pub use crate::handlers::pipeline_types::{
    CancelPipelineResponse, EnhancedPipelineStatusResponse, PipelineMessageResponse,
    QueueMetricsResponse, StoreContentionMetrics,
};

/// Get enhanced pipeline status with history messages.
#[utoipa::path(
    get,
    path = "/api/v1/pipeline/status",
    tag = "Pipeline",
    responses(
        (status = 200, description = "Pipeline status retrieved", body = EnhancedPipelineStatusResponse)
    )
)]
pub async fn get_pipeline_status(
    State(state): State<AppState>,
) -> ApiResult<Json<EnhancedPipelineStatusResponse>> {
    // Get pipeline state snapshot
    let snapshot = state.tasks.pipeline_state.get_status().await;

    // Get task statistics
    // WHY: Pipeline status shows global statistics across all tenants.
    // This is intentional as pipeline is a shared resource.
    // Per-tenant statistics are available via /api/v1/tasks endpoint.
    let stats = state
        .tasks
        .storage
        .get_statistics(edgequake_tasks::storage::TaskFilter::default())
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get statistics: {}", e)))?;

    Ok(Json(EnhancedPipelineStatusResponse {
        is_busy: snapshot.is_busy || stats.processing > 0,
        job_name: snapshot.job_name,
        job_start: snapshot.job_start,
        total_documents: snapshot.total_documents,
        processed_documents: snapshot.processed_documents,
        current_batch: snapshot.current_batch,
        total_batches: snapshot.total_batches,
        latest_message: snapshot.latest_message,
        history_messages: snapshot
            .history_messages
            .into_iter()
            .map(|m| PipelineMessageResponse {
                timestamp: m.timestamp,
                level: m.level,
                message: m.message,
            })
            .collect(),
        cancellation_requested: snapshot.cancellation_requested,
        pending_tasks: stats.pending as usize,
        processing_tasks: stats.processing as usize,
        completed_tasks: stats.indexed as usize,
        failed_tasks: stats.failed as usize,
    }))
}

/// SPEC-048: Pipeline activity — Busy SSOT (working docs + processing tasks).
#[utoipa::path(
    get,
    path = "/api/v1/pipeline/activity",
    tag = "Pipeline",
    responses(
        (status = 200, description = "Pipeline activity", body = PipelineActivityResponse)
    )
)]
pub async fn get_pipeline_activity(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
) -> ApiResult<Json<PipelineActivityResponse>> {
    use crate::handlers::ingestion_types::PipelineActivityTask;
    use crate::services::document_metadata_scan::load_scoped_document_metadata_for_progress;
    use crate::services::progress_facade::{assemble_pipeline_activity, classify_activity_doc};
    use crate::services::tenant_guard::has_full_tenant_context;
    use edgequake_tasks::storage::{Pagination, TaskFilter};
    use edgequake_tasks::TaskStatus;

    let mut classified = Vec::new();
    if has_full_tenant_context(&tenant_ctx) {
        // SPEC-086: staging-aware so in-flight MD counts in queued/working.
        let metadata_values = load_scoped_document_metadata_for_progress(
            state.storage.kv_storage.as_ref(),
            &tenant_ctx,
        )
        .await?;
        for value in metadata_values {
            if let Some(obj) = value.as_object() {
                if let Some(pair) = classify_activity_doc(obj) {
                    classified.push(pair);
                }
            }
        }
    }

    let mut activity_tasks = Vec::new();
    let mut filter = TaskFilter::default();
    if let Some(ref tid) = tenant_ctx.tenant_id {
        if let Ok(u) = uuid::Uuid::parse_str(tid) {
            filter.tenant_id = Some(u);
        }
    }
    if let Some(ref wid) = tenant_ctx.workspace_id {
        if let Ok(u) = uuid::Uuid::parse_str(wid) {
            filter.workspace_id = Some(u);
        }
    }
    filter.status = Some(TaskStatus::Processing);

    let pagination = Pagination {
        page: 1,
        page_size: 100,
        ..Pagination::default()
    };
    if let Ok(task_list) = state.tasks.storage.list_tasks(filter, pagination).await {
        for task in task_list.tasks {
            activity_tasks.push(PipelineActivityTask {
                id: task.track_id.clone(),
                kind: format!("{:?}", task.task_type).to_lowercase(),
                document_id: None,
            });
        }
    }

    Ok(Json(assemble_pipeline_activity(classified, activity_tasks)))
}

/// Request cancellation of the current pipeline job.
#[utoipa::path(
    post,
    path = "/api/v1/pipeline/cancel",
    tag = "Pipeline",
    responses(
        (status = 200, description = "Cancellation requested or pipeline already idle", body = CancelPipelineResponse)
    )
)]
pub async fn cancel_pipeline(
    State(state): State<AppState>,
) -> ApiResult<Json<CancelPipelineResponse>> {
    // WHY: Cancellation is an idempotent user action. If the pipeline already
    // finished between dialog open and confirm click, the desired state is
    // already achieved, so return success instead of surfacing a noisy 409.
    if !state.tasks.pipeline_state.is_busy().await {
        return Ok(Json(CancelPipelineResponse {
            status: "already_idle".to_string(),
            message: "Pipeline is already idle; there is nothing left to cancel.".to_string(),
        }));
    }

    // Signal every in-flight task via the shared cancel helper (registry + task rows).
    let results = apply_cancel_all_active(&state.tasks.storage, &state.tasks.cancellation_registry)
        .await
        .map_err(ApiError::Internal)?;

    // SPEC-057 P0 + SPEC-059: sync linked doc KV and retract indexes.
    for applied in &results {
        if applied.cancelled {
            if let Some(ref task) = applied.task {
                if let Err(e) = sync_doc_cancelled_for_task(
                    Arc::clone(&state.storage.kv_storage),
                    task,
                    "Task cancelled by user",
                )
                .await
                {
                    tracing::warn!(
                        track_id = %applied.track_id,
                        error = %e,
                        "Pipeline cancel: doc KV sync failed"
                    );
                }
                let ws = task.workspace_id.to_string();
                let vector =
                    crate::services::get_workspace_vector_storage_for_delete(&state, &ws).await;
                crate::services::retract_indexes_for_task(
                    &state.storage.graph_storage,
                    &vector,
                    task,
                )
                .await;
            }
        }
    }

    // Legacy flag retained for status snapshots / older clients.
    state.tasks.pipeline_state.request_cancellation().await;

    Ok(Json(CancelPipelineResponse {
        status: "cancellation_requested".to_string(),
        message: format!(
            "Pipeline cancellation requested for {} in-flight task(s). Cooperative stop at next checkpoint.",
            results.len()
        ),
    }))
}

/// Query parameters for queue metrics filtering.
///
/// @implements OODA-04: Multi-tenant isolation for queue metrics
#[derive(Debug, Deserialize, IntoParams)]
pub struct QueueMetricsQuery {
    /// Filter by tenant ID. If not provided, uses context from headers.
    pub tenant_id: Option<String>,
    /// Filter by workspace ID. If not provided, uses context from headers.
    pub workspace_id: Option<String>,
}

/// Get queue metrics for task queue visibility.
///
/// ## Implements
///
/// - **FEAT0570**: Queue metrics API endpoint
/// - **OODA-20**: Iteration 20 - Queue metrics REST API
/// - **OODA-04**: Multi-tenant isolation for queue metrics
///
/// ## WHY: Objective B Requirement + Multi-Tenant Isolation
///
/// The Pipeline Monitor UI needs real-time visibility into the task queue:
/// - Queue depth (pending_count)
/// - Worker utilization
/// - Throughput rate
/// - Wait time estimates
///
/// CRITICAL: Metrics MUST be filtered by tenant/workspace to prevent
/// users from seeing processing activity from other tenants.
#[utoipa::path(
    get,
    path = "/api/v1/pipeline/queue-metrics",
    tag = "Pipeline",
    params(QueueMetricsQuery),
    responses(
        (status = 200, description = "Queue metrics retrieved", body = QueueMetricsResponse)
    )
)]
pub async fn get_queue_metrics(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Query(params): Query<QueueMetricsQuery>,
) -> ApiResult<Json<QueueMetricsResponse>> {
    // OODA-04: Use tenant context from headers, or explicit query params
    //
    // WHY: Multi-tenant isolation is CRITICAL. Without filtering, users can
    // see processing activity from other tenants, which is a privacy violation.
    //
    // Priority:
    // 1. Explicit query params (for admin/debugging)
    // 2. TenantContext from headers (normal operation)
    // 3. None (shows all - admin only in production)
    let tenant_id = params
        .tenant_id
        .as_ref()
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .or_else(|| {
            tenant_ctx
                .tenant_id
                .as_ref()
                .and_then(|s| uuid::Uuid::parse_str(s).ok())
        });

    let workspace_id = params
        .workspace_id
        .as_ref()
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .or_else(|| {
            tenant_ctx
                .workspace_id
                .as_ref()
                .and_then(|s| uuid::Uuid::parse_str(s).ok())
        });

    let metrics = state
        .tasks
        .storage
        .get_queue_metrics_filtered(tenant_id, workspace_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get queue metrics: {}", e)))?;

    let pressure = crate::task_queue_pressure::assess_queue_pressure(metrics.pending_count);
    let failed_count = state
        .tasks
        .storage
        .get_statistics(edgequake_tasks::storage::TaskFilter::default())
        .await
        .map(|s| s.failed)
        .unwrap_or(0);
    crate::task_queue_pressure::publish_queue_observability(
        metrics.pending_count,
        metrics.processing_count,
        failed_count,
        &pressure,
    );

    let (
        tenant_park_waiters,
        tenant_park_waiters_ingest,
        tenant_park_waiters_lifecycle,
        max_tasks_per_tenant,
        max_lifecycle_tasks_per_tenant,
    ) = match &state.tasks.tenant_limiter {
        Some(limiter) => {
            let stats = limiter.stats().await;
            (
                stats.park_waiters,
                stats.park_waiters_ingest,
                stats.park_waiters_lifecycle,
                stats.max_per_tenant as u64,
                stats.max_lifecycle_per_tenant as u64,
            )
        }
        None => (0, 0, 0, 0, 0),
    };

    #[cfg(feature = "postgres")]
    let pool_util = state.pg_pool.as_ref().and_then(|pool| {
        crate::store_contention::pool_utilization(pool.size(), pool.num_idle() as u32)
    });
    #[cfg(not(feature = "postgres"))]
    let pool_util: Option<f64> = None;
    let store = crate::store_contention::assess_store_contention(pool_util);
    let store_contention = crate::handlers::pipeline_types::StoreContentionMetrics {
        level: store.level.as_str().to_string(),
        db_pool_utilization: store.db_pool_utilization,
        db_pool_util_warn: store.db_pool_util_warn,
        db_pool_util_critical: store.db_pool_util_critical,
        compensation_quarantine_total: store.compensation_quarantine_total,
        compensation_quarantine_warn: store.compensation_quarantine_warn,
        compensation_quarantine_critical: store.compensation_quarantine_critical,
        compensate_shared_entity_skipped_total:
            edgequake_storage::compensate_shared_entity_skipped_total(),
        retract_on_cancel_total: crate::services::retract_on_cancel_total(),
        vector_dim_mismatch_rejected_total: edgequake_storage::vector_dim_mismatch_rejected_total(),
        operator_action: store.operator_action.clone(),
    };
    // Prefer queue pressure action; surface store action when queue is normal.
    let operator_action = pressure.operator_action.or(store.operator_action);

    Ok(Json(QueueMetricsResponse {
        pending_count: metrics.pending_count,
        processing_count: metrics.processing_count,
        active_workers: metrics.active_workers,
        max_workers: metrics.max_workers,
        worker_utilization: metrics.worker_utilization,
        avg_wait_time_seconds: metrics.avg_wait_time_seconds,
        max_wait_time_seconds: metrics.max_wait_time_seconds,
        throughput_per_minute: metrics.throughput_per_minute,
        estimated_queue_time_seconds: metrics.estimated_queue_time_seconds,
        rate_limited: metrics.rate_limited,
        timestamp: metrics.timestamp.to_rfc3339(),
        pressure: pressure.level.as_str().to_string(),
        pending_warn_threshold: pressure.pending_warn_threshold,
        pending_critical_threshold: pressure.pending_critical_threshold,
        operator_action,
        tenant_park_waiters,
        tenant_park_waiters_ingest,
        tenant_park_waiters_lifecycle,
        cancel_intent_count: state
            .tasks
            .cancellation_registry
            .cancel_intent_count()
            .await as u64,
        cancel_intent_total: state.tasks.cancellation_registry.cancel_intent_total(),
        max_tasks_per_tenant,
        max_lifecycle_tasks_per_tenant,
        store_contention,
    }))
}
