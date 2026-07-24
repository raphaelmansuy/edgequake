//! Event emission methods for `PipelineState`.
//!
//! Contains: `emit_chunk_progress`, `emit_chunk_failure`, `emit_pdf_page_progress`,
//! `emit_stage_transition`.

use super::event::PipelineEvent;
use super::PipelineState;

impl PipelineState {
    /// Emit a chunk-level progress event.
    ///
    /// @implements SPEC-001/Objective-A: Chunk-Level Progress Visibility
    ///
    /// WHY: This method sends real-time chunk progress to WebSocket subscribers,
    /// enabling the frontend to display granular progress like:
    /// "Chunk 12/35 (34%) - ETA: 53s - Cost: $0.0089"
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

    /// Emit a unified-stage transition (SPEC-086).
    ///
    /// WHY: Small markdown docs may never hit the every-3rd-chunk metadata tick.
    /// Stage enter events keep progress UX format-agnostic without PDF page SSE.
    pub fn emit_stage_transition(
        &self,
        document_id: String,
        task_id: String,
        stage: String,
        stage_message: String,
        stage_progress: Option<f32>,
    ) {
        let _ = self.tx.send(PipelineEvent::StageTransition {
            document_id,
            task_id,
            stage,
            stage_message,
            stage_progress,
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
}
