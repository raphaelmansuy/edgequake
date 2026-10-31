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
    ///
    /// This is the **prompt** set (post-CE / truncate). LLM generation uses these.
    pub chunks: Vec<RetrievedChunk>,

    /// Optional L2/citation chunk set (026 Mix∪CE dual-list).
    ///
    /// When set, API `sources` emit these instead of `chunks` so evidence_recall
    /// can retain Mix first-stage membership while the prompt stays CE-ordered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub citation_chunks: Option<Vec<RetrievedChunk>>,

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

    /// Stamp stable [`RetrievedChunk::citation_id`] values before format/API (X-20).
    ///
    /// Prompt chunks keep existing IDs and receive the smallest unreserved IDs
    /// for unstamped chunks. Dual-list `citation_chunks` reuse the same id for
    /// overlapping chunk ids, then continue the sequence for Mix-only members.
    pub fn ensure_stable_citation_ids(&mut self) {
        let mut used = self
            .chunks
            .iter()
            .filter_map(|c| c.citation_id)
            .chain(
                self.citation_chunks
                    .iter()
                    .flatten()
                    .filter_map(|c| c.citation_id),
            )
            .collect();
        crate::context_format::assign_stable_citation_ids_avoiding(&mut self.chunks, &mut used);
        if let Some(ref mut citation) = self.citation_chunks {
            let mut by_id: HashMap<String, usize> = self
                .chunks
                .iter()
                .filter_map(|c| c.citation_id.map(|id| (c.id.clone(), id)))
                .collect();
            for chunk in citation.iter() {
                if let Some(id) = chunk.citation_id {
                    by_id.entry(chunk.id.clone()).or_insert(id);
                }
            }
            let mut next = 1;
            for chunk in citation.iter_mut() {
                if let Some(&id) = by_id.get(&chunk.id) {
                    chunk.citation_id = Some(id);
                } else {
                    while used.contains(&next) {
                        next += 1;
                    }
                    chunk.citation_id = Some(next);
                    used.insert(next);
                    by_id.insert(chunk.id.clone(), next);
                }
            }
        }
    }

    /// Build a text representation for LLM context.
    ///
    /// SPEC-047 Q1.1: chunk headers include `page=` / `modality=` via
    /// [`crate::context_format`] so the LLM can ground answers on retrieved pages.
    ///
    /// Layout: `EDGEQUAKE_CONTEXT_FORMAT` (`flat` \| `path`) and optional
    /// `EDGEQUAKE_PASSAGE_PACK` (021 F2/F4).
    pub fn to_context_string(&self) -> String {
        crate::context_format::format_query_context(
            &self.entities,
            &self.relationships,
            &self.chunks,
        )
    }

    /// Check if the context is empty.
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty() && self.entities.is_empty() && self.relationships.is_empty()
    }
}

/// A retrieved text chunk.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RetrievedChunk {
    /// Chunk identifier.
    pub id: String,

    /// Chunk content.
    pub content: String,

    /// Relevance score.
    pub score: f32,

    /// Source document ID.
    pub document_id: Option<String>,

    /// Human document title / file name for prompt headers + citations (SPEC-142).
    /// Resolved from document metadata; never invented by the LLM.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_name: Option<String>,

    /// Token count.
    pub token_count: usize,

    /// Start line number in the document.
    pub start_line: Option<usize>,

    /// End line number in the document.
    pub end_line: Option<usize>,

    /// Chunk index in the document.
    pub chunk_index: Option<usize>,

    /// PDF page number (1-indexed) where this chunk starts.
    /// Present only when the source is a PDF with page-aware chunking.
    /// Enables the UI to deep-link to `#page=N` in the PDF viewer.
    pub page_start: Option<u32>,

    /// PDF page number where this chunk ends (SPEC-135: may exceed page_start).
    pub page_end: Option<u32>,

    /// Retrieval modality stamped at ingest (`chart`, `figure`, `table`, `equation`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modality: Option<String>,

    /// What the chunk was made from (`text`, `image`). SPEC-180: an image chunk has no text
    /// body, so text-only scoring must not treat it as an empty — i.e. irrelevant — chunk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_modality: Option<String>,

    /// Stable citation index for prompt `[N]` and API `reference_id` (SPEC-083 X-20).
    ///
    /// Assigned once before format; never renumbered after reorder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub citation_id: Option<usize>,
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
            document_name: None,
            token_count,
            start_line: None,
            end_line: None,
            chunk_index: None,
            page_start: None,
            page_end: None,
            modality: None,
            source_modality: None,
            citation_id: None,
        }
    }

    /// Set the human-readable document name (SPEC-142).
    pub fn with_document_name(mut self, name: impl Into<String>) -> Self {
        self.document_name = Some(name.into());
        self
    }

    /// Set a stable citation id (prompt `[N]` / API `reference_id`).
    pub fn with_citation_id(mut self, citation_id: usize) -> Self {
        self.citation_id = Some(citation_id);
        self
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

    /// Set PDF page attribution (SPEC-032 W-09). Single-page helper.
    pub fn with_page(self, page: u32) -> Self {
        self.with_page_span(page, Some(page))
    }

    /// Set a page span. `end` defaults to `start` when None (SPEC-135).
    pub fn with_page_span(mut self, start: u32, end: Option<u32>) -> Self {
        self.page_start = Some(start);
        self.page_end = Some(end.unwrap_or(start));
        self
    }

    /// Set retrieval modality (SPEC-047 MV-23 / MV-32 telemetry).
    pub fn with_modality(mut self, modality: impl Into<String>) -> Self {
        self.modality = Some(modality.into());
        self
    }

    /// Record what the chunk was made from (`text` or `image`). SPEC-180.
    pub fn with_source_modality(mut self, modality: impl Into<String>) -> Self {
        self.source_modality = Some(modality.into());
        self
    }

    /// True when this chunk's payload is pixels, so it has no text to score.
    ///
    /// SPEC-180 09: a cross-encoder cannot rank an image and would score it 0, dropping the
    /// very evidence a search-by-image query asked for. Callers protect these rows instead.
    pub fn is_image_chunk(&self) -> bool {
        self.source_modality
            .as_deref()
            .is_some_and(|m| m.eq_ignore_ascii_case("image"))
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

    /// Source document ID (single — legacy, may be overwritten during reconciliation).
    #[serde(default)]
    pub source_document_id: Option<String>,

    /// Union of ALL source document IDs for this entity.
    /// Populated from the `source_document_ids` JSON array on the graph node.
    /// An entity that appears in docs A + B carries both IDs here.
    /// @implements SPEC-031: Multi-document entity lineage
    #[serde(default)]
    pub source_document_ids: Vec<String>,

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
            source_document_ids: Vec::new(),
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

    /// Set union of all source document IDs.
    /// @implements SPEC-031
    pub fn with_source_document_ids(mut self, doc_ids: Vec<String>) -> Self {
        self.source_document_ids = doc_ids;
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
    /// Source entity graph node id (identity / navigation).
    pub source: String,

    /// Target entity graph node id (identity / navigation).
    pub target: String,

    /// Human presentation for source (073). Empty → fall back to [`Self::source`].
    #[serde(default)]
    pub source_label: String,

    /// Human presentation for target (073). Empty → fall back to [`Self::target`].
    #[serde(default)]
    pub target_label: String,

    /// Relationship type.
    pub relation_type: String,

    /// Relationship description.
    pub description: String,

    /// Relevance score.
    pub score: f32,

    /// Source chunk ID where this relationship was extracted (for citations).
    /// Legacy singular — prefer [`Self::source_chunk_ids`] (052 / LightRAG parity).
    #[serde(default)]
    pub source_chunk_id: Option<String>,

    /// All chunk IDs that contributed this relationship (049 union → Mix pool).
    #[serde(default)]
    pub source_chunk_ids: Vec<String>,

    /// Source document ID (single — legacy, may be overwritten during reconciliation).
    #[serde(default)]
    pub source_document_id: Option<String>,

    /// Union of ALL source document IDs for this relationship.
    /// @implements SPEC-031: Multi-document relationship lineage
    #[serde(default)]
    pub source_document_ids: Vec<String>,

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
        let source = source.into();
        let target = target.into();
        Self {
            source_label: source.clone(),
            target_label: target.clone(),
            source,
            target,
            relation_type: relation_type.into(),
            description: String::new(),
            score: 0.0,
            source_chunk_id: None,
            source_chunk_ids: Vec::new(),
            source_document_id: None,
            source_document_ids: Vec::new(),
            source_file_path: None,
        }
    }

    /// Presentation label for source (073).
    pub fn display_source(&self) -> &str {
        if self.source_label.trim().is_empty() {
            &self.source
        } else {
            &self.source_label
        }
    }

    /// Presentation label for target (073).
    pub fn display_target(&self) -> &str {
        if self.target_label.trim().is_empty() {
            &self.target
        } else {
            &self.target_label
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

    /// Set source chunk ID (also ensures it appears in [`Self::source_chunk_ids`]).
    pub fn with_source_chunk_id(mut self, chunk_id: impl Into<String>) -> Self {
        let id = chunk_id.into();
        self.source_chunk_id = Some(id.clone());
        if !self.source_chunk_ids.iter().any(|c| c == &id) {
            self.source_chunk_ids.insert(0, id);
        }
        self
    }

    /// Set all source chunk IDs (LightRAG relation `source_id` multi-part / 052).
    pub fn with_source_chunk_ids(mut self, chunk_ids: Vec<String>) -> Self {
        self.source_chunk_ids = chunk_ids;
        self.source_chunk_id = self.source_chunk_ids.first().cloned();
        self
    }

    /// All provenance chunk ids (plural preferred; singular fallback).
    pub fn all_source_chunk_ids(&self) -> Vec<String> {
        if !self.source_chunk_ids.is_empty() {
            return self.source_chunk_ids.clone();
        }
        self.source_chunk_id.iter().cloned().collect()
    }

    /// Set source document ID.
    pub fn with_source_document_id(mut self, doc_id: impl Into<String>) -> Self {
        self.source_document_id = Some(doc_id.into());
        self
    }

    /// Set union of all source document IDs.
    /// @implements SPEC-031
    pub fn with_source_document_ids(mut self, doc_ids: Vec<String>) -> Self {
        self.source_document_ids = doc_ids;
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

        assert!(s.contains("### Chunks"));
        assert!(s.contains("Test content"));
        assert!(s.contains("### Entities"));
    }

    #[test]
    fn test_context_to_string_includes_page_and_modality() {
        let mut ctx = QueryContext::new();
        ctx.add_chunk(
            RetrievedChunk::new("c1", "Chart shows 42%", 0.9)
                .with_page(11)
                .with_modality("chart"),
        );

        let s = ctx.to_context_string();
        assert!(s.contains("page=11"), "expected page header, got: {s}");
        assert!(s.contains("modality=chart"), "expected modality, got: {s}");
        assert!(s.contains("page=N"));
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
