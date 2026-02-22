//! Pipeline progress callback adapter for PDF extraction.
//!
//! ## Implements
//!
//! - [`SPEC-007`]: PDF Upload Support with progress tracking
//! - [`OODA-08`]: BroadcastingProgressCallback adapter
//! - [`OODA-10`]: Dual event system (PipelineState + ProgressBroadcaster)
//!
//! ## Use Cases
//!
//! - [`UC0710`]: User sees page-by-page progress during PDF extraction
//! - [`UC0711`]: System reports errors for specific pages via WebSocket
//!
//! ## WHY This Module?
//!
//! This adapter bridges `edgequake_pdf2md::ConversionProgressCallback` to both event systems:
//!
//! ```text
//! ┌─────────────────────┐    ┌──────────────────────────┐    ┌─────────────────┐
//! │  edgequake-pdf2md   │───►│ PipelineProgressCallback │───►│  PipelineState  │
//! │                     │    │                          │    │ (internal)      │
//! │ convert_from_bytes()│    │ on_page_complete(5,10,..)│    └─────────────────┘
//! │                     │    │   ───────────────────►   │            │
//! └─────────────────────┘    │                          │            ▼
//!                            │                          │    ┌─────────────────┐
//!                            │                          │───►│ ProgressBroad-  │
//!                            └──────────────────────────┘    │ caster (WS)     │
//!                                                            └─────────────────┘
//!                                                                    │
//!                                                                    ▼
//!                                                            ┌─────────────────┐
//!                                                            │ WebSocket       │
//!                                                            │ clients         │
//!                                                            └─────────────────┘
//! ```

use crate::handlers::websocket_types::ProgressEvent;
use crate::handlers::ProgressBroadcaster;
use edgequake_pdf2md::ConversionProgressCallback;
use edgequake_storage::traits::KVStorage;
use edgequake_tasks::progress::PipelinePhase;
use edgequake_tasks::PipelineState;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::runtime::Handle;

/// Adapter that forwards PDF extraction progress to PipelineState and ProgressBroadcaster.
///
/// ## OODA-10: Dual Event System
///
/// This adapter sends events to **both** systems:
/// 1. `PipelineState` - For internal pipeline coordination (edgequake-tasks)
/// 2. `ProgressBroadcaster` - For WebSocket clients (edgequake-api)
///
/// ## Example
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use edgequake_api::PipelineProgressCallback;
///
/// let callback = Arc::new(PipelineProgressCallback::new(
///     pipeline_state.clone(),
///     pdf_id.clone(),
///     task_id.clone(),
/// ).with_broadcaster(progress_broadcaster.clone()));
///
/// edgequake_pdf2md::convert_from_bytes(&pdf_bytes, &config).await?;
/// ```
pub struct PipelineProgressCallback {
    /// Pipeline state for emitting internal events.
    pipeline_state: PipelineState,
    /// Optional broadcaster for WebSocket clients.
    /// OODA-10: Added for dual event system.
    progress_broadcaster: Option<ProgressBroadcaster>,
    /// PDF document ID.
    pdf_id: String,
    /// Task tracking ID.
    task_id: String,
    /// Original filename for progress display.
    /// OODA-13: Added for persistent progress storage.
    filename: String,
    /// Total pages (set on extraction_start).
    total_pages: AtomicUsize,
    /// Document ID for updating metadata with progress.
    document_id: Option<String>,
    /// KV storage for updating document metadata.
    kv_storage: Option<Arc<dyn KVStorage>>,
    /// OODA-04: Tokio runtime handle for spawning async tasks from sync context.
    ///
    /// WHY: PDF extraction runs in rayon thread pool (sync), but we need to spawn
    /// async tasks for persistence. Capturing the handle at construction time allows
    /// us to spawn on the correct runtime from any thread context.
    runtime_handle: Handle,
    /// OODA-PERF-02: Last page number that triggered a metadata update.
    ///
    /// WHY: Prevents excessive KV storage writes (39 updates for 39 pages).
    /// Instead, update every N pages OR on last page for completion.
    last_metadata_page: AtomicUsize,
}

impl PipelineProgressCallback {
    /// Create a new pipeline progress callback.
    ///
    /// # Arguments
    ///
    /// * `pipeline_state` - The pipeline state for emitting events
    /// * `pdf_id` - PDF document ID for event correlation
    /// * `task_id` - Task tracking ID for event correlation
    ///
    /// # Panics
    ///
    /// Panics if called outside of a Tokio runtime context. The callback must
    /// be created from within an async context (e.g., a Tokio task or block_on).
    pub fn new(pipeline_state: PipelineState, pdf_id: String, task_id: String) -> Self {
        Self {
            pipeline_state,
            progress_broadcaster: None,
            pdf_id,
            task_id,
            filename: String::new(),
            total_pages: AtomicUsize::new(0),
            document_id: None,
            kv_storage: None,
            // OODA-04: Capture runtime handle at construction time
            runtime_handle: Handle::current(),
            // OODA-PERF-02: Start at 0 (no pages updated yet)
            last_metadata_page: AtomicUsize::new(0),
        }
    }

    /// Add the original filename for progress display.
    ///
    /// OODA-13: Enables persistent progress storage with human-readable filename.
    #[must_use]
    pub fn with_filename(mut self, filename: String) -> Self {
        self.filename = filename;
        self
    }

    /// Add document ID and KV storage for real-time metadata updates.
    ///
    /// WHY: Updates document metadata with page-by-page progress so users see
    /// "Converting PDF: page 5/10 (50%)" in the documents list without waiting
    /// for WebSocket or manual refresh.
    #[must_use]
    pub fn with_document_metadata(
        mut self,
        document_id: String,
        kv_storage: Arc<dyn KVStorage>,
    ) -> Self {
        self.document_id = Some(document_id);
        self.kv_storage = Some(kv_storage);
        self
    }

    /// Add a ProgressBroadcaster for WebSocket event delivery.
    ///
    /// OODA-10: Enables dual event system where events go to both
    /// PipelineState (internal) and ProgressBroadcaster (WebSocket).
    #[must_use]
    pub fn with_broadcaster(mut self, broadcaster: ProgressBroadcaster) -> Self {
        self.progress_broadcaster = Some(broadcaster);
        self
    }

    /// Send a ProgressEvent to WebSocket clients if broadcaster is configured.
    fn broadcast_event(&self, event: ProgressEvent) {
        if let Some(ref broadcaster) = self.progress_broadcaster {
            // Ignore send errors (no subscribers is OK)
            broadcaster.broadcast(event);
        }
    }

    /// Update document metadata with current progress.
    ///
    /// WHY: Users polling /documents see real-time progress without WebSocket.
    fn update_document_metadata(&self, stage_message: String, stage_progress: f64) {
        if let (Some(ref doc_id), Some(ref kv)) = (&self.document_id, &self.kv_storage) {
            let doc_id = doc_id.clone();
            let kv = Arc::clone(kv);
            let handle = self.runtime_handle.clone();

            handle.spawn(async move {
                let metadata_key = format!("{}-metadata", doc_id);
                if let Ok(Some(existing)) = kv.get_by_id(&metadata_key).await {
                    if let Some(mut obj) = existing.as_object().cloned() {
                        obj.insert(
                            "stage_message".to_string(),
                            serde_json::json!(stage_message),
                        );
                        obj.insert(
                            "stage_progress".to_string(),
                            serde_json::json!(stage_progress),
                        );
                        obj.insert(
                            "updated_at".to_string(),
                            serde_json::json!(chrono::Utc::now().to_rfc3339()),
                        );

                        if let Err(e) = kv.upsert(&[(metadata_key, serde_json::json!(obj))]).await {
                            tracing::warn!("Failed to update document metadata: {}", e);
                        }
                    }
                }
            });
        }
    }
}

impl ConversionProgressCallback for PipelineProgressCallback {
    fn on_conversion_start(&self, total_pages: usize) {
        self.total_pages.store(total_pages, Ordering::SeqCst);

        // Emit start event to PipelineState (internal)
        self.pipeline_state.emit_pdf_page_progress(
            self.pdf_id.clone(),
            self.task_id.clone(),
            0,
            total_pages as u32,
            "extraction".to_string(),
            0,
            true,
            None,
        );

        // OODA-10: Also broadcast to WebSocket clients
        self.broadcast_event(ProgressEvent::PdfPageProgress {
            pdf_id: self.pdf_id.clone(),
            task_id: self.task_id.clone(),
            page_num: 0,
            total_pages: total_pages as u32,
            phase: "extraction".to_string(),
            markdown_len: 0,
            success: true,
            error: None,
        });

        // OODA-13: Persist to queryable storage (async via spawn)
        // OODA-04: Use captured runtime handle to spawn from sync context
        let state = self.pipeline_state.clone();
        let track_id = self.task_id.clone();
        let pdf_id = self.pdf_id.clone();
        let filename = self.filename.clone();
        let pages = total_pages;
        self.runtime_handle.spawn(async move {
            state
                .start_pdf_progress(&track_id, &pdf_id, &filename)
                .await;
            state
                .start_pdf_phase(&track_id, PipelinePhase::PdfConversion, pages)
                .await;
        });
    }

    fn on_page_start(&self, page_num: usize, total_pages: usize) {
        // Store total pages in case extraction_start wasn't called
        self.total_pages.store(total_pages, Ordering::SeqCst);

        // Emit "starting page N" event to PipelineState
        self.pipeline_state.emit_pdf_page_progress(
            self.pdf_id.clone(),
            self.task_id.clone(),
            page_num as u32,
            total_pages as u32,
            "extracting".to_string(),
            0,
            true,
            None,
        );

        // OODA-10: Also broadcast to WebSocket clients
        self.broadcast_event(ProgressEvent::PdfPageProgress {
            pdf_id: self.pdf_id.clone(),
            task_id: self.task_id.clone(),
            page_num: page_num as u32,
            total_pages: total_pages as u32,
            phase: "extracting".to_string(),
            markdown_len: 0,
            success: true,
            error: None,
        });
    }

    fn on_page_complete(&self, page_num: usize, total_pages: usize, markdown_len: usize) {
        // Store total_pages for use in debounce logic
        self.total_pages.store(total_pages, Ordering::SeqCst);
        let total = total_pages;

        // Emit to PipelineState
        self.pipeline_state.emit_pdf_page_progress(
            self.pdf_id.clone(),
            self.task_id.clone(),
            page_num as u32,
            total as u32,
            "extracted".to_string(),
            markdown_len,
            true,
            None,
        );

        // OODA-10: Also broadcast to WebSocket clients
        self.broadcast_event(ProgressEvent::PdfPageProgress {
            pdf_id: self.pdf_id.clone(),
            task_id: self.task_id.clone(),
            page_num: page_num as u32,
            total_pages: total as u32,
            phase: "extracted".to_string(),
            markdown_len,
            success: true,
            error: None,
        });

        // OODA-PERF-02: Update document metadata with debouncing
        // WHY: Prevents excessive KV writes (40 updates for 40 pages).
        // STRATEGY: Update on first page, every 5 pages, OR on last page.
        //
        // BUG FIX: page_num is 0-indexed (0..page_count-1) while total is
        // page_count. Previous code used `page_num >= total` which NEVER
        // matched the last page (e.g., page 39 < 40). Also, progress
        // calculation used page_num/total giving 0% for the first completed
        // page. Now uses (page_num + 1)/total for accurate 1-indexed display.
        let last_updated = self.last_metadata_page.load(Ordering::SeqCst);
        let is_first_page = page_num == 0;
        let is_last_page = page_num + 1 >= total; // Fix: 0-indexed → 39+1 >= 40 = true
        let gap_met = page_num.saturating_sub(last_updated) >= 5;
        let should_update = is_first_page || is_last_page || gap_met;

        if should_update {
            self.last_metadata_page.store(page_num, Ordering::SeqCst);

            // Use (page_num + 1) for 1-indexed display and accurate percentage
            let completed_pages = page_num + 1;
            let progress_percent = if total > 0 {
                (completed_pages as f64 / total as f64) * 100.0
            } else {
                0.0
            };
            self.update_document_metadata(
                format!(
                    "Converting PDF to Markdown: page {}/{} ({:.0}%)",
                    completed_pages, total, progress_percent
                ),
                progress_percent / 100.0, // Normalize to 0.0-1.0
            );
        }

        // OODA-13: Persist to queryable storage (async via spawn)
        // OODA-04: Use captured runtime handle to spawn from sync context
        let state = self.pipeline_state.clone();
        let track_id = self.task_id.clone();
        let page = page_num;
        let total_pages = total;
        self.runtime_handle.spawn(async move {
            state
                .update_pdf_phase(
                    &track_id,
                    PipelinePhase::PdfConversion,
                    page,
                    &format!("Extracted page {} of {}", page, total_pages),
                )
                .await;
        });
    }

    fn on_page_error(&self, page_num: usize, total_pages: usize, error: String) {
        // Store total_pages for consistency
        self.total_pages.store(total_pages, Ordering::SeqCst);
        let total = total_pages;

        // Emit to PipelineState
        self.pipeline_state.emit_pdf_page_progress(
            self.pdf_id.clone(),
            self.task_id.clone(),
            page_num as u32,
            total as u32,
            "extraction_error".to_string(),
            0,
            false,
            Some(error.clone()),
        );

        // OODA-10: Also broadcast to WebSocket clients
        self.broadcast_event(ProgressEvent::PdfPageProgress {
            pdf_id: self.pdf_id.clone(),
            task_id: self.task_id.clone(),
            page_num: page_num as u32,
            total_pages: total as u32,
            phase: "extraction_error".to_string(),
            markdown_len: 0,
            success: false,
            error: Some(error.to_string()),
        });

        // OODA-13: Update phase with error message (still tracks progress)
        // OODA-04: Use captured runtime handle to spawn from sync context
        let state = self.pipeline_state.clone();
        let track_id = self.task_id.clone();
        let page = page_num;
        let total_pages = total;
        let err_msg = error.to_string();
        self.runtime_handle.spawn(async move {
            state
                .update_pdf_phase(
                    &track_id,
                    PipelinePhase::PdfConversion,
                    page,
                    &format!("Error on page {}/{}: {}", page, total_pages, err_msg),
                )
                .await;
        });
    }

    fn on_conversion_complete(&self, total_pages: usize, success_count: usize) {
        // Emit completion event
        let phase = if success_count == total_pages {
            "complete".to_string()
        } else {
            format!("partial_complete_{}_of_{}", success_count, total_pages)
        };
        let error_msg = if success_count < total_pages {
            Some(format!(
                "Extracted {}/{} pages successfully",
                success_count, total_pages
            ))
        } else {
            None
        };

        // Emit to PipelineState
        self.pipeline_state.emit_pdf_page_progress(
            self.pdf_id.clone(),
            self.task_id.clone(),
            total_pages as u32,
            total_pages as u32,
            phase.clone(),
            0,
            success_count > 0,
            error_msg.clone(),
        );

        // OODA-10: Also broadcast to WebSocket clients
        self.broadcast_event(ProgressEvent::PdfPageProgress {
            pdf_id: self.pdf_id.clone(),
            task_id: self.task_id.clone(),
            page_num: total_pages as u32,
            total_pages: total_pages as u32,
            phase,
            markdown_len: 0,
            success: success_count > 0,
            error: error_msg,
        });

        // BUG FIX: Update KV metadata to 100% on completion.
        // WHY: Previously, on_page_complete's debounce might skip the last page
        // (e.g., stuck at 35/40). This ensures the metadata always reaches 100%
        // when extraction finishes, so users see complete progress in the UI
        // even before the next pipeline stage begins.
        self.update_document_metadata(
            format!(
                "PDF conversion complete: {}/{} pages extracted",
                success_count, total_pages
            ),
            1.0,
        );

        // OODA-13: Complete the PdfConversion phase in persistent storage
        // OODA-04: Use captured runtime handle to spawn from sync context
        let state = self.pipeline_state.clone();
        let track_id = self.task_id.clone();
        self.runtime_handle.spawn(async move {
            state
                .complete_pdf_phase(&track_id, PipelinePhase::PdfConversion)
                .await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pipeline_progress_callback_page_complete() {
        // Create a pipeline state and subscribe to events
        let state = PipelineState::new();
        let mut rx = state.subscribe();

        let callback = PipelineProgressCallback::new(
            state.clone(),
            "pdf-123".to_string(),
            "task-456".to_string(),
        );

        // Simulate extraction flow
        callback.on_conversion_start(10);
        callback.on_page_complete(5, 10, 2048);

        // Skip the start event
        let _ = rx.try_recv();

        // Verify page complete event
        let event = rx.try_recv().unwrap();
        match event {
            edgequake_tasks::PipelineEvent::PdfPageProgress {
                pdf_id,
                task_id,
                page_num,
                total_pages,
                markdown_len,
                success,
                ..
            } => {
                assert_eq!(pdf_id, "pdf-123");
                assert_eq!(task_id, "task-456");
                assert_eq!(page_num, 5);
                assert_eq!(total_pages, 10);
                assert_eq!(markdown_len, 2048);
                assert!(success);
            }
            _ => panic!("Expected PdfPageProgress event"),
        }
    }

    #[tokio::test]
    async fn test_pipeline_progress_callback_page_error() {
        let state = PipelineState::new();
        let mut rx = state.subscribe();

        let callback = PipelineProgressCallback::new(
            state.clone(),
            "pdf-err".to_string(),
            "task-err".to_string(),
        );

        callback.on_conversion_start(5);
        callback.on_page_error(3, 5, "Corrupt image data".to_string());

        // Skip start event
        let _ = rx.try_recv();

        let event = rx.try_recv().unwrap();
        match event {
            edgequake_tasks::PipelineEvent::PdfPageProgress {
                page_num,
                success,
                error,
                phase,
                ..
            } => {
                assert_eq!(page_num, 3);
                assert!(!success);
                assert_eq!(phase, "extraction_error");
                assert!(error.unwrap().contains("Corrupt image"));
            }
            _ => panic!("Expected PdfPageProgress event"),
        }
    }

    #[tokio::test]
    async fn test_pipeline_progress_callback_complete() {
        let state = PipelineState::new();
        let mut rx = state.subscribe();

        let callback = PipelineProgressCallback::new(
            state.clone(),
            "pdf-done".to_string(),
            "task-done".to_string(),
        );

        callback.on_conversion_start(10);
        callback.on_conversion_complete(10, 10);

        // Skip start event
        let _ = rx.try_recv();

        let event = rx.try_recv().unwrap();
        match event {
            edgequake_tasks::PipelineEvent::PdfPageProgress {
                phase,
                success,
                error,
                ..
            } => {
                assert_eq!(phase, "complete");
                assert!(success);
                assert!(error.is_none());
            }
            _ => panic!("Expected PdfPageProgress event"),
        }
    }

    #[tokio::test]
    async fn test_pipeline_progress_callback_partial_complete() {
        let state = PipelineState::new();
        let mut rx = state.subscribe();

        let callback = PipelineProgressCallback::new(
            state.clone(),
            "pdf-partial".to_string(),
            "task-partial".to_string(),
        );

        callback.on_conversion_start(10);
        callback.on_conversion_complete(10, 8); // 2 pages failed

        // Skip start event
        let _ = rx.try_recv();

        let event = rx.try_recv().unwrap();
        match event {
            edgequake_tasks::PipelineEvent::PdfPageProgress {
                phase,
                success,
                error,
                ..
            } => {
                assert!(phase.contains("partial"));
                assert!(success); // Still success because some pages worked
                assert!(error.unwrap().contains("8/10"));
            }
            _ => panic!("Expected PdfPageProgress event"),
        }
    }

    /// OODA-10: Test that with_broadcaster enables dual event delivery.
    #[tokio::test]
    async fn test_pipeline_progress_callback_with_broadcaster() {
        let state = PipelineState::new();
        let _internal_rx = state.subscribe();

        // Create broadcaster and subscribe BEFORE callback fires events
        let broadcaster = ProgressBroadcaster::new(16);
        let mut ws_rx = broadcaster.subscribe();

        let callback = PipelineProgressCallback::new(
            state.clone(),
            "pdf-ws-test".to_string(),
            "task-ws-test".to_string(),
        )
        .with_broadcaster(broadcaster);

        // Fire an event
        callback.on_conversion_start(5);

        // Verify WebSocket subscriber received the event
        let ws_event = ws_rx.try_recv().unwrap();
        match ws_event {
            ProgressEvent::PdfPageProgress {
                pdf_id,
                task_id,
                page_num,
                total_pages,
                phase,
                success,
                ..
            } => {
                assert_eq!(pdf_id, "pdf-ws-test");
                assert_eq!(task_id, "task-ws-test");
                assert_eq!(page_num, 0);
                assert_eq!(total_pages, 5);
                assert_eq!(phase, "extraction");
                assert!(success);
            }
            _ => panic!("Expected PdfPageProgress event from broadcaster"),
        }
    }

    /// OODA-13: Test that callbacks persist progress to queryable storage.
    #[tokio::test]
    async fn test_pipeline_progress_callback_persists_progress() {
        use edgequake_tasks::progress::PhaseStatus;

        let state = PipelineState::new();
        let _internal_rx = state.subscribe();

        let callback = PipelineProgressCallback::new(
            state.clone(),
            "pdf-persist-test".to_string(),
            "task-persist-test".to_string(),
        )
        .with_filename("test_document.pdf".to_string());

        // Fire extraction start and page complete
        callback.on_conversion_start(10);
        callback.on_page_complete(5, 10, 2048);

        // Wait for spawned tasks to complete
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Verify progress was persisted
        let progress = state.get_pdf_progress("task-persist-test").await;
        assert!(progress.is_some(), "Progress should be stored");

        let progress = progress.unwrap();
        assert_eq!(progress.track_id, "task-persist-test");
        assert_eq!(progress.pdf_id, "pdf-persist-test");
        assert_eq!(progress.filename, "test_document.pdf");

        // PdfConversion phase should be active (index 1)
        let pdf_phase = &progress.phases[PipelinePhase::PdfConversion.index()];
        assert_eq!(pdf_phase.status, PhaseStatus::Active);
        assert_eq!(pdf_phase.total, 10);
        assert_eq!(pdf_phase.current, 5);
    }

    /// OODA-13: Test that on_extraction_complete marks phase as completed.
    #[tokio::test]
    async fn test_pipeline_progress_callback_completes_phase() {
        use edgequake_tasks::progress::PhaseStatus;

        let state = PipelineState::new();
        let _internal_rx = state.subscribe();

        let callback = PipelineProgressCallback::new(
            state.clone(),
            "pdf-complete-test".to_string(),
            "task-complete-test".to_string(),
        )
        .with_filename("completed.pdf".to_string());

        // Full extraction flow
        callback.on_conversion_start(5);
        callback.on_page_complete(1, 5, 1000);
        callback.on_page_complete(2, 5, 1000);
        callback.on_page_complete(3, 5, 1000);
        callback.on_page_complete(4, 5, 1000);
        callback.on_page_complete(5, 5, 1000);
        callback.on_conversion_complete(5, 5);

        // Wait for spawned tasks to complete
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Verify phase is marked as complete
        let progress = state.get_pdf_progress("task-complete-test").await;
        assert!(progress.is_some());

        let progress = progress.unwrap();
        let pdf_phase = &progress.phases[PipelinePhase::PdfConversion.index()];
        assert_eq!(pdf_phase.status, PhaseStatus::Complete);
        assert_eq!(pdf_phase.current, 5);
        assert_eq!(pdf_phase.total, 5);
    }
}
