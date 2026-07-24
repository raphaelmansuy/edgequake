//! DTOs for WebSocket progress streaming.
//!
//! This module contains the core types used for real-time progress streaming
//! via WebSocket connections.

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

// ============================================================================
// Deletion Phase Types (SPEC-050)
// ============================================================================

/// Discrete phases of a single-document deletion operation.
///
/// @implements SPEC-050: Delete progress parity with ingestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeletionPhaseKind {
    /// Cancelling an in-flight ingestion task (if document is processing).
    CancellingTask,
    /// Removing vector embeddings from pgvector.
    RemovingVectors,
    /// Cascading removal of graph entities and edges.
    RemovingGraph,
    /// Removing KV records (chunks, content, metadata).
    RemovingKv,
    /// Cleanup finalisation (content-hash key, relational rows).
    Finalizing,
}

impl DeletionPhaseKind {
    /// Human-readable label for this phase.
    pub fn label(&self) -> &'static str {
        match self {
            Self::CancellingTask => "Cancelling in-flight task",
            Self::RemovingVectors => "Removing vector embeddings",
            Self::RemovingGraph => "Removing graph entities & edges",
            Self::RemovingKv => "Removing document records",
            Self::Finalizing => "Finalizing",
        }
    }
}

// ============================================================================
// Progress Event Types
// ============================================================================

/// Progress event types sent over WebSocket.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum ProgressEvent {
    /// Pipeline job started.
    JobStarted {
        job_name: String,
        total_documents: u32,
        total_batches: u32,
    },
    /// Document processing progress.
    DocumentProgress {
        document_id: String,
        entities_extracted: usize,
        processed: u32,
        total: u32,
    },
    /// Document processing failed.
    DocumentFailed {
        document_id: String,
        error: String,
        processed: u32,
        total: u32,
    },
    /// Batch completed.
    BatchCompleted { batch: u32, total_batches: u32 },
    /// Pipeline job finished.
    JobFinished {
        total_processed: u32,
        duration_ms: u64,
    },
    /// Pipeline message (info, warn, error).
    Message {
        level: String,
        message: String,
        timestamp: String,
    },
    /// Status snapshot (for initial sync or periodic updates).
    StatusSnapshot {
        is_busy: bool,
        job_name: Option<String>,
        processed_documents: u32,
        total_documents: u32,
        current_batch: u32,
        total_batches: u32,
    },
    /// Heartbeat/ping for connection keepalive.
    Heartbeat { timestamp: String },
    /// Connection established confirmation.
    Connected { message: String },
    /// Cancellation requested.
    CancellationRequested,
    /// Chunk extraction failure notification.
    ///
    /// @implements SPEC-003: Chunk-level resilience with failure visibility
    ///
    /// WHY: When using process_with_resilience, some chunks may fail while
    /// others succeed. This event notifies WebSocket clients about individual
    /// chunk failures for UI display and potential retry.
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
    /// PDF page-level extraction progress.
    ///
    /// @implements SPEC-001-upload-pdf: Page-by-page progress during PDF conversion
    /// @implements OODA-06: Add PdfPageProgress event
    ///
    /// WHY: Large PDFs (30+ pages) take significant time to extract.
    /// This event provides page-by-page feedback so users see continuous progress
    /// instead of a generic "Processing..." message.
    ///
    /// ## Event Lifecycle
    ///
    /// For a 5-page PDF, events arrive in this order:
    /// 1. PdfPageProgress { phase: "start", page_num: 0 } - extraction begins
    /// 2. PdfPageProgress { phase: "extraction", page_num: 1 } - page 1 extracting
    /// 3. PdfPageProgress { phase: "extraction", page_num: 1, success: true }
    /// 4. ... repeat for pages 2-5 ...
    /// 5. PdfPageProgress { phase: "complete", page_num: 5, success: true }
    PdfPageProgress {
        /// PDF document ID.
        pdf_id: String,
        /// Task tracking ID.
        task_id: String,
        /// Current page number (1-indexed, 0 for start/complete events).
        page_num: u32,
        /// Total pages in PDF.
        total_pages: u32,
        /// Current phase: "start", "extraction", "complete".
        phase: String,
        /// Markdown length for this page (0 if not yet rendered).
        markdown_len: usize,
        /// Whether this page completed successfully.
        success: bool,
        /// Error message if this page failed.
        error: Option<String>,
    },
    /// Chunk extraction progress (SPEC-048 DEF-02).
    ChunkProgress {
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
    },
    /// Unified-stage transition for all Insert tracks (SPEC-086).
    StageTransition {
        document_id: String,
        task_id: String,
        stage: String,
        stage_message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        stage_progress: Option<f32>,
    },
    /// Graph storage / merge progress (SPEC-048 DEF-02).
    GraphStorageProgress {
        track_id: String,
        document_id: String,
        sub_phase: String,
        sub_phase_label: String,
        entities_processed: u32,
        entities_total: u32,
        entities_created: u32,
        entities_updated: u32,
        relationships_processed: u32,
        relationships_total: u32,
        relationships_created: u32,
        relationships_updated: u32,
        elapsed_ms: u64,
        eta_ms: Option<u64>,
    },

    // -----------------------------------------------------------------------
    // Deletion events (SPEC-050)
    // -----------------------------------------------------------------------
    /// Single-document deletion started.
    ///
    /// @implements SPEC-050: Delete progress broadcast.
    DeletionStarted {
        /// Document being deleted.
        document_id: String,
        /// Transient operation ID (UUID) for tracking this deletion.
        track_id: String,
    },

    /// Phase progress within a single-document deletion.
    ///
    /// @implements SPEC-050: Phase-granular delete progress.
    DeletionPhase {
        /// Document being deleted.
        document_id: String,
        /// Operation tracking ID.
        track_id: String,
        /// Which phase is now active.
        phase: DeletionPhaseKind,
        /// Human-readable label for this phase.
        phase_label: String,
        /// Items processed so far in this phase (e.g. chunks deleted).
        items_processed: u32,
        /// Total items in this phase (0 if unknown).
        items_total: u32,
    },

    /// Single-document deletion completed (success or partial failure).
    ///
    /// @implements SPEC-050: Delete completion broadcast.
    DeletionCompleted {
        /// Document that was deleted.
        document_id: String,
        /// Operation tracking ID.
        track_id: String,
        /// Chunks removed from KV.
        chunks_deleted: usize,
        /// Entities removed from graph (exclusive to this document).
        entities_removed: usize,
        /// Relationships removed from graph.
        relationships_removed: usize,
        /// Vector embeddings deleted.
        embeddings_deleted: usize,
        /// True if one or more phases failed non-fatally (e.g. graph cascade error).
        partial_failure: bool,
        /// Description of the partial failure, if any.
        error: Option<String>,
    },

    /// Single-document deletion failed hard (status reset off `deleting`).
    DeletionFailed {
        /// Document that failed to delete.
        document_id: String,
        /// Operation tracking ID.
        track_id: String,
        /// Human-readable failure reason.
        error: String,
    },

    /// Bulk deletion started.
    ///
    /// @implements SPEC-050: Bulk delete progress broadcast.
    BulkDeletionStarted {
        /// Total documents to delete.
        total: usize,
        /// Durable wipe correlation id (task track / wipe_track_id).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        wipe_track_id: Option<String>,
        /// Workspace being wiped.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        workspace_id: Option<String>,
    },

    /// Per-document progress during a bulk deletion.
    ///
    /// @implements SPEC-050: Per-document progress during bulk delete.
    BulkDeletionItemProgress {
        /// Document that was just deleted.
        document_id: String,
        /// Number of documents deleted so far (including this one).
        completed: usize,
        /// Total documents to delete.
        total: usize,
        /// Entities removed during this document's deletion.
        entities_removed: usize,
        /// Relationships removed during this document's deletion.
        relationships_removed: usize,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        wipe_track_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        workspace_id: Option<String>,
    },

    /// Bulk deletion finished.
    ///
    /// @implements SPEC-050: Bulk delete completion broadcast.
    BulkDeletionCompleted {
        /// Total documents successfully deleted.
        deleted_count: usize,
        /// Documents skipped (still processing).
        skipped_count: usize,
        /// Total entities removed across all documents.
        total_entities_removed: usize,
        /// Total relationships removed across all documents.
        total_relationships_removed: usize,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        wipe_track_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        workspace_id: Option<String>,
    },

    /// Bulk / workspace wipe failed permanently.
    ///
    /// @implements SPEC-050: Bulk delete failure broadcast.
    BulkDeletionFailed {
        wipe_track_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        workspace_id: Option<String>,
        error_message: String,
        /// Documents purged before failure (best-effort).
        #[serde(default)]
        deleted_count: usize,
    },
}

// ============================================================================
// Progress Broadcaster
// ============================================================================

/// Arguments for [`ProgressBroadcaster::bulk_deletion_item_progress`].
#[derive(Debug, Clone, Copy)]
pub struct BulkDeletionItemProgressArgs<'a> {
    pub document_id: &'a str,
    pub completed: usize,
    pub total: usize,
    pub entities_removed: usize,
    pub relationships_removed: usize,
    pub wipe_track_id: Option<&'a str>,
    pub workspace_id: Option<&'a str>,
}

/// Broadcast channel for progress events.
///
/// This struct manages the broadcast channel that distributes progress events
/// to all connected WebSocket clients.
#[derive(Clone)]
pub struct ProgressBroadcaster {
    sender: broadcast::Sender<ProgressEvent>,
}

impl Default for ProgressBroadcaster {
    fn default() -> Self {
        Self::new(1024)
    }
}

impl ProgressBroadcaster {
    /// Create a new progress broadcaster with specified channel capacity.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Subscribe to progress events.
    pub fn subscribe(&self) -> broadcast::Receiver<ProgressEvent> {
        self.sender.subscribe()
    }

    /// Broadcast a progress event to all subscribers.
    pub fn broadcast(&self, event: ProgressEvent) {
        // Ignore send errors (no subscribers)
        let _ = self.sender.send(event);
    }

    /// Broadcast job started event.
    pub fn job_started(&self, job_name: &str, total_documents: u32, total_batches: u32) {
        self.broadcast(ProgressEvent::JobStarted {
            job_name: job_name.to_string(),
            total_documents,
            total_batches,
        });
    }

    /// Broadcast document progress event.
    pub fn document_progress(
        &self,
        document_id: &str,
        entities_extracted: usize,
        processed: u32,
        total: u32,
    ) {
        self.broadcast(ProgressEvent::DocumentProgress {
            document_id: document_id.to_string(),
            entities_extracted,
            processed,
            total,
        });
    }

    /// Broadcast document failed event.
    pub fn document_failed(&self, document_id: &str, error: &str, processed: u32, total: u32) {
        self.broadcast(ProgressEvent::DocumentFailed {
            document_id: document_id.to_string(),
            error: error.to_string(),
            processed,
            total,
        });
    }

    /// Broadcast batch completed event.
    pub fn batch_completed(&self, batch: u32, total_batches: u32) {
        self.broadcast(ProgressEvent::BatchCompleted {
            batch,
            total_batches,
        });
    }

    /// Broadcast job finished event.
    pub fn job_finished(&self, total_processed: u32, duration_ms: u64) {
        self.broadcast(ProgressEvent::JobFinished {
            total_processed,
            duration_ms,
        });
    }

    /// Broadcast a message event.
    pub fn message(&self, level: &str, message: &str) {
        self.broadcast(ProgressEvent::Message {
            level: level.to_string(),
            message: message.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }

    /// Broadcast cancellation requested event.
    pub fn cancellation_requested(&self) {
        self.broadcast(ProgressEvent::CancellationRequested);
    }

    /// Broadcast chunk failure event.
    ///
    /// @implements SPEC-003: Chunk-level resilience with failure visibility
    ///
    /// WHY: This enables the frontend to show which chunks failed during
    /// document processing and why, supporting the resilient extraction feature.
    #[allow(clippy::too_many_arguments)]
    pub fn broadcast_chunk_failure(
        &self,
        document_id: String,
        task_id: String,
        chunk_index: u32,
        total_chunks: u32,
        error_message: String,
        was_timeout: bool,
        retry_attempts: u32,
    ) {
        self.broadcast(ProgressEvent::ChunkFailure {
            document_id,
            task_id,
            chunk_index,
            total_chunks,
            error_message,
            was_timeout,
            retry_attempts,
        });
    }

    // -----------------------------------------------------------------------
    // Deletion broadcast helpers (SPEC-050)
    // -----------------------------------------------------------------------

    /// Broadcast that a single-document deletion has started.
    pub fn deletion_started(&self, document_id: &str, track_id: &str) {
        self.broadcast(ProgressEvent::DeletionStarted {
            document_id: document_id.to_string(),
            track_id: track_id.to_string(),
        });
    }

    /// Broadcast a deletion phase transition.
    pub fn deletion_phase(
        &self,
        document_id: &str,
        track_id: &str,
        phase: DeletionPhaseKind,
        items_processed: u32,
        items_total: u32,
    ) {
        self.broadcast(ProgressEvent::DeletionPhase {
            document_id: document_id.to_string(),
            track_id: track_id.to_string(),
            phase_label: phase.label().to_string(),
            phase,
            items_processed,
            items_total,
        });
    }

    /// Broadcast that a single-document deletion completed.
    #[allow(clippy::too_many_arguments)]
    pub fn deletion_completed(
        &self,
        document_id: &str,
        track_id: &str,
        chunks_deleted: usize,
        entities_removed: usize,
        relationships_removed: usize,
        embeddings_deleted: usize,
        partial_failure: bool,
        error: Option<String>,
    ) {
        self.broadcast(ProgressEvent::DeletionCompleted {
            document_id: document_id.to_string(),
            track_id: track_id.to_string(),
            chunks_deleted,
            entities_removed,
            relationships_removed,
            embeddings_deleted,
            partial_failure,
            error,
        });
    }

    /// Broadcast that a single-document deletion failed (self-heal terminal).
    pub fn deletion_failed(&self, document_id: &str, track_id: &str, error: &str) {
        self.broadcast(ProgressEvent::DeletionFailed {
            document_id: document_id.to_string(),
            track_id: track_id.to_string(),
            error: error.to_string(),
        });
    }

    /// Broadcast that a bulk deletion has started.
    pub fn bulk_deletion_started(
        &self,
        total: usize,
        wipe_track_id: Option<&str>,
        workspace_id: Option<&str>,
    ) {
        self.broadcast(ProgressEvent::BulkDeletionStarted {
            total,
            wipe_track_id: wipe_track_id.map(|s| s.to_string()),
            workspace_id: workspace_id.map(|s| s.to_string()),
        });
    }

    /// Broadcast per-document progress during a bulk deletion.
    pub fn bulk_deletion_item_progress(&self, args: BulkDeletionItemProgressArgs<'_>) {
        self.broadcast(ProgressEvent::BulkDeletionItemProgress {
            document_id: args.document_id.to_string(),
            completed: args.completed,
            total: args.total,
            entities_removed: args.entities_removed,
            relationships_removed: args.relationships_removed,
            wipe_track_id: args.wipe_track_id.map(|s| s.to_string()),
            workspace_id: args.workspace_id.map(|s| s.to_string()),
        });
    }

    /// Broadcast that a bulk deletion has finished.
    pub fn bulk_deletion_completed(
        &self,
        deleted_count: usize,
        skipped_count: usize,
        total_entities_removed: usize,
        total_relationships_removed: usize,
        wipe_track_id: Option<&str>,
        workspace_id: Option<&str>,
    ) {
        self.broadcast(ProgressEvent::BulkDeletionCompleted {
            deleted_count,
            skipped_count,
            total_entities_removed,
            total_relationships_removed,
            wipe_track_id: wipe_track_id.map(|s| s.to_string()),
            workspace_id: workspace_id.map(|s| s.to_string()),
        });
    }

    /// Broadcast that a durable workspace wipe failed permanently.
    pub fn bulk_deletion_failed(
        &self,
        wipe_track_id: &str,
        workspace_id: Option<&str>,
        error_message: &str,
        deleted_count: usize,
    ) {
        self.broadcast(ProgressEvent::BulkDeletionFailed {
            wipe_track_id: wipe_track_id.to_string(),
            workspace_id: workspace_id.map(|s| s.to_string()),
            error_message: error_message.to_string(),
            deleted_count,
        });
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_event_job_started_serialization() {
        let event = ProgressEvent::JobStarted {
            job_name: "test-job".to_string(),
            total_documents: 10,
            total_batches: 2,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("JobStarted"));
        assert!(json.contains("test-job"));
        assert!(json.contains("10"));
    }

    #[test]
    fn test_progress_event_document_progress_serialization() {
        let event = ProgressEvent::DocumentProgress {
            document_id: "doc-1".to_string(),
            entities_extracted: 5,
            processed: 1,
            total: 10,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("DocumentProgress"));
        assert!(json.contains("doc-1"));
    }

    #[test]
    fn test_progress_event_document_failed_serialization() {
        let event = ProgressEvent::DocumentFailed {
            document_id: "doc-2".to_string(),
            error: "Parse error".to_string(),
            processed: 2,
            total: 10,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("DocumentFailed"));
        assert!(json.contains("Parse error"));
    }

    #[test]
    fn test_progress_event_batch_completed_serialization() {
        let event = ProgressEvent::BatchCompleted {
            batch: 1,
            total_batches: 5,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("BatchCompleted"));
    }

    #[test]
    fn test_progress_event_job_finished_serialization() {
        let event = ProgressEvent::JobFinished {
            total_processed: 100,
            duration_ms: 5000,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("JobFinished"));
        assert!(json.contains("5000"));
    }

    #[test]
    fn test_progress_event_message_serialization() {
        let event = ProgressEvent::Message {
            level: "info".to_string(),
            message: "Processing started".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Message"));
        assert!(json.contains("Processing started"));
    }

    #[test]
    fn test_progress_event_status_snapshot_serialization() {
        let event = ProgressEvent::StatusSnapshot {
            is_busy: true,
            job_name: Some("ingestion".to_string()),
            processed_documents: 50,
            total_documents: 100,
            current_batch: 3,
            total_batches: 10,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("StatusSnapshot"));
        assert!(json.contains("ingestion"));
    }

    #[test]
    fn test_progress_event_heartbeat_serialization() {
        let event = ProgressEvent::Heartbeat {
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Heartbeat"));
    }

    #[test]
    fn test_progress_event_connected_serialization() {
        let event = ProgressEvent::Connected {
            message: "Connected successfully".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Connected"));
    }

    #[test]
    fn test_progress_event_cancellation_serialization() {
        let event = ProgressEvent::CancellationRequested;

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("CancellationRequested"));
    }

    #[tokio::test]
    async fn test_progress_broadcaster_creation() {
        let broadcaster = ProgressBroadcaster::new(100);
        let mut rx = broadcaster.subscribe();

        broadcaster.job_started("test-job", 10, 2);

        let event = rx.recv().await.unwrap();
        match event {
            ProgressEvent::JobStarted {
                job_name,
                total_documents,
                total_batches,
            } => {
                assert_eq!(job_name, "test-job");
                assert_eq!(total_documents, 10);
                assert_eq!(total_batches, 2);
            }
            _ => panic!("Unexpected event type"),
        }
    }

    #[tokio::test]
    async fn test_progress_broadcaster_multiple_subscribers() {
        let broadcaster = ProgressBroadcaster::new(100);
        let mut rx1 = broadcaster.subscribe();
        let mut rx2 = broadcaster.subscribe();

        broadcaster.document_progress("doc-1", 5, 1, 10);

        let event1 = rx1.recv().await.unwrap();
        let event2 = rx2.recv().await.unwrap();

        match (&event1, &event2) {
            (
                ProgressEvent::DocumentProgress {
                    document_id: id1, ..
                },
                ProgressEvent::DocumentProgress {
                    document_id: id2, ..
                },
            ) => {
                assert_eq!(id1, "doc-1");
                assert_eq!(id2, "doc-1");
            }
            _ => panic!("Unexpected event types"),
        }
    }

    /// Test PdfPageProgress event serialization.
    ///
    /// @implements OODA-06: Verify PdfPageProgress event structure
    #[test]
    fn test_progress_event_pdf_page_progress_serialization() {
        let event = ProgressEvent::PdfPageProgress {
            pdf_id: "pdf-123".to_string(),
            task_id: "task-456".to_string(),
            page_num: 5,
            total_pages: 30,
            phase: "extraction".to_string(),
            markdown_len: 1024,
            success: true,
            error: None,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("PdfPageProgress"));
        assert!(json.contains("pdf-123"));
        assert!(json.contains("\"page_num\":5"));
        assert!(json.contains("\"total_pages\":30"));
        assert!(json.contains("\"phase\":\"extraction\""));
        assert!(json.contains("\"success\":true"));
    }

    /// Test PdfPageProgress event with error.
    #[test]
    fn test_progress_event_pdf_page_progress_with_error() {
        let event = ProgressEvent::PdfPageProgress {
            pdf_id: "pdf-123".to_string(),
            task_id: "task-456".to_string(),
            page_num: 3,
            total_pages: 10,
            phase: "extraction".to_string(),
            markdown_len: 0,
            success: false,
            error: Some("Failed to decode font".to_string()),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("PdfPageProgress"));
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("Failed to decode font"));
    }
}
