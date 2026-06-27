//! SPEC-025 5.2 / 5.3 — ingestion pipeline builder parity.

use edgequake_llm::MockProvider;
use edgequake_pipeline::prompts::EntityExtractionSchema;
use edgequake_pipeline::{
    build_ingestion_pipeline, calculate_adaptive_chunk_size, IngestionPipelineOptions,
};
use std::sync::Arc;

#[test]
fn adaptive_chunk_thresholds_match_library() {
    assert_eq!(calculate_adaptive_chunk_size(30_000), 1200);
    assert_eq!(calculate_adaptive_chunk_size(80_000), 800);
    assert_eq!(calculate_adaptive_chunk_size(150_000), 600);
}

#[test]
fn ingestion_pipeline_applies_document_size() {
    let llm = Arc::new(MockProvider::new()) as Arc<dyn edgequake_llm::LLMProvider>;
    let embedding =
        Arc::new(MockProvider::new()) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>;

    let pipeline = build_ingestion_pipeline(
        llm,
        embedding,
        EntityExtractionSchema::server_default(),
        IngestionPipelineOptions::from_document_size(120_000),
    );

    // Mock embedding provider caps chunk_size to max_tokens/2; adaptive target is 600.
    assert_eq!(calculate_adaptive_chunk_size(120_000), 600);
    assert!(pipeline.config().chunker.chunk_size <= 600);
    assert!(pipeline.config().chunker.chunk_size > 0);
}
