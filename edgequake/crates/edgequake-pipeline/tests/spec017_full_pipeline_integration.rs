//! SPEC-017 — Full pipeline proof: chunk → extract (mock LLM + JSON parser) → merge → graph.
//!
//! Proves the refactored pipeline path end-to-end without external LLM:
//! - Chunker honors strategy (default token chunking)
//! - LLMExtractor + JsonExtractionParser + normalize_entity_name (DRY-001/003/005)
//! - KnowledgeGraphMerger stores normalized graph keys (SOLID-L-002)

use std::sync::Arc;

use edgequake_llm::MockProvider;
use edgequake_pipeline::{
    chunker::{Chunker, ChunkerConfig},
    extractor::{EntityExtractor, LLMExtractor},
    merger::normalize_entity_name,
    prompts::JsonExtractionParser,
    KnowledgeGraphMerger, MergerConfig, Pipeline, PipelineConfig,
};
use edgequake_storage::{
    GraphStorage, GraphStorageReadOps, MemoryGraphStorage, MemoryVectorStorage, VectorStorage,
};

const SPEC017_DOC: &str = r#"
EdgeQuake is a high-performance RAG system built in Rust.
Sarah Chen designed EdgeQuake as lead architect.
EdgeQuake uses Apache AGE for graph storage in production deployments.
"#;

const SPEC017_EXTRACTION_JSON: &str = r#"{
  "entities": [
    {"name": "EdgeQuake", "type": "TECHNOLOGY", "description": "RAG system in Rust"},
    {"name": "Sarah Chen", "type": "PERSON", "description": "Lead architect"},
    {"name": "Apache AGE", "type": "TECHNOLOGY", "description": "Graph extension for PostgreSQL"}
  ],
  "relationships": [
    {"source": "Sarah Chen", "target": "EdgeQuake", "type": "DESIGNED", "description": "Sarah designed EdgeQuake"},
    {"source": "EdgeQuake", "target": "Apache AGE", "type": "USES", "description": "Uses AGE for graphs"}
  ]
}"#;

async fn mock_extractor_with_responses(count: usize) -> Arc<dyn EntityExtractor> {
    let mock = Arc::new(MockProvider::new());
    for _ in 0..count {
        mock.add_response(SPEC017_EXTRACTION_JSON).await;
    }
    Arc::new(LLMExtractor::new(mock))
}

#[tokio::test]
async fn spec017_full_pipeline_chunk_extract_merge_graph() {
    let chunker = Chunker::new(ChunkerConfig {
        chunk_size: 400,
        chunk_overlap: 50,
        min_chunk_size: 20,
        ..ChunkerConfig::default()
    });
    let chunks = chunker.chunk(SPEC017_DOC, "spec017-doc").expect("chunk");
    assert!(!chunks.is_empty(), "chunk stage must produce chunks");

    let extractor = mock_extractor_with_responses(chunks.len().max(4)).await;

    let config = PipelineConfig {
        enable_entity_extraction: true,
        enable_relationship_extraction: true,
        enable_chunk_embeddings: false,
        enable_entity_embeddings: false,
        chunker: ChunkerConfig {
            chunk_size: 400,
            chunk_overlap: 50,
            min_chunk_size: 20,
            ..ChunkerConfig::default()
        },
        ..Default::default()
    };

    let pipeline = Pipeline::new(config).with_extractor(extractor);

    let result = pipeline
        .process("spec017-doc", SPEC017_DOC)
        .await
        .expect("pipeline process");

    assert!(result.stats.chunk_count > 0, "stats must record chunks");
    assert!(
        !result.extractions.is_empty(),
        "extract stage must produce extractions"
    );
    assert!(
        result.stats.entity_count > 0,
        "entity_count must be > 0 after mock extraction"
    );

    // Parser + normalizer alignment on first extraction
    let first = &result.extractions[0];
    assert!(
        first.entities.iter().any(|e| e.name == "EDGEQUAKE"),
        "parser must normalize EdgeQuake → EDGEQUAKE"
    );

    let graph = Arc::new(MemoryGraphStorage::new("spec017-full"));
    let vector = Arc::new(MemoryVectorStorage::new("spec017-full", 384));
    graph.initialize().await.unwrap();
    vector.initialize().await.unwrap();

    let merger = KnowledgeGraphMerger::new(MergerConfig::default(), graph.clone(), vector);
    let merge_stats = merger
        .merge(result.extractions.clone())
        .await
        .expect("merge");

    assert!(
        merge_stats.entities_created >= 1,
        "merge must create entities"
    );
    assert!(
        merge_stats.relationships_created >= 1,
        "merge must create relationships"
    );

    // Graph keys must match normalizer (SOLID-L-002)
    assert!(
        graph.get_node("EDGEQUAKE").await.unwrap().is_some(),
        "graph node EDGEQUAKE"
    );
    assert!(
        graph.get_node("SARAH_CHEN").await.unwrap().is_some(),
        "graph node SARAH_CHEN"
    );
    assert!(
        graph
            .get_edge("SARAH_CHEN", "EDGEQUAKE")
            .await
            .unwrap()
            .is_some(),
        "relationship SARAH_CHEN → EDGEQUAKE"
    );

    // Cross-check parser output keys == merger keys
    let parser = JsonExtractionParser::new();
    let parsed = parser
        .parse(SPEC017_EXTRACTION_JSON, "chunk-0")
        .expect("parse");
    for entity in &parsed.entities {
        assert_eq!(
            normalize_entity_name(&entity.name),
            entity.name,
            "parsed names already normalized"
        );
    }
}

#[tokio::test]
async fn spec017_full_pipeline_resilience_path() {
    let mock = Arc::new(MockProvider::new());
    // First chunk succeeds; second returns invalid JSON (filtered to empty)
    mock.add_response(SPEC017_EXTRACTION_JSON).await;
    mock.add_response("not valid json at all").await;
    for _ in 0..4 {
        mock.add_response(SPEC017_EXTRACTION_JSON).await;
    }

    let extractor: Arc<dyn EntityExtractor> = Arc::new(LLMExtractor::new(mock));
    let pipeline = Pipeline::new(PipelineConfig {
        enable_entity_extraction: true,
        enable_relationship_extraction: true,
        enable_chunk_embeddings: false,
        enable_entity_embeddings: false,
        chunker: ChunkerConfig {
            chunk_size: 200,
            chunk_overlap: 30,
            min_chunk_size: 10,
            ..ChunkerConfig::default()
        },
        ..Default::default()
    })
    .with_extractor(extractor);

    let result = pipeline
        .process_with_resilience("spec017-resilience", SPEC017_DOC, None)
        .await
        .expect("resilience process");

    assert!(result.stats.chunk_count > 0);
    assert!(
        result.stats.successful_chunks >= 1,
        "at least one chunk must succeed"
    );
    assert!(
        !result.extractions.is_empty(),
        "partial extractions must be retained"
    );
}
