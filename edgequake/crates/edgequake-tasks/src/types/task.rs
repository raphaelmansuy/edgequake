//! Core Task struct and lifecycle management.
//!
//! Contains the Task struct (the main unit of work), its lifecycle
//! methods (mark_processing, mark_success, mark_failed, etc.),
//! circuit breaker logic, and the track ID generator.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::failure::TaskFailureInfo;
use super::progress::{ChunkProgress, TaskProgress};
use super::status::{TaskStatus, TaskType};

/// A background task representing a unit of work.
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

impl Task {
    /// PDF id from `task_data.pdf_id` when present (P-G14 admission).
    pub fn pdf_id(&self) -> Option<Uuid> {
        self.task_data
            .get("pdf_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
    }

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
