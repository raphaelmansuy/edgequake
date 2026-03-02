//! Error types for task processing.

use thiserror::Error;

/// Task processing errors
#[derive(Debug, Error)]
pub enum TaskError {
    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Task cannot be cancelled in status: {0}")]
    CannotCancel(String),

    #[error("Task cannot be retried: {0}")]
    CannotRetry(String),

    #[error("Queue is full")]
    QueueFull,

    #[error("Queue is closed")]
    QueueClosed,

    #[error("Worker error: {0}")]
    WorkerError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    #[cfg(feature = "postgres")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    #[cfg(feature = "redis-queue")]
    RedisError(#[from] redis::RedisError),

    #[error("Task execution error: {0}")]
    ExecutionError(String),

    #[error("Invalid task data: {0}")]
    InvalidTaskData(String),

    /// Pipeline processing error.
    #[error("Processing error: {0}")]
    Process(String),

    /// Storage operation error.
    #[error("Storage error: {0}")]
    Storage(String),

    /// Invalid task payload.
    #[error("Invalid payload: {0}")]
    InvalidPayload(String),

    /// Unsupported operation.
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    /// Resource not found.
    #[error("Not found: {0}")]
    NotFound(String),

    /// Processing error (extraction, transformation).
    #[error("Processing error: {0}")]
    Processing(String),

    /// Operation timed out.
    /// WHY: Vision extraction and LLM calls can hang indefinitely when the
    /// provider is unresponsive (e.g., Ollama not running in Docker).
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Task was cancelled via the cancel API.
    /// WHY: When a user or system requests cancellation, every pipeline
    /// stage should check the CancellationToken and return this error
    /// to unwind cleanly, releasing worker slots for other tasks.
    #[error("Task cancelled: {0}")]
    Cancelled(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Result type for task operations
pub type TaskResult<T> = Result<T, TaskError>;
