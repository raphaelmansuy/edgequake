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
//! # WHY: LLM-Based Extraction
//!
//! Using LLMs for extraction provides:
//! 1. Domain-agnostic entity recognition (no training required)
//! 2. Rich semantic descriptions (not just labels)
//! 3. Relationship inference beyond co-occurrence
//!
//! # Extraction Strategies
//!
//! | Strategy | Description | Use Case |
//! |----------|-------------|----------|
//! | [`SOTAExtractor`] | Tuple-based parsing | Production (robust) |
//! | [`SimpleExtractor`] | JSON-based parsing | Development/testing |
//! | [`GleaningExtractor`] | Iterative re-extraction | High-stakes domains |

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::chunker::TextChunk;
use crate::error::Result;

/// Result of entity and relationship extraction.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractionResult {
    /// Extracted entities.
    pub entities: Vec<ExtractedEntity>,

    /// Extracted relationships.
    pub relationships: Vec<ExtractedRelationship>,

    /// Source chunk ID.
    pub source_chunk_id: String,

    /// Processing metadata.
    pub metadata: HashMap<String, serde_json::Value>,

    /// Input tokens used for this extraction.
    pub input_tokens: usize,

    /// Output tokens generated for this extraction.
    pub output_tokens: usize,

    /// Extraction time in milliseconds.
    pub extraction_time_ms: u64,
}

impl ExtractionResult {
    /// Create a new empty extraction result.
    pub fn new(source_chunk_id: impl Into<String>) -> Self {
        Self {
            entities: Vec::new(),
            relationships: Vec::new(),
            source_chunk_id: source_chunk_id.into(),
            metadata: HashMap::new(),
            input_tokens: 0,
            output_tokens: 0,
            extraction_time_ms: 0,
        }
    }

    /// Add an entity.
    pub fn add_entity(&mut self, entity: ExtractedEntity) {
        self.entities.push(entity);
    }

    /// Add a relationship.
    pub fn add_relationship(&mut self, rel: ExtractedRelationship) {
        self.relationships.push(rel);
    }

    /// Set token usage information.
    pub fn with_token_usage(mut self, input_tokens: usize, output_tokens: usize) -> Self {
        self.input_tokens = input_tokens;
        self.output_tokens = output_tokens;
        self
    }

    /// Set extraction timing.
    pub fn with_timing(mut self, extraction_time_ms: u64) -> Self {
        self.extraction_time_ms = extraction_time_ms;
        self
    }
}

/// An extracted entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    /// Entity name (normalized).
    pub name: String,

    /// Entity type (e.g., "PERSON", "ORGANIZATION", "CONCEPT").
    pub entity_type: String,

    /// Description of the entity.
    pub description: String,

    /// Importance score (0.0 to 1.0).
    pub importance: f32,

    /// Source text spans.
    pub source_spans: Vec<String>,

    /// Entity embedding.
    pub embedding: Option<Vec<f32>>,

    /// Source chunk IDs where this entity was mentioned.
    /// Used for citation tracking back to original document chunks.
    #[serde(default)]
    pub source_chunk_ids: Vec<String>,

    /// Source document ID (the document this entity was extracted from).
    #[serde(default)]
    pub source_document_id: Option<String>,

    /// Original file path of the source document.
    #[serde(default)]
    pub source_file_path: Option<String>,
}

impl ExtractedEntity {
    /// Create a new extracted entity.
    pub fn new(
        name: impl Into<String>,
        entity_type: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            entity_type: entity_type.into(),
            description: description.into(),
            importance: 0.5,
            source_spans: Vec::new(),
            embedding: None,
            source_chunk_ids: Vec::new(),
            source_document_id: None,
            source_file_path: None,
        }
    }

    /// Set the importance score.
    pub fn with_importance(mut self, importance: f32) -> Self {
        self.importance = importance.clamp(0.0, 1.0);
        self
    }

    /// Add a source span.
    pub fn with_source_span(mut self, span: impl Into<String>) -> Self {
        self.source_spans.push(span.into());
        self
    }

    /// Add a source chunk ID.
    pub fn with_source_chunk_id(mut self, chunk_id: impl Into<String>) -> Self {
        let id = chunk_id.into();
        if !self.source_chunk_ids.contains(&id) {
            self.source_chunk_ids.push(id);
        }
        self
    }

    /// Set the source document ID.
    pub fn with_source_document_id(mut self, document_id: impl Into<String>) -> Self {
        self.source_document_id = Some(document_id.into());
        self
    }

    /// Set the source file path.
    pub fn with_source_file_path(mut self, file_path: impl Into<String>) -> Self {
        self.source_file_path = Some(file_path.into());
        self
    }

    /// Add source chunk ID (mutable reference version).
    pub fn add_source_chunk_id(&mut self, chunk_id: impl Into<String>) {
        let id = chunk_id.into();
        if !self.source_chunk_ids.contains(&id) {
            self.source_chunk_ids.push(id);
        }
    }
}

/// An extracted relationship between entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedRelationship {
    /// Source entity name.
    pub source: String,

    /// Target entity name.
    pub target: String,

    /// Relationship type/description.
    pub relation_type: String,

    /// Relationship description.
    pub description: String,

    /// Weight/strength (0.0 to 1.0).
    pub weight: f32,

    /// Keywords associated with this relationship.
    pub keywords: Vec<String>,

    /// Relationship embedding (for similarity search).
    /// Computed from: keywords + source + target + description
    pub embedding: Option<Vec<f32>>,

    /// Source chunk ID where this relationship was extracted.
    #[serde(default)]
    pub source_chunk_id: Option<String>,

    /// Source document ID.
    #[serde(default)]
    pub source_document_id: Option<String>,

    /// Original file path of the source document.
    #[serde(default)]
    pub source_file_path: Option<String>,
}

impl ExtractedRelationship {
    /// Create a new extracted relationship.
    pub fn new(
        source: impl Into<String>,
        target: impl Into<String>,
        relation_type: impl Into<String>,
    ) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            relation_type: relation_type.into(),
            description: String::new(),
            weight: 0.5,
            keywords: Vec::new(),
            embedding: None,
            source_chunk_id: None,
            source_document_id: None,
            source_file_path: None,
        }
    }

    /// Set the description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set the weight.
    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight.clamp(0.0, 1.0);
        self
    }

    /// Add keywords.
    pub fn with_keywords(mut self, keywords: Vec<String>) -> Self {
        self.keywords = keywords;
        self
    }

    /// Set the source chunk ID.
    pub fn with_source_chunk_id(mut self, chunk_id: impl Into<String>) -> Self {
        self.source_chunk_id = Some(chunk_id.into());
        self
    }

    /// Set the source document ID.
    pub fn with_source_document_id(mut self, document_id: impl Into<String>) -> Self {
        self.source_document_id = Some(document_id.into());
        self
    }

    /// Set the source file path.
    pub fn with_source_file_path(mut self, file_path: impl Into<String>) -> Self {
        self.source_file_path = Some(file_path.into());
        self
    }
}

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

fn extract_json_from_response(response: &str) -> String {
    let response = response.trim();

    // Try to find JSON block markers
    if let Some(start) = response.find("```json") {
        if let Some(end) = response[start + 7..].find("```") {
            return response[start + 7..start + 7 + end].trim().to_string();
        }
    }

    // Try to find JSON starting with {
    if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            if end > start {
                return response[start..=end].to_string();
            }
        }
    }

    response.to_string()
}

mod gleaning;
mod llm;
mod simple;
mod sota;

pub use gleaning::{GleaningConfig, GleaningExtractor};
pub use llm::LLMExtractor;
pub use simple::SimpleExtractor;
pub use sota::SOTAExtractor;

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

        // Relationship has source_chunk_id as Option<String> (singular)
        assert_eq!(rel.source_chunk_id, Some("chunk-005".to_string()));
        assert_eq!(rel.source_document_id, Some("doc-xyz789".to_string()));
        assert_eq!(rel.source_file_path, Some("/documents/team.md".to_string()));
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

        // Should find "John Doe" as a person
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
