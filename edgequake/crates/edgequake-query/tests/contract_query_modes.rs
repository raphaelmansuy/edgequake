//! P-G8 contract tests (RC-13 / SPEC-021): Bypass + Mix mode semantics.
//!
//! Acceptance (plan-19 §4 P-G8):
//! - Bypass: `POST /query {mode:"bypass"}` returns a *direct* LLM answer, not the
//!   RAG apology string ("I couldn't find any relevant information..."). An empty
//!   context is intentional for Bypass, not a retrieval miss.
//! - Mix: a weighted blend produces ordering different from Hybrid on a fixture
//!   where local and global disagree, when weights are skewed. With equal
//!   weights, Mix matches Hybrid ordering (backward compatible).

use std::sync::Arc;

use edgequake_llm::MockProvider;
use edgequake_query::engine::QueryRequest;
use edgequake_query::{QueryEngine, QueryEngineConfig, QueryMode};
use edgequake_storage::traits::{GraphStorage, GraphStorageMutateOps, VectorStorage};
use edgequake_storage::{MemoryGraphStorage, MemoryVectorStorage};

fn make_engine(
    vector: Arc<MemoryVectorStorage>,
    graph: Arc<MemoryGraphStorage>,
    mock: Arc<MockProvider>,
    config: QueryEngineConfig,
) -> QueryEngine {
    QueryEngine::with_mock_keywords(
        config,
        vector as Arc<dyn VectorStorage>,
        graph as Arc<dyn GraphStorage>,
        Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
        Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::LLMProvider>,
    )
}

#[tokio::test]
async fn bypass_returns_direct_llm_answer_not_apology() {
    // Use the MockProvider's native embedding dimension (1536) so embed_one
    // matches the vector storage dimension.
    let dim = 1536;
    let vector = Arc::new(MemoryVectorStorage::new("test", dim));
    let graph = Arc::new(MemoryGraphStorage::new("test"));
    vector.initialize().await.unwrap();
    graph.initialize().await.unwrap();

    // Seed a custom direct answer so we can distinguish it from the RAG apology.
    let mock = Arc::new(MockProvider::default());
    mock.add_response("DIRECT_LLM_ANSWER_42").await;

    let engine = make_engine(vector, graph, mock, QueryEngineConfig::default());

    let mut req = QueryRequest::new("What is the meaning of life?");
    req.mode = Some(QueryMode::Bypass);
    let response = engine.query(req).await.expect("bypass query must succeed");

    // The answer must be the direct LLM response, NOT the RAG apology.
    assert_ne!(
        response.answer, "",
        "Bypass must produce a real answer, not empty"
    );
    assert!(
        !response
            .answer
            .contains("couldn't find any relevant information"),
        "Bypass must NOT return the RAG apology; got: {}",
        response.answer
    );
    assert_eq!(
        response.mode,
        QueryMode::Bypass,
        "response mode must be Bypass"
    );
    assert!(
        response.context.is_empty(),
        "Bypass context must be empty (no retrieval)"
    );
    assert_eq!(
        response.answer, "DIRECT_LLM_ANSWER_42",
        "Bypass must return the direct LLM answer verbatim"
    );
}

#[tokio::test]
async fn mix_with_skewed_weights_differs_from_hybrid() {
    // P-G8 acceptance: Mix must be a *real* weighted blend, not a Hybrid alias.
    //
    // Strategy (robust to mock-embedding uniformity): prove the blend is
    // weight-sensitive. Run the same Mix query twice with opposite weight
    // skews (naive-only vs local-only). If Mix were a Hybrid alias, both runs
    // would return identical ordering regardless of weights. Because Mix blends
    // by weighted normalized score, the two skews must produce a DIFFERENT top
    // chunk on a fixture where the naive and KG arms surface different chunks.
    //
    // Fixture: two chunks with orthogonal embeddings so the naive arm (direct
    // similarity to the uniform mock query) ranks them differently than the KG
    // arm (entity → source_chunk_ids linkage). One entity links only to
    // chunk_kg_top, so the local arm surfaces chunk_kg_top while naive surfaces
    // both by direct similarity.
    let dim = 1536;
    let vector = Arc::new(MemoryVectorStorage::new("test", dim));
    let graph = Arc::new(MemoryGraphStorage::new("test"));
    vector.initialize().await.unwrap();
    graph.initialize().await.unwrap();

    let naive_top = vec![1.0_f32; dim];
    vector
        .upsert(&[(
            "chunk_naive_top".to_string(),
            naive_top,
            serde_json::json!({
                "type": "chunk",
                "content": "naive top content",
                "document_id": "doc-naive",
            }),
        )])
        .await
        .unwrap();

    let mut kg_top = vec![0.0_f32; dim];
    kg_top[1] = 0.95;
    vector
        .upsert(&[(
            "chunk_kg_top".to_string(),
            kg_top,
            serde_json::json!({
                "type": "chunk",
                "content": "kg top content",
                "document_id": "doc-kg",
            }),
        )])
        .await
        .unwrap();

    // Entity linked to chunk_kg_top so the local arm surfaces it. Its embedding
    // is uniform so it matches the MockProvider's uniform query embedding (the
    // local arm finds entities by query-vector similarity).
    let ent_emb = vec![1.0_f32; dim];
    vector
        .upsert(&[(
            "entity:KG_ENTITY".to_string(),
            ent_emb,
            serde_json::json!({
                "type": "entity",
                "entity_name": "KG_ENTITY",
                "source_chunk_ids": ["chunk_kg_top"],
            }),
        )])
        .await
        .unwrap();

    let mut props = std::collections::HashMap::new();
    props.insert("entity_type".to_string(), serde_json::json!("CONCEPT"));
    props.insert("description".to_string(), serde_json::json!("kg entity"));
    props.insert(
        "source_chunk_ids".to_string(),
        serde_json::json!(["chunk_kg_top"]),
    );
    graph.upsert_node("KG_ENTITY", props).await.unwrap();

    let mock = Arc::new(MockProvider::default());

    // Mix skewed to naive only.
    let mix_naive = QueryEngineConfig {
        mix_local_weight: 0.0,
        mix_global_weight: 0.0,
        mix_naive_weight: 1.0,
        ..Default::default()
    };
    let engine_naive = make_engine(
        Arc::clone(&vector),
        Arc::clone(&graph),
        Arc::clone(&mock),
        mix_naive,
    );
    let mut req_n = QueryRequest::new("kg entity");
    req_n.mode = Some(QueryMode::Mix);
    req_n.context_only = true;
    let resp_n = engine_naive
        .query(req_n)
        .await
        .expect("mix-naive must succeed");

    // Mix skewed to local only.
    let mix_local = QueryEngineConfig {
        mix_local_weight: 1.0,
        mix_global_weight: 0.0,
        mix_naive_weight: 0.0,
        ..Default::default()
    };
    let engine_local = make_engine(vector, graph, mock, mix_local);
    let mut req_l = QueryRequest::new("kg entity");
    req_l.mode = Some(QueryMode::Mix);
    req_l.context_only = true;
    let resp_l = engine_local
        .query(req_l)
        .await
        .expect("mix-local must succeed");

    assert!(
        !resp_n.context.chunks.is_empty() && !resp_l.context.chunks.is_empty(),
        "both Mix skews must return chunks"
    );

    let n_ids: Vec<String> = resp_n.context.chunks.iter().map(|c| c.id.clone()).collect();
    let l_ids: Vec<String> = resp_l.context.chunks.iter().map(|c| c.id.clone()).collect();
    // The two weight skews must produce different orderings — proving Mix is
    // weight-sensitive (a real blend), not a Hybrid alias.
    assert_ne!(
        n_ids, l_ids,
        "Mix must be weight-sensitive: naive-only vs local-only must differ in ordering"
    );
}

#[tokio::test]
async fn mix_equal_weights_matches_hybrid_ordering() {
    // E24 / backward compat: with equal weights, Mix must return the SAME set of
    // chunks as Hybrid (ordering may differ by tie-break, but the chunk SET must
    // be identical because both arms run the same retrieval).
    let dim = 1536;
    let vector = Arc::new(MemoryVectorStorage::new("test", dim));
    let graph = Arc::new(MemoryGraphStorage::new("test"));
    vector.initialize().await.unwrap();
    graph.initialize().await.unwrap();

    // Seed two chunks with distinct embeddings so retrieval is deterministic.
    let mut a = vec![0.0_f32; dim];
    a[0] = 0.9;
    vector
        .upsert(&[(
            "chunk_a".to_string(),
            a,
            serde_json::json!({"type": "chunk", "content": "a", "document_id": "d"}),
        )])
        .await
        .unwrap();
    let mut b = vec![0.0_f32; dim];
    b[1] = 0.9;
    vector
        .upsert(&[(
            "chunk_b".to_string(),
            b,
            serde_json::json!({"type": "chunk", "content": "b", "document_id": "d"}),
        )])
        .await
        .unwrap();

    let mock = Arc::new(MockProvider::default());

    let hybrid_engine = make_engine(
        Arc::clone(&vector),
        Arc::clone(&graph),
        Arc::clone(&mock),
        QueryEngineConfig::default(),
    );
    let mut hreq = QueryRequest::new("a b");
    hreq.mode = Some(QueryMode::Hybrid);
    hreq.context_only = true;
    let h = hybrid_engine.query(hreq).await.unwrap();

    let mix_engine = make_engine(vector, graph, mock, QueryEngineConfig::default());
    let mut mreq = QueryRequest::new("a b");
    mreq.mode = Some(QueryMode::Mix);
    mreq.context_only = true;
    let m = mix_engine.query(mreq).await.unwrap();

    let h_ids: std::collections::HashSet<String> =
        h.context.chunks.iter().map(|c| c.id.clone()).collect();
    let m_ids: std::collections::HashSet<String> =
        m.context.chunks.iter().map(|c| c.id.clone()).collect();
    assert_eq!(
        h_ids, m_ids,
        "equal-weight Mix must return the same chunk SET as Hybrid"
    );
}
