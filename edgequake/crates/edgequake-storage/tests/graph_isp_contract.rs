//! Compile-time ISP contract: capability traits cover all GraphStorage backends.

use edgequake_storage::adapters::memory::MemoryGraphStorage;
use edgequake_storage::traits::{
    GraphStorage, GraphStorageAnalyticsCap, GraphStorageMutator, GraphStorageReader,
};

fn assert_graph_storage_isp<T>()
where
    T: GraphStorage + GraphStorageReader + GraphStorageMutator + GraphStorageAnalyticsCap,
{
}

#[test]
fn memory_graph_storage_satisfies_isp_contract() {
    assert_graph_storage_isp::<MemoryGraphStorage>();
}

#[tokio::test]
async fn read_cap_can_query_nodes_without_mutation_api() {
    let storage = MemoryGraphStorage::new("isp-test");
    storage.initialize().await.unwrap();
    storage.upsert_node("A", Default::default()).await.unwrap();

    let reader: &dyn GraphStorageReader = &storage;
    assert!(reader.has_node("A").await.unwrap());
}
