//! ISP capability boundaries (SPEC-017 Phase 2b).
//!
//! Marker traits bind to method-level operation traits. Composite [`GraphStorage`](super::graph::GraphStorage)
//! remains the adapter surface; [`GraphReadView`](super::graph_read_view::GraphReadView) narrows read paths.

pub use super::graph_analytics_ops::GraphStorageAnalyticsOps;
pub use super::graph_mutate_ops::GraphStorageMutateOps;
pub use super::graph_read_ops::GraphStorageReadOps;
pub use super::graph_scan_ops::GraphScanOps;

/// Read-only graph access.
pub trait GraphStorageReader: GraphStorageReadOps + GraphScanOps {}

/// Graph mutation.
pub trait GraphStorageMutator: GraphStorageMutateOps {}

/// Analytics and counts.
pub trait GraphStorageAnalyticsCap: GraphStorageAnalyticsOps {}

impl<T: GraphStorageReadOps + GraphScanOps> GraphStorageReader for T {}
impl<T: GraphStorageMutateOps> GraphStorageMutator for T {}
impl<T: GraphStorageAnalyticsOps> GraphStorageAnalyticsCap for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::memory::MemoryGraphStorage;

    #[test]
    fn memory_backend_satisfies_isp_capabilities() {
        fn assert_reader<T: GraphStorageReader>() {}
        fn assert_mutator<T: GraphStorageMutator>() {}
        fn assert_analytics<T: GraphStorageAnalyticsCap>() {}
        assert_reader::<MemoryGraphStorage>();
        assert_mutator::<MemoryGraphStorage>();
        assert_analytics::<MemoryGraphStorage>();
    }
}
