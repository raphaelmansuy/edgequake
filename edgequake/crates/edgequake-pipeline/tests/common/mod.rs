//! Shared test fixtures for pipeline integration tests (DRY).

use edgequake_pipeline::{
    ExtractedEntity, ExtractedRelationship, ExtractionResult, IngestionPersistContext,
    ProcessingResult, TextChunk,
};

pub const EMBED_DIM: usize = 4;

#[allow(dead_code)] // shared fixtures for optional integration tests
pub fn sample_processing_result() -> ProcessingResult {
    let embedding = vec![0.1, 0.2, 0.3, 0.4];
    let chunk = TextChunk {
        id: "doc1-chunk-0".to_string(),
        content: "Sarah Chen leads EdgeQuake.".to_string(),
        index: 0,
        embedding: Some(embedding),
        start_line: 1,
        end_line: 1,
        start_offset: 0,
        end_offset: 28,
        token_count: 5,
        section: None,
        page_start: None,
        page_end: None,
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

#[allow(dead_code)] // shared fixtures for optional integration tests
pub fn sample_persist_context() -> IngestionPersistContext {
    IngestionPersistContext::new("doc1", None, None)
}
