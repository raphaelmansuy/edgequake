//! Chat DTO types.
//!
//! This module contains all Data Transfer Objects for the unified chat completions API.
//! Extracted from chat.rs for modularity and single responsibility.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::handlers::query::{QueryStats, SourceReference};

// ============================================================================
// Default value helper functions
// ============================================================================

/// Default streaming mode for chat (true).
pub fn chat_default_stream() -> bool {
    true
}

// ============================================================================
// Request DTOs
// ============================================================================

/// Unified chat completion request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ChatCompletionRequest {
    /// Existing conversation ID. If null, creates a new conversation.
    pub conversation_id: Option<Uuid>,

    /// User message content.
    pub message: String,

    /// Query mode (local, global, hybrid, naive).
    #[serde(default)]
    pub mode: Option<String>,

    /// Whether to stream the response.
    #[serde(default = "chat_default_stream")]
    pub stream: bool,

    /// Maximum tokens for response.
    #[serde(default)]
    pub max_tokens: Option<usize>,

    /// Temperature for generation (0.0-2.0).
    #[serde(default)]
    pub temperature: Option<f32>,

    /// Top K for retrieval.
    #[serde(default)]
    pub top_k: Option<usize>,

    /// Parent message ID for threading.
    #[serde(default)]
    pub parent_id: Option<Uuid>,

    /// LLM provider ID to use for this query (e.g., "openai", "ollama", "lmstudio").
    /// If not provided, uses the workspace or server default.
    ///
    /// @implements SPEC-032: Provider selection in query interface
    #[serde(default)]
    pub provider: Option<String>,

    /// Specific model name within the provider (e.g., "gpt-4o-mini", "gemma3:12b").
    /// When combined with provider, allows full model selection from models.toml.
    /// If not provided, uses the provider's default chat model.
    ///
    /// @implements SPEC-032: Full model selection in query interface
    #[serde(default)]
    pub model: Option<String>,
}

// ============================================================================
// Response DTOs
// ============================================================================

/// Non-streaming chat completion response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ChatCompletionResponse {
    /// Conversation ID (created or existing).
    pub conversation_id: Uuid,

    /// User message ID.
    pub user_message_id: Uuid,

    /// Assistant message ID.
    pub assistant_message_id: Uuid,

    /// Assistant response content.
    pub content: String,

    /// Query mode used.
    pub mode: String,

    /// Sources retrieved.
    pub sources: Vec<SourceReference>,

    /// Generation statistics.
    pub stats: QueryStats,

    /// Tokens used for generation.
    pub tokens_used: u32,

    /// Duration in milliseconds.
    pub duration_ms: u64,

    /// LLM provider used (lineage tracking).
    /// @implements SPEC-032: Provider lineage in query responses
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_provider: Option<String>,

    /// LLM model used (lineage tracking).
    /// @implements SPEC-032: Model lineage in query responses
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,
}

// ============================================================================
// Streaming Event Types
// ============================================================================

/// Streaming SSE event types.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatStreamEvent {
    /// Conversation and user message created/confirmed.
    Conversation {
        conversation_id: Uuid,
        user_message_id: Uuid,
    },

    /// Context/sources retrieved.
    Context { sources: Vec<SourceReference> },

    /// Token generated during streaming.
    Token { content: String },

    /// Thinking/reasoning phase content.
    Thinking { content: String },

    /// Stream complete - assistant message saved.
    Done {
        assistant_message_id: Uuid,
        tokens_used: u32,
        duration_ms: u64,
        /// LLM provider used (lineage tracking).
        /// @implements SPEC-032: Provider lineage in streaming responses
        #[serde(skip_serializing_if = "Option::is_none")]
        llm_provider: Option<String>,
        /// LLM model used (lineage tracking).
        /// @implements SPEC-032: Model lineage in streaming responses
        #[serde(skip_serializing_if = "Option::is_none")]
        llm_model: Option<String>,
    },

    /// Error occurred.
    Error { message: String, code: String },
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_default_stream() {
        assert!(chat_default_stream());
    }

    #[test]
    fn test_chat_request_minimal() {
        let json = r#"{"message": "Hello, AI!"}"#;
        let req: ChatCompletionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.message, "Hello, AI!");
        assert!(req.stream); // default is true
        assert!(req.conversation_id.is_none());
        assert!(req.mode.is_none());
    }

    #[test]
    fn test_chat_request_full() {
        let json = r#"{
            "conversation_id": "550e8400-e29b-41d4-a716-446655440000",
            "message": "What is RAG?",
            "mode": "hybrid",
            "stream": false,
            "max_tokens": 500,
            "temperature": 0.7,
            "top_k": 10
        }"#;
        let req: ChatCompletionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.message, "What is RAG?");
        assert!(!req.stream);
        assert_eq!(req.mode, Some("hybrid".to_string()));
        assert_eq!(req.max_tokens, Some(500));
        assert_eq!(req.temperature, Some(0.7));
        assert_eq!(req.top_k, Some(10));
    }

    #[test]
    fn test_chat_stream_event_conversation() {
        let event = ChatStreamEvent::Conversation {
            conversation_id: Uuid::nil(),
            user_message_id: Uuid::nil(),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "conversation");
    }

    #[test]
    fn test_chat_stream_event_token() {
        let event = ChatStreamEvent::Token {
            content: "Hello".to_string(),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "token");
        assert_eq!(json["content"], "Hello");
    }

    #[test]
    fn test_chat_stream_event_done() {
        let event = ChatStreamEvent::Done {
            assistant_message_id: Uuid::nil(),
            tokens_used: 150,
            duration_ms: 1200,
            llm_provider: Some("ollama".to_string()),
            llm_model: Some("gemma3:12b".to_string()),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "done");
        assert_eq!(json["tokens_used"], 150);
        assert_eq!(json["duration_ms"], 1200);
        assert_eq!(json["llm_provider"], "ollama");
        assert_eq!(json["llm_model"], "gemma3:12b");
    }

    #[test]
    fn test_chat_stream_event_done_no_provider() {
        let event = ChatStreamEvent::Done {
            assistant_message_id: Uuid::nil(),
            tokens_used: 100,
            duration_ms: 500,
            llm_provider: None,
            llm_model: None,
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "done");
        // Provider fields should be absent when None (skip_serializing_if)
        assert!(json.get("llm_provider").is_none());
        assert!(json.get("llm_model").is_none());
    }

    #[test]
    fn test_chat_stream_event_error() {
        let event = ChatStreamEvent::Error {
            message: "Something went wrong".to_string(),
            code: "INTERNAL_ERROR".to_string(),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "error");
        assert!(json["message"].as_str().unwrap().contains("wrong"));
        assert_eq!(json["code"], "INTERNAL_ERROR");
    }

    #[test]
    fn test_chat_stream_event_thinking() {
        let event = ChatStreamEvent::Thinking {
            content: "Analyzing the query...".to_string(),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "thinking");
        assert!(json["content"].as_str().unwrap().contains("Analyzing"));
    }
}
