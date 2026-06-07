//! Compile-time + runtime ISP contract: capability traits cover all GraphStorage backends.

use edgequake_storage::adapters::memory::MemoryGraphStorage;
use edgequake_storage::traits::{
    GraphStorage, GraphStorageAnalyticsCap, GraphStorageAnalyticsOps, GraphStorageMutateOps,
    GraphStorageMutator, GraphStorageReadOps, GraphStorageReader,
};

#[cfg(feature = "postgres")]
use edgequake_storage::PostgresAGEGraphStorage;

fn assert_graph_storage_isp<T>()
where
    T: GraphStorage
        + GraphStorageReadOps
        + GraphStorageMutateOps
        + GraphStorageAnalyticsOps
        + GraphStorageReader
        + GraphStorageMutator
        + GraphStorageAnalyticsCap,
{
}

#[test]
fn memory_graph_read_ops_compile_time() {
    fn assert_ops<T: GraphStorageReadOps>() {}
    assert_ops::<MemoryGraphStorage>();
}

#[test]
fn memory_graph_storage_satisfies_isp_contract() {
    assert_graph_storage_isp::<MemoryGraphStorage>();
}

#[cfg(feature = "postgres")]
#[test]
fn postgres_graph_storage_satisfies_isp_contract() {
    assert_graph_storage_isp::<PostgresAGEGraphStorage>();
}

#[tokio::test]
async fn read_cap_can_query_nodes_without_mutation_api() {
    let storage = MemoryGraphStorage::new("isp-test");
    storage.initialize().await.unwrap();
    storage.upsert_node("A", Default::default()).await.unwrap();

    let reader: &dyn GraphStorageReader = &storage;
    assert!(reader.has_node("A").await.unwrap());
    assert!(reader.get_node("A").await.unwrap().is_some());
}

#[tokio::test]
async fn analytics_cap_can_count_without_mutation_api() {
    let storage = MemoryGraphStorage::new("isp-analytics");
    storage.initialize().await.unwrap();
    storage.upsert_node("N1", Default::default()).await.unwrap();

    let analytics: &dyn GraphStorageAnalyticsCap = &storage;
    assert_eq!(analytics.node_count().await.unwrap(), 1);
    assert_eq!(analytics.edge_count().await.unwrap(), 0);
}

#[tokio::test]
async fn mutator_cap_can_upsert_via_narrow_bound() {
    let storage = MemoryGraphStorage::new("isp-mutator");
    storage.initialize().await.unwrap();

    let mutator: &dyn GraphStorageMutator = &storage;
    mutator.upsert_node("M1", Default::default()).await.unwrap();
    assert!(storage.has_node("M1").await.unwrap());
}

#[tokio::test]
async fn graph_read_view_delegates_without_mutation_surface() {
    use edgequake_storage::GraphReadView;

    let storage = MemoryGraphStorage::new("isp-read-view");
    storage.initialize().await.unwrap();
    storage.upsert_node("X", Default::default()).await.unwrap();

    let view = GraphReadView::new(&storage);
    assert!(view.has_node("X").await.unwrap());
    assert_eq!(view.node_count().await.unwrap(), 1);
}
