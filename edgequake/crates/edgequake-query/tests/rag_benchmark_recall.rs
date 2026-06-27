//! SPEC-023 I3 — lightweight retrieval recall@k benchmark (mock provider, CI-safe).

use std::sync::Arc;

use edgequake_llm::MockProvider;
use edgequake_query::{QueryEngine, QueryEngineConfig, QueryMode, QueryRequest};
use edgequake_storage::traits::VectorStorage;
use edgequake_storage::{GraphStorage, MemoryGraphStorage, MemoryVectorStorage};
use serde_json::json;

fn make_engine(
    vector: Arc<MemoryVectorStorage>,
    graph: Arc<MemoryGraphStorage>,
    mock: Arc<MockProvider>,
    config: QueryEngineConfig,
) -> QueryEngine {
    QueryEngine::with_mock_keywords(
        config,
        vector as Arc<dyn VectorStorage>,
        graph,
        mock.clone() as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
        mock as Arc<dyn edgequake_llm::traits::LLMProvider>,
    )
}

#[tokio::test]
async fn rag_benchmark_recall_at_5_naive_arm() {
    let dim = 1536;
    let vector = Arc::new(MemoryVectorStorage::new("bench", dim));
    let graph = Arc::new(MemoryGraphStorage::new("bench"));
    vector.initialize().await.unwrap();
    graph.initialize().await.unwrap();

    let golden_id = "golden-chunk-recall";
    vector
        .upsert(&[(
            golden_id.to_string(),
            vec![1.0_f32; dim],
            json!({
                "type": "chunk",
                "content": "Sarah Chen EdgeQuake Zurich research lab",
                "document_id": "golden-doc",
            }),
        )])
        .await
        .unwrap();

    for i in 0..8 {
        let mut emb = vec![0.0_f32; dim];
        emb[1] = 0.95 - (i as f32 * 0.05);
        vector
            .upsert(&[(
                format!("distractor-{i}"),
                emb,
                json!({
                    "type": "chunk",
                    "content": format!("noise {i}"),
                    "document_id": format!("noise-{i}"),
                }),
            )])
            .await
            .unwrap();
    }

    let mock = Arc::new(MockProvider::default());
    let config = QueryEngineConfig {
        mix_local_weight: 0.0,
        mix_global_weight: 0.0,
        mix_naive_weight: 1.0,
        ..Default::default()
    };

    let engine = make_engine(vector, graph, mock, config);
    let mut req = QueryRequest::new("Sarah Chen EdgeQuake Zurich");
    req.mode = Some(QueryMode::Mix);
    req.context_only = true;
    req.enable_rerank = Some(false);

    let response = engine.query(req).await.expect("mix query");
    let top5: Vec<String> = response
        .context
        .chunks
        .iter()
        .take(5)
        .map(|c| c.id.clone())
        .collect();

    assert!(
        top5.iter().any(|id| id == golden_id),
        "recall@5 must include golden chunk; got {:?}",
        top5
    );
}
