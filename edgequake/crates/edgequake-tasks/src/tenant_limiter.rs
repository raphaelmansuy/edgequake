//! Per-tenant concurrency limiter for fair task scheduling.
//!
//! ## WHY Per-Tenant Fair Scheduling?
//!
//! Without tenant isolation, one tenant uploading 50 PDFs monopolizes all
//! worker threads, forcing other tenants to wait until the entire batch
//! finishes. This violates multi-tenant fairness guarantees.
//!
//! ## Strategy: Semaphore-Based Fair Share
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────┐
//! │                    WORKER POOL (N workers)                │
//! │                                                           │
//! │  Worker 0 picks task ──► Tenant A at capacity?            │
//! │                           ├── NO  → acquire permit → run  │
//! │                           └── YES → requeue + pick next   │
//! │                                                           │
//! │  Tenant A: ██████░░ (3/4 permits)                        │
//! │  Tenant B: ██░░░░░░ (1/4 permits)                        │
//! │  Tenant C: ░░░░░░░░ (0/4 permits — next task runs)       │
//! └───────────────────────────────────────────────────────────┘
//! ```
//!
//! Each tenant gets `max_tasks_per_tenant` permits. Workers use `try_acquire()`
//! to check capacity without blocking. If a tenant is full, the task is
//! requeued with a short delay and the worker picks the next task.
//!
//! ## Implements
//!
//! - **FEAT-TENANT-FAIRNESS**: At least 1 worker slot per tenant
//! - **BR-TENANT-ISOLATION**: One tenant cannot block other tenants

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, RwLock, Semaphore};
use tracing::debug;
use uuid::Uuid;

/// Per-tenant concurrency limiter using semaphores.
///
/// Ensures no single tenant can consume more than `max_per_tenant` worker
/// slots simultaneously. Workers that can't acquire a permit for a task's
/// tenant will requeue the task and move on to the next one.
#[derive(Clone)]
pub struct TenantConcurrencyLimiter {
    /// Max concurrent tasks per tenant.
    max_per_tenant: usize,
    /// Per-tenant semaphores. Created lazily on first task for each tenant.
    semaphores: Arc<RwLock<HashMap<Uuid, Arc<Semaphore>>>>,
}

impl TenantConcurrencyLimiter {
    /// Create a new limiter.
    ///
    /// # Arguments
    /// * `max_per_tenant` - Maximum concurrent tasks per tenant.
    ///   Recommended: `max(1, num_workers / 2)` to guarantee at least
    ///   one slot remains available for other tenants.
    pub fn new(max_per_tenant: usize) -> Self {
        let max_per_tenant = max_per_tenant.max(1); // Always allow at least 1
        Self {
            max_per_tenant,
            semaphores: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Try to acquire a processing slot for the given tenant.
    ///
    /// Returns `Some(permit)` if the tenant has available capacity,
    /// `None` if the tenant is at its concurrency limit.
    ///
    /// The permit MUST be held for the duration of task processing.
    /// When dropped, it automatically releases the slot.
    pub async fn try_acquire(&self, tenant_id: Uuid) -> Option<OwnedSemaphorePermit> {
        let semaphore = {
            // Fast path: read lock to check if semaphore exists
            let read_guard = self.semaphores.read().await;
            if let Some(sem) = read_guard.get(&tenant_id) {
                Arc::clone(sem)
            } else {
                drop(read_guard);
                // Slow path: create semaphore for new tenant
                let mut write_guard = self.semaphores.write().await;
                // Double-check after acquiring write lock
                let sem = write_guard.entry(tenant_id).or_insert_with(|| {
                    debug!(
                        tenant_id = %tenant_id,
                        max_concurrent = self.max_per_tenant,
                        "Created tenant concurrency semaphore"
                    );
                    Arc::new(Semaphore::new(self.max_per_tenant))
                });
                Arc::clone(sem)
            }
        };

        match semaphore.clone().try_acquire_owned() {
            Ok(permit) => Some(permit),
            Err(_) => {
                debug!(
                    tenant_id = %tenant_id,
                    max_concurrent = self.max_per_tenant,
                    "Tenant at concurrency limit, task will be requeued"
                );
                None
            }
        }
    }

    /// Get current active task count for a tenant (for metrics/logging).
    pub async fn active_count(&self, tenant_id: &Uuid) -> usize {
        let read_guard = self.semaphores.read().await;
        if let Some(sem) = read_guard.get(tenant_id) {
            self.max_per_tenant - sem.available_permits()
        } else {
            0
        }
    }

    /// Clean up semaphores for tenants with no active tasks.
    /// Call periodically to prevent unbounded memory growth.
    pub async fn cleanup_idle(&self) {
        let mut write_guard = self.semaphores.write().await;
        let before = write_guard.len();
        write_guard.retain(|_tenant_id, sem| {
            // Keep only if there are active tasks (permits in use)
            sem.available_permits() < self.max_per_tenant
        });
        let removed = before - write_guard.len();
        if removed > 0 {
            debug!(
                removed = removed,
                remaining = write_guard.len(),
                "Cleaned up idle tenant semaphores"
            );
        }
    }

    /// Get configured max per tenant.
    pub fn max_per_tenant(&self) -> usize {
        self.max_per_tenant
    }
}

impl std::fmt::Debug for TenantConcurrencyLimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TenantConcurrencyLimiter")
            .field("max_per_tenant", &self.max_per_tenant)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tenant_a() -> Uuid {
        Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
    }

    fn tenant_b() -> Uuid {
        Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap()
    }

    #[tokio::test]
    async fn test_basic_acquire_release() {
        let limiter = TenantConcurrencyLimiter::new(2);

        // First acquire should succeed
        let permit1 = limiter.try_acquire(tenant_a()).await;
        assert!(permit1.is_some());
        assert_eq!(limiter.active_count(&tenant_a()).await, 1);

        // Second acquire should succeed
        let permit2 = limiter.try_acquire(tenant_a()).await;
        assert!(permit2.is_some());
        assert_eq!(limiter.active_count(&tenant_a()).await, 2);

        // Third acquire should fail (max=2)
        let permit3 = limiter.try_acquire(tenant_a()).await;
        assert!(permit3.is_none());

        // Drop one permit, then acquire should succeed again
        drop(permit1);
        let permit4 = limiter.try_acquire(tenant_a()).await;
        assert!(permit4.is_some());
    }

    #[tokio::test]
    async fn test_tenant_isolation() {
        let limiter = TenantConcurrencyLimiter::new(1);

        // Tenant A at capacity
        let _permit_a = limiter.try_acquire(tenant_a()).await;
        assert!(_permit_a.is_some());

        // Tenant B should still get a slot
        let permit_b = limiter.try_acquire(tenant_b()).await;
        assert!(permit_b.is_some());

        // Tenant A at capacity
        let permit_a2 = limiter.try_acquire(tenant_a()).await;
        assert!(permit_a2.is_none());
    }

    #[tokio::test]
    async fn test_min_one_permit() {
        // Even with 0 config, should allow at least 1
        let limiter = TenantConcurrencyLimiter::new(0);
        let permit = limiter.try_acquire(tenant_a()).await;
        assert!(permit.is_some());
    }

    #[tokio::test]
    async fn test_cleanup_idle() {
        let limiter = TenantConcurrencyLimiter::new(2);

        // Acquire and release for tenant A
        let permit = limiter.try_acquire(tenant_a()).await.unwrap();
        drop(permit);

        // Tenant A semaphore should be cleaned up (all permits available)
        limiter.cleanup_idle().await;

        // Should still work after cleanup (re-creates semaphore)
        let permit = limiter.try_acquire(tenant_a()).await;
        assert!(permit.is_some());
    }

    #[tokio::test]
    async fn test_active_while_in_flight() {
        let limiter = TenantConcurrencyLimiter::new(3);

        let _p1 = limiter.try_acquire(tenant_a()).await.unwrap();
        let _p2 = limiter.try_acquire(tenant_a()).await.unwrap();

        assert_eq!(limiter.active_count(&tenant_a()).await, 2);

        // Cleanup should NOT remove tenant A (has active permits)
        limiter.cleanup_idle().await;
        assert_eq!(limiter.active_count(&tenant_a()).await, 2);
    }
}
