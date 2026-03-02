//! # EdgeQuake Tasks
//!
//! Background task processing system for EdgeQuake.
//!
//! ## Implements
//!
//! - [`FEAT0901`]: Async background task processing
//! - [`FEAT0902`]: Multi-backend storage (memory, PostgreSQL)
//! - [`FEAT0903`]: Worker pool with configurable concurrency
//! - [`FEAT0904`]: Automatic retry with exponential backoff
//! - [`FEAT0905`]: Real-time task status tracking
//!
//! ## Enforces
//!
//! - [`BR0901`]: Failed tasks retry with backoff
//! - [`BR0902`]: Task status visible via API
//! - [`BR0903`]: Completed tasks retain for audit
//!
//! ## Use Cases
//!
//! - [`UC0901`]: System processes document upload async
//! - [`UC0902`]: User monitors pipeline progress
//! - [`UC0903`]: Admin views task queue status
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
//! use uuid::Uuid;
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
//! // Create and enqueue a task with tenant/workspace context
//! let tenant_id = Uuid::new_v4();
//! let workspace_id = Uuid::new_v4();
//! let task = types::Task::new(
//!     tenant_id,
//!     workspace_id,
//!     types::TaskType::Upload,
//!     serde_json::json!({"file_path": "/tmp/document.pdf"}),
//! );
//! storage.create_task(&task).await?;
//! queue.send(task).await?;
//!
//! # Ok(())
//! # }
//! ```

pub mod cancellation;
pub mod error;
pub mod memory;
pub mod pipeline_state;
#[cfg(feature = "postgres")]
pub mod postgres;
pub mod progress;
pub mod queue;
pub mod storage;
pub mod tenant_limiter;
pub mod types;
pub mod worker;

// Re-export commonly used types
pub use cancellation::CancellationRegistry;
pub use error::{TaskError, TaskResult};
pub use pipeline_state::{PipelineEvent, PipelineMessage, PipelineState, PipelineStatusSnapshot};
pub use progress::{PdfUploadProgress, PhaseError, PhaseProgress, PhaseStatus, PipelinePhase};
pub use queue::{ChannelTaskQueue, SharedTaskQueue, TaskQueue, UnboundedChannelTaskQueue};
pub use storage::{
    Pagination, SharedTaskStorage, SortField, SortOrder, TaskFilter, TaskList, TaskStatistics,
    TaskStorage,
};
pub use tenant_limiter::TenantConcurrencyLimiter;
pub use types::{
    ChunkProgress, DirectoryScanData, DocumentUploadData, PdfProcessingData, ReindexData, Task,
    TaskFailureInfo, TaskProgress, TaskStatus, TaskType, TextInsertData,
};
pub use worker::{SharedTaskProcessor, TaskProcessor, WorkerPool, WorkerPoolConfig};
