//! GitHub #276 E2E — strict entity types enforced through gleaning (re-extraction).
//!
//! Regression: with Strict limit on, gleaning bypassed workspace entity-type schema and
//! leaked types such as `Library`, `Concept`, `Unknown` into the graph (v0.13.3+).
//!
//! ```bash
//! cargo test -p edgequake-pipeline --test e2e_issue276_gleaning_strict
//! ```

use std::collections::HashSet;
use std::sync::Arc;

use edgequake_llm::MockProvider;
use edgequake_pipeline::prompts::EntityExtractionSchema;
use edgequake_pipeline::{
    build_ingestion_pipeline, IngestionPipelineOptions, Pipeline, PipelineConfig,
};

const ISSUE_276_DOC: &str = r#"
EdgeQuake exposes a REST API for knowledge-graph ingestion.
The pipeline uses vector storage and graph merge stages.
"#;

/// Primary extraction pass — types already on the workspace allow-list.
const BASE_EXTRACTION_JSON: &str = r#"{
  "entities": [
    {"name": "REST API", "type": "API_OR_INTERFACE", "description": "Public HTTP API"}
  ],
  "relationships": []
}"#;

/// Gleaning pass — reproduces unauthorized types reported in GitHub #276.
const GLEANING_EXTRACTION_JSON: &str = r#"{
  "entities": [
    {"name": "Redis Client", "type": "Library", "description": "Client library"},
    {"name": "Vector Index", "type": "Concept", "description": "Abstract concept"},
    {"name": "Mystery Node", "type": "Unknown", "description": "Unknown category"},
    {"name": "Meta Label", "type": "Entity Type", "description": "Meta type"},
    {"name": "Hash Map", "type": "Data Structure", "description": "Data structure"},
    {"name": "Copilot", "type": "Ai Tool", "description": "AI assistant tool"}
  ],
  "relationships": []
}"#;

fn issue_276_strict_schema() -> EntityExtractionSchema {
    EntityExtractionSchema {
        types: vec![
            "API_OR_INTERFACE".into(),
            "CODE_ELEMENT".into(),
            "SOFTWARE_COMPONENT".into(),
            "OTHER".into(),
        ],
        strict: true,
    }
}

async fn queue_base_then_gleaning(mock: &MockProvider, rounds: usize) {
    for _ in 0..rounds {
        mock.add_response(BASE_EXTRACTION_JSON).await;
        mock.add_response(GLEANING_EXTRACTION_JSON).await;
    }
}

#[tokio::test]
async fn issue_276_gleaning_strict_entity_types_e2e() {
    let schema = issue_276_strict_schema();
    let allowed: HashSet<String> = schema.types.iter().cloned().collect();

    let mock = Arc::new(MockProvider::new());
    // Each chunk: base LLMExtractor call + one gleaning iteration.
    queue_base_then_gleaning(mock.as_ref(), 8).await;

    let embedding =
        Arc::new(MockProvider::new()) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>;

    let built = build_ingestion_pipeline(
        mock.clone(),
        embedding,
        schema,
        IngestionPipelineOptions::from_document_size(500).with_gleaning(true, 1),
    );

    let extractor = built.extractor().expect("ingestion pipeline extractor");
    let pipeline = Pipeline::new(PipelineConfig {
        enable_entity_extraction: true,
        enable_relationship_extraction: true,
        enable_chunk_embeddings: false,
        enable_entity_embeddings: false,
        chunker: built.config().chunker.clone(),
        chunk_strategy: built.config().chunk_strategy,
        ..Default::default()
    })
    .with_extractor(extractor);

    let result = pipeline
        .process("issue-276-doc", ISSUE_276_DOC)
        .await
        .expect("pipeline process");

    assert!(
        !result.extractions.is_empty(),
        "expected at least one extraction after base + gleaning passes"
    );

    let mut seen_types = HashSet::new();
    for extraction in &result.extractions {
        for entity in &extraction.entities {
            seen_types.insert(entity.entity_type.clone());
            assert!(
                allowed.contains(&entity.entity_type),
                "entity '{}' has unauthorized type '{}' (strict allow-list: {:?})",
                entity.name,
                entity.entity_type,
                allowed
            );
        }
    }

    // Gleaning must have contributed entities that were remapped to OTHER.
    assert!(
        seen_types.contains("OTHER"),
        "gleaning entities with Library/Concept/Unknown must remap to OTHER, got {:?}",
        seen_types
    );
    assert!(
        seen_types.contains("API_OR_INTERFACE"),
        "allowed types from base pass must be preserved, got {:?}",
        seen_types
    );

    // Gleaning iterations recorded when gleaning wrapper ran.
    let gleaning_ran = result.extractions.iter().any(|ex| {
        ex.metadata
            .get("gleaning_iterations")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            >= 1
    });
    assert!(
        gleaning_ran,
        "gleaning wrapper must run with max_gleaning=1"
    );
}
