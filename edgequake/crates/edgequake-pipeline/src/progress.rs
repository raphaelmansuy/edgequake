//! Progress and Cost Tracking.
//!
//! Provides real-time progress monitoring and cost estimation for
//! ingestion pipeline operations.
//!
//! @implements FEAT0012 (Progress Reporting)
//! @implements FEAT0013 (Cost Tracking)
//!
//! # Progress Tracking
//!
//! Track ingestion progress across pipeline stages:
//! - Preprocessing, Chunking, Extracting, Gleaning
//! - Merging, Summarizing, Embedding, Storing
//!
//! # Cost Estimation
//!
//! Estimate and track LLM API costs based on token usage
//! with configurable pricing for different models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

// ============================================================================
// Cost Tracking
// ============================================================================

/// Model pricing information (per 1K tokens).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Model name.
    pub model: String,
    /// Cost per 1K input tokens (USD).
    pub input_cost_per_1k: f64,
    /// Cost per 1K output tokens (USD).
    pub output_cost_per_1k: f64,
}

impl ModelPricing {
    /// Create new pricing config.
    pub fn new(model: impl Into<String>, input_cost: f64, output_cost: f64) -> Self {
        Self {
            model: model.into(),
            input_cost_per_1k: input_cost,
            output_cost_per_1k: output_cost,
        }
    }

    /// Calculate cost for token usage.
    pub fn calculate_cost(&self, input_tokens: usize, output_tokens: usize) -> f64 {
        let input_cost = (input_tokens as f64 / 1000.0) * self.input_cost_per_1k;
        let output_cost = (output_tokens as f64 / 1000.0) * self.output_cost_per_1k;
        input_cost + output_cost
    }
}

/// Cost for a single operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OperationCost {
    /// Operation type (extract, glean, summarize, embed).
    pub operation: String,
    /// Number of calls.
    pub call_count: usize,
    /// Total input tokens.
    pub input_tokens: usize,
    /// Total output tokens.
    pub output_tokens: usize,
    /// Total cost (USD).
    pub total_cost_usd: f64,
}

impl OperationCost {
    /// Create new operation cost tracker.
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            ..Default::default()
        }
    }

    /// Add usage to this operation.
    pub fn add(&mut self, input: usize, output: usize, cost: f64) {
        self.call_count += 1;
        self.input_tokens += input;
        self.output_tokens += output;
        self.total_cost_usd += cost;
    }
}

/// Complete cost breakdown for an ingestion job.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CostBreakdown {
    /// Job ID.
    pub job_id: String,
    /// Model used.
    pub model: String,
    /// Per-operation costs.
    pub operations: HashMap<String, OperationCost>,
    /// Total input tokens.
    pub total_input_tokens: usize,
    /// Total output tokens.
    pub total_output_tokens: usize,
    /// Total cost (USD).
    pub total_cost_usd: f64,
}

impl CostBreakdown {
    /// Create new cost breakdown.
    pub fn new(job_id: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            job_id: job_id.into(),
            model: model.into(),
            ..Default::default()
        }
    }

    /// Add cost for an operation.
    pub fn add_operation_cost(&mut self, operation: &str, input: usize, output: usize, cost: f64) {
        let op = self
            .operations
            .entry(operation.to_string())
            .or_insert_with(|| OperationCost::new(operation));

        op.add(input, output, cost);
        self.total_input_tokens += input;
        self.total_output_tokens += output;
        self.total_cost_usd += cost;
    }

    /// Get formatted cost string.
    pub fn formatted_cost(&self) -> String {
        format!("${:.4}", self.total_cost_usd)
    }
}

/// Thread-safe cost tracker.
#[derive(Debug)]
pub struct CostTracker {
    inner: Arc<RwLock<CostBreakdown>>,
    pricing: ModelPricing,
}

impl CostTracker {
    /// Create new cost tracker.
    pub fn new(job_id: impl Into<String>, model: impl Into<String>, pricing: ModelPricing) -> Self {
        let model_str = model.into();
        Self {
            inner: Arc::new(RwLock::new(CostBreakdown::new(job_id, &model_str))),
            pricing,
        }
    }

    /// Create with default gpt-4.1-nano pricing (recommended cost-effective model).
    pub fn new_gpt5_nano(job_id: impl Into<String>) -> Self {
        let pricing = ModelPricing::new("gpt-4.1-nano", 0.00015, 0.0006);
        Self::new(job_id, "gpt-4.1-nano", pricing)
    }

    /// Create with gpt-4o-mini pricing (legacy, prefer gpt-4.1-nano).
    ///
    /// # Deprecation Notice
    /// This function is deprecated. Use `new_gpt5_nano()` instead for better
    /// cost efficiency and availability. gpt-4o-mini quotas may be exceeded.
    #[deprecated(
        since = "0.1.0",
        note = "Use new_gpt5_nano() for better cost efficiency"
    )]
    pub fn new_gpt4o_mini(job_id: impl Into<String>) -> Self {
        let pricing = ModelPricing::new("gpt-4o-mini", 0.00015, 0.0006);
        Self::new(job_id, "gpt-4o-mini", pricing)
    }

    /// Create with gpt-4o pricing.
    pub fn new_gpt4o(job_id: impl Into<String>) -> Self {
        let pricing = ModelPricing::new("gpt-4o", 0.005, 0.015);
        Self::new(job_id, "gpt-4o", pricing)
    }

    /// Record token usage for an operation.
    pub async fn record(&self, operation: &str, input_tokens: usize, output_tokens: usize) {
        let cost = self.pricing.calculate_cost(input_tokens, output_tokens);
        let mut breakdown = self.inner.write().await;
        breakdown.add_operation_cost(operation, input_tokens, output_tokens, cost);
    }

    /// Get current cost breakdown.
    pub async fn snapshot(&self) -> CostBreakdown {
        self.inner.read().await.clone()
    }

    /// Get total cost so far.
    pub async fn total_cost(&self) -> f64 {
        self.inner.read().await.total_cost_usd
    }
}

impl Clone for CostTracker {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            pricing: self.pricing.clone(),
        }
    }
}

/// Default model pricing configurations.
pub fn default_model_pricing() -> HashMap<String, ModelPricing> {
    let mut pricing = HashMap::new();

    // OpenAI models
    pricing.insert(
        "gpt-4.1-nano".to_string(),
        ModelPricing::new("gpt-4.1-nano", 0.00015, 0.0006),
    );
    pricing.insert(
        "gpt-4o-mini".to_string(),
        ModelPricing::new("gpt-4o-mini", 0.00015, 0.0006),
    );
    pricing.insert(
        "gpt-4o".to_string(),
        ModelPricing::new("gpt-4o", 0.005, 0.015),
    );
    pricing.insert(
        "gpt-4-turbo".to_string(),
        ModelPricing::new("gpt-4-turbo", 0.01, 0.03),
    );
    pricing.insert(
        "gpt-3.5-turbo".to_string(),
        ModelPricing::new("gpt-3.5-turbo", 0.0005, 0.0015),
    );

    // Anthropic models
    pricing.insert(
        "claude-3-haiku".to_string(),
        ModelPricing::new("claude-3-haiku", 0.00025, 0.00125),
    );
    pricing.insert(
        "claude-3-sonnet".to_string(),
        ModelPricing::new("claude-3-sonnet", 0.003, 0.015),
    );
    pricing.insert(
        "claude-3-opus".to_string(),
        ModelPricing::new("claude-3-opus", 0.015, 0.075),
    );

    // Embedding models
    pricing.insert(
        "text-embedding-3-small".to_string(),
        ModelPricing::new("text-embedding-3-small", 0.00002, 0.0),
    );
    pricing.insert(
        "text-embedding-3-large".to_string(),
        ModelPricing::new("text-embedding-3-large", 0.00013, 0.0),
    );

    pricing
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

    #[test]
    fn test_model_pricing() {
        let pricing = ModelPricing::new("gpt-4o-mini", 0.00015, 0.0006);

        let cost = pricing.calculate_cost(1000, 500);
        // 1000 input = $0.00015, 500 output = $0.0003
        assert!((cost - 0.00045).abs() < 0.00001);
    }

    #[tokio::test]
    async fn test_cost_tracker() {
        let tracker = CostTracker::new_gpt5_nano("job-1");

        tracker.record("extract", 1000, 500).await;
        tracker.record("extract", 2000, 1000).await;

        let breakdown = tracker.snapshot().await;
        assert_eq!(breakdown.operations.len(), 1);
        assert_eq!(breakdown.operations["extract"].call_count, 2);
        assert_eq!(breakdown.total_input_tokens, 3000);
        assert_eq!(breakdown.total_output_tokens, 1500);
    }

    #[test]
    fn test_default_model_pricing() {
        let pricing = default_model_pricing();
        assert!(pricing.contains_key("gpt-4o-mini"));
        assert!(pricing.contains_key("claude-3-haiku"));
        assert!(pricing.contains_key("text-embedding-3-small"));
    }
}
