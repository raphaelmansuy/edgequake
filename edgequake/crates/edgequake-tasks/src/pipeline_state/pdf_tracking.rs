//! PDF upload progress tracking methods for `PipelineState`.
//!
//! OODA-12: These methods provide queryable PDF upload progress storage.
//! Progress is stored in-memory and keyed by `track_id`.
//! This enables the `GET /api/v1/documents/pdf/:id/progress` endpoint.

use std::time::Instant;

use crate::progress::{PdfUploadProgress, PhaseError, PipelinePhase};

use super::{event::PipelineEvent, PipelineState};

impl PipelineState {
    /// Start tracking a new PDF upload.
    ///
    /// Creates a new `PdfUploadProgress` entry with all 6 phases set to Pending.
    /// Call this when PDF processing begins.
    pub async fn start_pdf_progress(&self, track_id: &str, pdf_id: &str, filename: &str) {
        let progress = PdfUploadProgress::new(
            track_id.to_string(),
            pdf_id.to_string(),
            filename.to_string(),
        );
        let mut inner = self.inner.write().await;
        inner.pdf_progress.insert(track_id.to_string(), progress);
        // Reset graph storage timer
        inner.graph_storage_start.remove(track_id);
    }

    /// Get current progress for a PDF upload.
    ///
    /// Returns `None` if no progress exists for this `track_id` (either not started
    /// or already cleaned up).
    pub async fn get_pdf_progress(&self, track_id: &str) -> Option<PdfUploadProgress> {
        let inner = self.inner.read().await;
        inner.pdf_progress.get(track_id).cloned()
    }

    /// Start a phase with known total items.
    ///
    /// Use this when beginning a phase like `PdfConversion` with `total_pages`.
    pub async fn start_pdf_phase(&self, track_id: &str, phase: PipelinePhase, total: usize) {
        let mut inner = self.inner.write().await;
        if let Some(progress) = inner.pdf_progress.get_mut(track_id) {
            progress.start_phase(phase, total);
        }
        // Record start time for GraphStorage phase (for ETA calculation)
        if phase == PipelinePhase::GraphStorage {
            inner
                .graph_storage_start
                .insert(track_id.to_string(), Instant::now());
        }
    }

    /// Update progress for a phase.
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
    pub async fn remove_pdf_progress(&self, track_id: &str) {
        let mut inner = self.inner.write().await;
        inner.pdf_progress.remove(track_id);
        inner.graph_storage_start.remove(track_id);
    }

    /// Get all active PDF progress entries (for admin/monitoring).
    pub async fn list_pdf_progress(&self) -> Vec<PdfUploadProgress> {
        let inner = self.inner.read().await;
        inner.pdf_progress.values().cloned().collect()
    }

    /// Broadcast a knowledge-graph merge sub-phase progress event (SPEC-032 W-04).
    ///
    /// This method is called from the `MergeProgressCallback` wired into
    /// `IngestionPersistConfig.merge_progress` in `persist.rs`.
    ///
    /// # Design
    ///
    /// - Does NOT acquire the write lock — reads elapsed time and broadcasts
    ///   via the existing `self.tx` channel (clones cheaply).
    /// - The `Instant` for ETA calculation is stored under `graph_storage_start`
    ///   in `PipelineStateInner` (set by `start_pdf_phase(GraphStorage, ...)`).
    /// - The broadcast is fire-and-forget: dropped if no subscriber.
    #[allow(clippy::too_many_arguments)]
    pub async fn broadcast_graph_storage_progress(
        &self,
        track_id: &str,
        document_id: &str,
        sub_phase: &str,
        sub_phase_label: &str,
        entities_processed: u32,
        entities_total: u32,
        entities_created: u32,
        entities_updated: u32,
        relationships_processed: u32,
        relationships_total: u32,
        relationships_created: u32,
        relationships_updated: u32,
    ) {
        // Read elapsed time without taking a write lock
        let elapsed_ms = {
            let inner = self.inner.read().await;
            inner
                .graph_storage_start
                .get(track_id)
                .map(|t| t.elapsed().as_millis() as u64)
                .unwrap_or(0)
        };

        // Compute ETA: if some entities have been processed, extrapolate
        let eta_ms = if entities_total > 0 && entities_processed > 0 && elapsed_ms > 0 {
            let rate = entities_processed as f64 / elapsed_ms as f64; // entities/ms
            let remaining = entities_total.saturating_sub(entities_processed) as f64;
            Some((remaining / rate) as u64)
        } else {
            None
        };

        let event = PipelineEvent::GraphStorageProgress {
            track_id: track_id.to_string(),
            document_id: document_id.to_string(),
            sub_phase: sub_phase.to_string(),
            sub_phase_label: sub_phase_label.to_string(),
            entities_processed,
            entities_total,
            entities_created,
            entities_updated,
            relationships_processed,
            relationships_total,
            relationships_created,
            relationships_updated,
            elapsed_ms,
            eta_ms,
        };

        let _ = self.tx.send(event);
    }
}
