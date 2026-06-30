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
///
/// This is the persisted form of a `SourceReference` attached to an
/// assistant message. Adding page attribution here (SPEC-033 FP-033-1) ensures
/// that PDF page grouping survives the write → persist → reload cycle and is
/// not silently dropped when a conversation is stored to the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSource {
    /// Source identifier (chunk id or entity id).
    pub id: String,
    /// Source title (file path or document id).
    pub title: Option<String>,
    /// Content snippet.
    pub content: Option<String>,
    /// Relevance score.
    pub score: f32,
    /// Document ID this came from.
    pub document_id: Option<String>,
    /// Source type: "chunk", "entity", or "relationship".
    /// Required for correct filtering in the frontend source-mapper.
    /// @implements SPEC-033 FP-033-1
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,
    /// PDF page number (1-indexed) where this chunk starts.
    /// Present only for PDFs processed with page-aware chunking (SPEC-033).
    /// Persisting this allows the citation panel to group passages by page
    /// even after the conversation is reloaded from the database.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_start: Option<u32>,
    /// PDF page number (1-indexed) where this chunk ends.
    /// Always equals `page_start` — chunks never cross page boundaries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_end: Option<u32>,
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
            source_type: None,
            page_start: None,
            page_end: None,
        }
    }
}
