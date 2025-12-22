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

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Result type for task operations
pub type TaskResult<T> = Result<T, TaskError>;
