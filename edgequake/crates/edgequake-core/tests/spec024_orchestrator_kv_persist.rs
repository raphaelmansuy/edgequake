//! SPEC-024 pass 13 — library `EdgeQuake::insert` writes chunk KV via IngestionPersister.

#![cfg(feature = "pipeline")]

use std::sync::Arc;

use edgequake_core::{EdgeQuake, EdgeQuakeConfig, StorageBackend, StorageConfig};
use edgequake_llm::MockProvider;
use edgequake_storage::traits::KVStorage;
use edgequake_storage::{
    GraphStorage, MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage, VectorStorage,
};

const EXTRACTION_JSON: &str = edgequake_pipeline::SPEC021_SARAH_CHEN_EXTRACTION_JSON;

#[tokio::test]
async fn spec024_orchestrator_insert_persists_chunk_kv() {
    let mock = Arc::new(MockProvider::new());
    mock.add_response(EXTRACTION_JSON).await;

    let kv = Arc::new(MemoryKVStorage::new("orch-kv"));
    let vector = Arc::new(MemoryVectorStorage::new("orch-kv", 1536));
    let graph = Arc::new(MemoryGraphStorage::new("orch-kv"));
    vector.initialize().await.unwrap();
    graph.initialize().await.unwrap();

    let mut eq = EdgeQuake::new(
        EdgeQuakeConfig::new()
            .with_namespace("orch-kv")
            .with_gleaning(false, 0)
            .with_storage(StorageConfig {
                backend: StorageBackend::Memory,
                ..Default::default()
            }),
    )
    .with_storage_backends(
        Arc::clone(&kv) as Arc<dyn KVStorage>,
        Arc::clone(&vector) as Arc<dyn edgequake_storage::traits::VectorStorage>,
        Arc::clone(&graph) as Arc<dyn edgequake_storage::traits::GraphStorage>,
    )
    .with_providers(
        Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::LLMProvider>,
        Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
    );
    eq.initialize().await.expect("init");

    let out = eq
        .insert("Sarah Chen leads EdgeQuake in Zurich.", Some("doc-orch-kv"))
        .await
        .expect("insert");

    assert!(out.success);
    assert!(out.chunks_created > 0);

    let chunk_id = format!("{}-chunk-0", out.document_id);
    let chunk_kv = kv.get_by_id(&chunk_id).await.expect("kv read");
    assert!(
        chunk_kv.is_some(),
        "orchestrator insert must store chunk text in KV via persister"
    );
}
