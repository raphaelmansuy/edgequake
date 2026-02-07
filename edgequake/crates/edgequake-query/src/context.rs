//! Query context building and management.
//!
//! ## Implements
//!
//! - **FEAT0116**: Query context data structure
//! - **FEAT0117**: Context token counting
//! - **FEAT0118**: Context truncation tracking
//!
//! ## Use Cases
//!
//! - **UC2230**: System builds context from retrieved items
//! - **UC2231**: System tracks token usage for budget enforcement
//! - **UC2232**: System marks context as truncated when budget exceeded
//!
//! ## Enforces
//!
//! - **BR0115**: Token count must be updated on each addition
//! - **BR0116**: Truncation flag must be set when content removed

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Context assembled for answering a query.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryContext {
    /// Retrieved chunks with their relevance scores.
    pub chunks: Vec<RetrievedChunk>,

    /// Retrieved entities from the knowledge graph.
    pub entities: Vec<RetrievedEntity>,

    /// Retrieved relationships.
    pub relationships: Vec<RetrievedRelationship>,

    /// Total token count of the context.
    pub token_count: usize,

    /// Whether the context was truncated.
    pub is_truncated: bool,

    /// Retrieval metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl QueryContext {
    /// Create a new empty query context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a retrieved chunk.
    pub fn add_chunk(&mut self, chunk: RetrievedChunk) {
        self.token_count += chunk.token_count;
        self.chunks.push(chunk);
    }

    /// Add a retrieved entity.
    pub fn add_entity(&mut self, entity: RetrievedEntity) {
        self.entities.push(entity);
    }

    /// Add a retrieved relationship.
    pub fn add_relationship(&mut self, rel: RetrievedRelationship) {
        self.relationships.push(rel);
    }

    /// Build a text representation for LLM context.
    pub fn to_context_string(&self) -> String {
        let mut parts = Vec::new();

        if !self.entities.is_empty() {
            parts.push("### Knowledge Graph Data (Entities)\n\n".to_string());
            for entity in &self.entities {
                let degree_info = if entity.degree > 0 {
                    format!(" [connections: {}]", entity.degree)
                } else {
                    String::new()
                };
                parts.push(format!(
                    "- **{}** ({}){}: {}\n",
                    entity.name, entity.entity_type, degree_info, entity.description
                ));
            }
            parts.push("\n".to_string());
        }

        if !self.relationships.is_empty() {
            parts.push("### Knowledge Graph Data (Relationships)\n\n".to_string());
            for rel in &self.relationships {
                if rel.description.is_empty() {
                    parts.push(format!(
                        "- {} --[{}]--> {}\n",
                        rel.source, rel.relation_type, rel.target
                    ));
                } else {
                    parts.push(format!(
                        "- {} --[{}]--> {}: {}\n",
                        rel.source, rel.relation_type, rel.target, rel.description
                    ));
                }
            }
            parts.push("\n".to_string());
        }

        if !self.chunks.is_empty() {
            parts.push("### Document Chunks\n\n".to_string());
            for (i, chunk) in self.chunks.iter().enumerate() {
                let ref_id = i + 1;
                parts.push(format!(
                    "[{}] (score: {:.3})\n{}\n\n",
                    ref_id, chunk.score, chunk.content
                ));
            }
        }

        parts.join("")
    }

    /// Check if the context is empty.
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty() && self.entities.is_empty() && self.relationships.is_empty()
    }
}

/// A retrieved text chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedChunk {
    /// Chunk identifier.
    pub id: String,

    /// Chunk content.
    pub content: String,

    /// Relevance score.
    pub score: f32,

    /// Source document ID.
    pub document_id: Option<String>,

    /// Token count.
    pub token_count: usize,

    /// Start line number in the document.
    pub start_line: Option<usize>,

    /// End line number in the document.
    pub end_line: Option<usize>,

    /// Chunk index in the document.
    pub chunk_index: Option<usize>,
}

impl RetrievedChunk {
    /// Create a new retrieved chunk.
    pub fn new(id: impl Into<String>, content: impl Into<String>, score: f32) -> Self {
        let content = content.into();
        let token_count = (content.len() as f32 / 4.0).ceil() as usize;
        Self {
            id: id.into(),
            content,
            score,
            document_id: None,
            token_count,
            start_line: None,
            end_line: None,
            chunk_index: None,
        }
    }

    /// Set the document ID.
    pub fn with_document_id(mut self, doc_id: impl Into<String>) -> Self {
        self.document_id = Some(doc_id.into());
        self
    }

    /// Set line numbers.
    pub fn with_lines(mut self, start: usize, end: usize) -> Self {
        self.start_line = Some(start);
        self.end_line = Some(end);
        self
    }

    /// Set chunk index.
    pub fn with_chunk_index(mut self, index: usize) -> Self {
        self.chunk_index = Some(index);
        self
    }
}

/// A retrieved entity from the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedEntity {
    /// Entity name.
    pub name: String,

    /// Entity type.
    pub entity_type: String,

    /// Entity description.
    pub description: String,

    /// Relevance score.
    pub score: f32,

    /// Number of connections in the graph.
    pub degree: usize,

    /// Source chunk IDs where this entity was mentioned (for citations).
    #[serde(default)]
    pub source_chunk_ids: Vec<String>,

    /// Source document ID.
    #[serde(default)]
    pub source_document_id: Option<String>,

    /// Original file path of the source document.
    #[serde(default)]
    pub source_file_path: Option<String>,
}

impl RetrievedEntity {
    /// Create a new retrieved entity.
    pub fn new(
        name: impl Into<String>,
        entity_type: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            entity_type: entity_type.into(),
            description: description.into(),
            score: 0.0,
            degree: 0,
            source_chunk_ids: Vec::new(),
            source_document_id: None,
            source_file_path: None,
        }
    }

    /// Set the score.
    pub fn with_score(mut self, score: f32) -> Self {
        self.score = score;
        self
    }

    /// Set the degree.
    pub fn with_degree(mut self, degree: usize) -> Self {
        self.degree = degree;
        self
    }

    /// Set source chunk IDs.
    pub fn with_source_chunk_ids(mut self, chunk_ids: Vec<String>) -> Self {
        self.source_chunk_ids = chunk_ids;
        self
    }

    /// Set source document ID.
    pub fn with_source_document_id(mut self, doc_id: impl Into<String>) -> Self {
        self.source_document_id = Some(doc_id.into());
        self
    }

    /// Set source file path.
    pub fn with_source_file_path(mut self, file_path: impl Into<String>) -> Self {
        self.source_file_path = Some(file_path.into());
        self
    }
}

/// A retrieved relationship.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedRelationship {
    /// Source entity.
    pub source: String,

    /// Target entity.
    pub target: String,

    /// Relationship type.
    pub relation_type: String,

    /// Relationship description.
    pub description: String,

    /// Relevance score.
    pub score: f32,

    /// Source chunk ID where this relationship was extracted (for citations).
    #[serde(default)]
    pub source_chunk_id: Option<String>,

    /// Source document ID.
    #[serde(default)]
    pub source_document_id: Option<String>,

    /// Original file path of the source document.
    #[serde(default)]
    pub source_file_path: Option<String>,
}

impl RetrievedRelationship {
    /// Create a new retrieved relationship.
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
            score: 0.0,
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

    /// Set the score.
    pub fn with_score(mut self, score: f32) -> Self {
        self.score = score;
        self
    }

    /// Set source chunk ID.
    pub fn with_source_chunk_id(mut self, chunk_id: impl Into<String>) -> Self {
        self.source_chunk_id = Some(chunk_id.into());
        self
    }

    /// Set source document ID.
    pub fn with_source_document_id(mut self, doc_id: impl Into<String>) -> Self {
        self.source_document_id = Some(doc_id.into());
        self
    }

    /// Set source file path.
    pub fn with_source_file_path(mut self, file_path: impl Into<String>) -> Self {
        self.source_file_path = Some(file_path.into());
        self
    }
}

/// Context retrieved from storage.
#[derive(Debug, Clone, Default)]
pub struct RetrievedContext {
    /// Vector search results.
    pub vector_results: Vec<(String, f32)>,

    /// Graph entities.
    pub graph_entities: Vec<String>,

    /// Graph relationships.
    pub graph_edges: Vec<(String, String, String)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_context_building() {
        let mut ctx = QueryContext::new();

        ctx.add_chunk(RetrievedChunk::new("c1", "Hello world", 0.9));
        ctx.add_entity(RetrievedEntity::new("Alice", "PERSON", "A person"));
        ctx.add_relationship(RetrievedRelationship::new("Alice", "Bob", "KNOWS"));

        assert!(!ctx.is_empty());
        assert_eq!(ctx.chunks.len(), 1);
        assert_eq!(ctx.entities.len(), 1);
        assert_eq!(ctx.relationships.len(), 1);
    }

    #[test]
    fn test_context_to_string() {
        let mut ctx = QueryContext::new();
        ctx.add_chunk(RetrievedChunk::new("c1", "Test content", 0.85));
        ctx.add_entity(RetrievedEntity::new("Test", "CONCEPT", "A test concept"));

        let s = ctx.to_context_string();

        assert!(s.contains("Document Chunks"));
        assert!(s.contains("Test content"));
        assert!(s.contains("Knowledge Graph Data (Entities)"));
    }

    #[test]
    fn test_retrieved_chunk_builder() {
        let chunk = RetrievedChunk::new("id", "content", 0.95).with_document_id("doc-1");

        assert_eq!(chunk.document_id, Some("doc-1".to_string()));
        assert_eq!(chunk.score, 0.95);
    }

    #[test]
    fn test_retrieved_entity_source_tracking() {
        let entity = RetrievedEntity::new("Sarah Chen", "PERSON", "Lead researcher")
            .with_source_chunk_ids(vec!["chunk-001".to_string(), "chunk-002".to_string()])
            .with_source_document_id("doc-abc123")
            .with_source_file_path("/documents/research.pdf");

        assert_eq!(entity.source_chunk_ids.len(), 2);
        assert!(entity.source_chunk_ids.contains(&"chunk-001".to_string()));
        assert!(entity.source_chunk_ids.contains(&"chunk-002".to_string()));
        assert_eq!(entity.source_document_id, Some("doc-abc123".to_string()));
        assert_eq!(
            entity.source_file_path,
            Some("/documents/research.pdf".to_string())
        );
    }

    #[test]
    fn test_retrieved_relationship_source_tracking() {
        let rel = RetrievedRelationship::new("Alice", "Bob", "KNOWS")
            .with_source_chunk_id("chunk-005")
            .with_source_document_id("doc-xyz789")
            .with_source_file_path("/documents/team.md");

        assert_eq!(rel.source_chunk_id, Some("chunk-005".to_string()));
        assert_eq!(rel.source_document_id, Some("doc-xyz789".to_string()));
        assert_eq!(rel.source_file_path, Some("/documents/team.md".to_string()));
    }
}
