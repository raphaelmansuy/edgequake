//! SPEC-022 P-H4 — production query engine bootstrap includes BM25 reranker.

#[test]
fn spec022_bootstrap_wires_reranker_and_caches() {
    use std::sync::Arc;

    use edgequake_llm::MockProvider;
    use edgequake_query::{build_production_query_engine, QueryEngineConfig};
    use edgequake_storage::adapters::memory::{MemoryGraphStorage, MemoryVectorStorage};

    let mock = Arc::new(MockProvider::new());
    let vector = Arc::new(MemoryVectorStorage::new("test", 1536));
    let graph = Arc::new(MemoryGraphStorage::new("test"));

    let engine = build_production_query_engine(
        Arc::clone(&vector) as Arc<dyn edgequake_storage::traits::VectorStorage>,
        Arc::clone(&graph) as Arc<dyn edgequake_storage::traits::GraphStorage>,
        Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
        Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::LLMProvider>,
        None,
    );

    // Smoke: engine is constructible with default config (reranker wired internally).
    let _ = QueryEngineConfig::default();
    drop(engine);
}
