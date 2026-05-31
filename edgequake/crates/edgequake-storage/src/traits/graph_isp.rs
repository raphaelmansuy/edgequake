//! ISP capability boundaries for [`GraphStorage`](super::GraphStorage).
//!
//! SPEC-017: The full [`GraphStorage`] trait remains the adapter surface, but
//! callers can bound on narrower capability traits to document intent and
//! prevent accidental writes in read-only code paths.
//!
//! All capability traits are automatically implemented for any `GraphStorage`
//! backend via blanket impls — zero adapter churn.

pub use super::graph::GraphStorage;

/// Read-only graph access: nodes, edges, traversal, and search.
pub trait GraphStorageReader: GraphStorage {}

/// Graph mutation: upsert/delete nodes and edges, workspace clears.
pub trait GraphStorageMutator: GraphStorage {}

/// Analytics: counts, fast estimates, workspace-scoped statistics.
pub trait GraphStorageAnalyticsCap: GraphStorage {}

impl<T: GraphStorage> GraphStorageReader for T {}
impl<T: GraphStorage> GraphStorageMutator for T {}
impl<T: GraphStorage> GraphStorageAnalyticsCap for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::memory::MemoryGraphStorage;

    fn assert_reader<T: GraphStorageReader>() {}
    fn assert_mutator<T: GraphStorageMutator>() {}
    fn assert_analytics<T: GraphStorageAnalyticsCap>() {}

    #[test]
    fn memory_backend_satisfies_isp_capabilities() {
        assert_reader::<MemoryGraphStorage>();
        assert_mutator::<MemoryGraphStorage>();
        assert_analytics::<MemoryGraphStorage>();
    }
}
