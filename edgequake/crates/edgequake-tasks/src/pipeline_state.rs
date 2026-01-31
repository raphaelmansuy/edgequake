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
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::progress::{PdfUploadProgress, PhaseError, PipelinePhase};

/// Events emitted by the pipeline for real-time updates.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum PipelineEvent {
    /// A new log message.
    Log(PipelineMessage),
    /// Progress update.
    Progress {
        processed: u32,
        total: u32,
        batch: u32,
        total_batches: u32,
    },
    /// State change (start/stop).
    StateChange {
        is_busy: bool,
        job_name: Option<String>,
    },
    /// Chunk-level progress update for a document.
    ///
    /// @implements SPEC-001/Objective-A: Chunk-Level Progress Visibility
    ///
    /// WHY: This event provides real-time chunk-level progress to WebSocket
    /// clients, enabling the frontend to show granular progress like:
    /// "Chunk 12/35 (34%) - ETA: 53s"
    ChunkProgress {
        /// Document being processed.
        document_id: String,
        /// Task tracking ID.
        task_id: String,
        /// Current chunk index (0-based).
        chunk_index: u32,
        /// Total chunks in document.
        total_chunks: u32,
        /// Preview of current chunk (first 80 chars).
        chunk_preview: String,
        /// Time taken for this chunk (milliseconds).
        time_ms: u64,
        /// Estimated time remaining (seconds).
        eta_seconds: u64,
        /// Cumulative input tokens.
        tokens_in: u64,
        /// Cumulative output tokens.
        tokens_out: u64,
        /// Cumulative cost (USD).
        cost_usd: f64,
    },
    /// Chunk extraction failure notification.
    ///
    /// @implements SPEC-003: Chunk-level resilience with failure visibility
    ///
    /// WHY: When using process_with_resilience, some chunks may fail while
    /// others succeed. This event notifies WebSocket clients about individual
    /// chunk failures, enabling:
    /// - UI display of which chunks failed
    /// - Error details for debugging
    /// - Potential retry functionality
    ChunkFailure {
        /// Document being processed.
        document_id: String,
        /// Task tracking ID.
        task_id: String,
        /// Failed chunk index (0-based).
        chunk_index: u32,
        /// Total chunks in document.
        total_chunks: u32,
        /// Error message describing the failure.
        error_message: String,
        /// Whether the failure was due to timeout.
        was_timeout: bool,
        /// Number of retry attempts before giving up.
        retry_attempts: u32,
    },
    /// PDF page extraction progress notification.
    ///
    /// @implements SPEC-007: PDF Upload Support with progress tracking
    /// @implements OODA-07: PDF page-level progress visibility
    ///
    /// WHY: When extracting PDF to Markdown, users need real-time feedback
    /// on which page is being processed. This event enables:
    /// - UI display like "Extracting page 5/10 (50%)"
    /// - Error isolation per page
    /// - ETA calculation based on pages remaining
    PdfPageProgress {
        /// PDF document being processed.
        pdf_id: String,
        /// Task tracking ID.
        task_id: String,
        /// Current page number (1-based for display).
        page_num: u32,
        /// Total pages in PDF.
        total_pages: u32,
        /// Processing phase: "extraction", "rendering", etc.
        phase: String,
        /// Length of markdown generated for this page.
        markdown_len: usize,
        /// Whether page extraction succeeded.
        success: bool,
        /// Error message if extraction failed.
        error: Option<String>,
    },
}

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
    /// OODA-12: Active PDF upload progress, keyed by track_id.
    /// Enables queryable progress for GET /api/v1/documents/pdf/:id/progress
    pdf_progress: HashMap<String, PdfUploadProgress>,
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
        }
    }
}

/// Thread-safe pipeline state for tracking document processing.
#[derive(Clone)]
pub struct PipelineState {
    inner: Arc<RwLock<PipelineStateInner>>,
    tx: broadcast::Sender<PipelineEvent>,
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

    /// Emit a chunk-level progress event.
    ///
    /// @implements SPEC-001/Objective-A: Chunk-Level Progress Visibility
    ///
    /// WHY: This method sends real-time chunk progress to WebSocket subscribers,
    /// enabling the frontend to display granular progress like:
    /// "Chunk 12/35 (34%) - ETA: 53s - Cost: $0.0089"
    ///
    /// # Arguments
    ///
    /// * `document_id` - ID of the document being processed
    /// * `task_id` - Task tracking ID
    /// * `chunk_index` - Current chunk index (0-based)
    /// * `total_chunks` - Total chunks in document
    /// * `chunk_preview` - Preview text of current chunk (first ~80 chars)
    /// * `time_ms` - Time taken for this chunk (milliseconds)
    /// * `eta_seconds` - Estimated time remaining (seconds)
    /// * `tokens_in` - Cumulative input tokens
    /// * `tokens_out` - Cumulative output tokens
    /// * `cost_usd` - Cumulative cost (USD)
    #[allow(clippy::too_many_arguments)]
    pub fn emit_chunk_progress(
        &self,
        document_id: String,
        task_id: String,
        chunk_index: u32,
        total_chunks: u32,
        chunk_preview: String,
        time_ms: u64,
        eta_seconds: u64,
        tokens_in: u64,
        tokens_out: u64,
        cost_usd: f64,
    ) {
        let _ = self.tx.send(PipelineEvent::ChunkProgress {
            document_id,
            task_id,
            chunk_index,
            total_chunks,
            chunk_preview,
            time_ms,
            eta_seconds,
            tokens_in,
            tokens_out,
            cost_usd,
        });
    }

    /// Emit a chunk failure event.
    ///
    /// @implements SPEC-003: Chunk-level resilience with failure visibility
    ///
    /// WHY: This method sends real-time chunk failure notifications to WebSocket
    /// subscribers, enabling the frontend to display which chunks failed and why.
    /// This is part of the resilient extraction feature where partial failures
    /// don't abort the entire document.
    ///
    /// # Arguments
    ///
    /// * `document_id` - ID of the document being processed
    /// * `task_id` - Task tracking ID
    /// * `chunk_index` - Failed chunk index (0-based)
    /// * `total_chunks` - Total chunks in document
    /// * `error_message` - Error description
    /// * `was_timeout` - Whether failure was due to timeout
    /// * `retry_attempts` - Number of retries attempted
    #[allow(clippy::too_many_arguments)]
    pub fn emit_chunk_failure(
        &self,
        document_id: String,
        task_id: String,
        chunk_index: u32,
        total_chunks: u32,
        error_message: String,
        was_timeout: bool,
        retry_attempts: u32,
    ) {
        let _ = self.tx.send(PipelineEvent::ChunkFailure {
            document_id,
            task_id,
            chunk_index,
            total_chunks,
            error_message,
            was_timeout,
            retry_attempts,
        });
    }

    /// Emit a PDF page progress event.
    ///
    /// @implements SPEC-007: PDF Upload Support with progress tracking
    /// @implements OODA-07: PDF page-level progress visibility
    ///
    /// WHY: This method sends real-time PDF extraction progress to WebSocket
    /// subscribers, enabling the frontend to display page-by-page progress
    /// like "Extracting page 5 of 10...".
    ///
    /// # Arguments
    ///
    /// * `pdf_id` - ID of the PDF being processed
    /// * `task_id` - Task tracking ID
    /// * `page_num` - Current page number (1-based)
    /// * `total_pages` - Total pages in PDF
    /// * `phase` - Processing phase (e.g., "extraction", "rendering")
    /// * `markdown_len` - Length of markdown generated for this page
    /// * `success` - Whether page extraction succeeded
    /// * `error` - Error message if extraction failed
    #[allow(clippy::too_many_arguments)]
    pub fn emit_pdf_page_progress(
        &self,
        pdf_id: String,
        task_id: String,
        page_num: u32,
        total_pages: u32,
        phase: String,
        markdown_len: usize,
        success: bool,
        error: Option<String>,
    ) {
        let _ = self.tx.send(PipelineEvent::PdfPageProgress {
            pdf_id,
            task_id,
            page_num,
            total_pages,
            phase,
            markdown_len,
            success,
            error,
        });
    }

    // =========================================================================
    // OODA-12: PDF Upload Progress Tracking Methods
    // =========================================================================
    //
    // These methods provide queryable PDF upload progress storage.
    // Progress is stored in-memory and keyed by track_id.
    // This enables the GET /api/v1/documents/pdf/:id/progress endpoint.

    /// Start tracking a new PDF upload.
    ///
    /// Creates a new `PdfUploadProgress` entry with all 6 phases set to Pending.
    /// Call this when PDF processing begins.
    ///
    /// # Arguments
    ///
    /// * `track_id` - Unique tracking ID for this upload
    /// * `pdf_id` - PDF document ID
    /// * `filename` - Original filename for display
    pub async fn start_pdf_progress(&self, track_id: &str, pdf_id: &str, filename: &str) {
        let progress = PdfUploadProgress::new(
            track_id.to_string(),
            pdf_id.to_string(),
            filename.to_string(),
        );
        let mut inner = self.inner.write().await;
        inner.pdf_progress.insert(track_id.to_string(), progress);
    }

    /// Get current progress for a PDF upload.
    ///
    /// Returns `None` if no progress exists for this track_id (either not started
    /// or already cleaned up).
    pub async fn get_pdf_progress(&self, track_id: &str) -> Option<PdfUploadProgress> {
        let inner = self.inner.read().await;
        inner.pdf_progress.get(track_id).cloned()
    }

    /// Start a phase with known total items.
    ///
    /// Use this when beginning a phase like PdfConversion with total_pages.
    ///
    /// # Arguments
    ///
    /// * `track_id` - Upload tracking ID
    /// * `phase` - Which pipeline phase is starting
    /// * `total` - Total items to process (pages, chunks, entities)
    pub async fn start_pdf_phase(&self, track_id: &str, phase: PipelinePhase, total: usize) {
        let mut inner = self.inner.write().await;
        if let Some(progress) = inner.pdf_progress.get_mut(track_id) {
            progress.start_phase(phase, total);
        }
    }

    /// Update progress for a phase.
    ///
    /// # Arguments
    ///
    /// * `track_id` - Upload tracking ID
    /// * `phase` - Which phase to update
    /// * `current` - Current item index (0-based)
    /// * `message` - Human-readable status message
    pub async fn update_pdf_phase(
        &self,
        track_id: &str,
        phase: PipelinePhase,
        current: usize,
        message: &str,
    ) {
        let mut inner = self.inner.write().await;
        if let Some(progress) = inner.pdf_progress.get_mut(track_id) {
            progress.update_phase(phase, current, message);
        }
    }

    /// Mark a phase as complete.
    pub async fn complete_pdf_phase(&self, track_id: &str, phase: PipelinePhase) {
        let mut inner = self.inner.write().await;
        if let Some(progress) = inner.pdf_progress.get_mut(track_id) {
            progress.complete_phase(phase);
        }
    }

    /// Mark a phase as failed.
    pub async fn fail_pdf_phase(&self, track_id: &str, phase: PipelinePhase, error: PhaseError) {
        let mut inner = self.inner.write().await;
        if let Some(progress) = inner.pdf_progress.get_mut(track_id) {
            progress.fail_phase(phase, error);
        }
    }

    /// Remove progress entry (for cleanup after completion).
    ///
    /// Call this after the entire pipeline completes to free memory.
    /// Progress can also be garbage-collected by a background task based on age.
    pub async fn remove_pdf_progress(&self, track_id: &str) {
        let mut inner = self.inner.write().await;
        inner.pdf_progress.remove(track_id);
    }

    /// Get all active PDF progress entries (for admin/monitoring).
    pub async fn list_pdf_progress(&self) -> Vec<PdfUploadProgress> {
        let inner = self.inner.read().await;
        inner.pdf_progress.values().cloned().collect()
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
