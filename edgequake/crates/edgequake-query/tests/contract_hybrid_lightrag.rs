//! SPEC-024 — Hybrid mode LightRAG merge contract (round-robin + dedup + max_chunks).

use std::sync::Arc;

use edgequake_llm::MockProvider;
use edgequake_query::context::RetrievedChunk;
use edgequake_query::engine::QueryRequest;
use edgequake_query::hybrid_merge::{merge_hybrid_contexts, round_robin_merge_chunks};
use edgequake_query::{QueryEngine, QueryEngineConfig, QueryMode};
use edgequake_storage::traits::{GraphStorage, GraphStorageMutateOps, VectorStorage};
use edgequake_storage::{MemoryGraphStorage, MemoryVectorStorage};

#[test]
fn contract_hybrid_round_robin_kg_first_ordering() {
    let local = vec![
        RetrievedChunk::new("shared", "shared", 0.9),
        RetrievedChunk::new("local_only", "local", 0.8),
    ];
    let global = vec![
        RetrievedChunk::new("shared", "shared", 0.85),
        RetrievedChunk::new("global_only", "global", 0.7),
    ];
    let naive = vec![RetrievedChunk::new("naive_only", "naive", 0.6)];

    let merged = round_robin_merge_chunks(&local, &global, &naive, 20);
    let ids: Vec<_> = merged.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["shared", "naive_only", "local_only", "global_only"],
        "LightRAG hybrid: per slot local→global→naive; shared deduped once at local slot"
    );
}

#[test]
fn contract_hybrid_respects_max_chunks_cap() {
    let local: Vec<_> = (0..5)
        .map(|i| RetrievedChunk::new(format!("l{i}"), "x", 1.0))
        .collect();
    let global: Vec<_> = (0..5)
        .map(|i| RetrievedChunk::new(format!("g{i}"), "x", 1.0))
        .collect();
    let naive: Vec<_> = (0..5)
        .map(|i| RetrievedChunk::new(format!("n{i}"), "x", 1.0))
        .collect();

    let merged = round_robin_merge_chunks(&local, &global, &naive, 3);
    assert_eq!(merged.len(), 3, "Hybrid must truncate to max_chunks");
}

#[tokio::test]
async fn contract_hybrid_engine_deduplicates_shared_chunk() {
    let dim = 1536;
    let vector = Arc::new(MemoryVectorStorage::new("test", dim));
    let graph = Arc::new(MemoryGraphStorage::new("test"));
    vector.initialize().await.unwrap();
    graph.initialize().await.unwrap();

    vector
        .upsert(&[(
            "shared-chunk".into(),
            vec![1.0; dim],
            serde_json::json!({"type": "chunk", "content": "shared"}),
        )])
        .await
        .unwrap();

    vector
        .upsert(&[(
            "entity:ALPHA".into(),
            vec![1.0; dim],
            serde_json::json!({
                "type": "entity",
                "entity_name": "ALPHA",
                "source_chunk_ids": ["shared-chunk"],
            }),
        )])
        .await
        .unwrap();

    graph
        .upsert_node(
            "ALPHA",
            [(
                "source_chunk_ids".to_string(),
                serde_json::json!(["shared-chunk"]),
            )]
            .into_iter()
            .collect(),
        )
        .await
        .unwrap();

    let mock = Arc::new(MockProvider::default());
    let config = QueryEngineConfig {
        max_chunks: 10,
        ..Default::default()
    };

    let engine = QueryEngine::with_mock_keywords(
        config,
        vector as Arc<dyn VectorStorage>,
        graph as Arc<dyn GraphStorage>,
        Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
        Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::LLMProvider>,
    );

    let mut req = QueryRequest::new("alpha shared");
    req.mode = Some(QueryMode::Hybrid);
    req.context_only = true;

    let resp = engine.query(req).await.expect("hybrid query");
    let shared_count = resp
        .context
        .chunks
        .iter()
        .filter(|c| c.id == "shared-chunk")
        .count();
    assert_eq!(shared_count, 1, "Hybrid must dedupe chunk IDs across arms");
}

#[test]
fn contract_merge_hybrid_contexts_empty_naive_arm() {
    let mut local = edgequake_query::context::QueryContext::new();
    local.add_chunk(RetrievedChunk::new("c1", "one", 1.0));
    let merged = merge_hybrid_contexts(
        local,
        edgequake_query::context::QueryContext::new(),
        edgequake_query::context::QueryContext::new(),
        5,
    );
    assert_eq!(merged.chunks.len(), 1);
}
