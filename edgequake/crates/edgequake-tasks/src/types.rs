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

    /// Error message if failed
    pub error_message: Option<String>,

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
    }

    /// Mark task as failed
    pub fn mark_failed(&mut self, error: String) {
        self.status = TaskStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
        self.error_message = Some(error);
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
        self.status == TaskStatus::Failed && self.retry_count < self.max_retries
    }

    /// Check if task is terminal (completed or permanently failed)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            TaskStatus::Indexed | TaskStatus::Cancelled
        ) || (self.status == TaskStatus::Failed && !self.can_retry())
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
}
