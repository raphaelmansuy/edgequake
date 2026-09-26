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
use edgequake_tasks::{PdfPageProgressPayload, PipelineState};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::runtime::Handle;
use tokio::sync::mpsc;

/// Document-level converting progress bands (never regress across mixed groups).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertingProgressBand {
    /// Pass-A OCR pages: `0.00 .. 0.90 * completed/total`.
    Ocr,
    /// Asset render / figure filter after a group: `0.90 .. 0.95`.
    Assets,
    /// Empty-page recovery: `0.95 .. 0.97`.
    Recovery,
    /// Grounding verify: `0.97 .. 0.99`.
    Verify,
    /// Terminal converting complete.
    Complete,
}

struct MetadataWrite {
    seq: u64,
    stage_message: String,
    stage_progress: f64,
}

struct MetadataWriterHandle {
    tx: mpsc::UnboundedSender<MetadataWrite>,
}

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
    /// FIX-PROGRESS: Last metadata update timestamp in milliseconds (epoch).
    ///
    /// WHY: Count-based debounce (every 50 pages) creates 10-15 minute gaps for
    /// slow providers like Ollama. Time-based debounce (every 2s) ensures the
    /// frontend polling (also 2s) always sees fresh progress.
    last_metadata_update_ms: AtomicU64,
    /// Document-global count of unique physical pages that reached a terminal
    /// state (complete / error / resumed-from-checkpoint).
    completed_pages: AtomicUsize,
    /// Unique physical page numbers already counted toward `completed_pages`.
    completed_page_set: Mutex<HashSet<usize>>,
    /// When true, `total_pages` is the immutable physical document page count
    /// and must not be overwritten by per-group selected totals from pdf2md.
    physical_total_locked: AtomicBool,
    /// Monotonic sequence for fencing async metadata writes so older spawned
    /// patches cannot overwrite newer substages.
    metadata_seq: AtomicU64,
    /// Last emitted document-global stage progress (f64 bits) — never decreases.
    last_stage_progress_bits: AtomicU64,
    /// Single-writer queue for ordered metadata patches (optional).
    metadata_writer: Option<MetadataWriterHandle>,
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
            // FIX-PROGRESS: No metadata written yet
            last_metadata_update_ms: AtomicU64::new(0),
            // Document-global unique completed pages
            completed_pages: AtomicUsize::new(0),
            completed_page_set: Mutex::new(HashSet::new()),
            physical_total_locked: AtomicBool::new(false),
            metadata_seq: AtomicU64::new(0),
            last_stage_progress_bits: AtomicU64::new(0f64.to_bits()),
            metadata_writer: None,
        }
    }

    /// Lock the immutable physical page count for the whole document.
    ///
    /// Mixed modality converts fire `on_conversion_start` once per group with
    /// the *selected* count; callers must pin the physical total (e.g. 25) so
    /// progress stays `completed/25` across both groups.
    ///
    /// A zero `total` is ignored (page count unknown yet).
    #[must_use]
    pub fn with_physical_page_count(self, total: usize) -> Self {
        if total == 0 {
            return self;
        }
        let n = total.max(1);
        self.total_pages.store(n, Ordering::SeqCst);
        self.physical_total_locked.store(true, Ordering::SeqCst);
        self
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
    ///
    /// Starts a single-writer task so metadata patches apply in sequence order
    /// (avoids read-modify-write races across fire-and-forget spawns).
    #[must_use]
    pub fn with_document_metadata(
        mut self,
        document_id: String,
        kv_storage: Arc<dyn KVStorage>,
    ) -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel::<MetadataWrite>();
        let doc_id = document_id.clone();
        let kv = Arc::clone(&kv_storage);
        self.runtime_handle.spawn(async move {
            let mut applied_seq = 0u64;
            while let Some(update) = rx.recv().await {
                if update.seq < applied_seq {
                    continue;
                }
                applied_seq = update.seq;
                let stage_message = update.stage_message;
                let stage_progress = update.stage_progress;
                let seq = update.seq;
                if let Err(e) = crate::services::patch_document_metadata(&kv, &doc_id, |obj| {
                    let prev_seq = obj
                        .get("progress_seq")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    if seq < prev_seq {
                        return;
                    }
                    crate::services::sync_progress_counts_from_message(obj, &stage_message);
                    obj.insert(
                        "stage_message".to_string(),
                        serde_json::json!(stage_message),
                    );
                    obj.insert(
                        "stage_progress".to_string(),
                        serde_json::json!(stage_progress),
                    );
                    obj.insert("progress_seq".to_string(), serde_json::json!(seq));
                    obj.insert(
                        "updated_at".to_string(),
                        serde_json::json!(chrono::Utc::now().to_rfc3339()),
                    );
                })
                .await
                {
                    tracing::warn!(
                        doc_id = %doc_id,
                        error = %e,
                        "Failed to upsert document metadata progress"
                    );
                }
            }
        });
        self.document_id = Some(document_id);
        self.kv_storage = Some(kv_storage);
        self.metadata_writer = Some(MetadataWriterHandle { tx });
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

    /// Report post-VLM converting work (page images, asset persist, figure analyze).
    ///
    /// WHY: `on_conversion_complete` fires when pdf2md finishes page OCR, but vision
    /// still renders PNGs / persists mm-assets / may run multimodal analyze. Without
    /// these messages the UI freezes at "24/24 pages" and looks stalled.
    ///
    /// Absolute caller fractions are remapped into document bands and floored at the
    /// current OCR fraction so mixed-group asset hooks cannot regress progress.
    pub fn report_converting_status(&self, stage_message: impl Into<String>, stage_progress: f64) {
        let total = self.total_pages.load(Ordering::Relaxed).max(1);
        let completed = self.completed_pages.load(Ordering::Relaxed).min(total);
        let msg = stage_message.into();
        let message = if msg.contains(" — ") || msg.contains('/') {
            msg
        } else {
            format!("{completed}/{total} — {msg}")
        };
        let band = infer_converting_band(&message, stage_progress);
        let mapped = self.map_band_progress(band, stage_progress);
        self.update_document_metadata(message, mapped);
    }

    /// Explicit band-aware status report (preferred over absolute fractions).
    pub fn report_band_status(
        &self,
        band: ConvertingProgressBand,
        stage_message: impl Into<String>,
        local_progress: f64,
    ) {
        let total = self.total_pages.load(Ordering::Relaxed).max(1);
        let completed = self.completed_pages.load(Ordering::Relaxed).min(total);
        let msg = stage_message.into();
        let message = if matches!(band, ConvertingProgressBand::Complete) || msg.contains(" — ") {
            msg
        } else {
            format!("{completed}/{total} — {msg}")
        };
        let mapped = self.map_band_progress(band, local_progress);
        self.update_document_metadata(message, mapped);
    }

    fn map_band_progress(&self, band: ConvertingProgressBand, local: f64) -> f64 {
        let total = self.document_total() as f64;
        let completed = self.completed_pages.load(Ordering::Relaxed) as f64;
        let ocr_floor = if total > 0.0 {
            (completed / total) * 0.90
        } else {
            0.0
        };
        let local = local.clamp(0.0, 1.0);
        let candidate = match band {
            ConvertingProgressBand::Ocr => ocr_floor,
            ConvertingProgressBand::Assets => {
                // Map legacy 0.92..0.965 hooks into 0.90..0.95, floored at OCR.
                let t = if local >= 0.90 {
                    ((local - 0.90) / 0.07).clamp(0.0, 1.0)
                } else {
                    local
                };
                (0.90 + 0.05 * t).max(ocr_floor)
            }
            ConvertingProgressBand::Recovery => 0.95 + 0.02 * local,
            ConvertingProgressBand::Verify => 0.97 + 0.02 * local,
            ConvertingProgressBand::Complete => 1.0,
        };
        self.advance_stage_progress(candidate.clamp(0.0, 1.0))
    }

    fn advance_stage_progress(&self, candidate: f64) -> f64 {
        let candidate = candidate.clamp(0.0, 1.0);
        loop {
            let prev_bits = self.last_stage_progress_bits.load(Ordering::SeqCst);
            let prev = f64::from_bits(prev_bits);
            let next = candidate.max(prev);
            if self
                .last_stage_progress_bits
                .compare_exchange(
                    prev_bits,
                    next.to_bits(),
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                )
                .is_ok()
            {
                return next;
            }
        }
    }

    fn ocr_progress_fraction(&self, completed: usize) -> f64 {
        let total = self.document_total();
        let raw = if total > 0 {
            (completed as f64 / total as f64) * 0.90
        } else {
            0.0
        };
        self.advance_stage_progress(raw.clamp(0.0, 0.90))
    }

    /// Mark PdfConversion phase complete after post-page work finishes.
    ///
    /// Call only when convert + page assets + optional multimodal analyze are done.
    pub fn complete_pdf_conversion_phase(&self) {
        let state = self.pipeline_state.clone();
        let track_id = self.task_id.clone();
        self.runtime_handle.spawn(async move {
            state
                .complete_pdf_phase(&track_id, PipelinePhase::PdfConversion)
                .await;
        });
    }

    /// PdfPageProgress is forwarded solely via `pipeline_ws_bridge` from
    /// PipelineState — keep the broadcaster handle for API compatibility with
    /// `with_broadcaster`, but do not dual-emit page events here.
    #[allow(dead_code)]
    fn broadcast_event(&self, event: ProgressEvent) {
        if matches!(event, ProgressEvent::PdfPageProgress { .. }) {
            return;
        }
        if let Some(ref broadcaster) = self.progress_broadcaster {
            broadcaster.broadcast(event);
        }
    }

    /// Update document metadata with current progress (ordered single-writer).
    fn update_document_metadata(&self, stage_message: String, stage_progress: f64) {
        let Some(ref writer) = self.metadata_writer else {
            return;
        };
        let seq = self.metadata_seq.fetch_add(1, Ordering::SeqCst) + 1;
        let _ = writer.tx.send(MetadataWrite {
            seq,
            stage_message,
            stage_progress: stage_progress.clamp(0.0, 1.0),
        });
    }

    /// FIX-PROGRESS: Check if enough time has passed to warrant a metadata update.
    fn should_update_metadata(&self, interval_ms: u64) -> bool {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let last = self.last_metadata_update_ms.load(Ordering::Relaxed);
        if now_ms.saturating_sub(last) >= interval_ms {
            self.last_metadata_update_ms
                .compare_exchange(last, now_ms, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
        } else {
            false
        }
    }

    fn document_total(&self) -> usize {
        self.total_pages.load(Ordering::Relaxed).max(1)
    }

    /// Mark a physical page terminal (complete/error/resumed). Returns new unique count.
    fn mark_page_terminal(&self, page_num: usize) -> usize {
        let inserted = {
            let mut set = self
                .completed_page_set
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            set.insert(page_num)
        };
        if inserted {
            self.completed_pages.fetch_add(1, Ordering::SeqCst) + 1
        } else {
            self.completed_pages.load(Ordering::Relaxed)
        }
    }

    #[allow(clippy::too_many_arguments)] // maps onto PdfPageProgressPayload fields
    fn emit_pdf_progress(
        &self,
        page_num: u32,
        completed_pages: u32,
        total_pages: u32,
        phase: String,
        markdown_len: usize,
        success: bool,
        error: Option<String>,
    ) {
        self.pipeline_state
            .emit_pdf_page_progress(PdfPageProgressPayload {
                pdf_id: self.pdf_id.clone(),
                task_id: self.task_id.clone(),
                page_num,
                total_pages,
                completed_pages,
                phase,
                markdown_len,
                success,
                error,
            });
    }
}

impl ConversionProgressCallback for PipelineProgressCallback {
    fn on_conversion_start(&self, group_total: usize) {
        let locked = self.physical_total_locked.load(Ordering::Relaxed);
        let completed = self.completed_pages.load(Ordering::Relaxed);
        let total = if locked {
            self.document_total()
        } else if completed == 0 {
            let n = group_total.max(1);
            self.total_pages.store(n, Ordering::SeqCst);
            n
        } else {
            // Infer physical total when page_count was unknown at start:
            // completed so far + this group's selected pages (mixed 13+12 → 25).
            let inferred = completed.saturating_add(group_total).max(1);
            let prev = self.total_pages.load(Ordering::Relaxed);
            let n = inferred.max(prev);
            self.total_pages.store(n, Ordering::SeqCst);
            n
        };

        tracing::info!(
            group_total,
            document_total = total,
            completed,
            pdf_id = %self.pdf_id,
            physical_locked = locked,
            "PDF conversion started (group)"
        );

        // Suppress per-group start resets when physical total is already locked.
        if locked && completed > 0 {
            return;
        }

        self.emit_pdf_progress(
            0,
            completed as u32,
            total as u32,
            "extraction".to_string(),
            0,
            true,
            None,
        );

        self.update_document_metadata(
            format!("Converting PDF to Markdown ({completed}/{total} pages)"),
            self.ocr_progress_fraction(completed),
        );

        if !locked || completed == 0 {
            let state = self.pipeline_state.clone();
            let track_id = self.task_id.clone();
            let pdf_id = self.pdf_id.clone();
            let filename = self.filename.clone();
            let pages = total;
            self.runtime_handle.spawn(async move {
                state
                    .start_pdf_progress(&track_id, &pdf_id, &filename)
                    .await;
                state
                    .start_pdf_phase(&track_id, PipelinePhase::PdfConversion, pages)
                    .await;
            });
        }
    }

    fn on_page_start(&self, page_num: usize, _group_total: usize) {
        let total = self.document_total();
        let completed = self.completed_pages.load(Ordering::Relaxed);

        tracing::debug!(
            page_num,
            total_pages = total,
            completed,
            pdf_id = %self.pdf_id,
            "PDF page extraction starting"
        );

        self.emit_pdf_progress(
            page_num as u32,
            completed as u32,
            total as u32,
            "extracting".to_string(),
            0,
            true,
            None,
        );

        if self.should_update_metadata(2_000) {
            let progress = self.ocr_progress_fraction(completed);
            self.update_document_metadata(
                format!(
                    "Converting PDF to Markdown: starting page {page_num}/{total} ({completed} completed)"
                ),
                progress,
            );
        }
    }

    fn on_page_complete(&self, page_num: usize, _group_total: usize, markdown_len: usize) {
        let completed = self.mark_page_terminal(page_num);
        let total = self.document_total();

        tracing::debug!(
            page_num,
            total_pages = total,
            completed,
            markdown_len,
            pdf_id = %self.pdf_id,
            "PDF page extraction complete"
        );

        self.emit_pdf_progress(
            page_num as u32,
            completed as u32,
            total as u32,
            "extracted".to_string(),
            markdown_len,
            true,
            None,
        );

        let is_first_completed = completed == 1;
        let is_last_page = completed >= total;
        let milestone = total > 0 && {
            let pct = (completed * 100) / total;
            let prev_pct = ((completed - 1) * 100) / total;
            (pct / 25) > (prev_pct / 25)
        };
        let time_due = self.should_update_metadata(2_000);
        let should_update = is_first_completed || is_last_page || milestone || time_due;

        if should_update {
            self.last_metadata_page.store(page_num, Ordering::SeqCst);
            let progress = self.ocr_progress_fraction(completed);
            let progress_percent = (completed as f64 / total as f64) * 100.0;
            let remaining = total.saturating_sub(completed);
            let message = if total >= 100 {
                format!(
                    "Converting PDF to Markdown: page {completed}/{total} ({progress_percent:.0}%) — {remaining} remaining"
                )
            } else {
                format!(
                    "Converting PDF to Markdown: page {completed}/{total} ({progress_percent:.0}%)"
                )
            };
            self.update_document_metadata(message, progress);
        }

        let state = self.pipeline_state.clone();
        let track_id = self.task_id.clone();
        self.runtime_handle.spawn(async move {
            state
                .update_pdf_phase(
                    &track_id,
                    PipelinePhase::PdfConversion,
                    completed,
                    &format!("Extracted {completed} of {total} pages (page {page_num})"),
                )
                .await;
        });
    }

    fn on_page_error(&self, page_num: usize, _group_total: usize, error: String) {
        let completed = self.mark_page_terminal(page_num);
        let total = self.document_total();

        tracing::warn!(
            page_num,
            total_pages = total,
            completed,
            error = %error,
            pdf_id = %self.pdf_id,
            "PDF page extraction error"
        );

        self.emit_pdf_progress(
            page_num as u32,
            completed as u32,
            total as u32,
            "extraction_error".to_string(),
            0,
            false,
            Some(error.clone()),
        );

        let state = self.pipeline_state.clone();
        let track_id = self.task_id.clone();
        let err_msg = error;
        self.runtime_handle.spawn(async move {
            state
                .update_pdf_phase(
                    &track_id,
                    PipelinePhase::PdfConversion,
                    completed,
                    &format!("Error on page {page_num}/{total}: {err_msg}"),
                )
                .await;
        });
    }

    fn on_page_resumed(&self, page_num: usize, _group_total: usize) {
        let completed = self.mark_page_terminal(page_num);
        let total = self.document_total();
        tracing::debug!(
            page_num,
            completed,
            total_pages = total,
            pdf_id = %self.pdf_id,
            "PDF page resumed from checkpoint"
        );
        self.emit_pdf_progress(
            page_num as u32,
            completed as u32,
            total as u32,
            "resumed".to_string(),
            0,
            true,
            None,
        );
        if self.should_update_metadata(2_000) {
            let progress = self.ocr_progress_fraction(completed);
            self.update_document_metadata(
                format!("Converting PDF to Markdown: page {completed}/{total} (resumed)"),
                progress,
            );
        }
    }

    fn on_conversion_complete(&self, group_total: usize, success_count: usize) {
        let total = self.document_total();
        let completed = self.completed_pages.load(Ordering::Relaxed);
        tracing::info!(
            group_total,
            group_success = success_count,
            document_total = total,
            completed,
            pdf_id = %self.pdf_id,
            "PDF conversion group complete"
        );

        // Do not reset progress on group complete — mixed docs fire this twice.
        let phase = if completed >= total {
            "group_complete".to_string()
        } else {
            format!("group_partial_{success_count}_of_{group_total}")
        };

        self.emit_pdf_progress(
            completed as u32,
            completed as u32,
            total as u32,
            phase,
            0,
            success_count > 0,
            None,
        );

        self.update_document_metadata(
            format!("{completed}/{total} — rendering"),
            self.map_band_progress(ConvertingProgressBand::Assets, 0.0),
        );
    }
}

fn infer_converting_band(message: &str, stage_progress: f64) -> ConvertingProgressBand {
    let lower = message.to_ascii_lowercase();
    if lower.contains("verif") || lower.contains("grounding") {
        ConvertingProgressBand::Verify
    } else if lower.contains("recover") || lower.contains("empty page") {
        ConvertingProgressBand::Recovery
    } else if lower.contains("finished") || stage_progress >= 1.0 {
        ConvertingProgressBand::Complete
    } else if lower.contains("render")
        || lower.contains("figure")
        || lower.contains("asset")
        || lower.contains("chart")
        || lower.contains("filter")
        || lower.contains("saving page")
        || lower.contains("analyz")
        || stage_progress >= 0.90
    {
        ConvertingProgressBand::Assets
    } else {
        ConvertingProgressBand::Ocr
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
                completed_pages,
                markdown_len,
                success,
                ..
            } => {
                assert_eq!(pdf_id, "pdf-123");
                assert_eq!(task_id, "task-456");
                assert_eq!(page_num, 5);
                assert_eq!(total_pages, 10);
                assert_eq!(completed_pages, 1);
                assert_eq!(markdown_len, 2048);
                assert!(success);
            }
            _ => panic!("Expected PdfPageProgress event"),
        }
    }

    #[tokio::test]
    async fn unknown_totals_infer_physical_count_across_groups() {
        let state = PipelineState::new();
        let mut rx = state.subscribe();
        let callback =
            PipelineProgressCallback::new(state, "pdf-infer".into(), "task-infer".into());
        // No with_physical_page_count — simulate heal failure.
        callback.on_conversion_start(13);
        for p in 1..=13 {
            callback.on_page_complete(p, 13, 10);
        }
        callback.on_conversion_start(12);
        for p in 14..=25 {
            callback.on_page_complete(p, 12, 10);
        }

        let mut last_total = 0u32;
        let mut last_completed = 0u32;
        while let Ok(ev) = rx.try_recv() {
            if let edgequake_tasks::PipelineEvent::PdfPageProgress {
                total_pages,
                completed_pages,
                ..
            } = ev
            {
                last_total = total_pages;
                last_completed = completed_pages;
            }
        }
        assert_eq!(last_total, 25);
        assert_eq!(last_completed, 25);
    }

    #[tokio::test]
    async fn mixed_groups_keep_monotonic_document_progress() {
        let state = PipelineState::new();
        let mut rx = state.subscribe();
        let callback =
            PipelineProgressCallback::new(state.clone(), "pdf-mixed".into(), "task-mixed".into())
                .with_physical_page_count(25);

        // Print group (13 pages) then manuscript group (12 pages).
        callback.on_conversion_start(13);
        for p in 1..=13 {
            callback.on_page_complete(p, 13, 10);
        }
        callback.on_conversion_complete(13, 13);
        // Second group must not reset totals.
        callback.on_conversion_start(12);
        for p in 14..=25 {
            callback.on_page_complete(p, 12, 10);
        }
        callback.on_page_error(20, 12, "dup".into()); // already counted
        callback.on_page_resumed(21, 12); // already counted

        let mut last_completed = 0u32;
        while let Ok(ev) = rx.try_recv() {
            if let edgequake_tasks::PipelineEvent::PdfPageProgress {
                total_pages,
                completed_pages,
                phase,
                ..
            } = ev
            {
                assert_eq!(total_pages, 25, "phase={phase}");
                assert!(
                    completed_pages >= last_completed,
                    "completed went {last_completed} → {completed_pages} ({phase})"
                );
                last_completed = completed_pages;
            }
        }
        assert_eq!(last_completed, 25);
        assert_eq!(callback.completed_pages.load(Ordering::Relaxed), 25);
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
        )
        .with_physical_page_count(2);

        callback.on_conversion_start(2);
        callback.on_page_complete(1, 2, 10);
        callback.on_page_complete(2, 2, 10);
        callback.on_conversion_complete(2, 2);

        let mut saw_group_complete = false;
        while let Ok(ev) = rx.try_recv() {
            if let edgequake_tasks::PipelineEvent::PdfPageProgress { phase, success, .. } = ev {
                if phase == "group_complete" {
                    assert!(success);
                    saw_group_complete = true;
                }
            }
        }
        assert!(saw_group_complete);
    }

    #[tokio::test]
    async fn test_pipeline_progress_callback_partial_complete() {
        let state = PipelineState::new();
        let mut rx = state.subscribe();

        let callback = PipelineProgressCallback::new(
            state.clone(),
            "pdf-partial".to_string(),
            "task-partial".to_string(),
        )
        .with_physical_page_count(10);

        callback.on_conversion_start(8);
        for p in 1..=8 {
            callback.on_page_complete(p, 8, 10);
        }
        callback.on_conversion_complete(8, 8); // first mixed group only

        let mut saw_partial = false;
        while let Ok(ev) = rx.try_recv() {
            if let edgequake_tasks::PipelineEvent::PdfPageProgress { phase, success, .. } = ev {
                if phase.contains("partial") {
                    assert!(success);
                    assert!(phase.contains("8_of_8"));
                    saw_partial = true;
                }
            }
        }
        assert!(saw_partial);
    }

    #[tokio::test]
    async fn asset_status_does_not_regress_mixed_progress() {
        let state = PipelineState::new();
        let callback =
            PipelineProgressCallback::new(state, "pdf-assets".into(), "task-assets".into())
                .with_physical_page_count(25);

        callback.on_conversion_start(13);
        for p in 1..=13 {
            callback.on_page_complete(p, 13, 10);
        }
        // Group-local absolute asset fractions used to jump to 0.96 then OCR
        // of group 2 would drop to 14/25 — bands + floor prevent regression.
        callback.report_converting_status("Rendering page images…", 0.94);
        let after_assets = f64::from_bits(callback.last_stage_progress_bits.load(Ordering::SeqCst));
        assert!(after_assets >= 0.90 * (13.0 / 25.0));

        callback.on_conversion_start(12);
        callback.on_page_complete(14, 12, 10);
        let after_next = f64::from_bits(callback.last_stage_progress_bits.load(Ordering::SeqCst));
        assert!(
            after_next + f64::EPSILON >= after_assets,
            "progress regressed {after_assets} → {after_next}"
        );
    }

    /// OODA-10: PdfPageProgress is bridge-only; broadcaster is retained for API compat.
    #[tokio::test]
    async fn test_pipeline_progress_callback_with_broadcaster() {
        let state = PipelineState::new();
        let mut internal_rx = state.subscribe();

        let broadcaster = ProgressBroadcaster::new(16);
        let mut ws_rx = broadcaster.subscribe();

        let callback = PipelineProgressCallback::new(
            state.clone(),
            "pdf-dual".to_string(),
            "task-dual".to_string(),
        )
        .with_broadcaster(broadcaster);

        callback.on_conversion_start(5);
        callback.on_page_complete(1, 5, 100);

        // Internal PipelineState still receives events.
        let mut saw_internal = false;
        while let Ok(ev) = internal_rx.try_recv() {
            if matches!(ev, edgequake_tasks::PipelineEvent::PdfPageProgress { .. }) {
                saw_internal = true;
            }
        }
        assert!(saw_internal);

        // Direct broadcaster dual-emit for PdfPageProgress is intentionally disabled
        // (pipeline_ws_bridge is the single WS path).
        assert!(ws_rx.try_recv().is_err());
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

        // PdfConversion phase should be active; current is unique completed count.
        let pdf_phase = &progress.phases[PipelinePhase::PdfConversion.index()];
        assert_eq!(pdf_phase.status, PhaseStatus::Active);
        assert_eq!(pdf_phase.total, 10);
        assert_eq!(pdf_phase.current, 1);
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
        // Phase completion is deferred until post-OCR asset work finishes.
        callback.complete_pdf_conversion_phase();

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

    // ── Edge Case Tests (v0.6.1 upgrade) ─────────────────────────────────

    /// Edge case: Zero-page PDF — ensure no panics and correct events.
    #[tokio::test]
    async fn test_zero_page_document() {
        let state = PipelineState::new();
        let mut rx = state.subscribe();

        let callback = PipelineProgressCallback::new(
            state.clone(),
            "pdf-zero".to_string(),
            "task-zero".to_string(),
        );

        // Zero pages should not panic; totals floor at 1 for display safety.
        callback.on_conversion_start(0);
        callback.on_conversion_complete(0, 0);

        // Verify start event
        let event = rx.try_recv().unwrap();
        match event {
            edgequake_tasks::PipelineEvent::PdfPageProgress {
                total_pages, phase, ..
            } => {
                assert_eq!(total_pages, 1);
                assert_eq!(phase, "extraction");
            }
            _ => panic!("Expected PdfPageProgress event"),
        }

        // Verify group-complete event (document-global vocabulary).
        let event = rx.try_recv().unwrap();
        match event {
            edgequake_tasks::PipelineEvent::PdfPageProgress {
                total_pages, phase, ..
            } => {
                assert_eq!(total_pages, 1);
                assert!(phase.contains("group_"));
            }
            _ => panic!("Expected PdfPageProgress event"),
        }
    }

    /// Edge case: Single-page document — full lifecycle.
    #[tokio::test]
    async fn test_single_page_document() {
        let state = PipelineState::new();
        let mut rx = state.subscribe();

        let callback = PipelineProgressCallback::new(
            state.clone(),
            "pdf-single".to_string(),
            "task-single".to_string(),
        );

        callback.on_conversion_start(1);
        callback.on_page_start(0, 1);
        callback.on_page_complete(0, 1, 512);
        callback.on_conversion_complete(1, 1);

        // Drain start event
        let _ = rx.try_recv();
        // Drain page_start event
        let _ = rx.try_recv();

        // Verify page_complete event
        let event = rx.try_recv().unwrap();
        match event {
            edgequake_tasks::PipelineEvent::PdfPageProgress {
                page_num,
                total_pages,
                phase,
                success,
                ..
            } => {
                assert_eq!(page_num, 0);
                assert_eq!(total_pages, 1);
                assert_eq!(phase, "extracted");
                assert!(success);
            }
            _ => panic!("Expected PdfPageProgress event"),
        }

        // Verify group-complete event
        let event = rx.try_recv().unwrap();
        match event {
            edgequake_tasks::PipelineEvent::PdfPageProgress { phase, success, .. } => {
                assert_eq!(phase, "group_complete");
                assert!(success);
            }
            _ => panic!("Expected PdfPageProgress event"),
        }
    }

    /// Edge case: All pages fail — success_count = 0.
    #[tokio::test]
    async fn test_all_pages_fail() {
        let state = PipelineState::new();
        let mut rx = state.subscribe();

        let callback = PipelineProgressCallback::new(
            state.clone(),
            "pdf-allfail".to_string(),
            "task-allfail".to_string(),
        );

        callback.on_conversion_start(3);
        callback.on_page_error(0, 3, "API timeout".to_string());
        callback.on_page_error(1, 3, "Rate limited".to_string());
        callback.on_page_error(2, 3, "Content filter".to_string());
        callback.on_conversion_complete(3, 0); // 0 successes

        // Drain all intermediate events (start + 3 errors = 4)
        for _ in 0..4 {
            let _ = rx.try_recv();
        }

        // Verify complete event says NOT successful
        let event = rx.try_recv().unwrap();
        match event {
            edgequake_tasks::PipelineEvent::PdfPageProgress {
                phase,
                success,
                completed_pages,
                ..
            } => {
                assert_eq!(phase, "group_complete");
                assert!(!success); // 0 successes → false
                assert_eq!(completed_pages, 3);
            }
            _ => panic!("Expected PdfPageProgress event"),
        }
    }

    /// Edge case: Page error emits correct data.
    #[tokio::test]
    async fn test_page_error_event_data() {
        let state = PipelineState::new();
        let mut rx = state.subscribe();

        let callback = PipelineProgressCallback::new(
            state.clone(),
            "pdf-err".to_string(),
            "task-err".to_string(),
        );

        callback.on_conversion_start(5);
        // Drain start event
        let _ = rx.try_recv();

        // Error on page 3 (0-indexed) of 5
        callback.on_page_error(3, 5, "LLM API returned 500".to_string());

        let event = rx.try_recv().unwrap();
        match event {
            edgequake_tasks::PipelineEvent::PdfPageProgress {
                page_num,
                total_pages,
                phase,
                success,
                error,
                ..
            } => {
                assert_eq!(page_num, 3);
                assert_eq!(total_pages, 5);
                assert_eq!(phase, "extraction_error");
                assert!(!success);
                assert_eq!(error.unwrap(), "LLM API returned 500");
            }
            _ => panic!("Expected PdfPageProgress event"),
        }
    }

    /// Edge case: Interleaved errors and successes.
    #[tokio::test]
    async fn test_interleaved_errors_and_successes() {
        let state = PipelineState::new();
        let mut rx = state.subscribe();

        let callback = PipelineProgressCallback::new(
            state.clone(),
            "pdf-mixed".to_string(),
            "task-mixed".to_string(),
        );

        callback.on_conversion_start(4);
        // Drain start event
        let _ = rx.try_recv();

        // Mixed results (concurrent order)
        callback.on_page_complete(0, 4, 1024);
        callback.on_page_error(1, 4, "timeout".to_string());
        callback.on_page_complete(2, 4, 2048);
        callback.on_page_error(3, 4, "rate limit".to_string());
        callback.on_conversion_complete(4, 2); // 2 of 4 succeeded

        // Drain 4 intermediate events
        for _ in 0..4 {
            let _ = rx.try_recv();
        }

        // Verify group completion after all pages terminal
        let event = rx.try_recv().unwrap();
        match event {
            edgequake_tasks::PipelineEvent::PdfPageProgress {
                phase,
                success,
                completed_pages,
                ..
            } => {
                assert_eq!(phase, "group_complete");
                assert!(success); // 2 > 0, so still success
                assert_eq!(completed_pages, 4);
            }
            _ => panic!("Expected PdfPageProgress event"),
        }
    }

    /// Edge case: Out-of-order page completion (concurrent processing).
    #[tokio::test]
    async fn test_out_of_order_page_completion() {
        let state = PipelineState::new();
        let mut rx = state.subscribe();

        let callback = PipelineProgressCallback::new(
            state.clone(),
            "pdf-ooo".to_string(),
            "task-ooo".to_string(),
        );

        callback.on_conversion_start(5);
        // Drain start event
        let _ = rx.try_recv();

        // Pages complete out of order (as happens with concurrent processing)
        callback.on_page_complete(3, 5, 1000); // page 4 finishes first
        callback.on_page_complete(0, 5, 800); // then page 1
        callback.on_page_complete(4, 5, 1200); // then page 5
        callback.on_page_complete(1, 5, 900); // then page 2
        callback.on_page_complete(2, 5, 1100); // then page 3
        callback.on_conversion_complete(5, 5);

        // Verify all events are received without panic
        let mut page_nums = Vec::new();
        for _ in 0..5 {
            let event = rx.try_recv().unwrap();
            match event {
                edgequake_tasks::PipelineEvent::PdfPageProgress {
                    page_num, phase, ..
                } => {
                    assert_eq!(phase, "extracted");
                    page_nums.push(page_num);
                }
                _ => panic!("Expected PdfPageProgress event"),
            }
        }
        // Verify pages arrived in the order they completed (not sorted)
        assert_eq!(page_nums, vec![3, 0, 4, 1, 2]);
    }

    /// Edge case: Very large document page count (1000+ pages).
    #[tokio::test]
    async fn test_large_document_progress() {
        use edgequake_tasks::progress::PhaseStatus;

        let state = PipelineState::new();
        let _rx = state.subscribe();

        let callback = PipelineProgressCallback::new(
            state.clone(),
            "pdf-large".to_string(),
            "task-large".to_string(),
        )
        .with_filename("large_book.pdf".to_string());

        callback.on_conversion_start(1000);

        // Simulate some pages completing (not all — just enough to test debounce)
        callback.on_page_complete(0, 1000, 500); // First page → always updates
        callback.on_page_complete(10, 1000, 500); // Within debounce (50 for 1000+ pages)
        callback.on_page_complete(50, 1000, 500); // At debounce interval → updates
        callback.on_page_complete(249, 1000, 500); // 25% milestone → updates
        callback.on_page_complete(999, 1000, 500); // Last page → always updates

        // Wait for spawned async tasks
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Verify progress was persisted (by checking phase state)
        let progress = state.get_pdf_progress("task-large").await;
        assert!(progress.is_some());

        let progress = progress.unwrap();
        let pdf_phase = &progress.phases[PipelinePhase::PdfConversion.index()];
        assert_eq!(pdf_phase.status, PhaseStatus::Active);
        assert_eq!(pdf_phase.total, 1000);
    }

    /// FIX-PROGRESS: Time-based debounce correctly gates metadata updates.
    #[tokio::test]
    async fn test_time_based_debounce() {
        let state = PipelineState::new();
        let _rx = state.subscribe();

        let callback = PipelineProgressCallback::new(
            state,
            "pdf-debounce".to_string(),
            "task-debounce".to_string(),
        );

        // First call should always succeed (last_metadata_update_ms starts at 0)
        assert!(
            callback.should_update_metadata(2_000),
            "First call should always pass debounce"
        );

        // Immediate second call should be rejected (0ms elapsed < 2000ms interval)
        assert!(
            !callback.should_update_metadata(2_000),
            "Immediate second call should be debounced"
        );

        // With a 0ms interval, every call should pass
        assert!(
            callback.should_update_metadata(0),
            "Zero interval should always pass"
        );

        // Wait a bit and try with a very short interval
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(
            callback.should_update_metadata(10),
            "10ms interval should pass after 50ms sleep"
        );
    }

    /// FIX-PROGRESS: completed_pages counter increments correctly across
    /// concurrent page completions (simulated sequentially here).
    #[tokio::test]
    async fn test_completed_pages_counter() {
        let state = PipelineState::new();
        let _rx = state.subscribe();

        let callback = PipelineProgressCallback::new(
            state,
            "pdf-counter".to_string(),
            "task-counter".to_string(),
        );

        callback.on_conversion_start(10);

        // Pages may complete out of order
        callback.on_page_complete(2, 10, 100);
        callback.on_page_complete(0, 10, 100);
        callback.on_page_complete(5, 10, 100);

        let completed = callback.completed_pages.load(Ordering::Relaxed);
        assert_eq!(
            completed, 3,
            "Should track 3 completed pages regardless of order"
        );
    }

    /// Edge case: Errors still reach PipelineState (WS is bridge-only).
    #[tokio::test]
    async fn test_broadcaster_receives_errors() {
        let state = PipelineState::new();
        let mut internal_rx = state.subscribe();

        let broadcaster = ProgressBroadcaster::new(16);
        let mut ws_rx = broadcaster.subscribe();

        let callback = PipelineProgressCallback::new(
            state.clone(),
            "pdf-ws-err".to_string(),
            "task-ws-err".to_string(),
        )
        .with_broadcaster(broadcaster);

        callback.on_conversion_start(3);
        callback.on_page_error(1, 3, "GPU OOM".to_string());

        let mut saw_error = false;
        while let Ok(ev) = internal_rx.try_recv() {
            if let edgequake_tasks::PipelineEvent::PdfPageProgress {
                phase,
                success,
                error,
                page_num,
                ..
            } = ev
            {
                if phase == "extraction_error" {
                    assert!(!success);
                    assert_eq!(page_num, 1);
                    assert_eq!(error.unwrap(), "GPU OOM");
                    saw_error = true;
                }
            }
        }
        assert!(saw_error);
        // Direct broadcaster path remains suppressed for PdfPageProgress.
        assert!(ws_rx.try_recv().is_err());
    }

    /// Edge case: Completion with exact total sets 100% progress.
    #[tokio::test]
    async fn test_completion_metadata_reaches_100_percent() {
        use edgequake_tasks::progress::PhaseStatus;

        let state = PipelineState::new();
        let _rx = state.subscribe();

        let callback = PipelineProgressCallback::new(
            state.clone(),
            "pdf-100".to_string(),
            "task-100".to_string(),
        )
        .with_filename("complete.pdf".to_string());

        callback.on_conversion_start(3);
        callback.on_page_complete(0, 3, 100);
        callback.on_page_complete(1, 3, 100);
        // Skip page 2 (simulate debounce skipping it)
        callback.on_conversion_complete(3, 3);
        // PdfConversion phase completes only after post-OCR work (caller).
        callback.complete_pdf_conversion_phase();

        // Wait for spawned tasks
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Verify phase is marked complete
        let progress = state.get_pdf_progress("task-100").await;
        assert!(progress.is_some());

        let progress = progress.unwrap();
        let pdf_phase = &progress.phases[PipelinePhase::PdfConversion.index()];
        assert_eq!(pdf_phase.status, PhaseStatus::Complete);
    }
}
