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
//! This adapter bridges `edgequake_pdf::ProgressCallback` to both event systems:
//!
//! ```text
//! ┌─────────────────────┐    ┌──────────────────────────┐    ┌─────────────────┐
//! │   PdfExtractor      │───►│ PipelineProgressCallback │───►│  PipelineState  │
//! │                     │    │                          │    │ (internal)      │
//! │ extract_with_       │    │ on_page_complete(5, 2048)│    └─────────────────┘
//! │   progress(callback)│    │   ───────────────────►   │            │
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

use crate::handlers::ProgressBroadcaster;
use crate::handlers::websocket_types::ProgressEvent;
use edgequake_pdf::ProgressCallback;
use edgequake_tasks::PipelineState;
use std::sync::atomic::{AtomicUsize, Ordering};

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
/// use edgequake_pdf::PdfExtractor;
///
/// let callback = Arc::new(PipelineProgressCallback::new(
///     pipeline_state.clone(),
///     pdf_id.clone(),
///     task_id.clone(),
/// ).with_broadcaster(progress_broadcaster.clone()));
///
/// extractor.extract_to_markdown_with_progress(&pdf_bytes, callback).await?;
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
    /// Total pages (set on extraction_start).
    total_pages: AtomicUsize,
}

impl PipelineProgressCallback {
    /// Create a new pipeline progress callback.
    ///
    /// # Arguments
    ///
    /// * `pipeline_state` - The pipeline state for emitting events
    /// * `pdf_id` - PDF document ID for event correlation
    /// * `task_id` - Task tracking ID for event correlation
    pub fn new(pipeline_state: PipelineState, pdf_id: String, task_id: String) -> Self {
        Self {
            pipeline_state,
            progress_broadcaster: None,
            pdf_id,
            task_id,
            total_pages: AtomicUsize::new(0),
        }
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
}

impl ProgressCallback for PipelineProgressCallback {
    fn on_extraction_start(&self, total_pages: usize) {
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

    fn on_page_complete(&self, page_num: usize, markdown_len: usize) {
        let total = self.total_pages.load(Ordering::SeqCst);

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
    }

    fn on_page_error(&self, page_num: usize, error: &str) {
        let total = self.total_pages.load(Ordering::SeqCst);

        // Emit to PipelineState
        self.pipeline_state.emit_pdf_page_progress(
            self.pdf_id.clone(),
            self.task_id.clone(),
            page_num as u32,
            total as u32,
            "extraction_error".to_string(),
            0,
            false,
            Some(error.to_string()),
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
    }

    fn on_extraction_complete(&self, total_pages: usize, success_count: usize) {
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
    }

    fn on_progress(&self, phase: &str, percent: f32) {
        // Convert percentage to approximate page number
        let total = self.total_pages.load(Ordering::SeqCst);
        let approx_page = ((percent / 100.0) * total as f32).ceil() as u32;

        // Emit to PipelineState
        self.pipeline_state.emit_pdf_page_progress(
            self.pdf_id.clone(),
            self.task_id.clone(),
            approx_page,
            total as u32,
            phase.to_string(),
            0,
            true,
            None,
        );

        // OODA-10: Also broadcast to WebSocket clients
        self.broadcast_event(ProgressEvent::PdfPageProgress {
            pdf_id: self.pdf_id.clone(),
            task_id: self.task_id.clone(),
            page_num: approx_page,
            total_pages: total as u32,
            phase: phase.to_string(),
            markdown_len: 0,
            success: true,
            error: None,
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
        callback.on_extraction_start(10);
        callback.on_page_complete(5, 2048);

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

        callback.on_extraction_start(5);
        callback.on_page_error(3, "Corrupt image data");

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

        callback.on_extraction_start(10);
        callback.on_extraction_complete(10, 10);

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

        callback.on_extraction_start(10);
        callback.on_extraction_complete(10, 8); // 2 pages failed

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
        callback.on_extraction_start(5);

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
}
