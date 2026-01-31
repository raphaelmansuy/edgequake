//! Pipeline progress callback adapter for PDF extraction.
//!
//! ## Implements
//!
//! - [`SPEC-007`]: PDF Upload Support with progress tracking
//! - [`OODA-08`]: BroadcastingProgressCallback adapter
//!
//! ## Use Cases
//!
//! - [`UC0710`]: User sees page-by-page progress during PDF extraction
//! - [`UC0711`]: System reports errors for specific pages via WebSocket
//!
//! ## WHY This Module?
//!
//! This adapter bridges `edgequake_pdf::ProgressCallback` to `PipelineState` events:
//!
//! ```text
//! ┌─────────────────────┐    ┌──────────────────────────┐    ┌─────────────────┐
//! │   PdfExtractor      │───►│ PipelineProgressCallback │───►│  PipelineState  │
//! │                     │    │                          │    │                 │
//! │ extract_with_       │    │ on_page_complete(5, 2048)│    │ emit_pdf_page_  │
//! │   progress(callback)│    │   ───────────────────►   │    │   progress(...) │
//! └─────────────────────┘    └──────────────────────────┘    └─────────────────┘
//!                                       │
//!                                       ▼
//!                            ┌─────────────────────┐
//!                            │  WebSocket clients  │
//!                            │  (real-time events) │
//!                            └─────────────────────┘
//! ```

use edgequake_pdf::ProgressCallback;
use edgequake_tasks::PipelineState;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Adapter that forwards PDF extraction progress to PipelineState for WebSocket broadcast.
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
/// ));
///
/// extractor.extract_to_markdown_with_progress(&pdf_bytes, callback).await?;
/// ```
pub struct PipelineProgressCallback {
    /// Pipeline state for emitting events.
    pipeline_state: PipelineState,
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
            pdf_id,
            task_id,
            total_pages: AtomicUsize::new(0),
        }
    }
}

impl ProgressCallback for PipelineProgressCallback {
    fn on_extraction_start(&self, total_pages: usize) {
        self.total_pages.store(total_pages, Ordering::SeqCst);

        // Emit start event (page 0 indicates "starting")
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
    }

    fn on_page_start(&self, page_num: usize, total_pages: usize) {
        // Store total pages in case extraction_start wasn't called
        self.total_pages.store(total_pages, Ordering::SeqCst);

        // Emit "starting page N" event
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
    }

    fn on_page_complete(&self, page_num: usize, markdown_len: usize) {
        let total = self.total_pages.load(Ordering::SeqCst);

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
    }

    fn on_page_error(&self, page_num: usize, error: &str) {
        let total = self.total_pages.load(Ordering::SeqCst);

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
    }

    fn on_extraction_complete(&self, total_pages: usize, success_count: usize) {
        // Emit completion event
        let phase = if success_count == total_pages {
            "complete".to_string()
        } else {
            format!("partial_complete_{}_of_{}", success_count, total_pages)
        };

        self.pipeline_state.emit_pdf_page_progress(
            self.pdf_id.clone(),
            self.task_id.clone(),
            total_pages as u32,
            total_pages as u32,
            phase,
            0,
            success_count > 0,
            if success_count < total_pages {
                Some(format!(
                    "Extracted {}/{} pages successfully",
                    success_count, total_pages
                ))
            } else {
                None
            },
        );
    }

    fn on_progress(&self, phase: &str, percent: f32) {
        // Convert percentage to approximate page number
        let total = self.total_pages.load(Ordering::SeqCst);
        let approx_page = ((percent / 100.0) * total as f32).ceil() as u32;

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
}
