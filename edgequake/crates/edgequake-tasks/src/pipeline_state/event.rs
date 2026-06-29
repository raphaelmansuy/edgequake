//! Pipeline event types and data structures.
//!
//! Contains: `PipelineEvent`, `PipelineMessage`, `PipelineStatusSnapshot`.

use serde::Serialize;

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

    /// Knowledge-graph merge sub-phase progress (SPEC-032 W-04).
    ///
    /// WHY: The GraphStorage pipeline phase previously emitted only start/complete.
    /// For large KGs (100K+ entities) this creates a frozen progress bar for
    /// 10–30 minutes. This event gives clients per-batch progress within the
    /// merge, enabling meaningful ETA and sub-phase indicators.
    ///
    /// Emitted every 500 ms (debounced in the broadcaster) during:
    ///   1. entity_vectors  — storing entity embeddings
    ///   2. entity_graph    — merging entities into AGE graph
    ///   3. rel_vectors     — storing relationship embeddings
    ///   4. rel_graph       — merging relationships into AGE graph
    ///   5. finalizing      — completing community index refresh
    GraphStorageProgress {
        /// Task tracking ID (same as track_id in the PDF phases).
        track_id: String,
        /// Document ID being processed.
        document_id: String,
        /// Merge sub-phase label.
        sub_phase: String,
        /// Human-readable sub-phase description.
        sub_phase_label: String,
        /// Entities merged so far.
        entities_processed: u32,
        /// Total entities to merge.
        entities_total: u32,
        /// Entities newly created in this document.
        entities_created: u32,
        /// Entities updated (merged with existing).
        entities_updated: u32,
        /// Relationships merged so far.
        relationships_processed: u32,
        /// Total relationships to merge.
        relationships_total: u32,
        /// Relationships newly created.
        relationships_created: u32,
        /// Relationships updated.
        relationships_updated: u32,
        /// Elapsed time in ms since GraphStorage phase began.
        elapsed_ms: u64,
        /// ETA in ms (null if not enough data yet).
        eta_ms: Option<u64>,
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
            timestamp: chrono::Utc::now().to_rfc3339(),
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
