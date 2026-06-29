//! Graph query admission runtime — SPEC-006 / API-SOLID-I-001.
//!
//! Bundles materialization semaphore + query timeout budget so graph handlers
//! avoid pulling full [`AppState`].

use std::sync::Arc;

use edgequake_core::{GraphMaterializationSemaphore, ResourceBudgetConfig};

/// Semaphore + timeout budget for graph materialization endpoints.
#[derive(Clone)]
pub struct GraphQueryRuntime {
    pub materialize: Arc<GraphMaterializationSemaphore>,
    pub budget: ResourceBudgetConfig,
}

impl GraphQueryRuntime {
    pub fn from_parts(
        materialize: Arc<GraphMaterializationSemaphore>,
        budget: ResourceBudgetConfig,
    ) -> Self {
        Self {
            materialize,
            budget,
        }
    }
}
