//! Extraction domain types (entities, relationships, results).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
