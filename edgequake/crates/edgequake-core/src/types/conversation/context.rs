//! Message context types for source tracking and citations.
//!
//! These types attach provenance information to assistant responses,
//! enabling the UI to display source citations and entity references.

use serde::{Deserialize, Serialize};

/// Context attached to an assistant message (sources, entities).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageContext {
    /// Source references used to generate the response.
    #[serde(default)]
    pub sources: Vec<MessageSource>,
    /// Entities mentioned in the response with source tracking.
    #[serde(default)]
    pub entities: Vec<MessageContextEntity>,
    /// Relationships referenced in the response with source tracking.
    #[serde(default)]
    pub relationships: Vec<MessageContextRelationship>,
}

impl MessageContext {
    /// Create a new empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a source.
    pub fn with_source(mut self, source: MessageSource) -> Self {
        self.sources.push(source);
        self
    }

    /// Add sources.
    pub fn with_sources(mut self, sources: Vec<MessageSource>) -> Self {
        self.sources = sources;
        self
    }
}

/// Entity in message context with source tracking for citations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContextEntity {
    /// Entity name.
    pub name: String,
    /// Entity type.
    pub entity_type: String,
    /// Entity description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Relevance score.
    pub score: f32,
    /// Source document ID for citation link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_document_id: Option<String>,
    /// Original file path for citation display.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_file_path: Option<String>,
    /// Source chunk IDs for provenance.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_chunk_ids: Vec<String>,
}

impl MessageContextEntity {
    /// Create from entity name with minimal fields.
    pub fn from_name(name: impl Into<String>, score: f32) -> Self {
        Self {
            name: name.into(),
            entity_type: "UNKNOWN".to_string(),
            description: None,
            score,
            source_document_id: None,
            source_file_path: None,
            source_chunk_ids: Vec::new(),
        }
    }
}

/// Relationship in message context with source tracking for citations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContextRelationship {
    /// Source entity name.
    pub source: String,
    /// Target entity name.
    pub target: String,
    /// Relationship type.
    pub relation_type: String,
    /// Relationship description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Relevance score.
    pub score: f32,
    /// Source document ID for citation link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_document_id: Option<String>,
    /// Original file path for citation display.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_file_path: Option<String>,
}

impl MessageContextRelationship {
    /// Create from source/target with minimal fields.
    pub fn new(
        source: impl Into<String>,
        target: impl Into<String>,
        relation_type: impl Into<String>,
        score: f32,
    ) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            relation_type: relation_type.into(),
            description: None,
            score,
            source_document_id: None,
            source_file_path: None,
        }
    }
}

/// A source reference in message context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSource {
    /// Source identifier.
    pub id: String,
    /// Source title.
    pub title: Option<String>,
    /// Content snippet.
    pub content: Option<String>,
    /// Relevance score.
    pub score: f32,
    /// Document ID this came from.
    pub document_id: Option<String>,
}

impl MessageSource {
    /// Create a new message source.
    pub fn new(id: impl Into<String>, score: f32) -> Self {
        Self {
            id: id.into(),
            title: None,
            content: None,
            score,
            document_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_context_default_empty() {
        let ctx = MessageContext::default();
        assert!(ctx.sources.is_empty());
        assert!(ctx.entities.is_empty());
        assert!(ctx.relationships.is_empty());
    }

    #[test]
    fn test_message_context_with_source() {
        let src = MessageSource::new("s1", 0.9);
        let ctx = MessageContext::new().with_source(src);
        assert_eq!(ctx.sources.len(), 1);
        assert_eq!(ctx.sources[0].id, "s1");
    }

    #[test]
    fn test_message_context_with_sources() {
        let sources = vec![MessageSource::new("a", 0.8), MessageSource::new("b", 0.7)];
        let ctx = MessageContext::new().with_sources(sources);
        assert_eq!(ctx.sources.len(), 2);
    }

    #[test]
    fn test_message_context_entity_from_name() {
        let e = MessageContextEntity::from_name("ALICE", 0.95);
        assert_eq!(e.name, "ALICE");
        assert_eq!(e.entity_type, "UNKNOWN");
        assert!(e.description.is_none());
        assert_eq!(e.score, 0.95);
        assert!(e.source_chunk_ids.is_empty());
    }

    #[test]
    fn test_message_context_relationship_new() {
        let r = MessageContextRelationship::new("A", "B", "KNOWS", 0.8);
        assert_eq!(r.source, "A");
        assert_eq!(r.target, "B");
        assert_eq!(r.relation_type, "KNOWS");
        assert_eq!(r.score, 0.8);
        assert!(r.description.is_none());
    }

    #[test]
    fn test_message_source_new() {
        let s = MessageSource::new("chunk-123", 0.75);
        assert_eq!(s.id, "chunk-123");
        assert_eq!(s.score, 0.75);
        assert!(s.title.is_none());
        assert!(s.document_id.is_none());
    }
}
