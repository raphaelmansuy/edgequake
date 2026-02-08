//! Query DTO types.
//!
//! This module contains all Data Transfer Objects for the query API.
//! Extracted from query.rs for modularity and single responsibility.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ============================================================================
// Default value helper functions
// ============================================================================

/// Default enable reranking (true).
pub fn default_enable_rerank() -> bool {
    true
}

// ============================================================================
// Request DTOs
// ============================================================================

/// A single message in the conversation history.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ConversationMessage {
    /// Role of the message sender (user or assistant).
    pub role: String,

    /// Content of the message.
    pub content: String,
}

/// Query request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct QueryRequest {
    /// The query text.
    pub query: String,

    /// Query mode (naive, local, global, hybrid, mix).
    #[serde(default)]
    pub mode: Option<String>,

    /// Only return context, don't generate an answer.
    #[serde(default)]
    pub context_only: bool,

    /// Return the formatted prompt instead of calling the LLM.
    /// Useful for debugging or using your own LLM.
    #[serde(default)]
    pub prompt_only: bool,

    /// Include detailed reference metadata (document_id, file_path, reference_id) in sources.
    #[serde(default)]
    pub include_references: bool,

    /// Maximum number of results.
    #[serde(default)]
    pub max_results: Option<usize>,

    /// Conversation history for multi-turn context.
    #[serde(default)]
    pub conversation_history: Option<Vec<ConversationMessage>>,

    /// Enable reranking of retrieved chunks for better relevance.
    #[serde(default = "default_enable_rerank")]
    pub enable_rerank: bool,

    /// Rerank model to use (e.g., "cohere-rerank-v3").
    #[serde(default)]
    pub rerank_model: Option<String>,

    /// Top K chunks to keep after reranking.
    #[serde(default)]
    pub rerank_top_k: Option<usize>,

    /// LLM provider to use for this query (e.g., "openai", "ollama", "lmstudio").
    /// If not provided, uses the workspace or server default.
    /// @implements SPEC-032: Provider selection in query interface
    #[serde(default)]
    pub llm_provider: Option<String>,

    /// Specific model name within the provider (e.g., "gpt-4o-mini", "gemma3:12b").
    /// When combined with provider, allows full model selection from models.toml.
    /// If not provided, uses the provider's default chat model.
    /// @implements SPEC-032: Full model selection in query interface
    #[serde(default)]
    pub llm_model: Option<String>,
}

/// Streaming query request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct StreamQueryRequest {
    /// The query text.
    pub query: String,

    /// Query mode.
    #[serde(default)]
    pub mode: Option<String>,
}

// ============================================================================
// Response DTOs
// ============================================================================

/// Query response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct QueryResponse {
    /// Generated answer.
    pub answer: String,

    /// Query mode used.
    pub mode: String,

    /// Retrieved context sources.
    pub sources: Vec<SourceReference>,

    /// Query statistics.
    pub stats: QueryStats,

    /// Conversation ID for multi-turn context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,

    /// Whether reranking was applied.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub reranked: bool,
}

/// A source reference.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SourceReference {
    /// Source type (chunk, entity, relationship).
    pub source_type: String,

    /// Source ID.
    pub id: String,

    /// Relevance score.
    pub score: f32,

    /// Rerank score (if reranking was applied).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rerank_score: Option<f32>,

    /// Content snippet.
    pub snippet: Option<String>,

    /// Reference ID for citation (1, 2, 3, ...).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_id: Option<usize>,

    /// Document ID that this reference came from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,

    /// Original file path of the source document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,

    /// Start line number in the document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<usize>,

    /// End line number in the document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<usize>,

    /// Chunk index in the document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_index: Option<usize>,
}

/// Query statistics.
///
/// @implements SPEC-032 Item 18, 22: Token metrics and model lineage
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct QueryStats {
    /// Embedding time in ms.
    pub embedding_time_ms: u64,

    /// Retrieval time in ms.
    pub retrieval_time_ms: u64,

    /// Generation time in ms.
    pub generation_time_ms: u64,

    /// Total time in ms.
    pub total_time_ms: u64,

    /// Number of sources retrieved.
    pub sources_retrieved: usize,

    /// Rerank time in ms (if reranking was applied).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rerank_time_ms: Option<u64>,

    // ========================================================================
    // SPEC-032: Token metrics and model lineage (Items 18, 22)
    // ========================================================================
    /// Number of tokens generated in the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_used: Option<usize>,

    /// Tokens per second generation speed (calculated as tokens_used / generation_time_ms * 1000).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_per_second: Option<f32>,

    /// LLM provider used for generation (e.g., "ollama", "openai", "lmstudio").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_provider: Option<String>,

    /// LLM model name used for generation (e.g., "gemma3:12b", "gpt-4o-mini").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_enable_rerank() {
        assert!(default_enable_rerank());
    }

    #[test]
    fn test_query_request_minimal() {
        let json = r#"{"query": "What is RAG?"}"#;
        let req: QueryRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.query, "What is RAG?");
        assert!(req.enable_rerank); // default is true
        assert!(!req.context_only);
        assert!(!req.prompt_only);
    }

    #[test]
    fn test_query_request_full() {
        let json = r#"{
            "query": "What is AI?",
            "mode": "hybrid",
            "context_only": true,
            "include_references": true,
            "max_results": 10,
            "enable_rerank": false,
            "rerank_top_k": 5
        }"#;
        let req: QueryRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.mode, Some("hybrid".to_string()));
        assert!(req.context_only);
        assert!(req.include_references);
        assert!(!req.enable_rerank);
        assert_eq!(req.rerank_top_k, Some(5));
    }

    #[test]
    fn test_conversation_message() {
        let json = r#"{"role": "user", "content": "Hello"}"#;
        let msg: ConversationMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_stream_query_request() {
        let json = r#"{"query": "Tell me about embeddings", "mode": "local"}"#;
        let req: StreamQueryRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.query, "Tell me about embeddings");
        assert_eq!(req.mode, Some("local".to_string()));
    }

    #[test]
    fn test_source_reference_serialization() {
        let source = SourceReference {
            source_type: "chunk".to_string(),
            id: "chunk_123".to_string(),
            score: 0.95,
            rerank_score: Some(0.98),
            snippet: Some("This is a test snippet".to_string()),
            reference_id: Some(1),
            document_id: Some("doc_456".to_string()),
            file_path: Some("docs/test.md".to_string()),
            start_line: Some(10),
            end_line: Some(20),
            chunk_index: Some(2),
        };
        let json = serde_json::to_value(&source).unwrap();
        assert_eq!(json["source_type"], "chunk");
        // Use approximate comparison for floats
        let score = json["score"].as_f64().unwrap();
        assert!((score - 0.95).abs() < 0.01);
    }

    #[test]
    fn test_source_reference_minimal() {
        let source = SourceReference {
            source_type: "entity".to_string(),
            id: "ENT_ABC".to_string(),
            score: 0.8,
            rerank_score: None,
            snippet: None,
            reference_id: None,
            document_id: None,
            file_path: None,
            start_line: None,
            end_line: None,
            chunk_index: None,
        };
        let json = serde_json::to_value(&source).unwrap();
        assert!(json.get("rerank_score").is_none());
        assert!(json.get("reference_id").is_none());
    }

    #[test]
    fn test_query_stats_serialization() {
        let stats = QueryStats {
            embedding_time_ms: 50,
            retrieval_time_ms: 100,
            generation_time_ms: 500,
            total_time_ms: 650,
            sources_retrieved: 5,
            rerank_time_ms: Some(25),
            // SPEC-032 Item 18, 22: Token metrics and model lineage
            tokens_used: Some(124),
            tokens_per_second: Some(248.0),
            llm_provider: Some("ollama".to_string()),
            llm_model: Some("gemma3:12b".to_string()),
        };
        let json = serde_json::to_value(&stats).unwrap();
        assert_eq!(json["total_time_ms"], 650);
        assert_eq!(json["sources_retrieved"], 5);
        assert_eq!(json["rerank_time_ms"], 25);
        // SPEC-032: Verify new fields
        assert_eq!(json["tokens_used"], 124);
        assert_eq!(json["tokens_per_second"], 248.0);
        assert_eq!(json["llm_provider"], "ollama");
        assert_eq!(json["llm_model"], "gemma3:12b");
    }

    #[test]
    fn test_query_response_serialization() {
        let response = QueryResponse {
            answer: "RAG is Retrieval Augmented Generation".to_string(),
            mode: "hybrid".to_string(),
            sources: vec![],
            stats: QueryStats {
                embedding_time_ms: 10,
                retrieval_time_ms: 20,
                generation_time_ms: 100,
                total_time_ms: 130,
                sources_retrieved: 0,
                rerank_time_ms: None,
                // SPEC-032 Item 18, 22: Token metrics and model lineage (optional in test)
                tokens_used: None,
                tokens_per_second: None,
                llm_provider: None,
                llm_model: None,
            },
            conversation_id: None,
            reranked: false,
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["mode"], "hybrid");
        assert!(json.get("conversation_id").is_none());
        assert!(json.get("reranked").is_none()); // skip_serializing_if
    }
}
