//! Shared query types: request, response, stats, conversation history.
//!
//! # WHY THIS EXISTS (P-G6a / RC-11)
//!
//! Before this module these types lived in `engine.rs` alongside the legacy
//! `QueryEngine` struct. The legacy struct is dead (production routes through
//! `QueryEngine`), so `engine.rs` is being removed. The request/response
//! types, however, are the *contract* between the API layer and the query
//! engine and must survive the deletion. Hoisting them into a dedicated
//! `types` module keeps that contract explicit and decouples it from any
//! particular engine implementation (DRY: one definition, imported by every
//! engine and every caller).
//!
//! First principle: the query *protocol* (what a caller asks and what the
//! engine returns) is a more stable abstraction than the engine that
//! implements it. The protocol therefore owns its own module.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::context::QueryContext;
use crate::mix_weights::MixWeightOverride;
use crate::modes::QueryMode;

/// A query request — the caller-facing contract for asking the engine a
/// question.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    /// The query text.
    pub query: String,

    /// Query mode override.
    pub mode: Option<QueryMode>,

    /// Maximum results.
    pub max_results: Option<usize>,

    /// Whether to only retrieve context (no LLM generation).
    pub context_only: bool,

    /// Whether to return the formatted prompt instead of calling LLM.
    /// Useful for debugging or using your own LLM.
    pub prompt_only: bool,

    /// Additional parameters.
    pub params: HashMap<String, serde_json::Value>,

    /// Conversation history for multi-turn context.
    #[serde(default)]
    pub conversation_history: Vec<ConversationMessage>,

    /// Override: enable or disable reranking for this request.
    #[serde(default)]
    pub enable_rerank: Option<bool>,

    /// Override: rerank top K results.
    #[serde(default)]
    pub rerank_top_k: Option<usize>,

    /// Override: LLM provider to use for answer generation.
    /// Format: provider name (e.g., "ollama", "openai", "lmstudio").
    /// If not provided, uses the server default.
    /// @implements SPEC-032: Provider selection at query time
    #[serde(default)]
    pub llm_provider: Option<String>,

    /// Override: LLM model to use for answer generation.
    /// If not provided, uses the provider's default model.
    /// @implements SPEC-032: Model selection at query time
    #[serde(default)]
    pub llm_model: Option<String>,

    /// Optional system prompt extension injected between instructions and context.
    /// Extends (not replaces) the base RAG prompt with additional instructions.
    /// @implements SPEC-004: System prompt extension point
    #[serde(default)]
    pub system_prompt: Option<String>,

    /// Pre-resolved document IDs that match the user's date/pattern filters.
    /// When set, only chunks/entities/relationships from these documents are included
    /// in the query context. Resolved by the API layer from DocumentFilter criteria.
    /// @implements SPEC-005: Document date and pattern filters
    #[serde(default)]
    pub allowed_document_ids: Option<Vec<String>>,

    /// Optional images to include with the query (multimodal vision queries).
    /// Each entry is a base64-encoded image with its MIME type.
    /// When set, the engine forwards images to the vision-capable LLM.
    /// @implements FEAT0240: Image attachment in chat
    #[serde(default)]
    pub images: Option<Vec<edgequake_llm::traits::ImageData>>,

    /// Per-request Mix mode weight overrides (SPEC-022 P-H6).
    /// Unset fields inherit from `QueryEngineConfig` defaults (1.0 each).
    #[serde(default)]
    pub mix_weights: Option<MixWeightOverride>,
}

/// A single message in conversation history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    /// Role of the message sender (user, assistant, system).
    pub role: String,

    /// Content of the message.
    pub content: String,
}

impl QueryRequest {
    /// Create a new query request.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            mode: None,
            max_results: None,
            context_only: false,
            prompt_only: false,
            params: HashMap::new(),
            conversation_history: Vec::new(),
            enable_rerank: None,
            rerank_top_k: None,
            llm_provider: None,
            llm_model: None,
            system_prompt: None,
            allowed_document_ids: None,
            images: None,
            mix_weights: None,
        }
    }

    /// Set the query mode.
    pub fn with_mode(mut self, mode: QueryMode) -> Self {
        self.mode = Some(mode);
        self
    }

    /// Set context-only mode.
    pub fn context_only(mut self) -> Self {
        self.context_only = true;
        self
    }

    /// Set prompt-only mode.
    pub fn prompt_only(mut self) -> Self {
        self.prompt_only = true;
        self
    }

    /// Add conversation history.
    pub fn with_conversation_history(mut self, history: Vec<ConversationMessage>) -> Self {
        self.conversation_history = history;
        self
    }

    /// Set the LLM provider override for answer generation.
    /// Format: provider name (e.g., "ollama", "openai", "lmstudio").
    /// @implements SPEC-032: Provider selection at query time
    pub fn with_llm_provider(mut self, provider: impl Into<String>) -> Self {
        self.llm_provider = Some(provider.into());
        self
    }

    /// Set the LLM model override for answer generation.
    /// @implements SPEC-032: Model selection at query time
    pub fn with_llm_model(mut self, model: impl Into<String>) -> Self {
        self.llm_model = Some(model.into());
        self
    }

    /// Set the system prompt extension for this query.
    /// @implements SPEC-004: System prompt extension point
    pub fn with_system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(system_prompt.into());
        self
    }

    /// Set both LLM provider and model from a full model ID.
    /// Format: "provider/model" (e.g., "ollama/gemma3:12b").
    /// @implements SPEC-032: Full model ID parsing
    pub fn with_llm_full_id(mut self, full_id: impl AsRef<str>) -> Self {
        let full_id = full_id.as_ref();
        if let Some((provider, model)) = full_id.split_once('/') {
            self.llm_provider = Some(provider.to_string());
            self.llm_model = Some(model.to_string());
        } else {
            // No slash - treat as provider only
            self.llm_provider = Some(full_id.to_string());
        }
        self
    }

    /// Set tenant ID for filtering.
    pub fn with_tenant_id(mut self, tenant_id: impl Into<String>) -> Self {
        self.params
            .insert("tenant_id".to_string(), serde_json::json!(tenant_id.into()));
        self
    }

    /// Set workspace ID for filtering.
    pub fn with_workspace_id(mut self, workspace_id: impl Into<String>) -> Self {
        self.params.insert(
            "workspace_id".to_string(),
            serde_json::json!(workspace_id.into()),
        );
        self
    }

    /// Get tenant ID from params.
    pub fn tenant_id(&self) -> Option<String> {
        self.params
            .get("tenant_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Get workspace ID from params.
    pub fn workspace_id(&self) -> Option<String> {
        self.params
            .get("workspace_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Override reranking for this request.
    pub fn with_rerank(mut self, enable: bool) -> Self {
        self.enable_rerank = Some(enable);
        self
    }

    /// Set the rerank top K for this request.
    pub fn with_rerank_top_k(mut self, top_k: usize) -> Self {
        self.rerank_top_k = Some(top_k);
        self
    }

    /// Set pre-resolved document IDs for filtering.
    /// Only chunks/entities/relationships from these documents will be included.
    /// @implements SPEC-005: Document date and pattern filters
    pub fn with_allowed_document_ids(mut self, ids: Vec<String>) -> Self {
        self.allowed_document_ids = Some(ids);
        self
    }

    /// Attach images for a multimodal (vision) query.
    pub fn with_images(mut self, images: Vec<edgequake_llm::traits::ImageData>) -> Self {
        self.images = Some(images);
        self
    }
}

/// A query response — the engine's result contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    /// The generated answer.
    pub answer: String,

    /// Query context used for the answer.
    pub context: QueryContext,

    /// Query mode used.
    pub mode: QueryMode,

    /// Processing statistics.
    pub stats: QueryStats,
}

/// Query processing statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryStats {
    /// Time for embedding generation (ms).
    pub embedding_time_ms: u64,

    /// Time for retrieval (ms).
    pub retrieval_time_ms: u64,

    /// Time for LLM generation (ms).
    pub generation_time_ms: u64,

    /// Total time (ms).
    pub total_time_ms: u64,

    /// Number of tokens in the context.
    pub context_tokens: usize,

    /// Number of tokens generated.
    pub generated_tokens: usize,

    /// Time spent in reranking (ms), when a reranker was applied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rerank_time_ms: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_request_builder() {
        let request = QueryRequest::new("What is Rust?")
            .with_mode(QueryMode::Local)
            .context_only();

        assert_eq!(request.query, "What is Rust?");
        assert_eq!(request.mode, Some(QueryMode::Local));
        assert!(request.context_only);
        assert!(!request.prompt_only);
        assert!(request.system_prompt.is_none());

        // Test prompt_only mode
        let prompt_request = QueryRequest::new("What is Python?").prompt_only();

        assert!(prompt_request.prompt_only);
        assert!(!prompt_request.context_only);
    }

    /// @implements SPEC-004: system prompt builder test
    #[test]
    fn test_query_request_with_system_prompt() {
        let request =
            QueryRequest::new("Tell me about Rust").with_system_prompt("Always respond in French");

        assert_eq!(
            request.system_prompt.as_deref(),
            Some("Always respond in French")
        );

        // Default should be None
        let default_request = QueryRequest::new("Tell me about Rust");
        assert!(default_request.system_prompt.is_none());
    }

    /// @implements SPEC-004: system prompt serialization round-trip
    #[test]
    fn test_query_request_system_prompt_serde() {
        let request = QueryRequest::new("query").with_system_prompt("Be concise");

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"system_prompt\":\"Be concise\""));

        let deserialized: QueryRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.system_prompt.as_deref(), Some("Be concise"));

        // system_prompt should round-trip through serde
        let without_sp = QueryRequest::new("query");
        let json_without = serde_json::to_string(&without_sp).unwrap();
        let deserialized: QueryRequest = serde_json::from_str(&json_without).unwrap();
        assert!(deserialized.system_prompt.is_none());
    }
}
