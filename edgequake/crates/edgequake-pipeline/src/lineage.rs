//! Lineage Tracking.
//!
//! Provides comprehensive lineage tracking from documents through chunks
//! to extracted entities and relationships. Enables full auditability
//! and traceability of the extraction pipeline.
//!
//! ## Implements
//!
//! @implements FEAT0011 (Document-Chunk-Entity Lineage tracking)
//! @implements FEAT0019 (Source span tracking with line numbers)
//! @implements FEAT0020 (Description history for entity evolution)
//!
//! ## Use Cases
//!
//! - **UC2310**: User traces entity back to source document line
//! - **UC2311**: System tracks description evolution through merges
//! - **UC2312**: User views full extraction metadata
//!
//! ## Enforces
//!
//! - **BR0019**: Source spans must include line numbers
//! - **BR0020**: Description history must be append-only
//!
//! # Lineage Chain
//!
//! Document → Chunks → Entities/Relationships
//!
//! Each entity tracks all source documents and chunks it was extracted from,
//! along with description history showing how it evolved through merges.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Source location within a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSpan {
    /// Start line number (1-indexed).
    pub start_line: usize,
    /// End line number (1-indexed).
    pub end_line: usize,
    /// Start character offset.
    pub start_offset: usize,
    /// End character offset.
    pub end_offset: usize,
}

impl SourceSpan {
    /// Create a new source span.
    pub fn new(start_line: usize, end_line: usize, start_offset: usize, end_offset: usize) -> Self {
        Self {
            start_line,
            end_line,
            start_offset,
            end_offset,
        }
    }
}

/// Metadata about an extraction operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractionMetadata {
    /// LLM model used.
    pub llm_model: String,
    /// Number of gleaning iterations.
    pub gleaning_iterations: usize,
    /// Time taken for extraction (ms).
    pub extraction_time_ms: u64,
    /// Input tokens used.
    pub input_tokens: usize,
    /// Output tokens generated.
    pub output_tokens: usize,
    /// Whether cache was hit.
    pub cache_hit: bool,
    /// Cache entry ID if cached.
    pub cache_id: Option<String>,
}

impl ExtractionMetadata {
    /// Create new extraction metadata.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            llm_model: model.into(),
            ..Default::default()
        }
    }

    /// Set token usage.
    pub fn with_tokens(mut self, input: usize, output: usize) -> Self {
        self.input_tokens = input;
        self.output_tokens = output;
        self
    }

    /// Set timing.
    pub fn with_time(mut self, ms: u64) -> Self {
        self.extraction_time_ms = ms;
        self
    }

    /// Set cache info.
    pub fn with_cache(mut self, hit: bool, cache_id: Option<String>) -> Self {
        self.cache_hit = hit;
        self.cache_id = cache_id;
        self
    }
}

/// Lineage information for a single chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkLineage {
    /// Chunk ID.
    pub chunk_id: String,
    /// Index in document.
    pub chunk_index: usize,
    /// Start line in document.
    pub start_line: usize,
    /// End line in document.
    pub end_line: usize,
    /// Start offset in document.
    pub start_offset: usize,
    /// End offset in document.
    pub end_offset: usize,
    /// Entity IDs extracted from this chunk.
    pub entity_ids: Vec<String>,
    /// Relationship IDs extracted from this chunk.
    pub relationship_ids: Vec<String>,
    /// Extraction metadata.
    pub extraction_metadata: ExtractionMetadata,
}

impl ChunkLineage {
    /// Create new chunk lineage.
    pub fn new(chunk_id: impl Into<String>, chunk_index: usize) -> Self {
        Self {
            chunk_id: chunk_id.into(),
            chunk_index,
            start_line: 0,
            end_line: 0,
            start_offset: 0,
            end_offset: 0,
            entity_ids: Vec::new(),
            relationship_ids: Vec::new(),
            extraction_metadata: ExtractionMetadata::default(),
        }
    }

    /// Set line numbers.
    pub fn with_lines(mut self, start: usize, end: usize) -> Self {
        self.start_line = start;
        self.end_line = end;
        self
    }

    /// Set offsets.
    pub fn with_offsets(mut self, start: usize, end: usize) -> Self {
        self.start_offset = start;
        self.end_offset = end;
        self
    }

    /// Add entity ID.
    pub fn add_entity(&mut self, entity_id: impl Into<String>) {
        self.entity_ids.push(entity_id.into());
    }

    /// Add relationship ID.
    pub fn add_relationship(&mut self, rel_id: impl Into<String>) {
        self.relationship_ids.push(rel_id.into());
    }
}

/// Source document for an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySource {
    /// Document ID.
    pub document_id: String,
    /// Document filename.
    pub document_name: String,
    /// Chunk IDs where entity was found.
    pub chunk_ids: Vec<String>,
    /// Source spans in document.
    pub source_spans: Vec<SourceSpan>,
    /// When entity was extracted.
    pub extracted_at: DateTime<Utc>,
}

impl EntitySource {
    /// Create new entity source.
    pub fn new(document_id: impl Into<String>, document_name: impl Into<String>) -> Self {
        Self {
            document_id: document_id.into(),
            document_name: document_name.into(),
            chunk_ids: Vec::new(),
            source_spans: Vec::new(),
            extracted_at: Utc::now(),
        }
    }

    /// Add a chunk ID.
    pub fn add_chunk(&mut self, chunk_id: impl Into<String>, span: SourceSpan) {
        self.chunk_ids.push(chunk_id.into());
        self.source_spans.push(span);
    }
}

/// A version of an entity's description.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescriptionVersion {
    /// The description text.
    pub description: String,
    /// Source of this version (extraction, merge, summary).
    pub source: String,
    /// When this version was created.
    pub created_at: DateTime<Utc>,
}

impl DescriptionVersion {
    /// Create from extraction.
    pub fn from_extraction(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            source: "extraction".to_string(),
            created_at: Utc::now(),
        }
    }

    /// Create from merge.
    pub fn from_merge(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            source: "merge".to_string(),
            created_at: Utc::now(),
        }
    }

    /// Create from summary.
    pub fn from_summary(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            source: "summary".to_string(),
            created_at: Utc::now(),
        }
    }
}

/// Complete lineage for an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityLineage {
    /// Entity ID.
    pub entity_id: String,
    /// Entity name.
    pub entity_name: String,
    /// All source documents.
    pub sources: Vec<EntitySource>,
    /// Total extraction count.
    pub extraction_count: usize,
    /// Description history.
    pub description_history: Vec<DescriptionVersion>,
}

impl EntityLineage {
    /// Create new entity lineage.
    pub fn new(entity_id: impl Into<String>, entity_name: impl Into<String>) -> Self {
        Self {
            entity_id: entity_id.into(),
            entity_name: entity_name.into(),
            sources: Vec::new(),
            extraction_count: 0,
            description_history: Vec::new(),
        }
    }

    /// Add a source.
    pub fn add_source(&mut self, source: EntitySource) {
        self.extraction_count += source.chunk_ids.len();
        self.sources.push(source);
    }

    /// Add description version.
    pub fn add_description(&mut self, version: DescriptionVersion) {
        self.description_history.push(version);
    }

    /// Get current description.
    pub fn current_description(&self) -> Option<&str> {
        self.description_history
            .last()
            .map(|v| v.description.as_str())
    }
}

/// Lineage for a relationship.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipLineage {
    /// Relationship ID.
    pub relationship_id: String,
    /// Source entity name.
    pub source_entity: String,
    /// Target entity name.
    pub target_entity: String,
    /// Relationship type.
    pub relationship_type: String,
    /// Source documents.
    pub sources: Vec<EntitySource>,
    /// Extraction count.
    pub extraction_count: usize,
    /// Description history.
    pub description_history: Vec<DescriptionVersion>,
}

impl RelationshipLineage {
    /// Create new relationship lineage.
    pub fn new(
        relationship_id: impl Into<String>,
        source: impl Into<String>,
        target: impl Into<String>,
        rel_type: impl Into<String>,
    ) -> Self {
        Self {
            relationship_id: relationship_id.into(),
            source_entity: source.into(),
            target_entity: target.into(),
            relationship_type: rel_type.into(),
            sources: Vec::new(),
            extraction_count: 0,
            description_history: Vec::new(),
        }
    }
}

/// Complete lineage for a document ingestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentLineage {
    /// Document ID.
    pub document_id: String,
    /// Document filename.
    pub document_name: String,
    /// Job ID that created this lineage.
    pub job_id: String,
    /// All chunks from this document.
    pub chunks: Vec<ChunkLineage>,
    /// Entity lineage map (entity_id -> lineage).
    pub entities: HashMap<String, EntityLineage>,
    /// Relationship lineage map (rel_id -> lineage).
    pub relationships: HashMap<String, RelationshipLineage>,
    /// Total chunks.
    pub total_chunks: usize,
    /// Total unique entities.
    pub total_entities: usize,
    /// Total unique relationships.
    pub total_relationships: usize,
    /// When created.
    pub created_at: DateTime<Utc>,
    /// When last updated.
    pub updated_at: DateTime<Utc>,

    // === SPEC-032: Provider Lineage Tracking ===
    /// LLM provider used for entity extraction (e.g., "openai", "ollama").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_provider: Option<String>,
    /// LLM model used for entity extraction (e.g., "gpt-4.1-nano").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_model: Option<String>,
    /// Embedding provider used (e.g., "openai", "ollama").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_provider: Option<String>,
    /// Embedding model used (e.g., "text-embedding-3-small").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
    /// Embedding dimension used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_dimension: Option<usize>,
}

impl DocumentLineage {
    /// Create new document lineage.
    pub fn new(
        document_id: impl Into<String>,
        document_name: impl Into<String>,
        job_id: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            document_id: document_id.into(),
            document_name: document_name.into(),
            job_id: job_id.into(),
            chunks: Vec::new(),
            entities: HashMap::new(),
            relationships: HashMap::new(),
            total_chunks: 0,
            total_entities: 0,
            total_relationships: 0,
            created_at: now,
            updated_at: now,
            // SPEC-032: Initialize provider lineage as None
            extraction_provider: None,
            extraction_model: None,
            embedding_provider: None,
            embedding_model: None,
            embedding_dimension: None,
        }
    }

    /// SPEC-032: Set provider lineage information.
    ///
    /// This tracks which LLM and embedding providers were used to process
    /// this document, enabling lineage tracking and reproducibility.
    ///
    /// # Arguments
    ///
    /// * `extraction_provider` - The LLM provider (e.g., "openai", "ollama")
    /// * `extraction_model` - The LLM model (e.g., "gpt-4.1-nano")
    /// * `embedding_provider` - The embedding provider
    /// * `embedding_model` - The embedding model
    /// * `embedding_dimension` - The embedding vector dimension
    pub fn set_provider_lineage(
        &mut self,
        extraction_provider: impl Into<String>,
        extraction_model: impl Into<String>,
        embedding_provider: impl Into<String>,
        embedding_model: impl Into<String>,
        embedding_dimension: usize,
    ) {
        self.extraction_provider = Some(extraction_provider.into());
        self.extraction_model = Some(extraction_model.into());
        self.embedding_provider = Some(embedding_provider.into());
        self.embedding_model = Some(embedding_model.into());
        self.embedding_dimension = Some(embedding_dimension);
        self.updated_at = Utc::now();
    }

    /// Add a chunk.
    pub fn add_chunk(&mut self, chunk: ChunkLineage) {
        self.total_chunks += 1;
        self.chunks.push(chunk);
        self.updated_at = Utc::now();
    }

    /// Add or update entity lineage.
    pub fn add_entity(&mut self, entity_id: &str, lineage: EntityLineage) {
        if !self.entities.contains_key(entity_id) {
            self.total_entities += 1;
        }
        self.entities.insert(entity_id.to_string(), lineage);
        self.updated_at = Utc::now();
    }

    /// Add or update relationship lineage.
    pub fn add_relationship(&mut self, rel_id: &str, lineage: RelationshipLineage) {
        if !self.relationships.contains_key(rel_id) {
            self.total_relationships += 1;
        }
        self.relationships.insert(rel_id.to_string(), lineage);
        self.updated_at = Utc::now();
    }

    /// Get entity by ID.
    pub fn get_entity(&self, entity_id: &str) -> Option<&EntityLineage> {
        self.entities.get(entity_id)
    }

    /// Get chunks containing an entity.
    pub fn chunks_for_entity(&self, entity_id: &str) -> Vec<&ChunkLineage> {
        self.chunks
            .iter()
            .filter(|c| c.entity_ids.contains(&entity_id.to_string()))
            .collect()
    }
}

/// Builder for constructing document lineage during ingestion.
#[derive(Debug)]
pub struct LineageBuilder {
    lineage: DocumentLineage,
}

impl LineageBuilder {
    /// Create a new lineage builder.
    pub fn new(
        document_id: impl Into<String>,
        document_name: impl Into<String>,
        job_id: impl Into<String>,
    ) -> Self {
        Self {
            lineage: DocumentLineage::new(document_id, document_name, job_id),
        }
    }

    /// Record a chunk extraction.
    ///
    /// Takes all chunk position and metadata parameters directly for performance.
    /// Consider using a ChunkRecordParams struct if more parameters are needed.
    #[allow(clippy::too_many_arguments)]
    pub fn record_chunk(
        &mut self,
        chunk_id: &str,
        chunk_index: usize,
        start_line: usize,
        end_line: usize,
        start_offset: usize,
        end_offset: usize,
        metadata: ExtractionMetadata,
    ) {
        let chunk = ChunkLineage {
            chunk_id: chunk_id.to_string(),
            chunk_index,
            start_line,
            end_line,
            start_offset,
            end_offset,
            entity_ids: Vec::new(),
            relationship_ids: Vec::new(),
            extraction_metadata: metadata,
        };
        self.lineage.add_chunk(chunk);
    }

    /// Record an entity extraction.
    pub fn record_entity(
        &mut self,
        entity_id: &str,
        entity_name: &str,
        chunk_id: &str,
        span: SourceSpan,
        description: &str,
    ) {
        // Update chunk with entity ID
        if let Some(chunk) = self
            .lineage
            .chunks
            .iter_mut()
            .find(|c| c.chunk_id == chunk_id)
        {
            if !chunk.entity_ids.contains(&entity_id.to_string()) {
                chunk.entity_ids.push(entity_id.to_string());
            }
        }

        // Update or create entity lineage
        let lineage = self
            .lineage
            .entities
            .entry(entity_id.to_string())
            .or_insert_with(|| EntityLineage::new(entity_id, entity_name));

        // Add source
        let mut source = EntitySource::new(&self.lineage.document_id, &self.lineage.document_name);
        source.add_chunk(chunk_id, span);
        lineage.add_source(source);

        // Add description version
        if lineage.description_history.is_empty()
            || lineage.current_description() != Some(description)
        {
            lineage.add_description(DescriptionVersion::from_extraction(description));
        }

        self.lineage.total_entities = self.lineage.entities.len();
    }

    /// Record a relationship extraction.
    ///
    /// Takes all relationship metadata parameters directly for performance.
    #[allow(clippy::too_many_arguments)]
    pub fn record_relationship(
        &mut self,
        rel_id: &str,
        source_entity: &str,
        target_entity: &str,
        rel_type: &str,
        chunk_id: &str,
        span: SourceSpan,
        description: &str,
    ) {
        // Update chunk with relationship ID
        if let Some(chunk) = self
            .lineage
            .chunks
            .iter_mut()
            .find(|c| c.chunk_id == chunk_id)
        {
            if !chunk.relationship_ids.contains(&rel_id.to_string()) {
                chunk.relationship_ids.push(rel_id.to_string());
            }
        }

        // Update or create relationship lineage
        let lineage = self
            .lineage
            .relationships
            .entry(rel_id.to_string())
            .or_insert_with(|| {
                RelationshipLineage::new(rel_id, source_entity, target_entity, rel_type)
            });

        // Add source
        let mut source = EntitySource::new(&self.lineage.document_id, &self.lineage.document_name);
        source.add_chunk(chunk_id, span);
        lineage.sources.push(source);
        lineage.extraction_count += 1;

        // Add description version
        if lineage.description_history.is_empty() {
            lineage
                .description_history
                .push(DescriptionVersion::from_extraction(description));
        }

        self.lineage.total_relationships = self.lineage.relationships.len();
    }

    /// Build the final lineage.
    pub fn build(mut self) -> DocumentLineage {
        self.lineage.updated_at = Utc::now();
        self.lineage
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_span() {
        let span = SourceSpan::new(10, 20, 100, 200);
        assert_eq!(span.start_line, 10);
        assert_eq!(span.end_line, 20);
    }

    #[test]
    fn test_extraction_metadata() {
        let meta = ExtractionMetadata::new("gpt-4.1-nano")
            .with_tokens(1000, 500)
            .with_time(150)
            .with_cache(true, Some("cache-123".to_string()));

        assert_eq!(meta.llm_model, "gpt-4.1-nano");
        assert_eq!(meta.input_tokens, 1000);
        assert!(meta.cache_hit);
    }

    #[test]
    fn test_chunk_lineage() {
        let mut chunk = ChunkLineage::new("chunk-1", 0)
            .with_lines(1, 10)
            .with_offsets(0, 500);

        chunk.add_entity("entity-1");
        chunk.add_relationship("rel-1");

        assert_eq!(chunk.chunk_id, "chunk-1");
        assert_eq!(chunk.entity_ids.len(), 1);
        assert_eq!(chunk.relationship_ids.len(), 1);
    }

    #[test]
    fn test_entity_lineage() {
        let mut lineage = EntityLineage::new("entity-1", "ALICE");

        let source = EntitySource::new("doc-1", "test.txt");
        lineage.add_source(source);
        lineage.add_description(DescriptionVersion::from_extraction("Alice is a person"));

        assert_eq!(lineage.sources.len(), 1);
        assert_eq!(lineage.current_description(), Some("Alice is a person"));
    }

    #[test]
    fn test_document_lineage() {
        let mut lineage = DocumentLineage::new("doc-1", "test.txt", "job-1");

        let chunk = ChunkLineage::new("chunk-1", 0);
        lineage.add_chunk(chunk);

        let entity = EntityLineage::new("entity-1", "ALICE");
        lineage.add_entity("entity-1", entity);

        assert_eq!(lineage.total_chunks, 1);
        assert_eq!(lineage.total_entities, 1);
    }

    #[test]
    fn test_lineage_builder() {
        let mut builder = LineageBuilder::new("doc-1", "test.txt", "job-1");

        builder.record_chunk(
            "chunk-1",
            0,
            1,
            10,
            0,
            500,
            ExtractionMetadata::new("gpt-4.1-nano"),
        );

        builder.record_entity(
            "entity-1",
            "ALICE",
            "chunk-1",
            SourceSpan::new(1, 5, 0, 200),
            "Alice is a researcher",
        );

        let lineage = builder.build();

        assert_eq!(lineage.total_chunks, 1);
        assert_eq!(lineage.total_entities, 1);

        let entity = lineage.get_entity("entity-1").unwrap();
        assert_eq!(entity.entity_name, "ALICE");

        let chunks = lineage.chunks_for_entity("entity-1");
        assert_eq!(chunks.len(), 1);
    }
}
