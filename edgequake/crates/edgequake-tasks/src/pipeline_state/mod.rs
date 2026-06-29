//! Pipeline state management for real-time status updates (Phase 3).
//!
//! This module provides a thread-safe pipeline state that tracks:
//! - Current job progress (documents processed, batches completed)
//! - History of processing messages
//! - Cancellation requests
//!
//! The state is designed to be shared across worker threads and API handlers.

mod emitters;
mod event;
mod pdf_tracking;

pub use event::{PipelineEvent, PipelineMessage, PipelineStatusSnapshot};

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, RwLock};

use crate::progress::PdfUploadProgress;

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
    /// OODA-12: Active PDF upload progress, keyed by track_id.
    /// Enables queryable progress for GET /api/v1/documents/pdf/:id/progress
    pub(crate) pdf_progress: HashMap<String, PdfUploadProgress>,
    /// SPEC-032 W-04: GraphStorage phase start times for ETA calculation.
    /// Keyed by track_id, set when PipelinePhase::GraphStorage begins.
    pub(crate) graph_storage_start: HashMap<String, Instant>,
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
            pdf_progress: HashMap::new(),
            graph_storage_start: HashMap::new(),
        }
    }
}

/// Thread-safe pipeline state for tracking document processing.
#[derive(Clone)]
pub struct PipelineState {
    inner: Arc<RwLock<PipelineStateInner>>,
    pub(crate) tx: broadcast::Sender<PipelineEvent>,
}

impl Default for PipelineState {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineState {
    /// Create a new pipeline state.
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            inner: Arc::new(RwLock::new(PipelineStateInner::default())),
            tx,
        }
    }

    /// Create with custom max messages limit.
    pub fn with_max_messages(max_messages: usize) -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            inner: Arc::new(RwLock::new(PipelineStateInner {
                max_messages,
                ..Default::default()
            })),
            tx,
        }
    }

    /// Subscribe to pipeline events.
    pub fn subscribe(&self) -> broadcast::Receiver<PipelineEvent> {
        self.tx.subscribe()
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

        // Notify state change
        let _ = self.tx.send(PipelineEvent::StateChange {
            is_busy: true,
            job_name: Some(name.clone()),
        });

        // Log start message
        let msg = PipelineMessage::info(format!("Starting: {}", name));
        let _ = self.tx.send(PipelineEvent::Log(msg.clone()));
        Self::push_message(&mut inner, msg);
    }

    /// Log a message at the specified level.
    pub async fn log(&self, level: &str, message: String) {
        let mut inner = self.inner.write().await;
        let msg = PipelineMessage::new(level, message);
        let _ = self.tx.send(PipelineEvent::Log(msg.clone()));
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

        let _ = self.tx.send(PipelineEvent::Progress {
            processed: inner.processed_documents,
            total: inner.total_documents,
            batch: inner.current_batch,
            total_batches: inner.total_batches,
        });

        let msg = PipelineMessage::info(format!(
            "Batch {}/{}",
            inner.current_batch, inner.total_batches
        ));
        let _ = self.tx.send(PipelineEvent::Log(msg.clone()));
        Self::push_message(&mut inner, msg);
    }

    /// Mark a document as processed.
    pub async fn document_processed(&self, doc_id: &str, entities: usize) {
        let mut inner = self.inner.write().await;
        inner.processed_documents += 1;

        let _ = self.tx.send(PipelineEvent::Progress {
            processed: inner.processed_documents,
            total: inner.total_documents,
            batch: inner.current_batch,
            total_batches: inner.total_batches,
        });

        let msg = PipelineMessage::info(format!(
            "✓ {} ({} entities) - {}/{}",
            doc_id, entities, inner.processed_documents, inner.total_documents
        ));
        let _ = self.tx.send(PipelineEvent::Log(msg.clone()));
        Self::push_message(&mut inner, msg);
    }

    /// Mark a document as failed.
    pub async fn document_failed(&self, doc_id: &str, error: &str) {
        let mut inner = self.inner.write().await;
        inner.processed_documents += 1;

        let _ = self.tx.send(PipelineEvent::Progress {
            processed: inner.processed_documents,
            total: inner.total_documents,
            batch: inner.current_batch,
            total_batches: inner.total_batches,
        });

        let msg = PipelineMessage::error(format!(
            "✗ {} failed: {} - {}/{}",
            doc_id, error, inner.processed_documents, inner.total_documents
        ));
        let _ = self.tx.send(PipelineEvent::Log(msg.clone()));
        Self::push_message(&mut inner, msg);
    }

    /// Finish the current job.
    pub async fn finish_job(&self) {
        let mut inner = self.inner.write().await;
        let msg = PipelineMessage::info(format!(
            "Complete: {} documents processed",
            inner.processed_documents
        ));
        let _ = self.tx.send(PipelineEvent::Log(msg.clone()));
        Self::push_message(&mut inner, msg);

        inner.is_busy = false;
        inner.job_name = None;

        let _ = self.tx.send(PipelineEvent::StateChange {
            is_busy: false,
            job_name: None,
        });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::PipelinePhase;

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

    #[tokio::test]
    async fn test_emit_pdf_page_progress() {
        // OODA-07: Test PDF page progress event emission
        let state = PipelineState::new();
        let mut rx = state.subscribe();

        // Emit a PDF page progress event
        state.emit_pdf_page_progress(
            "pdf-123".to_string(),
            "task-456".to_string(),
            5,
            10,
            "extraction".to_string(),
            2048,
            true,
            None,
        );

        // Receive and verify the event
        let event = rx.try_recv().unwrap();
        match event {
            PipelineEvent::PdfPageProgress {
                pdf_id,
                task_id,
                page_num,
                total_pages,
                phase,
                markdown_len,
                success,
                error,
            } => {
                assert_eq!(pdf_id, "pdf-123");
                assert_eq!(task_id, "task-456");
                assert_eq!(page_num, 5);
                assert_eq!(total_pages, 10);
                assert_eq!(phase, "extraction");
                assert_eq!(markdown_len, 2048);
                assert!(success);
                assert!(error.is_none());
            }
            _ => panic!("Expected PdfPageProgress event"),
        }
    }

    #[tokio::test]
    async fn test_emit_pdf_page_progress_with_error() {
        // OODA-07: Test PDF page progress with extraction error
        let state = PipelineState::new();
        let mut rx = state.subscribe();

        state.emit_pdf_page_progress(
            "pdf-err".to_string(),
            "task-err".to_string(),
            3,
            5,
            "extraction".to_string(),
            0,
            false,
            Some("Page 3 extraction failed: corrupt image".to_string()),
        );

        let event = rx.try_recv().unwrap();
        match event {
            PipelineEvent::PdfPageProgress {
                success,
                error,
                page_num,
                ..
            } => {
                assert!(!success);
                assert_eq!(page_num, 3);
                assert!(error.unwrap().contains("corrupt image"));
            }
            _ => panic!("Expected PdfPageProgress event"),
        }
    }

    #[test]
    fn test_pdf_page_progress_serialization() {
        // OODA-07: Verify PdfPageProgress serializes correctly for WebSocket
        let event = PipelineEvent::PdfPageProgress {
            pdf_id: "pdf-ser".to_string(),
            task_id: "task-ser".to_string(),
            page_num: 7,
            total_pages: 15,
            phase: "rendering".to_string(),
            markdown_len: 4096,
            success: true,
            error: None,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"PdfPageProgress\""));
        assert!(json.contains("\"pdf_id\":\"pdf-ser\""));
        assert!(json.contains("\"page_num\":7"));
        assert!(json.contains("\"markdown_len\":4096"));
    }

    // =========================================================================
    // OODA-12: PDF Upload Progress Storage Tests
    // =========================================================================

    #[tokio::test]
    async fn test_start_pdf_progress() {
        let state = PipelineState::new();

        // Start tracking a PDF upload
        state
            .start_pdf_progress("track-001", "pdf-001", "test.pdf")
            .await;

        // Verify it was stored
        let progress = state.get_pdf_progress("track-001").await;
        assert!(progress.is_some());

        let p = progress.unwrap();
        assert_eq!(p.track_id, "track-001");
        assert_eq!(p.pdf_id, "pdf-001");
        assert_eq!(p.filename, "test.pdf");
        assert_eq!(p.phases.len(), 6);
        assert!(!p.is_complete);
    }

    #[tokio::test]
    async fn test_get_pdf_progress_not_found() {
        let state = PipelineState::new();

        // Query non-existent progress
        let progress = state.get_pdf_progress("nonexistent").await;
        assert!(progress.is_none());
    }

    #[tokio::test]
    async fn test_update_pdf_phase() {
        let state = PipelineState::new();

        // Start tracking
        state
            .start_pdf_progress("track-002", "pdf-002", "doc.pdf")
            .await;

        // Start PdfConversion phase with 10 pages
        state
            .start_pdf_phase("track-002", PipelinePhase::PdfConversion, 10)
            .await;

        // Update progress to page 5
        state
            .update_pdf_phase(
                "track-002",
                PipelinePhase::PdfConversion,
                5,
                "Extracting page 5 of 10...",
            )
            .await;

        // Verify update
        let progress = state.get_pdf_progress("track-002").await.unwrap();
        let conversion = progress.phase(PipelinePhase::PdfConversion).unwrap();
        assert_eq!(conversion.current, 5);
        assert_eq!(conversion.total, 10);
        assert_eq!(conversion.percentage, 50.0);
    }

    #[tokio::test]
    async fn test_complete_pdf_phase() {
        let state = PipelineState::new();

        state
            .start_pdf_progress("track-003", "pdf-003", "complete.pdf")
            .await;
        state
            .start_pdf_phase("track-003", PipelinePhase::Upload, 1)
            .await;
        state
            .complete_pdf_phase("track-003", PipelinePhase::Upload)
            .await;

        let progress = state.get_pdf_progress("track-003").await.unwrap();
        let upload = progress.phase(PipelinePhase::Upload).unwrap();
        assert!(upload.is_finished());
        assert_eq!(upload.percentage, 100.0);
    }

    #[tokio::test]
    async fn test_fail_pdf_phase() {
        use crate::progress::PhaseError;

        let state = PipelineState::new();

        state
            .start_pdf_progress("track-004", "pdf-004", "fail.pdf")
            .await;
        state
            .start_pdf_phase("track-004", PipelinePhase::PdfConversion, 5)
            .await;

        let error = PhaseError::pdf_parse(3, "Invalid font encoding");
        state
            .fail_pdf_phase("track-004", PipelinePhase::PdfConversion, error)
            .await;

        let progress = state.get_pdf_progress("track-004").await.unwrap();
        assert!(progress.is_failed);
        let conversion = progress.phase(PipelinePhase::PdfConversion).unwrap();
        assert!(conversion.error.is_some());
    }

    #[tokio::test]
    async fn test_remove_pdf_progress() {
        let state = PipelineState::new();

        state
            .start_pdf_progress("track-005", "pdf-005", "remove.pdf")
            .await;
        assert!(state.get_pdf_progress("track-005").await.is_some());

        state.remove_pdf_progress("track-005").await;
        assert!(state.get_pdf_progress("track-005").await.is_none());
    }

    #[tokio::test]
    async fn test_list_pdf_progress() {
        let state = PipelineState::new();

        // Start multiple uploads
        state
            .start_pdf_progress("track-a", "pdf-a", "file-a.pdf")
            .await;
        state
            .start_pdf_progress("track-b", "pdf-b", "file-b.pdf")
            .await;

        let list = state.list_pdf_progress().await;
        assert_eq!(list.len(), 2);
    }
}
