//! SPEC-021 P-G2 contract — one persist sequence, deduplicated graph identity.

use std::sync::Arc;

use edgequake_pipeline::{
    persist_processing_result, ChunkVectorBuildOptions, ExtractedEntity, ExtractedRelationship,
    ExtractionResult, IngestionPersistConfig, IngestionPersistContext, MergerConfig, NoopEntitySink,
    ProcessingResult, TextChunk,
};
use edgequake_storage::{
    EntityId, GraphStorageReadOps, MemoryGraphStorage, MemoryVectorStorage, VectorStorage,
};

const EMBED_DIM: usize = 4;

fn fixture_result() -> ProcessingResult {
    let embedding = vec![0.1, 0.2, 0.3, 0.4];
    let chunk = TextChunk {
        id: "doc1-chunk-0".to_string(),
        content: "Sarah Chen leads EdgeQuake.".to_string(),
        index: 0,
        embedding: Some(embedding.clone()),
        start_line: 1,
        end_line: 1,
        start_offset: 0,
        end_offset: 28,
        token_count: 5,
    };
    ProcessingResult {
        document_id: "doc1".to_string(),
        chunks: vec![chunk],
        extractions: vec![ExtractionResult {
            entities: vec![ExtractedEntity::new("Sarah Chen", "PERSON", "Engineer")
                .with_source_chunk_id("doc1-chunk-0")
                .with_importance(0.9)],
            relationships: vec![ExtractedRelationship::new(
                "Sarah Chen",
                "EdgeQuake",
                "LEADS",
            )],
            source_chunk_id: "doc1-chunk-0".to_string(),
            ..Default::default()
        }],
        stats: Default::default(),
        lineage: None,
    }
}

#[tokio::test]
async fn contract_double_persist_merges_to_single_normalized_entity() {
    let graph = Arc::new(MemoryGraphStorage::new("contract"));
    let vector = Arc::new(MemoryVectorStorage::new("contract", EMBED_DIM));
    vector.initialize().await.unwrap();

    let config = IngestionPersistConfig {
        merger_config: MergerConfig {
            use_llm_summarization: false,
            ..Default::default()
        },
        relational_sink: Arc::new(NoopEntitySink),
        llm_provider: None,
    };
    let ctx = IngestionPersistContext {
        document_id: "doc1".to_string(),
        tenant_id: None,
        workspace_id: None,
    };
    let options = ChunkVectorBuildOptions {
        include_lineage_metadata: true,
    };

    let result = fixture_result();
    persist_processing_result(
        graph.clone(),
        vector.clone(),
        &config,
        &ctx,
        &result,
        options,
    )
    .await
    .expect("first persist");

    persist_processing_result(
        graph.clone(),
        vector.clone(),
        &config,
        &ctx,
        &result,
        options,
    )
    .await
    .expect("second persist");

    let entity_id = EntityId::new("Sarah Chen");
    let node_id = entity_id.as_graph_node_id().to_string();
    assert!(
        graph.get_node(&node_id).await.unwrap().is_some(),
        "normalized graph node must exist"
    );
    let nodes = graph
        .get_nodes_by_ids(&[node_id.clone()])
        .await
        .unwrap();
    assert_eq!(nodes.len(), 1, "duplicate persist must not fork graph nodes");

    let chunk_vectors = vector
        .query(&vec![0.0_f32; EMBED_DIM], 100, None)
        .await
        .unwrap();
    let chunk_count = chunk_vectors
        .iter()
        .filter(|r| r.metadata.get("type").and_then(|v| v.as_str()) == Some("chunk"))
        .count();
    assert_eq!(chunk_count, 1, "chunk vectors should not duplicate on re-persist");
}
