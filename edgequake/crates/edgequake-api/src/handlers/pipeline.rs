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
use crate::state::AppState;

// Re-export DTOs from pipeline_types for backwards compatibility
pub use crate::handlers::pipeline_types::{
    CancelPipelineResponse, EnhancedPipelineStatusResponse, PipelineMessageResponse,
    QueueMetricsResponse,
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
    let snapshot = state.pipeline_state.get_status().await;

    // Get task statistics
    // WHY: Pipeline status shows global statistics across all tenants.
    // This is intentional as pipeline is a shared resource.
    // Per-tenant statistics are available via /api/v1/tasks endpoint.
    let stats = state
        .task_storage
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

/// Request cancellation of the current pipeline job.
#[utoipa::path(
    post,
    path = "/api/v1/pipeline/cancel",
    tag = "Pipeline",
    responses(
        (status = 200, description = "Cancellation requested", body = CancelPipelineResponse),
        (status = 409, description = "No job is currently running")
    )
)]
pub async fn cancel_pipeline(
    State(state): State<AppState>,
) -> ApiResult<Json<CancelPipelineResponse>> {
    // Check if pipeline is busy
    if !state.pipeline_state.is_busy().await {
        return Err(ApiError::Conflict(
            "No job is currently running".to_string(),
        ));
    }

    // Request cancellation
    state.pipeline_state.request_cancellation().await;

    Ok(Json(CancelPipelineResponse {
        status: "cancellation_requested".to_string(),
        message:
            "Pipeline cancellation has been requested. Processing will stop after current document."
                .to_string(),
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
        .task_storage
        .get_queue_metrics_filtered(tenant_id, workspace_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get queue metrics: {}", e)))?;

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
    }))
}
