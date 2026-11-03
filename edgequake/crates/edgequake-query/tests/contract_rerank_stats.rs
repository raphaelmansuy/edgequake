//! SPEC-024 Phase 4.1 — rerank_time_ms tracked separately from retrieval (no fabrication).

use std::sync::Arc;

use edgequake_llm::reranker::BM25Reranker;
use edgequake_query::engine::QueryRequest;
use edgequake_query::{QueryEngine, QueryEngineConfig, QueryMode};
use edgequake_storage::traits::{GraphStorage, VectorStorage};
use edgequake_storage::{MemoryGraphStorage, MemoryVectorStorage};

#[tokio::test]
async fn contract_rerank_time_ms_populated_when_reranker_applied() {
    let dim = 1536;
    let vector = Arc::new(MemoryVectorStorage::new("rerank-stats", dim));
    let graph = Arc::new(MemoryGraphStorage::new("rerank-stats"));
    vector.initialize().await.unwrap();
    graph.initialize().await.unwrap();

    vector
        .upsert(&[(
            "chunk-a".into(),
            vec![1.0; dim],
            serde_json::json!({
                "type": "chunk",
                "content": "EdgeQuake hybrid LightRAG retrieval pipeline",
            }),
        )])
        .await
        .unwrap();

    let mock = Arc::new(edgequake_llm::MockProvider::new());
    let engine = QueryEngine::with_mock_keywords(
        QueryEngineConfig::default(),
        vector as Arc<dyn edgequake_storage::traits::VectorStorage>,
        graph as Arc<dyn edgequake_storage::traits::GraphStorage>,
        Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
        Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::LLMProvider>,
    )
    .with_reranker(Arc::new(BM25Reranker::new_enhanced()));

    let mut request = QueryRequest::new("EdgeQuake hybrid");
    request.mode = Some(QueryMode::Naive);
    request.context_only = true;
    request.enable_rerank = Some(true);

    let response = engine
        .query(request)
        .await
        .expect("naive query with rerank");

    assert!(
        response.stats.rerank_time_ms.is_some(),
        "engine must report rerank_time_ms when reranker runs"
    );
}

#[test]
fn contract_query_execute_wires_engine_rerank_time_ms() {
    // The mapping moved during the handler split: the forwarding now lives in the stats mappers
    // rather than inline in `query_execute.rs`. This test used to grep the old file and had been
    // failing ever since, even though the behaviour is intact — so it asserts the behaviour's
    // *new* home instead of a path that no longer holds it.
    let stats = include_str!("../../edgequake-api/src/services/query_stats_mapper.rs");
    assert!(
        stats.contains("rerank_time_ms: engine.rerank_time_ms"),
        "API must forward engine rerank_time_ms (SPEC-024 4.1)"
    );
    let types = include_str!("../../edgequake-api/src/handlers/query_types.rs");
    assert!(
        types.contains("rerank_time_ms: stats.rerank_time_ms"),
        "the response DTO must carry the engine's rerank_time_ms (SPEC-024 4.1)"
    );
}

#[test]
fn contract_chat_completion_wires_engine_rerank_time_ms() {
    // Same relocation as above: the chat path maps through `context_bundle_mapper`.
    let bundle = include_str!("../../edgequake-api/src/services/context_bundle_mapper.rs");
    assert!(
        bundle.contains("rerank_time_ms: result.stats.rerank_time_ms"),
        "chat completion must forward engine rerank_time_ms (SPEC-024 4.7)"
    );
    assert!(
        !bundle.contains("rerank_time_ms: None,"),
        "chat completion must not hardcode rerank_time_ms: None"
    );
}
