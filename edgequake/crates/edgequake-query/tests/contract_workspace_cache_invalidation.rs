//! SPEC-024 Phase 1.4 — workspace-scoped query result cache invalidation.

use std::sync::Arc;

use edgequake_llm::MockProvider;
use edgequake_query::engine::QueryRequest;
use edgequake_query::{QueryEngine, QueryEngineConfig, QueryMode, QueryResultCacheInvalidator};
use edgequake_storage::traits::{GraphStorage, VectorStorage};
use edgequake_storage::{MemoryGraphStorage, MemoryVectorStorage};

async fn engine_with_cache() -> Arc<QueryEngine> {
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
async fn contract_workspace_invalidation_preserves_other_workspace_cache() {
    let engine = engine_with_cache().await;
    let cache = engine.result_cache().expect("cache");
    let port: Arc<dyn QueryResultCacheInvalidator> =
        Arc::clone(&engine) as Arc<dyn QueryResultCacheInvalidator>;

    let mut req_a = QueryRequest::new("workspace scoped cache a");
    req_a.context_only = true;
    req_a.mode = Some(QueryMode::Hybrid);
    req_a = req_a.with_workspace_id("workspace-a");

    let mut req_b = QueryRequest::new("workspace scoped cache b");
    req_b.context_only = true;
    req_b.mode = Some(QueryMode::Hybrid);
    req_b = req_b.with_workspace_id("workspace-b");

    engine.query(req_a.clone()).await.expect("query a");
    engine.query(req_b.clone()).await.expect("query b");
    assert_eq!(cache.misses(), 2);

    engine.query(req_a.clone()).await.expect("cached a");
    engine.query(req_b.clone()).await.expect("cached b");
    assert_eq!(cache.hits(), 2);

    port.invalidate_query_result_cache_for_workspace("workspace-a");

    engine.query(req_a).await.expect("a after invalidate");
    engine.query(req_b.clone()).await.expect("b still cached");
    assert_eq!(
        cache.hits(),
        3,
        "workspace-b entry must survive ws-a invalidation"
    );
    assert_eq!(
        cache.misses(),
        3,
        "workspace-a must miss after scoped invalidation"
    );
}
