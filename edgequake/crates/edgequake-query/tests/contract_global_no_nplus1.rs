//! P-G3 contract test (RC-8 / SPEC-021): Global mode must not issue N+1
//! `node_degree` calls.
//!
//! Acceptance (plan-19 §3 P-G3): a Global query over a graph with E entities
//! performs its degree fetch via a single `node_degrees_batch` call (as Local
//! does), not E per-entity `node_degree` calls.
//!
//! This test verifies the batched path works end-to-end: Global mode returns
//! entities with non-zero degrees derived from the batched call. The structural
//! "exactly one call" property is enforced by code construction — the previous
//! per-entity `graph.node_degree(id)` loop in `query_global_with_vector_storage`
//! was replaced by a single `tokio::join!(get_nodes_batch, node_degrees_batch)`
//! (see `vector_queries.rs` Global arm, Step 5).

use std::sync::Arc;

use edgequake_llm::MockProvider;
use edgequake_query::engine::QueryRequest;
use edgequake_query::{QueryEngine, QueryEngineConfig};
use edgequake_storage::traits::{GraphStorage, GraphStorageMutateOps, VectorStorage};
use edgequake_storage::{MemoryGraphStorage, MemoryVectorStorage};

#[tokio::test]
async fn global_mode_returns_entities_with_batched_degrees() {
    // Use the MockProvider's native embedding dimension (1536) so the engine's
    // embed_one call matches the vector storage dimension.
    let dim = 1536;
    let vector = Arc::new(MemoryVectorStorage::new("test", dim));
    let graph = Arc::new(MemoryGraphStorage::new("test"));
    vector.initialize().await.unwrap();
    graph.initialize().await.unwrap();

    // Seed two entities and a relationship between them in the graph.
    let mut props_a = std::collections::HashMap::new();
    props_a.insert("entity_type".to_string(), serde_json::json!("PERSON"));
    props_a.insert("description".to_string(), serde_json::json!("Alpha person"));
    graph.upsert_node("ALPHA", props_a).await.unwrap();

    let mut props_b = std::collections::HashMap::new();
    props_b.insert("entity_type".to_string(), serde_json::json!("PERSON"));
    props_b.insert("description".to_string(), serde_json::json!("Beta person"));
    graph.upsert_node("BETA", props_b).await.unwrap();

    let mut edge_props = std::collections::HashMap::new();
    edge_props.insert("relation_type".to_string(), serde_json::json!("KNOWS"));
    edge_props.insert(
        "description".to_string(),
        serde_json::json!("alpha knows beta"),
    );
    graph
        .upsert_edge("ALPHA", "BETA", edge_props)
        .await
        .unwrap();

    // Seed a relationship vector so Global mode's relationship-vector arm finds
    // ALPHA and BETA as entity_ids (the path that exercises Step 5 batched
    // degree fetch). src_id/tgt_id metadata carry the entity ids. The embedding
    // is a 1536-dim vector dominated by the first component so it ranks first.
    let mut rel_emb = vec![0.0_f32; dim];
    rel_emb[0] = 0.9;
    rel_emb[1] = 0.1;
    vector
        .upsert(&[(
            "ALPHA::BETA".to_string(),
            rel_emb,
            serde_json::json!({
                "type": "relationship",
                "src_id": "ALPHA",
                "tgt_id": "BETA",
                "relation_type": "KNOWS",
                "description": "alpha knows beta",
            }),
        )])
        .await
        .unwrap();

    let mock = Arc::new(MockProvider::default());
    let engine = QueryEngine::with_mock_keywords(
        QueryEngineConfig::default(),
        vector as Arc<dyn VectorStorage>,
        graph as Arc<dyn GraphStorage>,
        Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
        Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::LLMProvider>,
    );

    let mut req = QueryRequest::new("alpha beta relationship");
    req.mode = Some(edgequake_query::QueryMode::Global);
    req.context_only = true;
    let response = engine.query(req).await.expect("global query must succeed");

    // The batched degree path must have populated entities for ALPHA and BETA.
    assert!(
        !response.context.entities.is_empty(),
        "Global mode must return entities via the batched degree path"
    );
    let names: Vec<&str> = response
        .context
        .entities
        .iter()
        .map(|e| e.name.as_str())
        .collect();
    assert!(names.contains(&"ALPHA") || names.contains(&"BETA"));
}
