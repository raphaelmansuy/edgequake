//! Global cap on concurrent graph materializations — SPEC-006 P1 (SRP).

use std::sync::Arc;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use super::budget::ResourceBudgetConfig;

/// Limits concurrent in-process graph materialization (popular-nodes, stream, etc.).
#[derive(Debug)]
pub struct GraphMaterializationSemaphore {
    semaphore: Arc<Semaphore>,
    max_concurrent: usize,
}

impl GraphMaterializationSemaphore {
    pub fn new(max_concurrent: usize) -> Self {
        let max_concurrent = max_concurrent.clamp(1, 16);
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent,
        }
    }

    pub fn from_budget(budget: &ResourceBudgetConfig) -> Self {
        Self::new(budget.graph_materialize_concurrent)
    }

    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }

    /// Acquire a permit, waiting if all slots are in use.
    pub async fn acquire_owned(&self) -> Option<OwnedSemaphorePermit> {
        self.semaphore.clone().acquire_owned().await.ok()
    }

    /// Try to acquire without waiting — returns `None` when at capacity.
    pub fn try_acquire_owned(&self) -> Option<OwnedSemaphorePermit> {
        self.semaphore.clone().try_acquire_owned().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn serializes_when_max_concurrent_is_one() {
        let sem = GraphMaterializationSemaphore::new(1);
        let p1 = sem.acquire_owned().await.expect("first permit");
        assert!(sem.try_acquire_owned().is_none());
        drop(p1);
        assert!(sem.try_acquire_owned().is_some());
    }

    #[tokio::test]
    async fn allows_parallel_when_max_concurrent_is_two() {
        let sem = GraphMaterializationSemaphore::new(2);
        let _p1 = sem.acquire_owned().await;
        let _p2 = sem.acquire_owned().await;
        assert!(sem.try_acquire_owned().is_none());
    }
}
