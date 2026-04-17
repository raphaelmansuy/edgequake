//! Query-related types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Query mode.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum QueryMode {
    #[default]
    Hybrid,
    Local,
    Global,
    Naive,
    Mix,
}

/// Query request.
#[derive(Debug, Clone, Serialize, Default)]
pub struct QueryRequest {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<QueryMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub only_need_context: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// Stream format version: "v1" (raw text) or "v2" (structured JSON). @implements SPEC-006
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_format: Option<String>,
    /// LLM provider override. @implements SPEC-006 + SPEC-032
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_provider: Option<String>,
    /// LLM model override. @implements SPEC-006 + SPEC-032
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,
}

/// Source reference in query response.
/// @implements SPEC-006: Enriched source references
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SourceReference {
    #[serde(default)]
    pub document_id: Option<String>,
    #[serde(default)]
    pub chunk_id: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub score: Option<f64>,
    #[serde(default)]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    /// Original file path or title of the source document.
    #[serde(default)]
    pub file_path: Option<String>,
    /// Entity type (e.g., "PERSON", "ORGANIZATION"). @implements SPEC-006
    #[serde(default)]
    pub entity_type: Option<String>,
    /// Entity degree (number of relationships). @implements SPEC-006
    #[serde(default)]
    pub degree: Option<usize>,
    /// Source chunk IDs that mention this entity. @implements SPEC-006
    #[serde(default)]
    pub source_chunk_ids: Option<Vec<String>>,
}

/// Query response.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QueryResponse {
    #[serde(default)]
    pub answer: Option<String>,
    #[serde(default)]
    pub sources: Vec<SourceReference>,
    #[serde(default)]
    pub mode: Option<String>,
}

/// Query stream chunk (SSE event data).
/// @implements SPEC-006: Structured streaming events
#[derive(Debug, Clone, Deserialize)]
pub struct QueryStreamChunk {
    /// Event type: "context", "token", "thinking", "done", "error"
    #[serde(default, rename = "type")]
    pub event_type: Option<String>,
    /// Token content (present in token events)
    #[serde(default)]
    pub content: Option<String>,
    /// Legacy raw text chunk (v1 format)
    #[serde(default)]
    pub chunk: Option<String>,
    #[serde(default)]
    pub done: Option<bool>,
    #[serde(default)]
    pub sources: Option<Vec<SourceReference>>,
    /// Query mode used for retrieval (present in context events). @implements SPEC-006
    #[serde(default)]
    pub query_mode: Option<String>,
    /// Retrieval time in ms (present in context events). @implements SPEC-006
    #[serde(default)]
    pub retrieval_time_ms: Option<u64>,
    /// Statistics (present in done events). @implements SPEC-006
    #[serde(default)]
    pub stats: Option<QueryStreamStats>,
    /// LLM provider used (present in done events). @implements SPEC-032
    #[serde(default)]
    pub llm_provider: Option<String>,
    /// LLM model used (present in done events). @implements SPEC-032
    #[serde(default)]
    pub llm_model: Option<String>,
    /// Error message (present in error events)
    #[serde(default)]
    pub message: Option<String>,
    /// Error code (present in error events)
    #[serde(default)]
    pub code: Option<String>,
}

/// Statistics for streaming query responses.
/// @implements SPEC-006
#[derive(Debug, Clone, Deserialize)]
pub struct QueryStreamStats {
    #[serde(default)]
    pub embedding_time_ms: Option<u64>,
    #[serde(default)]
    pub retrieval_time_ms: Option<u64>,
    #[serde(default)]
    pub generation_time_ms: Option<u64>,
    #[serde(default)]
    pub total_time_ms: Option<u64>,
    #[serde(default)]
    pub sources_retrieved: Option<usize>,
    #[serde(default)]
    pub tokens_used: Option<u32>,
    #[serde(default)]
    pub tokens_per_second: Option<f32>,
    #[serde(default)]
    pub query_mode: Option<String>,
}
