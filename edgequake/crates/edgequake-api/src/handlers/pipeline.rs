//! Pipeline status and control handlers (Phase 3).

use axum::{extract::State, Json};
use serde::Serialize;
use utoipa::ToSchema;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// Enhanced pipeline status response with history messages.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EnhancedPipelineStatusResponse {
    /// Whether the pipeline is currently processing.
    pub is_busy: bool,

    /// Current job name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_name: Option<String>,

    /// When the current job started (ISO 8601).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_start: Option<String>,

    /// Total documents to process.
    pub total_documents: u32,

    /// Documents processed so far.
    pub processed_documents: u32,

    /// Current batch number.
    pub current_batch: u32,

    /// Total number of batches.
    pub total_batches: u32,

    /// Latest status message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_message: Option<String>,

    /// History of pipeline messages.
    pub history_messages: Vec<PipelineMessageResponse>,

    /// Whether cancellation has been requested.
    pub cancellation_requested: bool,

    /// Number of pending tasks.
    pub pending_tasks: usize,

    /// Number of processing tasks.
    pub processing_tasks: usize,

    /// Number of completed tasks.
    pub completed_tasks: usize,

    /// Number of failed tasks.
    pub failed_tasks: usize,
}

/// A pipeline message for the API response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PipelineMessageResponse {
    /// ISO 8601 timestamp.
    pub timestamp: String,
    /// Message level: "info", "warn", or "error".
    pub level: String,
    /// The message content.
    pub message: String,
}

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

/// Cancel pipeline response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CancelPipelineResponse {
    /// Status of the cancellation request.
    pub status: String,
    /// Message describing the result.
    pub message: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_pipeline_status_response_serialization() {
        let response = EnhancedPipelineStatusResponse {
            is_busy: true,
            job_name: Some("Processing documents".to_string()),
            job_start: Some("2024-01-01T00:00:00Z".to_string()),
            total_documents: 10,
            processed_documents: 5,
            current_batch: 2,
            total_batches: 3,
            latest_message: Some("Processing document 5...".to_string()),
            history_messages: vec![PipelineMessageResponse {
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                level: "info".to_string(),
                message: "Started processing".to_string(),
            }],
            cancellation_requested: false,
            pending_tasks: 2,
            processing_tasks: 1,
            completed_tasks: 7,
            failed_tasks: 0,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"is_busy\":true"));
        assert!(json.contains("Processing documents"));
        assert!(json.contains("\"total_documents\":10"));
    }

    #[test]
    fn test_cancel_pipeline_response_serialization() {
        let response = CancelPipelineResponse {
            status: "cancellation_requested".to_string(),
            message: "Pipeline cancellation has been requested.".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("cancellation_requested"));
    }

    #[test]
    fn test_pipeline_message_response_serialization() {
        let msg = PipelineMessageResponse {
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            level: "error".to_string(),
            message: "Failed to process".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"level\":\"error\""));
        assert!(json.contains("Failed to process"));
    }

    #[test]
    fn test_pipeline_status_idle_state() {
        let response = EnhancedPipelineStatusResponse {
            is_busy: false,
            job_name: None,
            job_start: None,
            total_documents: 0,
            processed_documents: 0,
            current_batch: 0,
            total_batches: 0,
            latest_message: None,
            history_messages: vec![],
            cancellation_requested: false,
            pending_tasks: 0,
            processing_tasks: 0,
            completed_tasks: 0,
            failed_tasks: 0,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"is_busy\":false"));
        // Optional fields should be skipped when None
        assert!(!json.contains("job_name"));
    }
}
