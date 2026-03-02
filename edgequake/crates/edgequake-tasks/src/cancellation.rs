//! Per-task cooperative cancellation registry.
//!
//! ## WHY
//!
//! The previous cancellation mechanism was a single global boolean in
//! `PipelineState`. This had two critical flaws:
//! 1. **Global scope**: Cancelling one task's processing would set the flag
//!    for ALL tasks.
//! 2. **Never checked**: No pipeline stage actually read the flag during
//!    processing, so cancellation had zero effect on in-flight work.
//!
//! This module provides per-task `CancellationToken`s that:
//! - Are scoped to a single task (identified by `track_id`)
//! - Are cooperatively checked at every stage boundary in the pipeline
//! - Allow the cancel API to immediately signal a running task to stop
//! - Are automatically cleaned up when a task completes
//!
//! ## Architecture
//!
//! ```text
//!  cancel_task API ──► CancellationRegistry::cancel("track-123")
//!                              │
//!                              ▼
//!                     CancellationToken::cancel()
//!                              │
//!                     ┌────────┴────────────────────────┐
//!                     ▼                                  ▼
//!           extraction loop checks              embedding batch checks
//!           token.is_cancelled()                token.is_cancelled()
//!                     │                                  │
//!                     ▼                                  ▼
//!              return Err(Cancelled)            return Err(Cancelled)
//! ```
//!
//! ## Implements
//!
//! - **FEAT-CANCEL**: Per-task cooperative cancellation
//!
//! ## Enforces
//!
//! - **BR-CANCEL-01**: Cancellation must be per-task, not global
//! - **BR-CANCEL-02**: All pipeline stages must check for cancellation
//! - **BR-CANCEL-03**: Tokens must be cleaned up after task completion

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

/// Registry that maps task track_ids to their cancellation tokens.
///
/// Shared between the worker pool (which registers tokens when tasks start)
/// and the cancel API handler (which triggers cancellation by track_id).
#[derive(Clone)]
pub struct CancellationRegistry {
    tokens: Arc<RwLock<HashMap<String, CancellationToken>>>,
}

impl Default for CancellationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CancellationRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new cancellation token for a task.
    ///
    /// Returns the token so the worker can pass it through the pipeline.
    /// The token is also stored in the registry so the cancel API can
    /// trigger it later.
    pub async fn register(&self, track_id: &str) -> CancellationToken {
        let token = CancellationToken::new();
        let mut tokens = self.tokens.write().await;
        tokens.insert(track_id.to_string(), token.clone());
        token
    }

    /// Cancel a task by its track_id.
    ///
    /// Returns `true` if the task was found and cancelled, `false` if not found.
    /// If the task already completed and was deregistered, returns `false`.
    pub async fn cancel(&self, track_id: &str) -> bool {
        let tokens = self.tokens.read().await;
        if let Some(token) = tokens.get(track_id) {
            token.cancel();
            true
        } else {
            false
        }
    }

    /// Remove a task's token from the registry (cleanup after completion).
    ///
    /// WHY: Without cleanup, completed tasks accumulate tokens in memory.
    /// For a system processing thousands of documents, this would leak.
    pub async fn deregister(&self, track_id: &str) {
        let mut tokens = self.tokens.write().await;
        tokens.remove(track_id);
    }

    /// Check if a specific task has been cancelled.
    pub async fn is_cancelled(&self, track_id: &str) -> bool {
        let tokens = self.tokens.read().await;
        tokens
            .get(track_id)
            .map(|t| t.is_cancelled())
            .unwrap_or(false)
    }

    /// Get the number of active tokens (for monitoring).
    pub async fn active_count(&self) -> usize {
        self.tokens.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_and_cancel() {
        let registry = CancellationRegistry::new();

        let token = registry.register("task-1").await;
        assert!(!token.is_cancelled());

        let cancelled = registry.cancel("task-1").await;
        assert!(cancelled);
        assert!(token.is_cancelled());
    }

    #[tokio::test]
    async fn test_cancel_nonexistent_returns_false() {
        let registry = CancellationRegistry::new();

        let cancelled = registry.cancel("does-not-exist").await;
        assert!(!cancelled);
    }

    #[tokio::test]
    async fn test_deregister_removes_token() {
        let registry = CancellationRegistry::new();

        let _token = registry.register("task-1").await;
        assert_eq!(registry.active_count().await, 1);

        registry.deregister("task-1").await;
        assert_eq!(registry.active_count().await, 0);

        // Cancel after deregister returns false
        let cancelled = registry.cancel("task-1").await;
        assert!(!cancelled);
    }

    #[tokio::test]
    async fn test_is_cancelled() {
        let registry = CancellationRegistry::new();

        let _token = registry.register("task-1").await;
        assert!(!registry.is_cancelled("task-1").await);

        registry.cancel("task-1").await;
        assert!(registry.is_cancelled("task-1").await);
    }

    #[tokio::test]
    async fn test_multiple_tasks_independent() {
        let registry = CancellationRegistry::new();

        let token1 = registry.register("task-1").await;
        let token2 = registry.register("task-2").await;

        // Cancel only task-1
        registry.cancel("task-1").await;

        assert!(token1.is_cancelled());
        assert!(!token2.is_cancelled());
    }

    #[tokio::test]
    async fn test_default_impl() {
        let registry = CancellationRegistry::default();
        assert_eq!(registry.active_count().await, 0);
    }
}
