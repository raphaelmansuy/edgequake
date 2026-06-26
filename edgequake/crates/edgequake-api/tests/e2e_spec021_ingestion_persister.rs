//! SPEC-021 P-G2 E2E — ingestion persister returns merge stats via memory stores.

use std::sync::Arc;

use edgequake_pipeline::{
    persist_processing_result, ChunkVectorBuildOptions, ExtractedEntity, ExtractedRelationship,
    ExtractionResult, IngestionPersistConfig, IngestionPersistContext, MergerConfig, NoopEntitySink,
    ProcessingResult, TextChunk,
};
use edgequake_storage::{
    GraphStorageReadOps, MemoryGraphStorage, MemoryVectorStorage, VectorStorage,
};

#[tokio::test]
async fn spec021_persist_processing_result_writes_entities() {
    let graph = Arc::new(MemoryGraphStorage::new("spec021-pg2"));
    let vector = Arc::new(MemoryVectorStorage::new("spec021-pg2", 4));
    vector.initialize().await.unwrap();

    let chunk = TextChunk {
        id: "e2e-chunk-0".to_string(),
        content: "Alice founded Nova Labs.".to_string(),
        index: 0,
        embedding: Some(vec![0.5, 0.5, 0.5, 0.5]),
        start_line: 1,
        end_line: 1,
        start_offset: 0,
        end_offset: 22,
        token_count: 4,
    };
    let result = ProcessingResult {
        document_id: "e2e-doc".to_string(),
        chunks: vec![chunk],
        extractions: vec![ExtractionResult {
            entities: vec![ExtractedEntity::new("Alice", "PERSON", "Founder")],
            relationships: vec![ExtractedRelationship::new("Alice", "Nova Labs", "FOUNDED")],
            source_chunk_id: "e2e-chunk-0".to_string(),
            ..Default::default()
        }],
        stats: Default::default(),
        lineage: None,
    };

    let out = persist_processing_result(
        graph.clone(),
        vector.clone(),
        &IngestionPersistConfig {
            merger_config: MergerConfig {
                use_llm_summarization: false,
                ..Default::default()
            },
            relational_sink: Arc::new(NoopEntitySink),
            llm_provider: None,
        },
        &IngestionPersistContext {
            document_id: "e2e-doc".to_string(),
            tenant_id: None,
            workspace_id: None,
        },
        &result,
        ChunkVectorBuildOptions::default(),
    )
    .await
    .expect("persist");

    assert_eq!(out.chunk_vector_ids.len(), 1);
    assert!(
        out.merge_stats.entities_created + out.merge_stats.entities_updated > 0,
        "expected entities merged"
    );
    assert!(
        graph.get_node("ALICE").await.unwrap().is_some(),
        "normalized graph node must exist"
    );
}
