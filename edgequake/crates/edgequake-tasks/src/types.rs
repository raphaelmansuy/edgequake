//! Task types and models for background processing.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Task status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    /// Task created but not yet started
    Pending,
    /// Task is currently being processed
    Processing,
    /// Task completed successfully and document is indexed
    Indexed,
    /// Task failed with error
    Failed,
    /// Task was cancelled
    Cancelled,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::Processing => write!(f, "processing"),
            TaskStatus::Indexed => write!(f, "indexed"),
            TaskStatus::Failed => write!(f, "failed"),
            TaskStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Task type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    /// File upload task
    Upload,
    /// Direct text insertion
    Insert,
    /// Directory scanning
    Scan,
    /// Reindex documents
    Reindex,
    /// PDF processing task (SPEC-007)
    #[serde(rename = "pdf_processing")]
    PdfProcessing,
}

impl fmt::Display for TaskType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskType::Upload => write!(f, "upload"),
            TaskType::Insert => write!(f, "insert"),
            TaskType::Scan => write!(f, "scan"),
            TaskType::Reindex => write!(f, "reindex"),
            TaskType::PdfProcessing => write!(f, "pdf_processing"),
        }
    }
}

/// Main Task structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique track ID: {type}-{uuid}
    pub track_id: String,

    /// Tenant ID for multi-tenancy isolation
    pub tenant_id: Uuid,

    /// Workspace ID for workspace-level isolation
    pub workspace_id: Uuid,

    /// Type of task
    pub task_type: TaskType,

    /// Current status
    pub status: TaskStatus,

    /// When task was created
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// When processing started
    pub started_at: Option<DateTime<Utc>>,

    /// When task completed (success or failure)
    pub completed_at: Option<DateTime<Utc>>,

    /// Error message if failed (kept for backward compatibility)
    pub error_message: Option<String>,

    /// Detailed error information (Phase 1 enhancement)
    pub error: Option<TaskFailureInfo>,

    /// Number of retry attempts
    pub retry_count: i32,

    /// Maximum retries allowed
    pub max_retries: i32,

    /// Number of consecutive timeout failures (for circuit breaker).
    ///
    /// @implements CIRCUIT_BREAKER: Track consecutive timeouts
    ///
    /// WHY: Timeouts indicate document is too large or LLM is overloaded.
    /// After 3 consecutive timeouts, we should fail permanently rather than
    /// waste resources retrying. Non-timeout failures (network errors, etc.)
    /// don't count toward this limit because they may resolve on retry.
    ///
    /// Reset to 0 on:
    /// - Successful processing
    /// - Non-timeout failure (network error, validation error, etc.)
    ///
    /// Increment on:
    /// - LLM timeout error
    /// - Embedding timeout error
    ///
    /// Circuit breaker trips when: consecutive_timeout_failures >= 3
    pub consecutive_timeout_failures: i32,

    /// Whether circuit breaker has permanently failed this task.
    ///
    /// @implements CIRCUIT_BREAKER: Permanent failure flag
    ///
    /// WHY: Prevents infinite retries on documents that consistently timeout.
    /// Once tripped, task is marked Failed and won't be retried again.
    pub circuit_breaker_tripped: bool,

    /// Task-specific payload
    pub task_data: serde_json::Value,

    /// Additional metadata
    pub metadata: Option<serde_json::Value>,

    /// Progress information
    pub progress: Option<TaskProgress>,

    /// Result data (on success)
    pub result: Option<serde_json::Value>,
}

/// Task progress tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgress {
    pub current_step: String,
    pub total_steps: u32,
    pub percent_complete: u8,

    /// Chunk-level progress for document processing.
    ///
    /// @implements SPEC-001/Objective-A: Chunk-Level Progress Visibility
    ///
    /// WHY: The real progression of document ingestion is chunks processed
    /// vs chunks remaining. This field provides granular visibility into
    /// the map-reduce extraction phase where each chunk is processed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_progress: Option<ChunkProgress>,
}

/// Chunk-level progress tracking for a document being processed.
///
/// @implements SPEC-001/Objective-A: Chunk-Level Progress Visibility
///
/// WHY: After chunking, documents go through a MAP-REDUCE extraction phase:
/// - MAP: Each chunk → LLM extraction + embedding generation
/// - REDUCE: Merge entities, deduplicate, build relationships
///
/// This struct tracks the MAP phase progress at chunk granularity.
///
/// ```text
/// ┌──────────────────────────────────────────────────────────────┐
/// │ CHUNK PROGRESS VISUALIZATION                                 │
/// ├──────────────────────────────────────────────────────────────┤
/// │ Chunks: [████████████░░░░░░░░░░░░░░░░░░] 12/35 (34%)        │
/// │ Current: Chunk 12 - "Section 3.2: Methodology..."            │
/// │ Avg time/chunk: 2.3s | ETA: ~53s remaining                   │
/// │ Tokens: 45,230 in / 8,450 out | Cost: $0.0089               │
/// └──────────────────────────────────────────────────────────────┘
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkProgress {
    /// Total number of chunks in the document.
    pub total_chunks: u32,

    /// Number of chunks fully processed (extracted + embedded).
    pub processed_chunks: u32,

    /// Current chunk index being processed (0-based).
    pub current_chunk_index: u32,

    /// Preview of current chunk content (first 80 chars, truncated).
    pub current_chunk_preview: String,

    /// Average time to process a single chunk (milliseconds).
    pub avg_chunk_time_ms: f64,

    /// Estimated time remaining (seconds).
    pub eta_seconds: u64,

    /// Total input tokens consumed so far.
    pub tokens_in: u64,

    /// Total output tokens consumed so far.
    pub tokens_out: u64,

    /// Running cost estimate (USD).
    pub cost_usd: f64,
}

impl ChunkProgress {
    /// Create a new ChunkProgress at the start of processing.
    pub fn new(total_chunks: u32) -> Self {
        Self {
            total_chunks,
            processed_chunks: 0,
            current_chunk_index: 0,
            current_chunk_preview: String::new(),
            avg_chunk_time_ms: 0.0,
            eta_seconds: 0,
            tokens_in: 0,
            tokens_out: 0,
            cost_usd: 0.0,
        }
    }

    /// Update progress after processing a chunk.
    pub fn update(
        &mut self,
        chunk_index: u32,
        chunk_preview: &str,
        elapsed_ms: u64,
        input_tokens: u64,
        output_tokens: u64,
        chunk_cost: f64,
    ) {
        self.processed_chunks = chunk_index + 1;
        self.current_chunk_index = chunk_index;
        self.current_chunk_preview = truncate_preview(chunk_preview, 80);
        self.tokens_in += input_tokens;
        self.tokens_out += output_tokens;
        self.cost_usd += chunk_cost;

        // Calculate running average time per chunk
        if self.processed_chunks > 0 {
            // Exponential moving average for smoother ETA
            let alpha = 0.3;
            if self.avg_chunk_time_ms == 0.0 {
                self.avg_chunk_time_ms = elapsed_ms as f64;
            } else {
                self.avg_chunk_time_ms =
                    alpha * (elapsed_ms as f64) + (1.0 - alpha) * self.avg_chunk_time_ms;
            }

            // Calculate ETA
            let remaining = self.total_chunks.saturating_sub(self.processed_chunks);
            self.eta_seconds = ((remaining as f64 * self.avg_chunk_time_ms) / 1000.0) as u64;
        }
    }

    /// Get completion percentage (0-100).
    pub fn percent_complete(&self) -> u8 {
        if self.total_chunks == 0 {
            return 0;
        }
        ((self.processed_chunks as f64 / self.total_chunks as f64) * 100.0) as u8
    }
}

/// Truncate a string to max_len characters, adding "..." if truncated.
fn truncate_preview(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        // Find a safe UTF-8 boundary
        let mut end = max_len.saturating_sub(3);
        while !s.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        format!("{}...", &s[..end])
    }
}

/// Detailed error information for failed tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFailureInfo {
    /// High-level error message.
    pub message: String,
    /// Processing step where failure occurred: "chunking", "embedding", "extraction", "indexing".
    pub step: String,
    /// Specific reason for the failure.
    pub reason: String,
    /// Suggested action to fix the issue.
    pub suggestion: String,
    /// Whether this error is retryable.
    pub retryable: bool,
}

impl TaskFailureInfo {
    /// Create a new task error.
    pub fn new(
        message: impl Into<String>,
        step: impl Into<String>,
        reason: impl Into<String>,
        suggestion: impl Into<String>,
        retryable: bool,
    ) -> Self {
        Self {
            message: message.into(),
            step: step.into(),
            reason: reason.into(),
            suggestion: suggestion.into(),
            retryable,
        }
    }

    /// Create a chunking error.
    pub fn chunking(reason: impl Into<String>) -> Self {
        Self::new(
            "Document chunking failed",
            "chunking",
            reason,
            "Check document format and encoding",
            true,
        )
    }

    /// Create a timeout error (LLM or embedding).
    ///
    /// @implements CIRCUIT_BREAKER: Timeout classification
    ///
    /// WHY: Timeouts need special handling via circuit breaker pattern.
    /// Consecutive timeouts indicate structural problem (doc too large,
    /// LLM overloaded) that won't resolve by retrying.
    pub fn timeout(step: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::new(
            "Operation timed out",
            step,
            reason,
            "Document may be too large. Try: 1) Use smaller chunk size, 2) Split document, 3) Use provider with longer timeout",
            false, // Not retryable after circuit breaker trips
        )
    }

    /// Check if this error represents a timeout.
    ///
    /// @implements CIRCUIT_BREAKER: Timeout detection
    pub fn is_timeout(&self) -> bool {
        self.message.to_lowercase().contains("timeout")
            || self.reason.to_lowercase().contains("timeout")
            || self.reason.to_lowercase().contains("timed out")
    }

    /// Create an embedding error.
    pub fn embedding(reason: impl Into<String>) -> Self {
        Self::new(
            "Embedding generation failed",
            "embedding",
            reason,
            "Check LLM provider connectivity and API limits",
            true,
        )
    }

    /// Create an extraction error.
    pub fn extraction(reason: impl Into<String>) -> Self {
        Self::new(
            "Entity extraction failed",
            "extraction",
            reason,
            "Check LLM provider connectivity and API limits",
            true,
        )
    }

    /// Create an indexing error.
    pub fn indexing(reason: impl Into<String>) -> Self {
        Self::new(
            "Graph indexing failed",
            "indexing",
            reason,
            "Check storage backend connectivity",
            true,
        )
    }

    /// Create a rate limit error.
    pub fn rate_limit(step: impl Into<String>) -> Self {
        Self::new(
            "Rate limit exceeded",
            step,
            "API rate limit exceeded",
            "Wait 30 seconds and retry, or reduce batch size",
            true,
        )
    }
}

impl Task {
    /// Create a new task
    pub fn new(
        tenant_id: Uuid,
        workspace_id: Uuid,
        task_type: TaskType,
        task_data: serde_json::Value,
    ) -> Self {
        let now = Utc::now();
        let track_id = generate_track_id(task_type);

        Self {
            track_id,
            tenant_id,
            workspace_id,
            task_type,
            status: TaskStatus::Pending,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
            error_message: None,
            error: None,
            retry_count: 0,
            max_retries: 3,
            consecutive_timeout_failures: 0,
            circuit_breaker_tripped: false,
            task_data,
            metadata: None,
            progress: None,
            result: None,
        }
    }

    /// Mark task as processing
    pub fn mark_processing(&mut self) {
        self.status = TaskStatus::Processing;
        self.started_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Mark task as completed successfully
    pub fn mark_success(&mut self, result: serde_json::Value) {
        self.status = TaskStatus::Indexed;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
        self.result = Some(result);
        self.error = None;
        self.error_message = None;
        // Reset timeout counter on success
        self.consecutive_timeout_failures = 0;
    }

    /// Mark task as failed with simple error message (backward compatible)
    pub fn mark_failed(&mut self, error: String) {
        self.status = TaskStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
        self.error_message = Some(error.clone());
        self.retry_count += 1;

        // Check if this is a timeout error
        let error_lower = error.to_lowercase();
        if error_lower.contains("timeout") || error_lower.contains("timed out") {
            self.consecutive_timeout_failures += 1;
            self.check_circuit_breaker();
        } else {
            // Non-timeout failures reset the counter
            self.consecutive_timeout_failures = 0;
        }
    }

    /// Mark task as failed with detailed error information (Phase 1 enhancement)
    pub fn mark_failed_with_details(&mut self, error: TaskFailureInfo) {
        self.status = TaskStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
        self.error_message = Some(error.message.clone());

        // Set error FIRST so check_circuit_breaker can modify it
        self.error = Some(error.clone());

        // Track consecutive timeouts for circuit breaker
        if error.is_timeout() {
            self.consecutive_timeout_failures += 1;
            self.check_circuit_breaker(); // Modifies self.error if circuit breaker trips
        } else {
            // Non-timeout failures reset the counter
            self.consecutive_timeout_failures = 0;
        }

        self.retry_count += 1;
    }

    /// Check circuit breaker and trip if threshold exceeded.
    ///
    /// @implements CIRCUIT_BREAKER: Threshold check
    ///
    /// WHY: After 3 consecutive timeouts, permanently fail the task.
    /// Prevents wasting resources on documents that consistently timeout.
    fn check_circuit_breaker(&mut self) {
        const CIRCUIT_BREAKER_THRESHOLD: i32 = 3;

        if self.consecutive_timeout_failures >= CIRCUIT_BREAKER_THRESHOLD {
            self.circuit_breaker_tripped = true;

            // Enhance error message to indicate circuit breaker tripped
            if let Some(ref mut error) = self.error {
                error.message = format!(
                    "Circuit breaker tripped: {} consecutive timeouts. Task permanently failed.",
                    self.consecutive_timeout_failures
                );
                error.retryable = false;
            } else {
                self.error_message = Some(format!(
                    "Circuit breaker tripped after {} consecutive timeouts. \
                    Document is too large for current LLM timeout settings. \
                    Suggestions: 1) Use smaller chunk size (adaptive chunking), \
                    2) Split document into smaller files, \
                    3) Switch to provider with longer timeout (Ollama: 300s vs OpenAI: 120s)",
                    self.consecutive_timeout_failures
                ));
            }
        }
    }

    /// Mark task as cancelled
    pub fn mark_cancelled(&mut self) {
        self.status = TaskStatus::Cancelled;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Update task progress
    pub fn update_progress(&mut self, current_step: String, total_steps: u32, percent: u8) {
        self.progress = Some(TaskProgress {
            current_step,
            total_steps,
            percent_complete: percent.min(100),
            chunk_progress: None,
        });
        self.updated_at = Utc::now();
    }

    /// Update task progress with chunk-level tracking
    pub fn update_progress_with_chunks(
        &mut self,
        current_step: String,
        total_steps: u32,
        percent: u8,
        chunk_progress: ChunkProgress,
    ) {
        self.progress = Some(TaskProgress {
            current_step,
            total_steps,
            percent_complete: percent.min(100),
            chunk_progress: Some(chunk_progress),
        });
        self.updated_at = Utc::now();
    }

    /// Check if task can be retried
    ///
    /// @implements CIRCUIT_BREAKER: Retry eligibility check
    ///
    /// Returns false if:
    /// - Circuit breaker is tripped (3+ consecutive timeouts)
    /// - Retry limit exceeded
    /// - Error marked as not retryable
    /// - Task is not in Failed status
    pub fn can_retry(&self) -> bool {
        // Circuit breaker prevents retries
        if self.circuit_breaker_tripped {
            return false;
        }

        let is_retryable = self.error.as_ref().map(|e| e.retryable).unwrap_or(true);
        self.status == TaskStatus::Failed && self.retry_count < self.max_retries && is_retryable
    }

    /// Check if task is terminal (completed or permanently failed)
    pub fn is_terminal(&self) -> bool {
        matches!(self.status, TaskStatus::Indexed | TaskStatus::Cancelled)
            || (self.status == TaskStatus::Failed && !self.can_retry())
    }
}

/// Generate a track ID for a task
pub fn generate_track_id(task_type: TaskType) -> String {
    let uuid = Uuid::new_v4();
    format!("{}-{}", task_type, uuid)
}

/// Document upload task payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentUploadData {
    pub file_path: String,
    pub content_type: String,
    pub workspace_id: String,
    pub metadata: Option<serde_json::Value>,
}

/// PDF processing task payload
///
/// @implements SPEC-007: PDF Upload Support
///
/// This structure contains all information needed to process a PDF document:
/// - Extract content (text or vision)
/// - Convert to markdown
/// - Ingest into knowledge graph
///
/// @implements SPEC-002: Unified Ingestion Pipeline
/// OODA-05: Added tenant_id for multi-tenant context propagation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfProcessingData {
    /// PDF document ID
    pub pdf_id: Uuid,

    /// Tenant ID for multi-tenant isolation
    /// OODA-05: Required for document metadata to be visible in workspace queries
    pub tenant_id: Uuid,

    /// Workspace ID for isolation
    pub workspace_id: Uuid,

    /// Enable vision LLM processing
    pub enable_vision: bool,

    /// Vision provider to use (openai, ollama)
    pub vision_provider: String,

    /// Optional vision model override
    pub vision_model: Option<String>,
}

/// Text insert task payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextInsertData {
    pub text: String,
    pub file_source: String,
    pub workspace_id: String,
    pub metadata: Option<serde_json::Value>,
}

/// Directory scan task payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryScanData {
    pub directory_path: String,
    pub recursive: bool,
    pub file_pattern: Option<String>,
    pub workspace_id: String,
}

/// Reindex task payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReindexData {
    pub document_ids: Vec<String>,
    pub workspace_id: String,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test helper constants for tenant/workspace IDs
    const TEST_TENANT_ID: &str = "00000000-0000-0000-0000-000000000001";
    const TEST_WORKSPACE_ID: &str = "00000000-0000-0000-0000-000000000002";

    fn test_tenant_id() -> uuid::Uuid {
        uuid::Uuid::parse_str(TEST_TENANT_ID).unwrap()
    }

    fn test_workspace_id() -> uuid::Uuid {
        uuid::Uuid::parse_str(TEST_WORKSPACE_ID).unwrap()
    }

    #[test]
    fn test_task_creation() {
        let data = serde_json::json!({
            "file_path": "/tmp/test.pdf",
            "workspace_id": "default"
        });

        let task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Upload,
            data,
        );

        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.task_type, TaskType::Upload);
        assert!(task.track_id.starts_with("upload-"));
        assert_eq!(task.retry_count, 0);
        assert_eq!(task.max_retries, 3);
    }

    #[test]
    fn test_task_lifecycle() {
        let data = serde_json::json!({"test": "data"});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            data,
        );

        assert_eq!(task.status, TaskStatus::Pending);

        task.mark_processing();
        assert_eq!(task.status, TaskStatus::Processing);
        assert!(task.started_at.is_some());

        task.mark_success(serde_json::json!({"result": "success"}));
        assert_eq!(task.status, TaskStatus::Indexed);
        assert!(task.completed_at.is_some());
        assert!(task.result.is_some());
    }

    #[test]
    fn test_task_retry_logic() {
        let data = serde_json::json!({});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Upload,
            data,
        );

        assert!(!task.is_terminal());

        task.mark_failed("Error 1".to_string());
        assert_eq!(task.retry_count, 1);
        assert!(task.can_retry());

        task.mark_failed("Error 2".to_string());
        assert_eq!(task.retry_count, 2);
        assert!(task.can_retry());

        task.mark_failed("Error 3".to_string());
        assert_eq!(task.retry_count, 3);
        assert!(!task.can_retry());
        assert!(task.is_terminal());
    }

    #[test]
    fn test_task_progress() {
        let data = serde_json::json!({});
        let mut task = Task::new(test_tenant_id(), test_workspace_id(), TaskType::Scan, data);

        task.update_progress("parsing_files".to_string(), 4, 25);
        assert!(task.progress.is_some());

        let progress = task.progress.as_ref().unwrap();
        assert_eq!(progress.current_step, "parsing_files");
        assert_eq!(progress.total_steps, 4);
        assert_eq!(progress.percent_complete, 25);
    }

    #[test]
    fn test_generate_track_id() {
        let track_id = generate_track_id(TaskType::Upload);
        assert!(track_id.starts_with("upload-"));

        let track_id2 = generate_track_id(TaskType::Insert);
        assert!(track_id2.starts_with("insert-"));

        // IDs should be unique
        assert_ne!(track_id, track_id2);
    }

    #[test]
    fn test_task_error_creation() {
        let error = TaskFailureInfo::new(
            "Test error",
            "chunking",
            "Invalid format",
            "Check the file format",
            true,
        );

        assert_eq!(error.message, "Test error");
        assert_eq!(error.step, "chunking");
        assert_eq!(error.reason, "Invalid format");
        assert_eq!(error.suggestion, "Check the file format");
        assert!(error.retryable);
    }

    #[test]
    fn test_task_error_helpers() {
        let chunking_error = TaskFailureInfo::chunking("Invalid UTF-8");
        assert_eq!(chunking_error.step, "chunking");
        assert!(chunking_error.retryable);

        let embedding_error = TaskFailureInfo::embedding("API timeout");
        assert_eq!(embedding_error.step, "embedding");

        let extraction_error = TaskFailureInfo::extraction("No entities found");
        assert_eq!(extraction_error.step, "extraction");

        let indexing_error = TaskFailureInfo::indexing("Database connection failed");
        assert_eq!(indexing_error.step, "indexing");

        let rate_limit_error = TaskFailureInfo::rate_limit("extraction");
        assert!(rate_limit_error.reason.contains("rate limit"));
    }

    #[test]
    fn test_task_failed_with_details() {
        let data = serde_json::json!({});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            data,
        );

        let error = TaskFailureInfo::extraction("API rate limit exceeded");
        task.mark_failed_with_details(error);

        assert_eq!(task.status, TaskStatus::Failed);
        assert!(task.error.is_some());
        assert_eq!(task.error.as_ref().unwrap().step, "extraction");
        assert_eq!(
            task.error_message.as_ref().unwrap(),
            "Entity extraction failed"
        );
    }

    #[test]
    fn test_non_retryable_error() {
        let data = serde_json::json!({});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            data,
        );

        let error = TaskFailureInfo::new(
            "Permanent error",
            "indexing",
            "Invalid data",
            "Contact support",
            false, // Not retryable
        );
        task.mark_failed_with_details(error);

        assert!(!task.can_retry()); // Should not be retryable
    }

    // ============================================
    // Circuit Breaker Tests
    // ============================================

    #[test]
    fn test_consecutive_timeout_increments() {
        // GIVEN: A new task
        let data = serde_json::json!({"test": "data"});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            data,
        );

        // WHEN: Task fails with timeout error
        let error = TaskFailureInfo::timeout("llm_extraction", "LLM request timed out after 300s");
        task.mark_failed_with_details(error);

        // THEN: Consecutive timeout counter increments to 1
        assert_eq!(task.consecutive_timeout_failures, 1);
        assert!(!task.circuit_breaker_tripped);
        assert_eq!(task.status, TaskStatus::Failed);
    }

    #[test]
    fn test_consecutive_timeout_resets_on_success() {
        // GIVEN: A task with 2 consecutive timeouts
        let data = serde_json::json!({"test": "data"});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            data,
        );

        task.mark_failed_with_details(TaskFailureInfo::timeout("step1", "timeout 1"));
        task.mark_failed_with_details(TaskFailureInfo::timeout("step2", "timeout 2"));
        assert_eq!(task.consecutive_timeout_failures, 2);

        // WHEN: Task succeeds
        task.mark_success(serde_json::json!({"result": "ok"}));

        // THEN: Counter resets to 0
        assert_eq!(task.consecutive_timeout_failures, 0);
        assert_eq!(task.status, TaskStatus::Indexed);
        assert!(!task.circuit_breaker_tripped);
    }

    #[test]
    fn test_consecutive_timeout_resets_on_non_timeout_failure() {
        // GIVEN: A task with 2 consecutive timeouts
        let data = serde_json::json!({"test": "data"});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            data,
        );

        task.mark_failed_with_details(TaskFailureInfo::timeout("step1", "timeout 1"));
        task.mark_failed_with_details(TaskFailureInfo::timeout("step2", "timeout 2"));
        assert_eq!(task.consecutive_timeout_failures, 2);

        // WHEN: Task fails with non-timeout error (network error)
        let network_error = TaskFailureInfo::new(
            "Connection refused",
            "network",
            "Network error",
            "Check network connectivity",
            true, // Retryable
        );
        task.mark_failed_with_details(network_error);

        // THEN: Counter resets to 0 (network errors transient)
        assert_eq!(task.consecutive_timeout_failures, 0);
        assert!(!task.circuit_breaker_tripped);
    }

    #[test]
    fn test_circuit_breaker_trips_at_threshold() {
        // GIVEN: A task with circuit breaker threshold = 3
        let data = serde_json::json!({"test": "data"});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            data,
        );
        task.max_retries = 10; // High retry limit to isolate circuit breaker

        // WHEN: Task fails with 3 consecutive timeouts
        task.mark_failed_with_details(TaskFailureInfo::timeout("llm", "timeout 1"));
        assert_eq!(task.consecutive_timeout_failures, 1);
        assert!(!task.circuit_breaker_tripped);

        task.mark_failed_with_details(TaskFailureInfo::timeout("llm", "timeout 2"));
        assert_eq!(task.consecutive_timeout_failures, 2);
        assert!(!task.circuit_breaker_tripped);

        task.mark_failed_with_details(TaskFailureInfo::timeout("llm", "timeout 3"));

        // THEN: Circuit breaker trips at 3rd consecutive timeout
        assert_eq!(task.consecutive_timeout_failures, 3);
        assert!(task.circuit_breaker_tripped);
        assert_eq!(task.status, TaskStatus::Failed);
    }

    #[test]
    fn test_can_retry_respects_circuit_breaker() {
        // GIVEN: A task with circuit breaker tripped
        let data = serde_json::json!({"test": "data"});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            data,
        );
        task.max_retries = 10; // High retry limit

        // Trigger circuit breaker
        task.mark_failed_with_details(TaskFailureInfo::timeout("llm", "timeout 1"));
        task.mark_failed_with_details(TaskFailureInfo::timeout("llm", "timeout 2"));
        task.mark_failed_with_details(TaskFailureInfo::timeout("llm", "timeout 3"));

        assert!(task.circuit_breaker_tripped);
        assert_eq!(task.retry_count, 3);

        // WHEN: Checking if task can retry
        let can_retry = task.can_retry();

        // THEN: Task cannot retry (circuit breaker prevents retry)
        assert!(!can_retry);
    }

    #[test]
    fn test_is_timeout_detection() {
        // Test various timeout error messages
        let timeout_cases = vec![
            "LLM request timed out after 300s",
            "Operation timed out",
            "Request timeout",
            "TIMEOUT: exceeded 120s limit",
            "Embedding timeout after 30s",
        ];

        for msg in timeout_cases {
            let failure = TaskFailureInfo::timeout("test", msg);
            assert!(failure.is_timeout(), "Should detect '{}' as timeout", msg);
        }

        // Test non-timeout error messages
        let non_timeout_cases = vec![
            "Connection refused",
            "Invalid API key",
            "Rate limit exceeded",
            "Server error 500",
            "Document parsing failed",
        ];

        for msg in non_timeout_cases {
            let failure = TaskFailureInfo::new(msg, "test", "error", "retry", true);
            assert!(
                !failure.is_timeout(),
                "Should NOT detect '{}' as timeout",
                msg
            );
        }
    }

    #[test]
    fn test_circuit_breaker_error_message_enhancement() {
        // GIVEN: A task approaching circuit breaker threshold
        let data = serde_json::json!({"test": "data"});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            data,
        );

        // WHEN: Task fails with 3rd consecutive timeout
        task.mark_failed_with_details(TaskFailureInfo::timeout("llm", "timeout 1"));
        task.mark_failed_with_details(TaskFailureInfo::timeout("llm", "timeout 2"));
        task.mark_failed_with_details(TaskFailureInfo::timeout("llm", "timeout 3"));

        // THEN: Error message enhanced with circuit breaker info
        let error = task.error.expect("Task should have structured error");
        assert!(
            error.message.contains("Circuit breaker"),
            "Error message should mention circuit breaker: {}",
            error.message
        );
        assert!(
            error.message.contains("3 consecutive timeout"),
            "Error message should mention consecutive timeouts: {}",
            error.message
        );
        assert!(!error.retryable, "Error should be marked as non-retryable");
    }

    #[test]
    fn test_mixed_failures_do_not_trip_circuit_breaker() {
        // GIVEN: A task with mixed failure types
        let data = serde_json::json!({"test": "data"});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            data,
        );
        task.max_retries = 15; // High retry limit

        // WHEN: Task fails with alternating timeout and network errors
        task.mark_failed_with_details(TaskFailureInfo::timeout("llm", "timeout 1"));
        assert_eq!(task.consecutive_timeout_failures, 1);

        task.mark_failed_with_details(TaskFailureInfo::new(
            "Network error",
            "network",
            "Connection refused",
            "retry",
            true,
        ));
        assert_eq!(task.consecutive_timeout_failures, 0); // Reset on non-timeout

        task.mark_failed_with_details(TaskFailureInfo::timeout("llm", "timeout 2"));
        assert_eq!(task.consecutive_timeout_failures, 1);

        task.mark_failed_with_details(TaskFailureInfo::new(
            "Rate limit exceeded",
            "llm",
            "Too many requests",
            "wait and retry",
            true,
        ));
        assert_eq!(task.consecutive_timeout_failures, 0); // Reset on non-timeout

        // THEN: Circuit breaker never trips (no 3 consecutive timeouts)
        assert!(!task.circuit_breaker_tripped);
        assert!(task.can_retry());
    }

    #[test]
    fn test_circuit_breaker_with_max_retries_exhausted() {
        // GIVEN: A task with low max_retries but circuit breaker threshold not reached
        let data = serde_json::json!({"test": "data"});
        let mut task = Task::new(
            test_tenant_id(),
            test_workspace_id(),
            TaskType::Insert,
            data,
        );
        task.max_retries = 2; // Lower than circuit breaker threshold (3)

        // WHEN: Task fails 2 times with non-timeout errors
        task.mark_failed_with_details(TaskFailureInfo::new(
            "Error 1", "step1", "fail", "retry", true,
        ));
        task.mark_failed_with_details(TaskFailureInfo::new(
            "Error 2", "step2", "fail", "retry", true,
        ));

        // THEN: Task cannot retry (max_retries exhausted, not circuit breaker)
        assert_eq!(task.retry_count, 2);
        assert_eq!(task.consecutive_timeout_failures, 0);
        assert!(!task.circuit_breaker_tripped);
        assert!(!task.can_retry()); // max_retries exhausted
    }
}
