//! P-G9 contract — repeated `context_only` queries hit result cache.

use std::sync::Arc;

use edgequake_llm::MockProvider;
use edgequake_query::engine::QueryRequest;
use edgequake_query::{QueryEngine, QueryEngineConfig, QueryMode, QueryResultCacheInvalidator};
use edgequake_storage::traits::{GraphStorage, VectorStorage};
use edgequake_storage::{MemoryGraphStorage, MemoryVectorStorage};

async fn engine_with_result_cache() -> Arc<QueryEngine> {
    let dim = 1536;
    let vector = Arc::new(MemoryVectorStorage::new("test", dim));
    let graph = Arc::new(MemoryGraphStorage::new("test"));
    vector.initialize().await.unwrap();
    graph.initialize().await.unwrap();

    let mock = Arc::new(MockProvider::default());
    Arc::new(
        QueryEngine::with_mock_keywords(
            QueryEngineConfig::default(),
            vector as Arc<dyn VectorStorage>,
            graph as Arc<dyn GraphStorage>,
            Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
            Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::LLMProvider>,
        )
        .with_result_cache(),
    )
}

#[tokio::test]
async fn contract_context_only_result_cache_hits_on_repeat() {
    let engine = engine_with_result_cache().await;
    let cache = engine.result_cache().expect("result cache wired");

    let mut req = QueryRequest::new("cached query text");
    req.context_only = true;
    req.mode = Some(QueryMode::Hybrid);

    engine.query(req.clone()).await.expect("first query");
    assert_eq!(cache.misses(), 1);
    assert_eq!(cache.hits(), 0);

    engine.query(req).await.expect("second query");
    assert_eq!(
        cache.hits(),
        1,
        "second identical context_only query must hit cache"
    );
}

#[tokio::test]
async fn contract_invalidator_trait_clears_result_cache() {
    let engine = engine_with_result_cache().await;
    let cache = engine.result_cache().expect("result cache wired");
    let port: Arc<dyn QueryResultCacheInvalidator> =
        Arc::clone(&engine) as Arc<dyn QueryResultCacheInvalidator>;

    let mut req = QueryRequest::new("trait port invalidation");
    req.context_only = true;
    req.mode = Some(QueryMode::Hybrid);

    engine.query(req.clone()).await.expect("first");
    assert_eq!(cache.misses(), 1);
    engine.query(req.clone()).await.expect("cached");
    assert_eq!(cache.hits(), 1);

    port.invalidate_query_result_cache();

    engine.query(req).await.expect("after trait invalidate");
    assert_eq!(
        cache.misses(),
        2,
        "QueryResultCacheInvalidator must force a retrieval miss"
    );
}
