//! Chunk detail and chunk lineage DTOs.
//!
//! - **WEBUI-006**: Chunk detail response
//! - **OODA-08**: Chunk lineage response

use serde::Serialize;
use utoipa::ToSchema;

/// Chunk detail response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ChunkDetailResponse {
    /// Chunk ID.
    pub chunk_id: String,
    /// Document ID this chunk belongs to.
    pub document_id: String,
    /// Document name.
    pub document_name: Option<String>,
    /// Full chunk content.
    pub content: String,
    /// Chunk index in document.
    pub index: usize,
    /// Character offset range.
    pub char_range: CharRange,
    /// Starting line number (1-based) in the source document.
    /// OODA-07: Added for lineage traceability — maps chunk to source location.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<usize>,
    /// Ending line number (1-based, inclusive) in the source document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<usize>,
    /// PDF page number (1-indexed) where this chunk starts.
    /// Present only for PDFs ingested with SPEC-032 page-aware chunking.
    /// @implements SPEC-033 — page attribution surfacing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_start: Option<u32>,
    /// PDF page number (1-indexed) where this chunk ends.
    /// Always equal to `page_start` — chunks never cross page boundaries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_end: Option<u32>,
    /// Token count.
    pub token_count: usize,
    /// Entities extracted from this chunk.
    pub entities: Vec<ExtractedEntityInfo>,
    /// Relationships extracted from this chunk.
    pub relationships: Vec<ExtractedRelationshipInfo>,
    /// Extraction metadata.
    pub extraction_metadata: Option<ExtractionMetadataInfo>,
}

/// Character range for chunk position.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CharRange {
    /// Start offset.
    pub start: usize,
    /// End offset.
    pub end: usize,
}

/// Entity extracted from chunk.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ExtractedEntityInfo {
    /// Entity ID/name.
    pub id: String,
    /// Entity name.
    pub name: String,
    /// Entity type.
    pub entity_type: String,
    /// Description.
    pub description: Option<String>,
}

/// Relationship extracted from chunk.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ExtractedRelationshipInfo {
    /// Source entity.
    pub source_name: String,
    /// Target entity.
    pub target_name: String,
    /// Relationship type/keywords.
    pub relation_type: String,
    /// Description.
    pub description: Option<String>,
}

/// Extraction metadata.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ExtractionMetadataInfo {
    /// LLM model used.
    pub model: String,
    /// Gleaning iterations.
    pub gleaning_iterations: usize,
    /// Extraction duration in ms.
    pub duration_ms: u64,
    /// Input tokens.
    pub input_tokens: usize,
    /// Output tokens.
    pub output_tokens: usize,
    /// Whether cached.
    pub cached: bool,
}

/// Complete chunk lineage response — parent document context + position + entities.
///
/// OODA-08: Provides single-call retrieval of a chunk's full lineage chain:
/// `Chunk → Document → PDF (optional) → Entities & Relationships`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ChunkLineageResponse {
    /// Chunk ID.
    pub chunk_id: String,
    /// Parent document ID.
    pub document_id: String,
    /// Document name/title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_name: Option<String>,
    /// Document type (pdf, markdown, text).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_type: Option<String>,
    /// Chunk index in parent document.
    pub index: usize,
    /// Starting line number (1-based).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<usize>,
    /// Ending line number (1-based, inclusive).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<usize>,
    /// Start byte offset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_offset: Option<usize>,
    /// End byte offset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_offset: Option<usize>,
    /// Token count.
    pub token_count: usize,
    /// Content preview (first 200 chars).
    pub content_preview: String,
    /// Entity count extracted from this chunk.
    pub entity_count: usize,
    /// Relationship count extracted from this chunk.
    pub relationship_count: usize,
    /// Entity names extracted from this chunk.
    pub entity_names: Vec<String>,
    /// Document metadata snapshot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_metadata: Option<serde_json::Value>,
}
