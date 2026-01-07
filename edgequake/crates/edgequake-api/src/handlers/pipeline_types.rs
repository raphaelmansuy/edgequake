//! DTOs for pipeline status and control handlers.
//!
//! This module contains the request and response types for pipeline operations.

use serde::Serialize;
use utoipa::ToSchema;

// ============================================================================
// Response Types
// ============================================================================

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

/// Cancel pipeline response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CancelPipelineResponse {
    /// Status of the cancellation request.
    pub status: String,
    /// Message describing the result.
    pub message: String,
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_pipeline_status_serialization() {
        let response = EnhancedPipelineStatusResponse {
            is_busy: true,
            job_name: Some("ingestion".to_string()),
            job_start: Some("2024-01-01T00:00:00Z".to_string()),
            total_documents: 100,
            processed_documents: 50,
            current_batch: 3,
            total_batches: 10,
            latest_message: Some("Processing...".to_string()),
            history_messages: vec![],
            cancellation_requested: false,
            pending_tasks: 50,
            processing_tasks: 5,
            completed_tasks: 45,
            failed_tasks: 0,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("is_busy"));
        assert!(json.contains("ingestion"));
    }

    #[test]
    fn test_pipeline_message_serialization() {
        let message = PipelineMessageResponse {
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            level: "info".to_string(),
            message: "Processing document 1/10".to_string(),
        };

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("timestamp"));
        assert!(json.contains("info"));
    }

    #[test]
    fn test_cancel_pipeline_response_serialization() {
        let response = CancelPipelineResponse {
            status: "success".to_string(),
            message: "Cancellation requested".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("success"));
        assert!(json.contains("Cancellation requested"));
    }
}
