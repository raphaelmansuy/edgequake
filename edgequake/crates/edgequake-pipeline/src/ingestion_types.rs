//! Unified ingestion types for PDF and Markdown processing.
//!
//! @implements SPEC-002: Unified Ingestion Pipeline
//! @implements FEAT0001: Document Ingestion Pipeline
//!
//! # Purpose
//!
//! This module extends the existing `progress.rs` types with:
//! 1. `SourceType` - to distinguish PDF, Markdown, Text sources
//! 2. `UnifiedStage` - includes `Converting` stage for PDFs
//! 3. Conversion utilities between stage types
//!
//! The existing `PipelineStage` and `IngestionProgress` in progress.rs remain
//! the canonical types for the pipeline. This module provides additional
//! types for unified status reporting to the frontend.
//!
//! # Stage Flow
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────────┐
//! │                         UNIFIED INGESTION STAGES                         │
//! ├──────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │  [uploading] → [converting?] → [preprocessing] → [chunking]              │
//! │       │              │               │               │                   │
//! │       │        (PDF only)            │               │                   │
//! │       ▼              ▼               ▼               ▼                   │
//! │  [extracting] → [gleaning] → [merging] → [summarizing]                   │
//! │       │              │           │             │                         │
//! │       ▼              ▼           ▼             ▼                         │
//! │  [embedding] → [storing] → [completed/failed]                            │
//! │                                                                          │
//! └──────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Relationship to progress.rs
//!
//! - `PipelineStage` (progress.rs) - Internal pipeline stages (no uploading/converting)
//! - `UnifiedStage` (this module) - Frontend-facing stages (includes all stages)
//! - Conversion: `UnifiedStage::from(PipelineStage)` and `UnifiedStage::to_pipeline_stage()`

use crate::progress::PipelineStage;
use serde::{Deserialize, Serialize};

/// Source type for ingestion.
///
/// Determines which conversion steps are needed:
/// - `Pdf`: Requires PDF → Markdown conversion step
/// - `Markdown`: Direct processing, skip conversion
/// - `Text`: Direct processing, skip conversion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    /// PDF document (requires conversion to Markdown)
    Pdf,
    /// Markdown document (direct processing)
    Markdown,
    /// Plain text content (direct processing)
    Text,
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceType::Pdf => write!(f, "pdf"),
            SourceType::Markdown => write!(f, "markdown"),
            SourceType::Text => write!(f, "text"),
        }
    }
}

/// Unified ingestion stage.
///
/// Used by both PDF and Markdown flows for consistent progress tracking.
/// The frontend status-badge.tsx uses these exact stage names.
///
/// # Stage Descriptions
///
/// | Stage | Description | PDF | MD |
/// |-------|-------------|-----|-----|
/// | `uploading` | File upload in progress | ✓ | ✓ |
/// | `converting` | PDF → Markdown | ✓ | - |
/// | `preprocessing` | Validation, parsing | ✓ | ✓ |
/// | `chunking` | Split into chunks | ✓ | ✓ |
/// | `extracting` | LLM entity extraction | ✓ | ✓ |
/// | `gleaning` | Re-extraction pass | ✓ | ✓ |
/// | `merging` | Graph merge | ✓ | ✓ |
/// | `summarizing` | Description gen | ✓ | ✓ |
/// | `embedding` | Vector generation | ✓ | ✓ |
/// | `storing` | Persist to DB | ✓ | ✓ |
/// | `completed` | Done successfully | ✓ | ✓ |
/// | `failed` | Error occurred | ✓ | ✓ |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash, Default)]
#[serde(rename_all = "lowercase")]
pub enum UnifiedStage {
    /// File/content being uploaded
    #[default]
    Uploading,
    /// PDF → Markdown conversion (PDF only, skipped for Markdown/Text)
    Converting,
    /// Validation, parsing, preparation
    Preprocessing,
    /// Document chunking with overlap
    Chunking,
    /// LLM entity/relationship extraction
    Extracting,
    /// Second pass extraction for missed entities
    Gleaning,
    /// Merging entities/relationships into graph
    Merging,
    /// Generating entity/relationship summaries
    Summarizing,
    /// Vector embedding generation
    Embedding,
    /// Persisting to graph and vector storage
    Storing,
    /// Successfully completed all stages
    Completed,
    /// Processing failed with error
    Failed,
}

impl UnifiedStage {
    /// Get all processing stages in order (excluding terminal states).
    pub fn processing_stages() -> Vec<UnifiedStage> {
        vec![
            UnifiedStage::Uploading,
            UnifiedStage::Converting,
            UnifiedStage::Preprocessing,
            UnifiedStage::Chunking,
            UnifiedStage::Extracting,
            UnifiedStage::Gleaning,
            UnifiedStage::Merging,
            UnifiedStage::Summarizing,
            UnifiedStage::Embedding,
            UnifiedStage::Storing,
        ]
    }

    /// Get stages for a specific source type.
    ///
    /// Markdown/Text skip the `Converting` stage.
    pub fn stages_for_source(source: SourceType) -> Vec<UnifiedStage> {
        let mut stages = Self::processing_stages();
        if source != SourceType::Pdf {
            stages.retain(|s| *s != UnifiedStage::Converting);
        }
        stages
    }

    /// Human-readable stage name.
    pub fn display_name(&self) -> &'static str {
        match self {
            UnifiedStage::Uploading => "Uploading",
            UnifiedStage::Converting => "Converting PDF",
            UnifiedStage::Preprocessing => "Preprocessing",
            UnifiedStage::Chunking => "Chunking",
            UnifiedStage::Extracting => "Extracting Entities",
            UnifiedStage::Gleaning => "Gleaning",
            UnifiedStage::Merging => "Merging Graph",
            UnifiedStage::Summarizing => "Summarizing",
            UnifiedStage::Embedding => "Generating Embeddings",
            UnifiedStage::Storing => "Storing",
            UnifiedStage::Completed => "Completed",
            UnifiedStage::Failed => "Failed",
        }
    }

    /// Check if this is a terminal state (completed or failed).
    pub fn is_terminal(&self) -> bool {
        matches!(self, UnifiedStage::Completed | UnifiedStage::Failed)
    }

    /// Check if this stage is active (not pending or terminal).
    pub fn is_active(&self) -> bool {
        !self.is_terminal()
    }

    /// Get the stage index for progress calculation.
    /// Returns None for terminal states.
    pub fn index(&self) -> Option<usize> {
        Self::processing_stages().iter().position(|s| s == self)
    }

    /// Convert from internal PipelineStage to UnifiedStage.
    ///
    /// Maps the internal pipeline stages to the frontend-facing unified stages.
    /// Note: PipelineStage doesn't have Uploading or Converting stages.
    pub fn from_pipeline_stage(stage: PipelineStage) -> Self {
        match stage {
            PipelineStage::Preprocessing => UnifiedStage::Preprocessing,
            PipelineStage::Chunking => UnifiedStage::Chunking,
            PipelineStage::Extracting => UnifiedStage::Extracting,
            PipelineStage::Gleaning => UnifiedStage::Gleaning,
            PipelineStage::Merging => UnifiedStage::Merging,
            PipelineStage::Summarizing => UnifiedStage::Summarizing,
            PipelineStage::Embedding => UnifiedStage::Embedding,
            PipelineStage::Storing => UnifiedStage::Storing,
            PipelineStage::Finalizing => UnifiedStage::Storing, // Map finalizing to storing
        }
    }

    /// Convert to internal PipelineStage.
    ///
    /// Returns None for stages that don't map to PipelineStage
    /// (Uploading, Converting, Completed, Failed).
    pub fn to_pipeline_stage(&self) -> Option<PipelineStage> {
        match self {
            UnifiedStage::Preprocessing => Some(PipelineStage::Preprocessing),
            UnifiedStage::Chunking => Some(PipelineStage::Chunking),
            UnifiedStage::Extracting => Some(PipelineStage::Extracting),
            UnifiedStage::Gleaning => Some(PipelineStage::Gleaning),
            UnifiedStage::Merging => Some(PipelineStage::Merging),
            UnifiedStage::Summarizing => Some(PipelineStage::Summarizing),
            UnifiedStage::Embedding => Some(PipelineStage::Embedding),
            UnifiedStage::Storing => Some(PipelineStage::Storing),
            // These stages don't exist in PipelineStage
            UnifiedStage::Uploading
            | UnifiedStage::Converting
            | UnifiedStage::Completed
            | UnifiedStage::Failed => None,
        }
    }
}

impl std::fmt::Display for UnifiedStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Status of a single pipeline stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum StageStatus {
    /// Not started yet
    #[default]
    Pending,
    /// Currently running
    Running,
    /// Successfully completed
    Completed,
    /// Skipped (not applicable for this source type)
    Skipped,
    /// Failed with error
    Failed,
}

/// Progress for a single ingestion stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageProgress {
    /// The stage this progress applies to
    pub stage: UnifiedStage,
    /// Current status of the stage
    pub status: StageStatus,
    /// Progress within the stage (0.0 to 1.0)
    pub progress: f32,
    /// Human-readable progress message
    pub message: Option<String>,
    /// When stage started (ISO 8601)
    pub started_at: Option<String>,
    /// When stage completed (ISO 8601)
    pub completed_at: Option<String>,
    /// Duration in milliseconds
    pub duration_ms: Option<u64>,
    /// Items processed / total items (for stages with countable work)
    pub items_completed: Option<usize>,
    /// Total items to process
    pub items_total: Option<usize>,
}

impl StageProgress {
    /// Create new pending stage progress.
    pub fn new(stage: UnifiedStage) -> Self {
        Self {
            stage,
            status: StageStatus::Pending,
            progress: 0.0,
            message: None,
            started_at: None,
            completed_at: None,
            duration_ms: None,
            items_completed: None,
            items_total: None,
        }
    }

    /// Create a skipped stage (for stages not applicable to source type).
    pub fn skipped(stage: UnifiedStage) -> Self {
        Self {
            stage,
            status: StageStatus::Skipped,
            progress: 1.0,
            message: Some("Skipped".to_string()),
            started_at: None,
            completed_at: None,
            duration_ms: None,
            items_completed: None,
            items_total: None,
        }
    }
}

/// Unified ingestion progress.
///
/// This is the main progress structure returned by progress APIs
/// and emitted via WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionProgress {
    /// Tracking ID for this ingestion job
    pub track_id: String,
    /// Document ID (may be None during early stages)
    pub document_id: Option<String>,
    /// Source type (pdf, markdown, text)
    pub source_type: SourceType,
    /// Original filename
    pub filename: Option<String>,
    /// Current stage
    pub current_stage: UnifiedStage,
    /// Progress for all stages
    pub stages: Vec<StageProgress>,
    /// Overall progress (0.0 to 1.0)
    pub overall_progress: f32,
    /// Current status message
    pub message: String,
    /// Error if failed
    pub error: Option<IngestionError>,
    /// Cost in USD (for LLM calls)
    pub cost_usd: Option<f64>,
    /// When ingestion started
    pub started_at: Option<String>,
    /// When last updated
    pub updated_at: Option<String>,
}

impl IngestionProgress {
    /// Create new ingestion progress.
    pub fn new(track_id: String, source_type: SourceType, filename: Option<String>) -> Self {
        let stages = UnifiedStage::stages_for_source(source_type)
            .into_iter()
            .map(StageProgress::new)
            .collect();

        Self {
            track_id,
            document_id: None,
            source_type,
            filename,
            current_stage: UnifiedStage::Uploading,
            stages,
            overall_progress: 0.0,
            message: "Starting...".to_string(),
            error: None,
            cost_usd: None,
            started_at: Some(chrono::Utc::now().to_rfc3339()),
            updated_at: Some(chrono::Utc::now().to_rfc3339()),
        }
    }

    /// Calculate overall progress from stage progress.
    pub fn calculate_overall_progress(&mut self) {
        let total_stages = self.stages.len() as f32;
        if total_stages == 0.0 {
            self.overall_progress = 0.0;
            return;
        }

        let completed_progress: f32 = self
            .stages
            .iter()
            .map(|s| match s.status {
                StageStatus::Completed | StageStatus::Skipped => 1.0,
                StageStatus::Running => s.progress,
                _ => 0.0,
            })
            .sum();

        self.overall_progress = (completed_progress / total_stages).clamp(0.0, 1.0);
        self.updated_at = Some(chrono::Utc::now().to_rfc3339());
    }

    /// Mark a stage as started.
    pub fn start_stage(&mut self, stage: UnifiedStage) {
        self.current_stage = stage;
        if let Some(sp) = self.stages.iter_mut().find(|s| s.stage == stage) {
            sp.status = StageStatus::Running;
            sp.started_at = Some(chrono::Utc::now().to_rfc3339());
            sp.message = Some(format!("{} in progress...", stage.display_name()));
        }
        self.message = format!("{} in progress...", stage.display_name());
        self.calculate_overall_progress();
    }

    /// Update progress for current stage.
    pub fn update_stage_progress(
        &mut self,
        stage: UnifiedStage,
        progress: f32,
        message: Option<String>,
    ) {
        if let Some(sp) = self.stages.iter_mut().find(|s| s.stage == stage) {
            sp.progress = progress.clamp(0.0, 1.0);
            if let Some(msg) = message {
                sp.message = Some(msg.clone());
                self.message = msg;
            }
        }
        self.calculate_overall_progress();
    }

    /// Mark a stage as completed.
    pub fn complete_stage(&mut self, stage: UnifiedStage, duration_ms: Option<u64>) {
        if let Some(sp) = self.stages.iter_mut().find(|s| s.stage == stage) {
            sp.status = StageStatus::Completed;
            sp.progress = 1.0;
            sp.completed_at = Some(chrono::Utc::now().to_rfc3339());
            sp.duration_ms = duration_ms;
            sp.message = Some(format!("{} complete", stage.display_name()));
        }
        self.calculate_overall_progress();
    }

    /// Mark ingestion as completed.
    pub fn complete(&mut self) {
        self.current_stage = UnifiedStage::Completed;
        self.overall_progress = 1.0;
        self.message = "Ingestion completed successfully".to_string();
        self.updated_at = Some(chrono::Utc::now().to_rfc3339());
    }

    /// Mark ingestion as failed.
    pub fn fail(&mut self, error: IngestionError) {
        self.current_stage = UnifiedStage::Failed;
        self.error = Some(error.clone());
        self.message = error.message.clone();
        self.updated_at = Some(chrono::Utc::now().to_rfc3339());
    }
}

/// Unified ingestion error.
///
/// Provides structured error information for both PDF and Markdown failures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionError {
    /// Error code (e.g., "PDF_CONVERSION_FAILED", "LLM_TIMEOUT")
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Stage where error occurred
    pub stage: UnifiedStage,
    /// Additional error details (page number, chunk index, etc.)
    pub details: Option<serde_json::Value>,
    /// Whether user can retry
    pub recoverable: bool,
}

impl IngestionError {
    /// Create a new ingestion error.
    pub fn new(code: impl Into<String>, message: impl Into<String>, stage: UnifiedStage) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            stage,
            details: None,
            recoverable: false,
        }
    }

    /// Add details to the error.
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Mark error as recoverable.
    pub fn recoverable(mut self) -> Self {
        self.recoverable = true;
        self
    }
}

/// Common error codes for ingestion failures.
pub mod error_codes {
    /// PDF conversion failed
    pub const PDF_CONVERSION_FAILED: &str = "PDF_CONVERSION_FAILED";
    /// PDF page extraction failed
    pub const PDF_PAGE_FAILED: &str = "PDF_PAGE_FAILED";
    /// LLM extraction failed
    pub const LLM_EXTRACTION_FAILED: &str = "LLM_EXTRACTION_FAILED";
    /// LLM timeout
    pub const LLM_TIMEOUT: &str = "LLM_TIMEOUT";
    /// LLM rate limited
    pub const LLM_RATE_LIMITED: &str = "LLM_RATE_LIMITED";
    /// Embedding generation failed
    pub const EMBEDDING_FAILED: &str = "EMBEDDING_FAILED";
    /// Storage write failed
    pub const STORAGE_FAILED: &str = "STORAGE_FAILED";
    /// Validation failed
    pub const VALIDATION_FAILED: &str = "VALIDATION_FAILED";
    /// Document too large
    pub const DOCUMENT_TOO_LARGE: &str = "DOCUMENT_TOO_LARGE";
    /// Unsupported format
    pub const UNSUPPORTED_FORMAT: &str = "UNSUPPORTED_FORMAT";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stages_for_pdf() {
        let stages = UnifiedStage::stages_for_source(SourceType::Pdf);
        assert!(stages.contains(&UnifiedStage::Converting));
        assert_eq!(stages.len(), 10);
    }

    #[test]
    fn test_stages_for_markdown() {
        let stages = UnifiedStage::stages_for_source(SourceType::Markdown);
        assert!(!stages.contains(&UnifiedStage::Converting));
        assert_eq!(stages.len(), 9);
    }

    #[test]
    fn test_progress_calculation() {
        let mut progress = IngestionProgress::new(
            "test-123".to_string(),
            SourceType::Markdown,
            Some("test.md".to_string()),
        );

        // Initially 0%
        assert_eq!(progress.overall_progress, 0.0);

        // Complete first stage
        progress.start_stage(UnifiedStage::Uploading);
        progress.complete_stage(UnifiedStage::Uploading, Some(100));

        // Should be ~11% (1/9 stages)
        assert!(progress.overall_progress > 0.1);

        // Complete all stages
        progress.complete();
        assert_eq!(progress.overall_progress, 1.0);
    }

    #[test]
    fn test_error_creation() {
        let error = IngestionError::new(
            error_codes::PDF_CONVERSION_FAILED,
            "Failed to extract page 5",
            UnifiedStage::Converting,
        )
        .with_details(serde_json::json!({"page": 5}))
        .recoverable();

        assert_eq!(error.code, "PDF_CONVERSION_FAILED");
        assert!(error.recoverable);
        assert!(error.details.is_some());
    }
}
