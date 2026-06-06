//! Resource safety — SPEC-006 ensure-perf.
//!
//! - [`ResourceBudgetConfig`]: DRY single source of truth for caps
//! - [`ResourceGuard`]: admission control before expensive operations

mod budget;
mod guard;
mod semaphore;

pub use budget::{
    ResourceBudgetConfig, DEFAULT_GRAPH_MATERIALIZE_CONCURRENT, DEFAULT_GRAPH_QUERY_TIMEOUT_SECS,
    DEFAULT_GRAPH_SCAN_THRESHOLD_NODES, DEFAULT_MEM_HEADROOM_RATIO, MAX_GRAPH_DEPTH,
    MAX_GRAPH_NODES, MAX_ORCHESTRATOR_CONTEXT_TOKENS, MAX_PAGE_SIZE, MAX_QUERY_CHARS,
    MAX_UPLOAD_BYTES, MIN_PAGE_SIZE,
};
pub use guard::{AdmissionDecision, GraphOperation, ResourceGuard};
pub use semaphore::GraphMaterializationSemaphore;
