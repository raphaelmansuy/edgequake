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
#[derive(Debug, Clone, Serialize)]
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
}

/// Source reference in query response.
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
#[derive(Debug, Clone, Deserialize)]
pub struct QueryStreamChunk {
    #[serde(default)]
    pub chunk: Option<String>,
    #[serde(default)]
    pub done: Option<bool>,
    #[serde(default)]
    pub sources: Option<Vec<SourceReference>>,
}
