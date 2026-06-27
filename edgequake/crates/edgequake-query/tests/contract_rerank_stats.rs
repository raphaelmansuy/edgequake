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
    let src = include_str!("../../edgequake-api/src/handlers/query/query_execute.rs");
    assert!(
        src.contains("result.stats.rerank_time_ms"),
        "API must forward engine rerank_time_ms (SPEC-024 4.1)"
    );
}

#[test]
fn contract_chat_completion_wires_engine_rerank_time_ms() {
    let src = include_str!("../../edgequake-api/src/handlers/chat/completion.rs");
    assert!(
        src.contains("result.stats.rerank_time_ms"),
        "chat completion must forward engine rerank_time_ms (SPEC-024 4.7)"
    );
    assert!(
        !src.contains("rerank_time_ms: None,"),
        "chat completion must not hardcode rerank_time_ms: None"
    );
}
