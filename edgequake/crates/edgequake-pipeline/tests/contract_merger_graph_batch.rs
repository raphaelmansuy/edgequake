//! P-G4-graph contract — batch merge creates normalized graph nodes.

mod common;

use std::sync::Arc;

use common::EMBED_DIM;
use edgequake_pipeline::{
    DefaultIngestionPersister, ExtractedEntity, ExtractedRelationship, ExtractionResult,
    IngestionPersistSettings, IngestionPersister, NoopEntitySink, ProcessingResult, TextChunk,
};
use edgequake_storage::{
    EntityId, GraphStorageReadOps, MemoryGraphStorage, MemoryVectorStorage, VectorStorage,
};

#[tokio::test]
async fn contract_batch_merge_creates_all_entity_nodes() {
    let graph = Arc::new(MemoryGraphStorage::new("batch-graph"));
    let vector = Arc::new(MemoryVectorStorage::new("batch-graph", EMBED_DIM));
    vector.initialize().await.unwrap();

    let mut entities = Vec::new();
    for i in 0..12 {
        entities.push(
            ExtractedEntity::new(format!("Entity {i}"), "CONCEPT", format!("desc {i}"))
                .with_source_chunk_id("doc-batch-chunk-0")
                .with_importance(0.5),
        );
    }

    let chunk = TextChunk {
        id: "doc-batch-chunk-0".to_string(),
        content: "batch entity test".to_string(),
        index: 0,
        embedding: Some(vec![0.1; EMBED_DIM]),
        start_line: 1,
        end_line: 1,
        start_offset: 0,
        end_offset: 10,
        token_count: 3,
        section: None,
        page_start: None,
        page_end: None,
    };

    let result = ProcessingResult {
        document_id: "doc-batch".to_string(),
        chunks: vec![chunk],
        extractions: vec![ExtractionResult {
            entities,
            relationships: vec![ExtractedRelationship::new(
                "Entity 0", "Entity 1", "RELATES",
            )],
            source_chunk_id: "doc-batch-chunk-0".to_string(),
            ..Default::default()
        }],
        stats: Default::default(),
        lineage: None,
    };

    let persister = DefaultIngestionPersister::from_settings(
        graph.clone(),
        vector,
        IngestionPersistSettings {
            use_llm_summarization: false,
        },
        Arc::new(NoopEntitySink),
        None,
        None,
    );

    persister
        .persist(
            &edgequake_pipeline::IngestionPersistContext::new("doc-batch", None, None),
            &result,
            edgequake_pipeline::ChunkVectorBuildOptions::STANDARD,
        )
        .await
        .expect("batch persist");

    for i in 0..12 {
        let id = EntityId::new(&format!("Entity {i}"))
            .as_graph_node_id()
            .to_string();
        assert!(
            graph.get_node(&id).await.unwrap().is_some(),
            "batch merge must create node {id}"
        );
    }
}
