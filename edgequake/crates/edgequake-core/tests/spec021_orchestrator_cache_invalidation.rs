//! SPEC-021 P-G9 — orchestrator `insert()` invalidates query cache when engine wired.

#![cfg(feature = "pipeline")]

use std::sync::Arc;

use edgequake_core::{EdgeQuake, EdgeQuakeConfig, StorageBackend, StorageConfig};
use edgequake_llm::MockProvider;
use edgequake_query::engine::QueryRequest;
use edgequake_query::{QueryEngine, QueryEngineConfig, QueryMode};
use edgequake_storage::{
    GraphStorage, MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage, VectorStorage,
};

const EXTRACTION_JSON: &str = edgequake_pipeline::SPEC021_SARAH_CHEN_EXTRACTION_JSON;

fn memory_config(namespace: &str) -> EdgeQuakeConfig {
    EdgeQuakeConfig::new()
        .with_namespace(namespace)
        .with_gleaning(false, 0)
        .with_storage(StorageConfig {
            backend: StorageBackend::Memory,
            ..Default::default()
        })
}

#[tokio::test]
async fn spec021_orchestrator_insert_invalidates_query_result_cache() {
    let mock = Arc::new(MockProvider::new());
    for _ in 0..8 {
        mock.add_response(EXTRACTION_JSON).await;
    }

    let kv = Arc::new(MemoryKVStorage::new("cache-inv"));
    let vector = Arc::new(MemoryVectorStorage::new("cache-inv", 1536));
    let graph = Arc::new(MemoryGraphStorage::new("cache-inv"));
    vector.initialize().await.unwrap();
    graph.initialize().await.unwrap();

    let engine = Arc::new(
        QueryEngine::with_mock_keywords(
            QueryEngineConfig::default(),
            Arc::clone(&vector) as Arc<dyn VectorStorage>,
            Arc::clone(&graph) as Arc<dyn GraphStorage>,
            Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
            Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::LLMProvider>,
        )
        .with_result_cache(),
    );
    let cache = engine.result_cache().expect("result cache");

    let mut eq = EdgeQuake::new(memory_config("cache-inv"))
        .with_storage_backends(
            Arc::clone(&kv) as Arc<dyn edgequake_storage::traits::KVStorage>,
            Arc::clone(&vector) as Arc<dyn VectorStorage>,
            Arc::clone(&graph) as Arc<dyn GraphStorage>,
        )
        .with_providers(
            Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::LLMProvider>,
            Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
        )
        .with_query_engine(Arc::clone(&engine));
    eq.initialize().await.expect("init");

    let mut req = QueryRequest::new("orchestrator cache bust sarah");
    req.context_only = true;
    req.mode = Some(QueryMode::Hybrid);

    engine.query(req.clone()).await.expect("prime");
    engine.query(req.clone()).await.expect("cached");
    assert_eq!(cache.hits(), 1);

    eq.insert("Sarah Chen works on EdgeQuake.", None)
        .await
        .expect("insert");

    engine.query(req).await.expect("post-insert");
    assert_eq!(
        cache.misses(),
        2,
        "orchestrator insert must invalidate query result cache when engine is wired"
    );
}

#[test]
fn spec021_orchestrator_ingestion_calls_invalidate_query_result_cache() {
    let src = include_str!("../src/orchestrator/ingestion.rs");
    assert!(
        src.contains("invalidate_query_result_cache_for_workspace"),
        "orchestrator ingestion must invalidate workspace-scoped cache when configured"
    );
}

#[tokio::test]
async fn spec021_orchestrator_default_engine_invalidates_cache_on_insert() {
    let mock = Arc::new(MockProvider::new());
    for _ in 0..8 {
        mock.add_response(edgequake_pipeline::SPEC021_SARAH_CHEN_EXTRACTION_JSON)
            .await;
    }

    let kv = Arc::new(MemoryKVStorage::new("cache-default"));
    let vector = Arc::new(MemoryVectorStorage::new("cache-default", 1536));
    let graph = Arc::new(MemoryGraphStorage::new("cache-default"));
    vector.initialize().await.unwrap();
    graph.initialize().await.unwrap();

    let mut eq = EdgeQuake::new(memory_config("cache-default"))
        .with_storage_backends(
            Arc::clone(&kv) as Arc<dyn edgequake_storage::traits::KVStorage>,
            Arc::clone(&vector) as Arc<dyn VectorStorage>,
            Arc::clone(&graph) as Arc<dyn GraphStorage>,
        )
        .with_providers(
            Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::LLMProvider>,
            Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
        );
    eq.initialize().await.expect("init");

    let engine = eq
        .query_engine()
        .expect("default initialize must wire query engine");
    let cache = engine
        .result_cache()
        .expect("default engine has result cache");

    let mut req = QueryRequest::new("default engine cache bust");
    req.context_only = true;
    req.mode = Some(QueryMode::Hybrid);

    engine.query(req.clone()).await.expect("prime");
    engine.query(req.clone()).await.expect("cached");
    assert_eq!(cache.hits(), 1);

    eq.insert("Sarah Chen works on EdgeQuake.", None)
        .await
        .expect("insert");

    engine.query(req).await.expect("post-insert");
    assert_eq!(cache.misses(), 2);
}
