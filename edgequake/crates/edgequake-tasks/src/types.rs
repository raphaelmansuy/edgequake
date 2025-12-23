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
}

impl fmt::Display for TaskType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskType::Upload => write!(f, "upload"),
            TaskType::Insert => write!(f, "insert"),
            TaskType::Scan => write!(f, "scan"),
            TaskType::Reindex => write!(f, "reindex"),
        }
    }
}

/// Main Task structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique track ID: {type}-{uuid}
    pub track_id: String,

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
    pub fn new(task_type: TaskType, task_data: serde_json::Value) -> Self {
        let now = Utc::now();
        let track_id = generate_track_id(task_type);

        Self {
            track_id,
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
    }

    /// Mark task as failed with simple error message (backward compatible)
    pub fn mark_failed(&mut self, error: String) {
        self.status = TaskStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
        self.error_message = Some(error);
        self.retry_count += 1;
    }

    /// Mark task as failed with detailed error information (Phase 1 enhancement)
    pub fn mark_failed_with_details(&mut self, error: TaskFailureInfo) {
        self.status = TaskStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
        self.error_message = Some(error.message.clone());
        self.error = Some(error);
        self.retry_count += 1;
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
        });
        self.updated_at = Utc::now();
    }

    /// Check if task can be retried
    pub fn can_retry(&self) -> bool {
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

    #[test]
    fn test_task_creation() {
        let data = serde_json::json!({
            "file_path": "/tmp/test.pdf",
            "workspace_id": "default"
        });

        let task = Task::new(TaskType::Upload, data);

        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.task_type, TaskType::Upload);
        assert!(task.track_id.starts_with("upload-"));
        assert_eq!(task.retry_count, 0);
        assert_eq!(task.max_retries, 3);
    }

    #[test]
    fn test_task_lifecycle() {
        let data = serde_json::json!({"test": "data"});
        let mut task = Task::new(TaskType::Insert, data);

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
        let mut task = Task::new(TaskType::Upload, data);

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
        let mut task = Task::new(TaskType::Scan, data);

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
        let mut task = Task::new(TaskType::Insert, data);

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
        let mut task = Task::new(TaskType::Insert, data);

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
}
