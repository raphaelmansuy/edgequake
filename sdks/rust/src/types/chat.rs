//! Chat-related types.
//!
//! WHY: EdgeQuake chat API uses `message` (singular string), NOT `messages` (array).
//! This is NOT OpenAI-compatible — it is EdgeQuake's native RAG-aware chat format.

use serde::{Deserialize, Serialize};

/// A chat message (used for conversation history display).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Chat completion request for POST /api/v1/chat/completions.
///
/// WHY: EdgeQuake uses `message` (singular string). The conversation threading
/// is handled server-side via `conversation_id`.
#[derive(Debug, Clone, Serialize)]
pub struct ChatCompletionRequest {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// A source reference in a chat/query response.
#[derive(Debug, Clone, Deserialize)]
pub struct SourceReference {
    #[serde(default)]
    pub source_type: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub score: Option<f64>,
    #[serde(default)]
    pub snippet: Option<String>,
    /// Document ID this source came from.
    #[serde(default)]
    pub document_id: Option<String>,
    /// Original file path or title of the source document.
    #[serde(default)]
    pub file_path: Option<String>,
}

/// Chat completion response from POST /api/v1/chat/completions.
///
/// WHY: EdgeQuake returns conversation-threaded response with RAG sources,
/// not OpenAI-style choices array.
#[derive(Debug, Clone, Deserialize)]
pub struct ChatCompletionResponse {
    #[serde(default)]
    pub conversation_id: Option<String>,
    #[serde(default)]
    pub user_message_id: Option<String>,
    #[serde(default)]
    pub assistant_message_id: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub sources: Vec<SourceReference>,
}

/// Stream chunk event for streaming chat completions.
///
/// The server sends tagged events with `{type: "...", ...}` format.
/// Consumers should check `r#type` to determine which fields are populated.
#[derive(Debug, Clone, Deserialize)]
pub struct ChatStreamChunk {
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub conversation_id: Option<String>,
    #[serde(default)]
    pub sources: Option<Vec<SourceReference>>,
    #[serde(default)]
    pub done: Option<bool>,
    /// Auto-generated conversation title (present in title_update events).
    /// @implements FEAT0505
    #[serde(default)]
    pub title: Option<String>,
    /// User message ID (present in conversation event).
    #[serde(default)]
    pub user_message_id: Option<String>,
    /// Assistant message ID (present in done event).
    #[serde(default)]
    pub assistant_message_id: Option<String>,
    /// Tokens used (present in done event).
    #[serde(default)]
    pub tokens_used: Option<u32>,
    /// Duration in milliseconds (present in done event).
    #[serde(default)]
    pub duration_ms: Option<u64>,
    /// LLM provider used (present in done event, lineage tracking).
    #[serde(default)]
    pub llm_provider: Option<String>,
    /// LLM model used (present in done event, lineage tracking).
    #[serde(default)]
    pub llm_model: Option<String>,
}
