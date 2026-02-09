//! PDF Upload Progress Tracking Types
//!
//! ## Implements
//!
//! - [`SPEC-001-upload-pdf`]: 6-phase pipeline progress monitoring
//! - [`FEAT0606`]: Multi-phase progress tracking with ETA
//! - [`FEAT0607`]: Phase-level error reporting
//!
//! ## Use Cases
//!
//! - [`UC0707`]: User sees 6 distinct pipeline phases
//! - [`UC0708`]: User sees progress percentage per phase
//! - [`UC0709`]: User sees estimated time remaining
//!
//! ## Enforces
//!
//! - [`BR0706`]: Progress visible for all active PDF uploads
//! - [`BR0707`]: ETA updates based on actual processing time
//!
//! ## WHY 6 Phases?
//!
//! PDF processing goes through distinct stages, each with different
//! characteristics and failure modes:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                          PDF PROCESSING PIPELINE                            │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │                                                                             │
//! │  ┌─────────┐   ┌─────────────┐   ┌─────────┐   ┌───────────┐   ┌─────────┐ │
//! │  │ Upload  │──►│ PDF→MD     │──►│Chunking │──►│ Embedding │──►│ Entity  │ │
//! │  │         │   │ Conversion │   │         │   │           │   │Extract  │ │
//! │  └─────────┘   └─────────────┘   └─────────┘   └───────────┘   └────┬────┘ │
//! │       │              │                │              │               │      │
//! │       │              │                │              │               ▼      │
//! │   Network I/O    CPU-bound       CPU-bound      LLM API call    ┌─────────┐ │
//! │   (fast)        (pdf parse)     (text split)    (slow, $$$)     │ Graph   │ │
//! │                                                                 │ Storage │ │
//! │   Phase 1        Phase 2         Phase 3        Phase 4         └─────────┘ │
//! │                                                                     │       │
//! │                                                      Phase 5 ◄──────┘       │
//! │                                                                             │
//! │                                                      Phase 6: DB writes     │
//! │                                                                             │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Pipeline phases for PDF processing.
///
/// WHY this ordering:
/// 1. Upload: File arrives at server, validated
/// 2. PdfConversion: Raw PDF → Markdown (CPU-bound)
/// 3. Chunking: Split text into chunks (CPU-bound)
/// 4. Embedding: Generate vector embeddings (LLM API call)
/// 5. Extraction: Extract entities/relationships (LLM API call)
/// 6. GraphStorage: Write to knowledge graph (DB I/O)
///
/// @implements SPEC-001-upload-pdf: 6-phase tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelinePhase {
    /// File upload & validation.
    /// Fast (seconds), checks: size, format, signature.
    Upload,

    /// PDF to Markdown conversion via edgequake-pdf.
    /// CPU-bound, 1-10 seconds per page depending on complexity.
    /// Shows page-by-page progress.
    PdfConversion,

    /// Text chunking for embedding.
    /// CPU-bound, fast (< 1 second total).
    /// Shows chunk count.
    Chunking,

    /// Vector embedding generation.
    /// LLM API call, variable latency (100ms-5s per chunk).
    /// Shows chunk progress with cost.
    Embedding,

    /// Entity and relationship extraction.
    /// LLM API call, 1-30 seconds per chunk.
    /// Shows entity count.
    Extraction,

    /// Graph storage and indexing.
    /// DB I/O, usually fast (< 5 seconds).
    /// Shows entity/relationship counts.
    GraphStorage,
}

impl PipelinePhase {
    /// Get all phases in order.
    pub fn all() -> &'static [PipelinePhase] {
        &[
            PipelinePhase::Upload,
            PipelinePhase::PdfConversion,
            PipelinePhase::Chunking,
            PipelinePhase::Embedding,
            PipelinePhase::Extraction,
            PipelinePhase::GraphStorage,
        ]
    }

    /// Get the zero-based index of this phase.
    pub fn index(&self) -> usize {
        match self {
            PipelinePhase::Upload => 0,
            PipelinePhase::PdfConversion => 1,
            PipelinePhase::Chunking => 2,
            PipelinePhase::Embedding => 3,
            PipelinePhase::Extraction => 4,
            PipelinePhase::GraphStorage => 5,
        }
    }

    /// Get the display name for this phase.
    pub fn display_name(&self) -> &'static str {
        match self {
            PipelinePhase::Upload => "Upload",
            PipelinePhase::PdfConversion => "PDF → Markdown",
            PipelinePhase::Chunking => "Chunking",
            PipelinePhase::Embedding => "Embedding",
            PipelinePhase::Extraction => "Entity Extraction",
            PipelinePhase::GraphStorage => "Graph Storage",
        }
    }

    /// Get a short description of what this phase does.
    pub fn description(&self) -> &'static str {
        match self {
            PipelinePhase::Upload => "Uploading and validating file",
            PipelinePhase::PdfConversion => "Converting PDF to Markdown",
            PipelinePhase::Chunking => "Splitting text into chunks",
            PipelinePhase::Embedding => "Generating vector embeddings",
            PipelinePhase::Extraction => "Extracting entities and relationships",
            PipelinePhase::GraphStorage => "Storing in knowledge graph",
        }
    }

    /// Check if this phase is complete relative to another phase.
    pub fn is_complete_relative_to(&self, current: &PipelinePhase) -> bool {
        self.index() < current.index()
    }

    /// Get the next phase, if any.
    pub fn next(&self) -> Option<PipelinePhase> {
        match self {
            PipelinePhase::Upload => Some(PipelinePhase::PdfConversion),
            PipelinePhase::PdfConversion => Some(PipelinePhase::Chunking),
            PipelinePhase::Chunking => Some(PipelinePhase::Embedding),
            PipelinePhase::Embedding => Some(PipelinePhase::Extraction),
            PipelinePhase::Extraction => Some(PipelinePhase::GraphStorage),
            PipelinePhase::GraphStorage => None,
        }
    }
}

impl fmt::Display for PipelinePhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Status of a single pipeline phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum PhaseStatus {
    /// Phase has not started yet.
    #[default]
    Pending,
    /// Phase is currently executing.
    Active,
    /// Phase completed successfully.
    Complete,
    /// Phase failed with an error.
    Failed,
    /// Phase was skipped (e.g., PDF conversion skipped for text files).
    Skipped,
}

/// Error information for a failed phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseError {
    /// Error message.
    pub message: String,
    /// Error code for programmatic handling.
    pub code: String,
    /// Whether this error is retryable.
    pub retryable: bool,
    /// Suggested action for the user.
    pub suggestion: String,
    /// Affected item (e.g., page number, chunk index).
    pub affected_item: Option<String>,
}

impl PhaseError {
    /// Create a new phase error.
    pub fn new(
        message: impl Into<String>,
        code: impl Into<String>,
        retryable: bool,
        suggestion: impl Into<String>,
    ) -> Self {
        Self {
            message: message.into(),
            code: code.into(),
            retryable,
            suggestion: suggestion.into(),
            affected_item: None,
        }
    }

    /// Create an error with an affected item.
    pub fn with_item(mut self, item: impl Into<String>) -> Self {
        self.affected_item = Some(item.into());
        self
    }

    /// Create a PDF parse error.
    pub fn pdf_parse(page: usize, details: impl Into<String>) -> Self {
        Self::new(
            format!("Failed to parse page {}: {}", page, details.into()),
            "PDF_PARSE_ERROR",
            false,
            "The PDF may be corrupted or use an unsupported format. Try re-saving the PDF.",
        )
        .with_item(format!("page_{}", page))
    }

    /// Create an LLM timeout error.
    pub fn llm_timeout(phase: &str) -> Self {
        Self::new(
            format!("LLM request timed out during {}", phase),
            "LLM_TIMEOUT",
            true,
            "The document may be too large. Try splitting it or using a faster model.",
        )
    }

    /// Create a rate limit error.
    pub fn rate_limit() -> Self {
        Self::new(
            "API rate limit exceeded",
            "RATE_LIMIT",
            true,
            "Wait 30 seconds and retry, or reduce batch size.",
        )
    }
}

/// Progress information for a single pipeline phase.
///
/// @implements SPEC-001-upload-pdf: Phase-level progress tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseProgress {
    /// Which phase this tracks.
    pub phase: PipelinePhase,

    /// Current status of the phase.
    pub status: PhaseStatus,

    /// Current item being processed (0-indexed).
    /// For PdfConversion: current page
    /// For Chunking: current chunk
    /// For Embedding/Extraction: current chunk
    /// For GraphStorage: current entity
    pub current: usize,

    /// Total items to process.
    /// May be 0 if unknown (e.g., before PDF is parsed).
    pub total: usize,

    /// Progress percentage (0.0 - 100.0).
    /// Calculated from current/total, or set directly.
    pub percentage: f32,

    /// Estimated time remaining in seconds.
    /// None if not calculable.
    pub eta_seconds: Option<u64>,

    /// Human-readable status message.
    /// E.g., "Extracting page 5 of 10..."
    pub message: String,

    /// Error information if phase failed.
    pub error: Option<PhaseError>,

    /// When this phase started.
    pub started_at: Option<DateTime<Utc>>,

    /// When this phase completed (success or failure).
    pub completed_at: Option<DateTime<Utc>>,

    /// Average time per item in milliseconds.
    /// Used for ETA calculation.
    avg_item_time_ms: f64,
}

impl PhaseProgress {
    /// Create a new pending phase progress.
    pub fn new(phase: PipelinePhase) -> Self {
        Self {
            phase,
            status: PhaseStatus::Pending,
            current: 0,
            total: 0,
            percentage: 0.0,
            eta_seconds: None,
            message: format!("Waiting for {}", phase.display_name()),
            error: None,
            started_at: None,
            completed_at: None,
            avg_item_time_ms: 0.0,
        }
    }

    /// Start this phase with a known total.
    pub fn start(&mut self, total: usize) {
        self.status = PhaseStatus::Active;
        self.total = total;
        self.current = 0;
        self.percentage = 0.0;
        self.started_at = Some(Utc::now());
        self.message = format!("Starting {}", self.phase.display_name());
    }

    /// Start this phase without knowing the total.
    pub fn start_indeterminate(&mut self) {
        self.status = PhaseStatus::Active;
        self.total = 0;
        self.current = 0;
        self.percentage = 0.0;
        self.started_at = Some(Utc::now());
        self.message = format!("Starting {}", self.phase.display_name());
    }

    /// Update progress with a new current value.
    pub fn update(&mut self, current: usize, message: impl Into<String>) {
        self.current = current;
        self.message = message.into();

        if self.total > 0 {
            self.percentage = (current as f32 / self.total as f32 * 100.0).min(100.0);
        }

        // Update ETA based on elapsed time
        if let Some(started_at) = self.started_at {
            let elapsed_ms = (Utc::now() - started_at).num_milliseconds() as f64;
            if current > 0 {
                // Exponential moving average for smoother ETA
                let new_avg = elapsed_ms / current as f64;
                if self.avg_item_time_ms == 0.0 {
                    self.avg_item_time_ms = new_avg;
                } else {
                    // EMA with alpha = 0.3
                    self.avg_item_time_ms = 0.3 * new_avg + 0.7 * self.avg_item_time_ms;
                }

                // Calculate remaining time
                let remaining = self.total.saturating_sub(current);
                let remaining_ms = remaining as f64 * self.avg_item_time_ms;
                self.eta_seconds = Some((remaining_ms / 1000.0).ceil() as u64);
            }
        }
    }

    /// Mark this phase as complete.
    pub fn complete(&mut self) {
        self.status = PhaseStatus::Complete;
        self.current = self.total;
        self.percentage = 100.0;
        self.eta_seconds = Some(0);
        self.completed_at = Some(Utc::now());
        self.message = format!("{} complete", self.phase.display_name());
    }

    /// Mark this phase as failed.
    pub fn fail(&mut self, error: PhaseError) {
        self.status = PhaseStatus::Failed;
        self.error = Some(error);
        self.completed_at = Some(Utc::now());
        self.message = format!("{} failed", self.phase.display_name());
    }

    /// Mark this phase as skipped.
    pub fn skip(&mut self, reason: impl Into<String>) {
        self.status = PhaseStatus::Skipped;
        self.completed_at = Some(Utc::now());
        self.message = reason.into();
    }

    /// Get the duration of this phase in milliseconds.
    pub fn duration_ms(&self) -> Option<i64> {
        match (self.started_at, self.completed_at) {
            (Some(start), Some(end)) => Some((end - start).num_milliseconds()),
            (Some(start), None) => Some((Utc::now() - start).num_milliseconds()),
            _ => None,
        }
    }

    /// Check if this phase is finished (complete, failed, or skipped).
    pub fn is_finished(&self) -> bool {
        matches!(
            self.status,
            PhaseStatus::Complete | PhaseStatus::Failed | PhaseStatus::Skipped
        )
    }
}

/// Overall progress for a PDF upload through all pipeline phases.
///
/// @implements SPEC-001-upload-pdf: Overall progress tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfUploadProgress {
    /// Unique tracking ID for this upload.
    pub track_id: String,

    /// PDF document ID.
    pub pdf_id: String,

    /// Document ID (assigned after text_insert phase).
    pub document_id: Option<String>,

    /// Original filename.
    pub filename: String,

    /// Progress for each phase.
    pub phases: Vec<PhaseProgress>,

    /// Overall progress percentage (0.0 - 100.0).
    /// Calculated as weighted average of phase progress.
    pub overall_percentage: f32,

    /// Overall estimated time remaining in seconds.
    pub eta_seconds: Option<u64>,

    /// Whether the entire pipeline is complete.
    pub is_complete: bool,

    /// Whether the pipeline failed.
    pub is_failed: bool,

    /// When processing started.
    pub started_at: DateTime<Utc>,

    /// When processing was last updated.
    pub updated_at: DateTime<Utc>,

    /// When processing completed (success or failure).
    pub completed_at: Option<DateTime<Utc>>,
}

impl PdfUploadProgress {
    /// Create a new upload progress tracker.
    pub fn new(track_id: String, pdf_id: String, filename: String) -> Self {
        let now = Utc::now();
        Self {
            track_id,
            pdf_id,
            document_id: None,
            filename,
            phases: PipelinePhase::all()
                .iter()
                .map(|p| PhaseProgress::new(*p))
                .collect(),
            overall_percentage: 0.0,
            eta_seconds: None,
            is_complete: false,
            is_failed: false,
            started_at: now,
            updated_at: now,
            completed_at: None,
        }
    }

    /// Get mutable reference to a phase.
    pub fn phase_mut(&mut self, phase: PipelinePhase) -> Option<&mut PhaseProgress> {
        self.phases.get_mut(phase.index())
    }

    /// Get reference to a phase.
    pub fn phase(&self, phase: PipelinePhase) -> Option<&PhaseProgress> {
        self.phases.get(phase.index())
    }

    /// Start a phase.
    pub fn start_phase(&mut self, phase: PipelinePhase, total: usize) {
        if let Some(p) = self.phase_mut(phase) {
            p.start(total);
            self.updated_at = Utc::now();
            self.recalculate_overall();
        }
    }

    /// Update a phase's progress.
    pub fn update_phase(
        &mut self,
        phase: PipelinePhase,
        current: usize,
        message: impl Into<String>,
    ) {
        if let Some(p) = self.phase_mut(phase) {
            p.update(current, message);
            self.updated_at = Utc::now();
            self.recalculate_overall();
        }
    }

    /// Complete a phase.
    pub fn complete_phase(&mut self, phase: PipelinePhase) {
        if let Some(p) = self.phase_mut(phase) {
            p.complete();
            self.updated_at = Utc::now();
            self.recalculate_overall();
        }

        // Check if all phases are complete
        if self.phases.iter().all(|p| p.is_finished()) {
            self.is_complete = true;
            self.completed_at = Some(Utc::now());
        }
    }

    /// Fail a phase.
    pub fn fail_phase(&mut self, phase: PipelinePhase, error: PhaseError) {
        if let Some(p) = self.phase_mut(phase) {
            p.fail(error);
            self.updated_at = Utc::now();
            self.is_failed = true;
            self.completed_at = Some(Utc::now());
            self.recalculate_overall();
        }
    }

    /// Recalculate overall progress from phases.
    fn recalculate_overall(&mut self) {
        // Weight each phase equally for now
        // TODO: Consider weighting by expected duration
        let phase_count = self.phases.len() as f32;
        let total_percentage: f32 = self.phases.iter().map(|p| p.percentage).sum();
        self.overall_percentage = total_percentage / phase_count;

        // Sum ETAs from incomplete phases
        let total_eta: u64 = self
            .phases
            .iter()
            .filter(|p| !p.is_finished())
            .filter_map(|p| p.eta_seconds)
            .sum();
        self.eta_seconds = if total_eta > 0 { Some(total_eta) } else { None };
    }

    /// Get the currently active phase.
    pub fn active_phase(&self) -> Option<&PhaseProgress> {
        self.phases.iter().find(|p| p.status == PhaseStatus::Active)
    }

    /// Get a human-readable status summary.
    pub fn status_summary(&self) -> String {
        if self.is_failed {
            if let Some(active) = self.phases.iter().find(|p| p.status == PhaseStatus::Failed) {
                return format!("Failed during {}", active.phase.display_name());
            }
            return "Failed".to_string();
        }

        if self.is_complete {
            return "Complete".to_string();
        }

        if let Some(active) = self.active_phase() {
            return active.message.clone();
        }

        "Pending".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_phase_ordering() {
        assert_eq!(PipelinePhase::Upload.index(), 0);
        assert_eq!(PipelinePhase::PdfConversion.index(), 1);
        assert_eq!(PipelinePhase::Chunking.index(), 2);
        assert_eq!(PipelinePhase::Embedding.index(), 3);
        assert_eq!(PipelinePhase::Extraction.index(), 4);
        assert_eq!(PipelinePhase::GraphStorage.index(), 5);

        assert!(PipelinePhase::Upload.is_complete_relative_to(&PipelinePhase::PdfConversion));
        assert!(!PipelinePhase::Extraction.is_complete_relative_to(&PipelinePhase::Chunking));
    }

    #[test]
    fn test_pipeline_phase_next() {
        assert_eq!(
            PipelinePhase::Upload.next(),
            Some(PipelinePhase::PdfConversion)
        );
        assert_eq!(PipelinePhase::GraphStorage.next(), None);
    }

    #[test]
    fn test_phase_progress_percentage() {
        let mut progress = PhaseProgress::new(PipelinePhase::PdfConversion);
        progress.start(10);
        assert_eq!(progress.percentage, 0.0);

        progress.update(5, "Processing page 5 of 10");
        assert_eq!(progress.percentage, 50.0);

        progress.update(10, "Processing page 10 of 10");
        assert_eq!(progress.percentage, 100.0);
    }

    #[test]
    fn test_phase_progress_complete() {
        let mut progress = PhaseProgress::new(PipelinePhase::Chunking);
        progress.start(100);
        progress.update(50, "Halfway");
        progress.complete();

        assert_eq!(progress.status, PhaseStatus::Complete);
        assert_eq!(progress.percentage, 100.0);
        assert!(progress.completed_at.is_some());
    }

    #[test]
    fn test_pdf_upload_progress() {
        let mut upload = PdfUploadProgress::new(
            "track_123".to_string(),
            "pdf_456".to_string(),
            "test.pdf".to_string(),
        );

        assert_eq!(upload.phases.len(), 6);
        assert_eq!(upload.overall_percentage, 0.0);
        assert!(!upload.is_complete);

        // Complete Upload phase
        upload.start_phase(PipelinePhase::Upload, 1);
        upload.complete_phase(PipelinePhase::Upload);

        // Overall should be ~16.67% (1/6 phases complete)
        assert!(upload.overall_percentage > 16.0 && upload.overall_percentage < 17.0);

        // Complete all phases
        for phase in PipelinePhase::all() {
            if *phase != PipelinePhase::Upload {
                upload.start_phase(*phase, 1);
                upload.complete_phase(*phase);
            }
        }

        assert_eq!(upload.overall_percentage, 100.0);
        assert!(upload.is_complete);
    }

    #[test]
    fn test_phase_error_creation() {
        let error = PhaseError::pdf_parse(5, "Invalid font reference");
        assert!(error.message.contains("page 5"));
        assert_eq!(error.code, "PDF_PARSE_ERROR");
        assert!(!error.retryable);

        let error = PhaseError::llm_timeout("embedding");
        assert!(error.retryable);
        assert_eq!(error.code, "LLM_TIMEOUT");
    }

    #[test]
    fn test_phase_fail() {
        let mut progress = PhaseProgress::new(PipelinePhase::Embedding);
        progress.start(10);
        progress.update(3, "Processing chunk 3");

        let error = PhaseError::rate_limit();
        progress.fail(error);

        assert_eq!(progress.status, PhaseStatus::Failed);
        assert!(progress.error.is_some());
        assert!(progress.error.as_ref().unwrap().retryable);
    }
}
