//! Query context service DTOs (SPEC-028).
//!
//! Agent-grade structured retrieval responses for Agentic Search and MCP.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::query_types::{default_enable_rerank, DocumentFilter, MixWeightRequest};

/// Content payload tier for context retrieval.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContentGranularity {
    /// Snippet-only (200 chars) — UI / legacy citation compat.
    Citation,
    /// Full chunk text + structured subgraph — default for agents.
    #[default]
    Agent,
    /// Full payload plus LLM context string.
    Debug,
}

/// Context retrieval request (`POST /api/v1/query/context`).
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ContextRetrievalRequest {
    /// Natural language query.
    pub query: String,

    /// Retrieval mode (naive, local, global, hybrid, mix).
    #[serde(default)]
    pub mode: Option<String>,

    /// Payload tier: citation | agent | debug.
    #[serde(default)]
    pub content_granularity: ContentGranularity,

    #[serde(default)]
    pub max_results: Option<usize>,

    #[serde(default)]
    pub conversation_history: Option<Vec<super::query_types::ConversationMessage>>,

    #[serde(default)]
    pub document_filter: Option<DocumentFilter>,

    #[serde(default)]
    pub mix_weights: Option<MixWeightRequest>,

    #[serde(default = "default_enable_rerank")]
    pub enable_rerank: bool,

    #[serde(default)]
    pub rerank_model: Option<String>,

    #[serde(default)]
    pub rerank_top_k: Option<usize>,

    #[serde(default = "default_true")]
    pub include_lineage: bool,

    #[serde(default = "default_true")]
    pub include_documents: bool,

    #[serde(default = "default_true")]
    pub include_agent_hints: bool,

    /// Include query-matched entities and relationships in `bundle.subgraph`.
    #[serde(default = "default_true")]
    pub include_subgraph: bool,
}

fn default_true() -> bool {
    true
}

/// Lightweight search request (`POST /api/v1/query/context/search`).
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ContextSearchRequest {
    pub query: String,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub max_results: Option<usize>,
    #[serde(default)]
    pub document_filter: Option<DocumentFilter>,
}

/// Search result summary for MCP `edgequake_search`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContextSearchResult {
    pub retrieval_id: String,
    pub title: String,
    pub snippet: String,
    pub url: String,
    pub score: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContextSearchResponse {
    pub results: Vec<ContextSearchResult>,
}

/// Full context retrieval response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContextRetrievalResponse {
    pub retrieval_id: String,
    pub query: String,
    pub mode: String,
    pub mode_selection: ModeSelection,
    pub bundle: ContextBundle,
    pub stats: ContextRetrievalStats,
    pub retrieval_quality: RetrievalQuality,
    pub truncation: TruncationInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_hints: Option<AgentHints>,
    pub retrieval_fingerprint: String,
    pub cached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModeSelection {
    pub requested: String,
    pub effective: String,
    pub adaptive: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ContextBundle {
    pub subgraph: SubgraphBundle,
    pub chunks: Vec<ContextChunk>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub documents: Vec<ContextDocumentSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_string: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct SubgraphBundle {
    pub entities: Vec<ContextEntity>,
    pub relationships: Vec<ContextRelationship>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContextEntity {
    pub id: String,
    pub name: String,
    pub entity_type: String,
    pub description: String,
    pub score: f32,
    pub degree: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lineage: Option<EntityLineage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContextRelationship {
    pub id: String,
    pub source: String,
    pub target: String,
    pub relation_type: String,
    pub description: String,
    pub score: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lineage: Option<RelationshipLineage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContextChunk {
    pub id: String,
    pub content: String,
    pub score: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rerank_score: Option<f32>,
    pub token_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_id: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_truncated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lineage: Option<ChunkLineage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EntityLineage {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_chunk_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_document_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RelationshipLineage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_chunk_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_document_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChunkLineage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_index: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContextDocumentSummary {
    pub document_id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    pub chunk_count_in_bundle: usize,
    pub entity_count_in_bundle: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContextRetrievalStats {
    pub embedding_time_ms: u64,
    pub retrieval_time_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rerank_time_ms: Option<u64>,
    pub total_time_ms: u64,
    pub items_retrieved: ItemsRetrieved,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords_extracted: Vec<String>,
    pub reranked: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ItemsRetrieved {
    pub chunks: usize,
    pub entities: usize,
    pub relationships: usize,
    pub documents: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RetrievalQuality {
    pub coverage_score: f32,
    pub is_sufficient: bool,
    pub empty_context: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TruncationInfo {
    pub is_truncated: bool,
    pub token_budget: usize,
    pub tokens_used: usize,
    pub dropped: DroppedCounts,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct DroppedCounts {
    pub chunks: usize,
    pub entities: usize,
    pub relationships: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentHints {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suggested_followups: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dominant_entity_types: Vec<String>,
    pub documents_touched: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub data_quality_warnings: Vec<String>,
}

// --- Agent artifact retrieval (SPEC-028 Phase 2) ---

/// Agent-facing artifact fetch response (`GET /query/context/artifacts/{type}/{id}`).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContextArtifactResponse {
    /// `document` | `chunk` | `figure` | `markdown` | `pdf`
    pub artifact_type: String,
    pub artifact_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document: Option<ContextArtifactDocument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk: Option<ContextArtifactChunk>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub figure: Option<ContextArtifactFigure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<ContextArtifactMarkdown>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf: Option<ContextArtifactPdf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContextArtifactDocument {
    pub document_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    pub chunk_count: usize,
    pub multimodal_item_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_summary: Option<String>,
    /// Full markdown/text body when `include_content=true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Same as `content` when markdown is available (explicit agent field).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
    /// `kv` or `pdf_storage` when body loaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_download_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_content_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContextArtifactChunk {
    pub chunk_id: String,
    pub document_id: String,
    pub content: String,
    pub chunk_index: usize,
    pub token_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContextArtifactFigure {
    pub item_id: String,
    pub document_id: String,
    pub modality: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analyzed_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContextArtifactMarkdown {
    pub document_id: String,
    pub markdown: String,
    /// `kv` or `pdf_storage`
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContextArtifactPdf {
    pub pdf_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
    pub filename: String,
    pub file_size_bytes: i64,
    pub content_type: String,
    pub is_processed: bool,
    /// REST path to download raw PDF bytes.
    pub download_path: String,
    /// REST path to PDF metadata + markdown JSON.
    pub content_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown_content: Option<String>,
}
