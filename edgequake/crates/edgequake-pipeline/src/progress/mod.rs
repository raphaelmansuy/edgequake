//! Progress and Cost Tracking.
//!
//! Provides real-time progress monitoring and cost estimation for
//! ingestion pipeline operations.
//!
//! @implements FEAT0012 (Progress Reporting)
//!
//! # Architecture
//!
//! - Progress tracking: pipeline stage types, ProgressTracker
//! - `cost`: LLM API cost estimation with per-model pricing

mod cost;

pub use cost::{default_model_pricing, CostBreakdown, CostTracker, ModelPricing, OperationCost};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Overall ingestion status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum IngestionStatus {
    /// Waiting to start.
    #[default]
    Pending,
    /// Currently processing.
    Running,
    /// Successfully completed.
    Completed,
    /// Failed with errors.
    Failed,
    /// Cancelled by user.
    Cancelled,
}

/// Pipeline processing stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum PipelineStage {
    /// Initial preprocessing (validation, parsing).
    Preprocessing,
    /// Document chunking.
    Chunking,
    /// Entity/relationship extraction.
    Extracting,
    /// Gleaning (re-extraction for missed entities).
    Gleaning,
    /// Merging entities into graph.
    Merging,
    /// Summarizing descriptions.
    Summarizing,
    /// Generating embeddings.
    Embedding,
    /// Storing results.
    Storing,
    /// Finalizing job.
    Finalizing,
}

impl PipelineStage {
    /// Get all stages in order.
    pub fn all() -> Vec<PipelineStage> {
        vec![
            PipelineStage::Preprocessing,
            PipelineStage::Chunking,
            PipelineStage::Extracting,
            PipelineStage::Gleaning,
            PipelineStage::Merging,
            PipelineStage::Summarizing,
            PipelineStage::Embedding,
            PipelineStage::Storing,
            PipelineStage::Finalizing,
        ]
    }

    /// Get stage name as string.
    pub fn name(&self) -> &'static str {
        match self {
            PipelineStage::Preprocessing => "Preprocessing",
            PipelineStage::Chunking => "Chunking",
            PipelineStage::Extracting => "Extracting",
            PipelineStage::Gleaning => "Gleaning",
            PipelineStage::Merging => "Merging",
            PipelineStage::Summarizing => "Summarizing",
            PipelineStage::Embedding => "Embedding",
            PipelineStage::Storing => "Storing",
            PipelineStage::Finalizing => "Finalizing",
        }
    }
}

/// Status of a single pipeline stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum StageStatus {
    /// Not started yet.
    #[default]
    Pending,
    /// Currently running.
    Running,
    /// Successfully completed.
    Completed,
    /// Skipped (not applicable).
    Skipped,
    /// Failed with error.
    Failed,
}

/// Progress for a single pipeline stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageProgress {
    /// The stage.
    pub stage: PipelineStage,
    /// Current status.
    pub status: StageStatus,
    /// Total items to process.
    pub total_items: usize,
    /// Items completed.
    pub completed_items: usize,
    /// Completion percentage (0-100).
    pub completion_percentage: f32,
    /// When stage started.
    pub started_at: Option<DateTime<Utc>>,
    /// When stage completed.
    pub completed_at: Option<DateTime<Utc>>,
}

impl StageProgress {
    /// Create new pending stage progress.
    pub fn new(stage: PipelineStage, total_items: usize) -> Self {
        Self {
            stage,
            status: StageStatus::Pending,
            total_items,
            completed_items: 0,
            completion_percentage: 0.0,
            started_at: None,
            completed_at: None,
        }
    }

    /// Mark stage as running.
    pub fn start(&mut self) {
        self.status = StageStatus::Running;
        self.started_at = Some(Utc::now());
    }

    /// Update progress.
    pub fn update(&mut self, completed: usize) {
        self.completed_items = completed;
        if self.total_items > 0 {
            self.completion_percentage = (completed as f32 / self.total_items as f32) * 100.0;
        }
    }

    /// Mark stage as completed.
    pub fn complete(&mut self) {
        self.status = StageStatus::Completed;
        self.completed_items = self.total_items;
        self.completion_percentage = 100.0;
        self.completed_at = Some(Utc::now());
    }

    /// Mark stage as failed.
    pub fn fail(&mut self) {
        self.status = StageStatus::Failed;
        self.completed_at = Some(Utc::now());
    }

    /// Mark stage as skipped.
    pub fn skip(&mut self) {
        self.status = StageStatus::Skipped;
        self.completed_at = Some(Utc::now());
    }
}

/// Message severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageLevel {
    Debug,
    Info,
    Warning,
    Error,
}

/// A progress message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressMessage {
    /// The message content.
    pub message: String,
    /// Message severity.
    pub level: MessageLevel,
    /// When the message was created.
    pub timestamp: DateTime<Utc>,
}

impl ProgressMessage {
    /// Create a new progress message.
    pub fn new(message: impl Into<String>, level: MessageLevel) -> Self {
        Self {
            message: message.into(),
            level,
            timestamp: Utc::now(),
        }
    }

    /// Create info message.
    pub fn info(message: impl Into<String>) -> Self {
        Self::new(message, MessageLevel::Info)
    }

    /// Create warning message.
    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(message, MessageLevel::Warning)
    }

    /// Create error message.
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(message, MessageLevel::Error)
    }
}

/// Error that occurred during ingestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionError {
    /// Error code (e.g., "E001").
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// Additional details.
    pub details: Option<String>,
    /// Stage where error occurred.
    pub stage: PipelineStage,
    /// Related item ID (chunk_id, entity_name, etc.).
    pub item_id: Option<String>,
    /// Whether error is recoverable.
    pub recoverable: bool,
    /// Number of retry attempts.
    pub retry_count: usize,
    /// When error occurred.
    pub occurred_at: DateTime<Utc>,
}

impl IngestionError {
    /// Create a new ingestion error.
    pub fn new(code: impl Into<String>, message: impl Into<String>, stage: PipelineStage) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
            stage,
            item_id: None,
            recoverable: false,
            retry_count: 0,
            occurred_at: Utc::now(),
        }
    }

    /// Set error details.
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    /// Set related item ID.
    pub fn with_item_id(mut self, item_id: impl Into<String>) -> Self {
        self.item_id = Some(item_id.into());
        self
    }

    /// Mark as recoverable.
    pub fn recoverable(mut self) -> Self {
        self.recoverable = true;
        self
    }
}

/// Complete ingestion progress snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionProgress {
    /// Job identifier.
    pub job_id: String,
    /// Document identifier.
    pub document_id: String,
    /// Overall status.
    pub status: IngestionStatus,
    /// Current stage.
    pub current_stage: PipelineStage,
    /// Progress for each stage.
    pub stages: Vec<StageProgress>,
    /// Overall completion percentage.
    pub completion_percentage: f32,
    /// Estimated time remaining (seconds).
    pub eta_seconds: Option<u64>,
    /// Latest status message.
    pub latest_message: String,
    /// Message history.
    pub history_messages: Vec<ProgressMessage>,
    /// Errors encountered.
    pub errors: Vec<IngestionError>,
    /// When job started.
    pub started_at: DateTime<Utc>,
    /// When last updated.
    pub updated_at: DateTime<Utc>,
    /// When job completed.
    pub completed_at: Option<DateTime<Utc>>,
}

impl IngestionProgress {
    /// Create new progress tracker for a job.
    pub fn new(job_id: impl Into<String>, document_id: impl Into<String>) -> Self {
        let now = Utc::now();
        let stages = PipelineStage::all()
            .into_iter()
            .map(|s| StageProgress::new(s, 0))
            .collect();

        Self {
            job_id: job_id.into(),
            document_id: document_id.into(),
            status: IngestionStatus::Pending,
            current_stage: PipelineStage::Preprocessing,
            stages,
            completion_percentage: 0.0,
            eta_seconds: None,
            latest_message: "Initializing...".to_string(),
            history_messages: Vec::new(),
            errors: Vec::new(),
            started_at: now,
            updated_at: now,
            completed_at: None,
        }
    }

    /// Calculate overall completion percentage.
    pub fn calculate_completion(&mut self) {
        let total_stages = self.stages.len() as f32;
        let completed: f32 = self
            .stages
            .iter()
            .map(|s| match s.status {
                StageStatus::Completed | StageStatus::Skipped => 1.0,
                StageStatus::Running => s.completion_percentage / 100.0,
                _ => 0.0,
            })
            .sum();

        self.completion_percentage = (completed / total_stages) * 100.0;
    }
}

/// Thread-safe progress tracker.
#[derive(Debug)]
pub struct ProgressTracker {
    inner: Arc<RwLock<IngestionProgress>>,
}

impl ProgressTracker {
    /// Create a new progress tracker.
    pub fn new(job_id: impl Into<String>, document_id: impl Into<String>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(IngestionProgress::new(job_id, document_id))),
        }
    }

    /// Start the job.
    pub async fn start(&self) {
        let mut progress = self.inner.write().await;
        progress.status = IngestionStatus::Running;
        progress.started_at = Utc::now();
        progress.updated_at = Utc::now();
    }

    /// Set current stage and item count.
    pub async fn set_stage(&self, stage: PipelineStage, total_items: usize) {
        let mut progress = self.inner.write().await;
        progress.current_stage = stage;

        if let Some(sp) = progress.stages.iter_mut().find(|s| s.stage == stage) {
            sp.total_items = total_items;
            sp.start();
        }

        progress.updated_at = Utc::now();
    }

    /// Update stage progress.
    pub async fn update_stage(&self, stage: PipelineStage, completed: usize) {
        let mut progress = self.inner.write().await;

        if let Some(sp) = progress.stages.iter_mut().find(|s| s.stage == stage) {
            sp.update(completed);
        }

        progress.calculate_completion();
        progress.updated_at = Utc::now();
    }

    /// Complete a stage.
    pub async fn complete_stage(&self, stage: PipelineStage) {
        let mut progress = self.inner.write().await;

        if let Some(sp) = progress.stages.iter_mut().find(|s| s.stage == stage) {
            sp.complete();
        }

        progress.calculate_completion();
        progress.updated_at = Utc::now();
    }

    /// Skip a stage.
    pub async fn skip_stage(&self, stage: PipelineStage) {
        let mut progress = self.inner.write().await;

        if let Some(sp) = progress.stages.iter_mut().find(|s| s.stage == stage) {
            sp.skip();
        }

        progress.calculate_completion();
        progress.updated_at = Utc::now();
    }

    /// Add a message.
    pub async fn add_message(&self, message: impl Into<String>, level: MessageLevel) {
        let mut progress = self.inner.write().await;
        let msg = ProgressMessage::new(message, level);
        progress.latest_message = msg.message.clone();
        progress.history_messages.push(msg);
        progress.updated_at = Utc::now();
    }

    /// Add an error.
    pub async fn add_error(&self, error: IngestionError) {
        let mut progress = self.inner.write().await;
        progress.errors.push(error);
        progress.updated_at = Utc::now();
    }

    /// Complete the job.
    pub async fn complete(&self) {
        let mut progress = self.inner.write().await;
        progress.status = IngestionStatus::Completed;
        progress.completion_percentage = 100.0;
        progress.completed_at = Some(Utc::now());
        progress.updated_at = Utc::now();
    }

    /// Fail the job.
    pub async fn fail(&self, error: IngestionError) {
        let mut progress = self.inner.write().await;
        progress.status = IngestionStatus::Failed;
        progress.errors.push(error);
        progress.completed_at = Some(Utc::now());
        progress.updated_at = Utc::now();
    }

    /// Get current progress snapshot.
    pub async fn snapshot(&self) -> IngestionProgress {
        self.inner.read().await.clone()
    }
}

impl Clone for ProgressTracker {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_stage_all() {
        let stages = PipelineStage::all();
        assert_eq!(stages.len(), 9);
        assert_eq!(stages[0], PipelineStage::Preprocessing);
        assert_eq!(stages[8], PipelineStage::Finalizing);
    }

    #[test]
    fn test_stage_progress() {
        let mut sp = StageProgress::new(PipelineStage::Extracting, 10);
        assert_eq!(sp.status, StageStatus::Pending);

        sp.start();
        assert_eq!(sp.status, StageStatus::Running);
        assert!(sp.started_at.is_some());

        sp.update(5);
        assert_eq!(sp.completed_items, 5);
        assert!((sp.completion_percentage - 50.0).abs() < 0.1);

        sp.complete();
        assert_eq!(sp.status, StageStatus::Completed);
        assert_eq!(sp.completion_percentage, 100.0);
    }

    #[test]
    fn test_progress_message() {
        let msg = ProgressMessage::info("Processing started");
        assert_eq!(msg.level, MessageLevel::Info);
        assert_eq!(msg.message, "Processing started");
    }

    #[test]
    fn test_ingestion_error() {
        let error = IngestionError::new("E001", "Extraction failed", PipelineStage::Extracting)
            .with_item_id("chunk-1")
            .with_details("LLM returned invalid response")
            .recoverable();

        assert_eq!(error.code, "E001");
        assert!(error.recoverable);
        assert_eq!(error.item_id, Some("chunk-1".to_string()));
    }

    #[tokio::test]
    async fn test_progress_tracker() {
        let tracker = ProgressTracker::new("job-1", "doc-1");

        tracker.start().await;
        let snapshot = tracker.snapshot().await;
        assert_eq!(snapshot.status, IngestionStatus::Running);

        tracker.set_stage(PipelineStage::Extracting, 10).await;
        tracker.update_stage(PipelineStage::Extracting, 5).await;

        let snapshot = tracker.snapshot().await;
        assert_eq!(snapshot.current_stage, PipelineStage::Extracting);

        tracker.complete_stage(PipelineStage::Extracting).await;
        tracker.complete().await;

        let snapshot = tracker.snapshot().await;
        assert_eq!(snapshot.status, IngestionStatus::Completed);
    }
}
