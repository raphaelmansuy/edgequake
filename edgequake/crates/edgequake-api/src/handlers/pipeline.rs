//! Pipeline status and control handlers (Phase 3).

use axum::{extract::State, Json};

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

// Re-export DTOs from pipeline_types for backwards compatibility
pub use crate::handlers::pipeline_types::{
    CancelPipelineResponse, EnhancedPipelineStatusResponse, PipelineMessageResponse,
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
    let stats = state
        .task_storage
        .get_statistics()
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
