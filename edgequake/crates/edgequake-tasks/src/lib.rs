//! # EdgeQuake Tasks
//!
//! Background task processing system for EdgeQuake.
//!
//! ## Features
//!
//! - Asynchronous task processing with tokio
//! - Multiple storage backends (memory, PostgreSQL)
//! - Task queuing with channels or Redis
//! - Worker pool with configurable concurrency
//! - Automatic retry with exponential backoff
//! - Task status tracking and monitoring
//!
//! ## Usage
//!
//! ```rust,no_run
//! use edgequake_tasks::*;
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create storage and queue
//! let storage = Arc::new(memory::MemoryTaskStorage::new());
//! let queue = Arc::new(queue::ChannelTaskQueue::new(100));
//!
//! // Create a task processor (implement your own)
//! // let processor = Arc::new(YourTaskProcessor::new());
//!
//! // Create and start worker pool
//! // let mut pool = worker::WorkerPool::new(
//! //     worker::WorkerPoolConfig::default(),
//! //     queue.clone(),
//! //     storage.clone(),
//! //     processor,
//! // );
//! // pool.start();
//!
//! // Create and enqueue a task
//! let task = types::Task::new(
//!     types::TaskType::Upload,
//!     serde_json::json!({"file_path": "/tmp/document.pdf"}),
//! );
//! storage.create_task(&task).await?;
//! queue.send(task).await?;
//!
//! # Ok(())
//! # }
//! ```

pub mod error;
pub mod memory;
pub mod pipeline_state;
#[cfg(feature = "postgres")]
pub mod postgres;
pub mod queue;
pub mod storage;
pub mod types;
pub mod worker;

// Re-export commonly used types
pub use error::{TaskError, TaskResult};
pub use pipeline_state::{PipelineMessage, PipelineState, PipelineStatusSnapshot};
pub use queue::{ChannelTaskQueue, SharedTaskQueue, TaskQueue, UnboundedChannelTaskQueue};
pub use storage::{
    Pagination, SharedTaskStorage, SortField, SortOrder, TaskFilter, TaskList, TaskStatistics,
    TaskStorage,
};
pub use types::{
    DirectoryScanData, DocumentUploadData, ReindexData, Task, TaskFailureInfo, TaskProgress,
    TaskStatus, TaskType, TextInsertData,
};
pub use worker::{SharedTaskProcessor, TaskProcessor, WorkerPool, WorkerPoolConfig};
