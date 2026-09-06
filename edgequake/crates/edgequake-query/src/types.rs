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

fn is_zero_u64(v: &u64) -> bool {
    *v == 0
}

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

    /// Pre-supplied high-level keywords (LightRAG `QueryParam.hl_keywords`).
    /// When either hl or ll is non-empty, keyword LLM extraction is skipped (083).
    #[serde(default)]
    pub hl_keywords: Option<Vec<String>>,

    /// Pre-supplied low-level keywords (LightRAG `QueryParam.ll_keywords`).
    #[serde(default)]
    pub ll_keywords: Option<Vec<String>>,

    /// Answer formatting cue (LightRAG `QueryParam.response_type`).
    /// Default when unset: `"Multiple Paragraphs"`.
    #[serde(default)]
    pub response_type: Option<String>,

    /// SPEC-109: effective reasoning effort for answer generation (post-clamp).
    /// When `Some`, forwarded on `CompletionOptions.reasoning_effort` and
    /// included in the SPEC-103 answer-cache hash.
    #[serde(default)]
    pub reasoning_effort: Option<String>,

    /// SPEC-146: cache isolation — principal key (`user:uuid` / `apikey:…`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authz_principal: Option<String>,

    /// SPEC-146: workspace policy generation for cache / allow-set invalidation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_generation: Option<u64>,

    /// SPEC-146: fingerprint of the document allow-set (never None under ABAC).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_fingerprint: Option<String>,
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
            hl_keywords: None,
            ll_keywords: None,
            response_type: None,
            reasoning_effort: None,
            authz_principal: None,
            policy_generation: None,
            allow_fingerprint: None,
        }
    }

    /// 083: LightRAG-shaped keyword override — skip KEYWORD LLM when either list is non-empty.
    pub fn has_keyword_override(&self) -> bool {
        self.hl_keywords
            .as_ref()
            .is_some_and(|v| v.iter().any(|s| !s.trim().is_empty()))
            || self
                .ll_keywords
                .as_ref()
                .is_some_and(|v| v.iter().any(|s| !s.trim().is_empty()))
    }

    /// Cleaned hl/ll lists for override (empty strings dropped).
    pub fn keyword_override_lists(&self) -> Option<(Vec<String>, Vec<String>)> {
        if !self.has_keyword_override() {
            return None;
        }
        let clean = |v: &Option<Vec<String>>| -> Vec<String> {
            v.as_ref()
                .map(|items| {
                    items
                        .iter()
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_default()
        };
        Some((clean(&self.hl_keywords), clean(&self.ll_keywords)))
    }

    pub fn with_hl_keywords(mut self, keywords: Vec<String>) -> Self {
        self.hl_keywords = Some(keywords);
        self
    }

    pub fn with_ll_keywords(mut self, keywords: Vec<String>) -> Self {
        self.ll_keywords = Some(keywords);
        self
    }

    pub fn with_response_type(mut self, response_type: impl Into<String>) -> Self {
        self.response_type = Some(response_type.into());
        self
    }

    /// LightRAG default `"Multiple Paragraphs"` when unset/blank.
    pub fn response_type_or_default(&self) -> &str {
        self.response_type
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("Multiple Paragraphs")
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

    /// SPEC-109: set effective reasoning effort for answer generation.
    pub fn with_reasoning_effort(mut self, effort: impl Into<String>) -> Self {
        let s = effort.into();
        let trimmed = s.trim();
        self.reasoning_effort = if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        };
        self
    }

    /// Set the system prompt extension for this query.
    /// @implements SPEC-004: System prompt extension point
    pub fn with_system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(system_prompt.into());
        self
    }

    /// Optional question-type label (e.g. GraphRAG-Bench "Complex Reasoning").
    /// Stored in `params["question_type"]` for answer-prompt scoping (047).
    pub fn with_question_type(mut self, question_type: impl Into<String>) -> Self {
        self.params.insert(
            "question_type".to_string(),
            serde_json::json!(question_type.into()),
        );
        self
    }

    /// Read `params["question_type"]` when present and non-empty.
    pub fn question_type(&self) -> Option<&str> {
        self.params
            .get("question_type")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
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

    /// SPEC-146: attach authz cache isolation fields.
    pub fn with_authz_cache_scope(
        mut self,
        principal: impl Into<String>,
        policy_generation: u64,
        allow_fingerprint: impl Into<String>,
    ) -> Self {
        self.authz_principal = Some(principal.into());
        self.policy_generation = Some(policy_generation);
        self.allow_fingerprint = Some(allow_fingerprint.into());
        self
    }

    /// SPEC-146: build AuthzCacheScope when principal + fingerprint present.
    pub fn authz_cache_scope(&self) -> Option<crate::cache::AuthzCacheScope> {
        let principal = self.authz_principal.as_ref()?;
        let fp = self.allow_fingerprint.as_ref()?;
        Some(crate::cache::AuthzCacheScope::from_parts(
            principal.clone(),
            self.policy_generation.unwrap_or(0),
            fp.clone(),
        ))
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

    /// SPEC-083 X-21: lightweight explainability derived from [`QueryStats`] arms.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explain: Option<ExplainTrace>,
}

/// Minimal retrieval explainability surface (X-21 MVP).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExplainTrace {
    /// Query mode label (`mix`, `hybrid`, `local`, …).
    pub mode: String,
    /// Comma-separated arms that ran (`local,global`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arms_run: Option<String>,
    /// Sparse fusion / FTS outcome label when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sparse_outcome: Option<String>,
    /// Intent label when keyword/intent extraction ran.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_intent: Option<String>,
}

impl ExplainTrace {
    /// Build from mode + absorbed [`QueryStats`] arm metadata.
    pub fn from_stats(mode: &QueryMode, stats: &QueryStats) -> Self {
        Self {
            mode: mode.to_string(),
            arms_run: stats.arms_run.clone(),
            sparse_outcome: stats.sparse_outcome.clone(),
            query_intent: stats.query_intent.clone(),
        }
    }
}

/// Query processing statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryStats {
    /// Time for embedding generation (ms) — pure embed path only (059).
    /// Does **not** include keyword LLM (see [`keyword_time_ms`]).
    pub embedding_time_ms: u64,

    /// Time for keyword extraction LLM / heuristic (ms). 059 C1b honesty:
    /// historically folded into `embedding_time_ms` and inflated "embed" ~2.5s.
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub keyword_time_ms: u64,

    /// Time for retrieval (ms).
    pub retrieval_time_ms: u64,

    /// Time for LLM generation (ms).
    pub generation_time_ms: u64,

    /// Time to first token from generation start (ms). Stream path / 064 UX.
    /// Unset for non-streaming `complete` unless answer-cache short-circuit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttft_ms: Option<u64>,

    /// True when answer served from product answer cache (064 / SPEC-103).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub answer_cache_hit: bool,

    /// True when keywords served from LLM response cache (SPEC-103 LAW-C8).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub keyword_cache_hit: bool,

    /// Total time (ms).
    pub total_time_ms: u64,

    /// Number of tokens in the context.
    pub context_tokens: usize,

    /// Number of tokens generated.
    pub generated_tokens: usize,

    /// Time spent in reranking (ms), when a reranker was applied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rerank_time_ms: Option<u64>,

    /// Per-arm wall time for Mix/Hybrid local retrieval (ms).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm_local_ms: Option<u64>,

    /// Per-arm wall time for Mix/Hybrid global retrieval (ms).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm_global_ms: Option<u64>,

    /// Per-arm wall time for Mix/Hybrid naive retrieval (ms).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm_naive_ms: Option<u64>,

    /// Chunks contributed by the local arm before merge (Mix/Hybrid).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm_local_chunks: Option<usize>,

    /// Chunks contributed by the global arm before merge (Mix/Hybrid).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm_global_chunks: Option<usize>,

    /// Chunks contributed by the naive arm before merge (Mix/Hybrid).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm_naive_chunks: Option<usize>,

    /// Comma-separated arms that actually ran (e.g. `"local,global"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arms_run: Option<String>,

    /// True when intent/weight gating skipped at least one arm.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arms_gated: Option<bool>,

    /// True when retrieval returned no chunks/entities/relationships.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub context_empty: bool,

    /// True when post-retrieval truncation removed context items.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub context_truncated: bool,

    /// True when local/global used popular-node graph fallback (OPS-P2.14).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub popular_node_fallback: bool,

    /// Arm that triggered popular-node fallback (`local` | `global`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub popular_node_arm: Option<String>,

    /// Sparse fusion path label (`postgres_fts`, `in_memory_bm25`, …).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sparse_outcome: Option<String>,

    /// True when FTS path degraded to BM25/vector-only (OPS-P2.15).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub fts_fallback: bool,

    /// True when chart-modality pre-filter was applied (SPEC-047 MV-32).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub chart_modality_filter: bool,

    /// Retrieved chunks tagged `modality=chart` (SPEC-047 MV-32).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retrieved_chart_chunks: Option<usize>,

    /// Optional online faithfulness sample score in `[0, 1]` (OPS-P2.20).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faithfulness_score: Option<f32>,

    /// LLM / heuristic query intent used for truncation + Mix gating (022 P3a).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_intent: Option<String>,

    /// SPEC-109: effective reasoning effort used for answer generation (when set).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
}

impl QueryStats {
    /// Copy Mix/Hybrid arm timing + OPS-P2 retrieval telemetry from context.
    pub fn absorb_arm_metadata(&mut self, context: &crate::context::QueryContext) {
        use crate::mix_weights::{
            META_ARMS_GATED, META_ARMS_RUN, META_ARM_GLOBAL_CHUNKS, META_ARM_GLOBAL_MS,
            META_ARM_LOCAL_CHUNKS, META_ARM_LOCAL_MS, META_ARM_NAIVE_CHUNKS, META_ARM_NAIVE_MS,
        };
        use crate::retrieval_telemetry::{
            META_CHART_MODALITY_FILTER, META_FTS_FALLBACK, META_POPULAR_NODE_ARM,
            META_POPULAR_NODE_FALLBACK, META_RETRIEVED_CHART_CHUNKS, META_SPARSE_OUTCOME,
        };
        self.arm_local_ms = context
            .metadata
            .get(META_ARM_LOCAL_MS)
            .and_then(|v| v.as_u64());
        self.arm_global_ms = context
            .metadata
            .get(META_ARM_GLOBAL_MS)
            .and_then(|v| v.as_u64());
        self.arm_naive_ms = context
            .metadata
            .get(META_ARM_NAIVE_MS)
            .and_then(|v| v.as_u64());
        self.arm_local_chunks = context
            .metadata
            .get(META_ARM_LOCAL_CHUNKS)
            .and_then(|v| v.as_u64().map(|n| n as usize));
        self.arm_global_chunks = context
            .metadata
            .get(META_ARM_GLOBAL_CHUNKS)
            .and_then(|v| v.as_u64().map(|n| n as usize));
        self.arm_naive_chunks = context
            .metadata
            .get(META_ARM_NAIVE_CHUNKS)
            .and_then(|v| v.as_u64().map(|n| n as usize));
        self.arms_run = context
            .metadata
            .get(META_ARMS_RUN)
            .and_then(|v| v.as_str().map(str::to_string));
        self.arms_gated = context
            .metadata
            .get(META_ARMS_GATED)
            .and_then(|v| v.as_bool());
        self.popular_node_fallback = context
            .metadata
            .get(META_POPULAR_NODE_FALLBACK)
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        self.popular_node_arm = context
            .metadata
            .get(META_POPULAR_NODE_ARM)
            .and_then(|v| v.as_str().map(str::to_string));
        self.sparse_outcome = context
            .metadata
            .get(META_SPARSE_OUTCOME)
            .and_then(|v| v.as_str().map(str::to_string));
        self.fts_fallback = context
            .metadata
            .get(META_FTS_FALLBACK)
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        self.chart_modality_filter = context
            .metadata
            .get(META_CHART_MODALITY_FILTER)
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        self.retrieved_chart_chunks = context
            .metadata
            .get(META_RETRIEVED_CHART_CHUNKS)
            .and_then(|v| v.as_u64())
            .map(|n| n as usize);
        self.query_intent = context
            .metadata
            .get("query_intent")
            .and_then(|v| v.as_str().map(str::to_string));
        self.context_empty = context.chunks.is_empty()
            && context.entities.is_empty()
            && context.relationships.is_empty();
        self.context_truncated = context.is_truncated;
    }
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

    #[test]
    fn keyword_override_and_response_type_serde() {
        let request = QueryRequest::new("staging for NSCLC")
            .with_hl_keywords(vec!["staging".into(), "NSCLC".into()])
            .with_ll_keywords(vec!["TNM".into()])
            .with_response_type("Bullet Points");
        assert!(request.has_keyword_override());
        assert_eq!(request.response_type_or_default(), "Bullet Points");
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("hl_keywords"));
        assert!(json.contains("ll_keywords"));
        assert!(json.contains("Bullet Points"));
        let back: QueryRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(
            back.keyword_override_lists(),
            Some((vec!["staging".into(), "NSCLC".into()], vec!["TNM".into()]))
        );
        assert!(!QueryRequest::new("q").has_keyword_override());
        assert_eq!(
            QueryRequest::new("q").response_type_or_default(),
            "Multiple Paragraphs"
        );
    }

    #[test]
    fn absorb_arm_metadata_from_context() {
        use crate::mix_weights::{
            META_ARMS_GATED, META_ARMS_RUN, META_ARM_NAIVE_CHUNKS, META_ARM_NAIVE_MS,
        };
        let mut ctx = QueryContext::new();
        ctx.metadata
            .insert(META_ARM_NAIVE_MS.into(), serde_json::json!(12u64));
        ctx.metadata
            .insert(META_ARM_NAIVE_CHUNKS.into(), serde_json::json!(7u64));
        ctx.metadata
            .insert(META_ARMS_RUN.into(), serde_json::json!("naive"));
        ctx.metadata
            .insert(META_ARMS_GATED.into(), serde_json::json!(true));
        let mut stats = QueryStats::default();
        stats.absorb_arm_metadata(&ctx);
        assert_eq!(stats.arm_naive_ms, Some(12));
        assert_eq!(stats.arm_naive_chunks, Some(7));
        assert_eq!(stats.arms_run.as_deref(), Some("naive"));
        assert_eq!(stats.arms_gated, Some(true));
        assert!(stats.context_empty);
    }

    #[test]
    fn contract_explain_trace_on_query_response() {
        let stats = QueryStats {
            arms_run: Some("local,global".into()),
            ..Default::default()
        };
        let explain = ExplainTrace::from_stats(&QueryMode::Mix, &stats);
        assert_eq!(explain.mode, "mix");
        assert_eq!(explain.arms_run.as_deref(), Some("local,global"));
        let resp = QueryResponse {
            answer: String::new(),
            context: QueryContext::new(),
            mode: QueryMode::Mix,
            stats,
            explain: Some(explain),
        };
        assert!(resp.explain.is_some());
    }

    #[test]
    fn e2e_query_vec_matches_question_only_embedding() {
        // D-38: pipeline embeds request.query only (not conversation history).
        let src = include_str!("engine_impl/query_entry/query_pipeline.rs");
        assert!(src.contains("D-38"));
        assert!(src.contains("embed_one(&request.query)"));
        assert!(src.contains("peek_cached"));
        assert!(
            !src.contains("embed_one(&keyword_query)") && !src.contains("embed_one(&conversation"),
            "must not embed conversation/keyword blob as query_vec"
        );
    }
}
