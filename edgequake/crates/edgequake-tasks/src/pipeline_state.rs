//! Pipeline state management for real-time status updates (Phase 3).
//!
//! This module provides a thread-safe pipeline state that tracks:
//! - Current job progress (documents processed, batches completed)
//! - History of processing messages
//! - Cancellation requests
//!
//! The state is designed to be shared across worker threads and API handlers.

use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A single pipeline message with timestamp and level.
#[derive(Debug, Clone, Serialize)]
pub struct PipelineMessage {
    /// ISO 8601 timestamp.
    pub timestamp: String,
    /// Message level: "info", "warn", or "error".
    pub level: String,
    /// The message content.
    pub message: String,
}

impl PipelineMessage {
    /// Create a new message with current timestamp.
    pub fn new(level: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now().to_rfc3339(),
            level: level.into(),
            message: message.into(),
        }
    }

    /// Create an info message.
    pub fn info(message: impl Into<String>) -> Self {
        Self::new("info", message)
    }

    /// Create a warning message.
    pub fn warn(message: impl Into<String>) -> Self {
        Self::new("warn", message)
    }

    /// Create an error message.
    pub fn error(message: impl Into<String>) -> Self {
        Self::new("error", message)
    }
}

/// Internal state of the pipeline.
struct PipelineStateInner {
    is_busy: bool,
    job_name: Option<String>,
    job_start: Option<DateTime<Utc>>,
    total_documents: u32,
    processed_documents: u32,
    current_batch: u32,
    total_batches: u32,
    messages: Vec<PipelineMessage>,
    cancellation_requested: bool,
    max_messages: usize,
}

impl Default for PipelineStateInner {
    fn default() -> Self {
        Self {
            is_busy: false,
            job_name: None,
            job_start: None,
            total_documents: 0,
            processed_documents: 0,
            current_batch: 0,
            total_batches: 0,
            messages: Vec::new(),
            cancellation_requested: false,
            max_messages: 100,
        }
    }
}

/// Thread-safe pipeline state for tracking document processing.
#[derive(Clone)]
pub struct PipelineState {
    inner: Arc<RwLock<PipelineStateInner>>,
}

impl Default for PipelineState {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineState {
    /// Create a new pipeline state.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(PipelineStateInner::default())),
        }
    }

    /// Create with custom max messages limit.
    pub fn with_max_messages(max_messages: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(PipelineStateInner {
                max_messages,
                ..Default::default()
            })),
        }
    }

    /// Start a new job.
    pub async fn start_job(&self, name: String, total_docs: u32, batches: u32) {
        let mut inner = self.inner.write().await;
        inner.is_busy = true;
        inner.job_name = Some(name.clone());
        inner.job_start = Some(Utc::now());
        inner.total_documents = total_docs;
        inner.processed_documents = 0;
        inner.current_batch = 0;
        inner.total_batches = batches;
        inner.cancellation_requested = false;

        // Log start message
        let msg = PipelineMessage::info(format!("Starting: {}", name));
        Self::push_message(&mut inner, msg);
    }

    /// Log a message at the specified level.
    pub async fn log(&self, level: &str, message: String) {
        let mut inner = self.inner.write().await;
        let msg = PipelineMessage::new(level, message);
        Self::push_message(&mut inner, msg);
    }

    /// Log an info message.
    pub async fn info(&self, message: impl Into<String>) {
        self.log("info", message.into()).await;
    }

    /// Log a warning message.
    pub async fn warn(&self, message: impl Into<String>) {
        self.log("warn", message.into()).await;
    }

    /// Log an error message.
    pub async fn error(&self, message: impl Into<String>) {
        self.log("error", message.into()).await;
    }

    /// Push a message to the history, respecting max limit.
    fn push_message(inner: &mut PipelineStateInner, msg: PipelineMessage) {
        inner.messages.push(msg);

        // Keep last N messages
        if inner.messages.len() > inner.max_messages {
            inner.messages.remove(0);
        }
    }

    /// Advance to the next batch.
    pub async fn advance_batch(&self) {
        let mut inner = self.inner.write().await;
        inner.current_batch += 1;
        let msg = PipelineMessage::info(format!(
            "Batch {}/{}",
            inner.current_batch, inner.total_batches
        ));
        Self::push_message(&mut inner, msg);
    }

    /// Mark a document as processed.
    pub async fn document_processed(&self, doc_id: &str, entities: usize) {
        let mut inner = self.inner.write().await;
        inner.processed_documents += 1;
        let msg = PipelineMessage::info(format!(
            "✓ {} ({} entities) - {}/{}",
            doc_id, entities, inner.processed_documents, inner.total_documents
        ));
        Self::push_message(&mut inner, msg);
    }

    /// Mark a document as failed.
    pub async fn document_failed(&self, doc_id: &str, error: &str) {
        let mut inner = self.inner.write().await;
        inner.processed_documents += 1;
        let msg = PipelineMessage::error(format!(
            "✗ {} failed: {} - {}/{}",
            doc_id, error, inner.processed_documents, inner.total_documents
        ));
        Self::push_message(&mut inner, msg);
    }

    /// Finish the current job.
    pub async fn finish_job(&self) {
        let mut inner = self.inner.write().await;
        let msg = PipelineMessage::info(format!(
            "Complete: {} documents processed",
            inner.processed_documents
        ));
        Self::push_message(&mut inner, msg);
        inner.is_busy = false;
        inner.job_name = None;
    }

    /// Request cancellation of the current job.
    pub async fn request_cancellation(&self) {
        let mut inner = self.inner.write().await;
        inner.cancellation_requested = true;
        let msg = PipelineMessage::warn("Cancellation requested".to_string());
        Self::push_message(&mut inner, msg);
    }

    /// Check if cancellation has been requested.
    pub async fn is_cancellation_requested(&self) -> bool {
        self.inner.read().await.cancellation_requested
    }

    /// Check if the pipeline is currently busy.
    pub async fn is_busy(&self) -> bool {
        self.inner.read().await.is_busy
    }

    /// Get a snapshot of the current pipeline status.
    pub async fn get_status(&self) -> PipelineStatusSnapshot {
        let inner = self.inner.read().await;
        PipelineStatusSnapshot {
            is_busy: inner.is_busy,
            job_name: inner.job_name.clone(),
            job_start: inner.job_start.map(|d| d.to_rfc3339()),
            total_documents: inner.total_documents,
            processed_documents: inner.processed_documents,
            current_batch: inner.current_batch,
            total_batches: inner.total_batches,
            latest_message: inner.messages.last().map(|m| m.message.clone()),
            history_messages: inner.messages.clone(),
            cancellation_requested: inner.cancellation_requested,
        }
    }

    /// Clear all messages.
    pub async fn clear_messages(&self) {
        let mut inner = self.inner.write().await;
        inner.messages.clear();
    }

    /// Reset the pipeline state entirely.
    pub async fn reset(&self) {
        let mut inner = self.inner.write().await;
        *inner = PipelineStateInner::default();
    }
}

/// A snapshot of the pipeline status for API responses.
#[derive(Debug, Clone, Serialize)]
pub struct PipelineStatusSnapshot {
    /// Whether the pipeline is currently processing.
    pub is_busy: bool,
    /// Current job name.
    pub job_name: Option<String>,
    /// When the current job started (ISO 8601).
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
    pub latest_message: Option<String>,
    /// History of pipeline messages.
    pub history_messages: Vec<PipelineMessage>,
    /// Whether cancellation has been requested.
    pub cancellation_requested: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pipeline_state_new() {
        let state = PipelineState::new();
        let snapshot = state.get_status().await;

        assert!(!snapshot.is_busy);
        assert!(snapshot.job_name.is_none());
        assert_eq!(snapshot.total_documents, 0);
        assert!(snapshot.history_messages.is_empty());
    }

    #[tokio::test]
    async fn test_pipeline_state_start_job() {
        let state = PipelineState::new();

        state.start_job("Test Job".to_string(), 10, 3).await;
        let snapshot = state.get_status().await;

        assert!(snapshot.is_busy);
        assert_eq!(snapshot.job_name, Some("Test Job".to_string()));
        assert_eq!(snapshot.total_documents, 10);
        assert_eq!(snapshot.total_batches, 3);
        assert_eq!(snapshot.history_messages.len(), 1);
        assert!(snapshot.history_messages[0].message.contains("Starting"));
    }

    #[tokio::test]
    async fn test_pipeline_state_document_processed() {
        let state = PipelineState::new();
        state.start_job("Test".to_string(), 5, 1).await;

        state.document_processed("doc-1", 3).await;
        state.document_processed("doc-2", 5).await;

        let snapshot = state.get_status().await;
        assert_eq!(snapshot.processed_documents, 2);
        assert_eq!(snapshot.history_messages.len(), 3); // start + 2 docs
    }

    #[tokio::test]
    async fn test_pipeline_state_cancellation() {
        let state = PipelineState::new();
        state.start_job("Test".to_string(), 10, 2).await;

        assert!(!state.is_cancellation_requested().await);
        state.request_cancellation().await;
        assert!(state.is_cancellation_requested().await);

        let snapshot = state.get_status().await;
        assert!(snapshot.cancellation_requested);
    }

    #[tokio::test]
    async fn test_pipeline_state_finish_job() {
        let state = PipelineState::new();
        state.start_job("Test".to_string(), 2, 1).await;
        state.document_processed("doc-1", 1).await;
        state.document_processed("doc-2", 2).await;
        state.finish_job().await;

        let snapshot = state.get_status().await;
        assert!(!snapshot.is_busy);
        assert!(snapshot.job_name.is_none());
        assert!(snapshot.latest_message.unwrap().contains("Complete"));
    }

    #[tokio::test]
    async fn test_pipeline_state_max_messages() {
        let state = PipelineState::with_max_messages(5);

        for i in 0..10 {
            state.info(format!("Message {}", i)).await;
        }

        let snapshot = state.get_status().await;
        assert_eq!(snapshot.history_messages.len(), 5);
        assert!(snapshot.history_messages[0].message.contains("Message 5"));
    }

    #[tokio::test]
    async fn test_pipeline_state_advance_batch() {
        let state = PipelineState::new();
        state.start_job("Test".to_string(), 10, 3).await;

        state.advance_batch().await;
        state.advance_batch().await;

        let snapshot = state.get_status().await;
        assert_eq!(snapshot.current_batch, 2);
    }

    #[tokio::test]
    async fn test_pipeline_message_levels() {
        let info = PipelineMessage::info("Info message");
        assert_eq!(info.level, "info");

        let warn = PipelineMessage::warn("Warning message");
        assert_eq!(warn.level, "warn");

        let error = PipelineMessage::error("Error message");
        assert_eq!(error.level, "error");
    }

    #[test]
    fn test_pipeline_status_snapshot_serialization() {
        let snapshot = PipelineStatusSnapshot {
            is_busy: true,
            job_name: Some("Test Job".to_string()),
            job_start: Some("2024-01-01T00:00:00Z".to_string()),
            total_documents: 10,
            processed_documents: 5,
            current_batch: 2,
            total_batches: 3,
            latest_message: Some("Processing...".to_string()),
            history_messages: vec![PipelineMessage::info("Started")],
            cancellation_requested: false,
        };

        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("\"is_busy\":true"));
        assert!(json.contains("Test Job"));
        assert!(json.contains("\"total_documents\":10"));
    }
}
