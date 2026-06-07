//! SPEC-006 resource runtime — DRY single injection point for AppState.

use std::sync::Arc;

use edgequake_core::{GraphMaterializationSemaphore, ResourceGuard};

/// Build shared resource guard + graph materialization semaphore from environment.
pub fn build_resource_runtime() -> (ResourceGuard, Arc<GraphMaterializationSemaphore>) {
    let guard = ResourceGuard::from_env();
    let semaphore = Arc::new(GraphMaterializationSemaphore::from_budget(guard.budget()));
    (guard, semaphore)
}
