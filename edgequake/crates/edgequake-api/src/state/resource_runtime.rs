//! SPEC-006 resource runtime — DRY single injection point for AppState.

use std::sync::Arc;

use edgequake_core::{GraphMaterializationSemaphore, PdfVisionSemaphore, ResourceGuard};

/// Build shared resource guard + admission semaphores from environment.
pub fn build_resource_runtime() -> (
    ResourceGuard,
    Arc<GraphMaterializationSemaphore>,
    Arc<PdfVisionSemaphore>,
) {
    let guard = ResourceGuard::from_env();
    let graph_materialize = Arc::new(GraphMaterializationSemaphore::from_budget(guard.budget()));
    let pdf_vision = Arc::new(PdfVisionSemaphore::from_budget(guard.budget()));
    (guard, graph_materialize, pdf_vision)
}
