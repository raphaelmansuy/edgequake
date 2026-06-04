//! Entity and relationship extraction via LLM.
//!
//! @implements FEAT0003
//! @implements FEAT0004
//! @implements FEAT0304
//!
//! # Implements
//!
//! - **FEAT0003**: Entity Extraction
//! - **FEAT0004**: Relationship Extraction
//! - **FEAT0304**: Gleaning (iterative re-extraction for completeness)
//!
//! # Enforces
//!
//! - **BR0003**: Entity types from configurable list
//! - **BR0004**: Relationship keywords max 5 per edge
//! - **BR0005**: Entity description max 512 tokens
//! - **BR0006**: Same-entity relationships forbidden
//! - **BR0008**: Entity names normalized (UPPERCASE_UNDERSCORE)
//!
//! # Extraction Strategies
//!
//! | Strategy | Description | Use Case |
//! |----------|-------------|----------|
//! | [`SOTAExtractor`] | Tuple-based parsing | Production (robust) |
//! | [`SimpleExtractor`] | JSON-based parsing | Development/testing |
//! | [`GleaningExtractor`] | Iterative re-extraction | High-stakes domains |

use async_trait::async_trait;

use crate::chunker::TextChunk;
use crate::error::Result;

mod completion_options;
mod gleaning;
mod llm;
mod schema;
mod simple;
mod sota;
mod temperature;
mod types;

pub use completion_options::{
    assign_token_usage, extraction_completion_options, recommended_chunk_size_for_bytes,
};
pub use schema::ConfigurableEntitySchema;
pub use temperature::effective_temperature_for_model;
pub use types::{ExtractedEntity, ExtractedRelationship, ExtractionResult};

pub use gleaning::{GleaningConfig, GleaningExtractor};
pub use llm::LLMExtractor;
pub use simple::SimpleExtractor;
pub use sota::SOTAExtractor;

/// Trait for entity extraction implementations.
#[async_trait]
/// @implements FEAT0009
pub trait EntityExtractor: Send + Sync {
    /// Extract entities and relationships from a text chunk.
    async fn extract(&self, chunk: &TextChunk) -> Result<ExtractionResult>;

    /// Extract from multiple chunks in batch.
    async fn extract_batch(&self, chunks: &[TextChunk]) -> Result<Vec<ExtractionResult>> {
        let mut results = Vec::with_capacity(chunks.len());
        for chunk in chunks {
            results.push(self.extract(chunk).await?);
        }
        Ok(results)
    }

    /// Get the name of this extractor.
    fn name(&self) -> &str;

    /// Get the model name used by this extractor (if applicable).
    fn model_name(&self) -> &str {
        "unknown"
    }

    /// Get the provider name used by this extractor (if applicable).
    ///
    /// @implements SPEC-032/OODA-226: Provider tracking in ProcessingStats
    fn provider_name(&self) -> &str {
        "unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extracted_entity_builder() {
        let entity = ExtractedEntity::new("John Doe", "PERSON", "A person named John")
            .with_importance(0.8)
            .with_source_span("John Doe is a developer");

        assert_eq!(entity.name, "John Doe");
        assert_eq!(entity.entity_type, "PERSON");
        assert_eq!(entity.importance, 0.8);
        assert_eq!(entity.source_spans.len(), 1);
    }

    #[test]
    fn test_extracted_entity_source_tracking() {
        let entity = ExtractedEntity::new("Sarah Chen", "PERSON", "Lead researcher")
            .with_source_chunk_id("chunk-001")
            .with_source_document_id("doc-abc123")
            .with_source_file_path("/documents/research.pdf");

        assert_eq!(entity.source_chunk_ids.len(), 1);
        assert_eq!(entity.source_chunk_ids[0], "chunk-001");
        assert_eq!(entity.source_document_id, Some("doc-abc123".to_string()));
        assert_eq!(
            entity.source_file_path,
            Some("/documents/research.pdf".to_string())
        );
    }

    #[test]
    fn test_extracted_entity_multiple_source_chunks() {
        let mut entity = ExtractedEntity::new("ACME Corp", "ORGANIZATION", "A company")
            .with_source_chunk_id("chunk-001")
            .with_source_document_id("doc-abc123");

        entity.add_source_chunk_id("chunk-002");
        entity.add_source_chunk_id("chunk-003");

        assert_eq!(entity.source_chunk_ids.len(), 3);
        assert!(entity.source_chunk_ids.contains(&"chunk-001".to_string()));
        assert!(entity.source_chunk_ids.contains(&"chunk-002".to_string()));
        assert!(entity.source_chunk_ids.contains(&"chunk-003".to_string()));
    }

    #[test]
    fn test_extracted_relationship_source_tracking() {
        let rel = ExtractedRelationship::new("Alice", "Bob", "KNOWS")
            .with_description("Alice knows Bob from work")
            .with_source_chunk_id("chunk-005")
            .with_source_document_id("doc-xyz789")
            .with_source_file_path("/documents/team.md");

        assert_eq!(rel.source_chunk_id, Some("chunk-005".to_string()));
        assert_eq!(rel.source_document_id, Some("doc-xyz789".to_string()));
        assert_eq!(rel.source_file_path, Some("/documents/team.md".to_string()));
    }

    #[test]
    fn test_effective_temperature_omits_override_for_reasoning_models() {
        assert_eq!(effective_temperature_for_model("gpt-5-nano", 0.0), None);
        assert_eq!(effective_temperature_for_model("gpt-5-mini", 0.0), None);
        assert_eq!(effective_temperature_for_model("o4-mini", 0.0), None);
    }

    #[test]
    fn test_effective_temperature_preserves_standard_models() {
        assert_eq!(effective_temperature_for_model("gpt-4o", 0.3), Some(0.3));
        assert_eq!(
            effective_temperature_for_model("claude-sonnet", 0.0),
            Some(0.0)
        );
    }

    #[test]
    fn test_extracted_relationship_builder() {
        let rel = ExtractedRelationship::new("Alice", "Bob", "KNOWS")
            .with_description("Alice knows Bob from work")
            .with_weight(0.9)
            .with_keywords(vec!["colleague".to_string(), "friend".to_string()]);

        assert_eq!(rel.source, "Alice");
        assert_eq!(rel.target, "Bob");
        assert_eq!(rel.weight, 0.9);
        assert_eq!(rel.keywords.len(), 2);
    }

    #[tokio::test]
    async fn test_simple_extractor() {
        let extractor = SimpleExtractor::new().unwrap();
        let chunk = TextChunk::new("chunk-1", "John Doe works at Acme Corp.", 0, 0, 30);

        let result = extractor.extract(&chunk).await.unwrap();

        assert!(result.entities.iter().any(|e| e.name == "John Doe"));
    }

    #[test]
    fn test_extraction_result() {
        let mut result = ExtractionResult::new("chunk-1");

        result.add_entity(ExtractedEntity::new("Test", "CONCEPT", "A test"));
        result.add_relationship(ExtractedRelationship::new("A", "B", "RELATED_TO"));

        assert_eq!(result.entities.len(), 1);
        assert_eq!(result.relationships.len(), 1);
    }
}
