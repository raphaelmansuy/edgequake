//! Query DTO types.
//!
//! This module contains all Data Transfer Objects for the query API.
//! Extracted from query.rs for modularity and single responsibility.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::context_types::ContentGranularity;

// ============================================================================
// Default value helper functions
// ============================================================================

/// Default enable reranking (true).
pub fn default_enable_rerank() -> bool {
    true
}

fn default_include_subgraph() -> bool {
    true
}

fn default_content_granularity() -> ContentGranularity {
    ContentGranularity::Citation
}

// ============================================================================
// Request DTOs
// ============================================================================

/// Document filter criteria for narrowing query scope.
///
/// Fields are AND-combined across types: date range AND (document_ids OR document_pattern).
/// Within `document_ids` and `document_pattern`, matches are OR-unioned.
///
/// @implements SPEC-005: Document date and pattern filters
/// @implements SPEC-031: Explicit document scope selection
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Default)]
pub struct DocumentFilter {
    /// Start date (inclusive) in ISO 8601 format (e.g., "2025-01-01T00:00:00Z").
    /// Only documents created on or after this date are included.
    #[serde(default)]
    pub date_from: Option<String>,

    /// End date (inclusive) in ISO 8601 format (e.g., "2025-12-31T23:59:59Z").
    /// Only documents created on or before this date are included.
    #[serde(default)]
    pub date_to: Option<String>,

    /// Case-insensitive substring pattern to match against document titles.
    /// Comma-separated values are treated as OR conditions.
    /// Example: "report,summary" matches documents containing "report" OR "summary".
    #[serde(default)]
    pub document_pattern: Option<String>,

    /// Explicit document IDs to restrict query scope.
    ///
    /// When set, only these documents contribute RAG context, subject to any
    /// active date_from/date_to constraints (AND logic across field types).
    /// Union with document_pattern when both are set (OR membership logic).
    ///
    /// An empty list `[]` is treated identically to `null` (no filtering).
    /// IDs not present in the workspace are silently ignored.
    ///
    /// @implements SPEC-031: Explicit document scope selection
    #[serde(default)]
    pub document_ids: Option<Vec<String>>,

    /// SPEC-180: keep chunks of these document kinds (`pdf`, `image`, `markdown`,
    /// `text`, `office`, `other`). Normal workspaces only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc_kinds: Option<Vec<String>>,

    /// SPEC-180: keep chunks of documents with these MIME types (exact, case-insensitive).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_types: Option<Vec<String>>,

    /// SPEC-180: keep chunks of this source modality (`text` or `image`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_modalities: Option<Vec<String>>,
}

impl DocumentFilter {
    /// SPEC-180: true when any chunk-kind predicate is set.
    pub fn has_kind_filters(&self) -> bool {
        self.doc_kinds.is_some() || self.mime_types.is_some() || self.source_modalities.is_some()
    }

    /// Returns true when no filter criteria are active (all-pass, no KV scan needed).
    pub fn is_empty(&self) -> bool {
        self.date_from.is_none()
            && self.date_to.is_none()
            && self.document_pattern.is_none()
            && self.document_ids.as_ref().is_none_or(|ids| ids.is_empty())
    }
}

/// A single message in the conversation history.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ConversationMessage {
    /// Role of the message sender (user or assistant).
    pub role: String,

    /// Content of the message.
    pub content: String,
}

/// Per-request Mix mode weight overrides (SPEC-022 P-H6).
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct MixWeightRequest {
    #[serde(default)]
    pub local: Option<f32>,
    #[serde(default)]
    pub global: Option<f32>,
    #[serde(default)]
    pub naive: Option<f32>,
}

impl MixWeightRequest {
    pub(crate) fn to_engine_override(&self) -> edgequake_query::MixWeightOverride {
        edgequake_query::MixWeightOverride {
            local: self.local,
            global: self.global,
            naive: self.naive,
        }
    }

    pub(crate) fn is_set(&self) -> bool {
        self.local.is_some() || self.global.is_some() || self.naive.is_some()
    }
}

/// Query request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct QueryRequest {
    /// The query text.
    pub query: String,

    /// Query mode: `naive` | `local` | `global` | `hybrid` | `mix` | `bypass`.
    ///
    /// EdgeQuake semantics (not identical to LightRAG namesakes):
    /// - `hybrid` — local ∥ global ∥ naive (round-robin or RRF via env)
    /// - `mix` — same three arms with weighted/RRF fusion + optional intent arm gate
    ///   (production default). LightRAG `hybrid` is local+global only.
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

    /// Include structured query-matched graph (entities + relationships) in the response.
    #[serde(default = "default_include_subgraph")]
    pub include_subgraph: bool,

    /// Maximum number of results.
    #[serde(default)]
    pub max_results: Option<usize>,

    /// Conversation history for multi-turn context.
    #[serde(default)]
    pub conversation_history: Option<Vec<ConversationMessage>>,

    /// Optional Langfuse / GenAI session id (SPEC-124).
    ///
    /// When set, exported OTEL spans carry `langfuse.session.id` /
    /// `gen_ai.conversation.id`. Never invent — omit when no durable session exists.
    /// Chat uses `conversation_id` instead; this is for direct `/query` clients.
    #[serde(default)]
    pub session_id: Option<String>,

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

    /// SPEC-109: reasoning effort override for this query (`none`/`minimal`/`low`/…).
    /// Omit or null = Auto (inherit workspace query role / provider default).
    #[serde(default)]
    pub reasoning_effort: Option<String>,

    /// Optional system prompt extension injected into the LLM prompt.
    /// Extends (not replaces) the base RAG prompt with additional instructions.
    /// @implements SPEC-004: System prompt extension point
    #[serde(default)]
    pub system_prompt: Option<String>,

    /// Optional question-type label (e.g. GraphRAG-Bench `Complex Reasoning`).
    /// Forwarded to the engine for type-scoped answer prompts (047).
    #[serde(default)]
    pub question_type: Option<String>,

    /// Optional document filter to narrow query scope by date range or name pattern.
    /// When set, only chunks/entities from matching documents are used in retrieval.
    /// @implements SPEC-005: Document date and pattern filters
    #[serde(default)]
    pub document_filter: Option<DocumentFilter>,

    /// Per-request Mix mode weight overrides (SPEC-022 P-H6).
    /// Example: `{"local": 0, "global": 0, "naive": 1}` for naive-only blend.
    #[serde(default)]
    pub mix_weights: Option<MixWeightRequest>,

    /// Optional HTTP headers to propagate to the upstream LLM provider call.
    ///
    /// Useful for B2B / multi-tenant deployments where the caller needs to pass
    /// tracing or identity metadata (e.g. `x-request-id`, `x-tenant-id`,
    /// `x-correlation-id`, `traceparent`, HMAC tokens) through to the LLM API.
    ///
    /// Reserved headers (`authorization`, `x-api-key`, `anthropic-version`,
    /// `content-type`, `content-length`, `host`, `user-agent`) are silently
    /// dropped by the provider to prevent accidental credential overrides.
    ///
    /// Only providers that support `with_extra_headers()` will forward these
    /// (openai-compatible, anthropic, gemini, vertexai, mistral, nvidia).
    /// Other providers silently ignore this field.
    #[serde(default)]
    pub extra_headers: Option<std::collections::HashMap<String, String>>,

    /// Payload tier for source snippets: citation (200 chars) | agent (full chunk) | debug.
    /// @implements SPEC-037 + SPEC-028
    #[serde(default = "default_content_granularity")]
    pub content_granularity: ContentGranularity,

    /// Pre-supplied high-level keywords (LightRAG `hl_keywords`). 083: skips keyword LLM when either hl or ll is non-empty.
    #[serde(default)]
    pub hl_keywords: Option<Vec<String>>,

    /// Pre-supplied low-level keywords (LightRAG `ll_keywords`).
    #[serde(default)]
    pub ll_keywords: Option<Vec<String>>,

    /// Answer formatting cue (LightRAG `response_type`). Default: Multiple Paragraphs.
    #[serde(default)]
    pub response_type: Option<String>,
}

/// Streaming query request.
///
/// @implements SPEC-006: Unified streaming protocol
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct StreamQueryRequest {
    /// The query text.
    pub query: String,

    /// Query mode.
    #[serde(default)]
    pub mode: Option<String>,

    /// Optional system prompt extension injected into the LLM prompt.
    /// @implements SPEC-004: System prompt extension point
    #[serde(default)]
    pub system_prompt: Option<String>,

    /// Optional question-type label for type-scoped answer prompts (047).
    #[serde(default)]
    pub question_type: Option<String>,

    /// Pre-supplied high-level keywords (083 / LightRAG).
    #[serde(default)]
    pub hl_keywords: Option<Vec<String>>,

    /// Pre-supplied low-level keywords (083 / LightRAG).
    #[serde(default)]
    pub ll_keywords: Option<Vec<String>>,

    /// Answer formatting cue (083 / LightRAG `response_type`).
    #[serde(default)]
    pub response_type: Option<String>,

    /// Optional document filter to narrow query scope by date range or name pattern.
    /// @implements SPEC-005 + SPEC-006: Document filters for streaming queries
    #[serde(default)]
    pub document_filter: Option<DocumentFilter>,

    /// LLM provider to use for this query (e.g., "openai", "ollama", "lmstudio").
    /// @implements SPEC-006 + SPEC-032: Provider selection in streaming queries
    #[serde(default)]
    pub llm_provider: Option<String>,

    /// Specific model name within the provider.
    /// @implements SPEC-006 + SPEC-032: Model selection in streaming queries
    #[serde(default)]
    pub llm_model: Option<String>,

    /// SPEC-109: reasoning effort override for streaming query.
    #[serde(default)]
    pub reasoning_effort: Option<String>,

    /// SPEC-175 EC-175-09: rejected with 422 `rerank_model_rejected`. Rerank is deployment-level.
    #[serde(default)]
    pub rerank_model: Option<String>,

    /// Stream format version: "v1" (raw text) or "v2" (structured JSON events, default).
    /// @implements SPEC-006: Backward compatibility
    #[serde(default)]
    pub stream_format: Option<String>,

    /// Optional HTTP headers to propagate to the upstream LLM provider call.
    ///
    /// Enables B2B / multi-tenant metadata (`x-request-id`, `x-tenant-id`,
    /// `x-correlation-id`, `traceparent`, HMAC tokens) to flow through to the
    /// LLM API on streaming queries. Same semantics as `QueryRequest.extra_headers`.
    #[serde(default)]
    pub extra_headers: Option<std::collections::HashMap<String, String>>,

    /// Include structured query-matched graph in stream context events (v2+).
    #[serde(default = "default_include_subgraph")]
    pub include_subgraph: bool,

    /// Payload tier for source snippets in context events.
    /// @implements SPEC-037 + SPEC-028
    #[serde(default = "default_content_granularity")]
    pub content_granularity: ContentGranularity,

    /// Optional Langfuse / GenAI session id (SPEC-124). Mirrors `QueryRequest.session_id`.
    #[serde(default)]
    pub session_id: Option<String>,
}

// ============================================================================
// Streaming Event Types (SPEC-006)
// ============================================================================

/// Streaming SSE event types for the query endpoint.
///
/// @implements SPEC-006: Unified streaming protocol for /query/stream
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QueryStreamEvent {
    /// Context/sources retrieved before generation starts.
    Context {
        sources: Vec<SourceReference>,
        query_mode: String,
        retrieval_time_ms: u64,
        /// SPEC-028 FP-028-09: structured graph on v2 stream (when bundle absent).
        #[serde(skip_serializing_if = "Option::is_none")]
        subgraph: Option<crate::handlers::context_types::SubgraphBundle>,
        /// SPEC-028: Full structured bundle (stream_format=v3 only).
        #[serde(skip_serializing_if = "Option::is_none")]
        bundle: Option<crate::handlers::context_types::ContextBundle>,
    },

    /// Token generated during LLM streaming.
    Token { content: String },

    /// Chain-of-thought reasoning content.
    Thinking { content: String },

    /// Stream complete — includes full statistics.
    ///
    /// SPEC-142: `answer` is the verified markdown (page chips + title tooltips)
    /// when sources were available; clients should replace streamed tokens with it.
    Done {
        stats: QueryStreamStats,
        #[serde(skip_serializing_if = "Option::is_none")]
        llm_provider: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        llm_model: Option<String>,
        /// Verified answer with `[p.N](href "Doc")` page chips (SPEC-142).
        #[serde(skip_serializing_if = "Option::is_none")]
        answer: Option<String>,
    },

    /// Error occurred during streaming.
    Error { message: String, code: String },
}

/// Statistics emitted in the `done` event.
///
/// SPEC-083 D-40: stream stats mirror sync [`QueryStats`] arm/timing diagnostics
/// plus stream-only UX fields (`ux_ttft_ms`, `query_mode`). Build via
/// [`QueryStreamStats::from_query_stats`].
///
/// @implements SPEC-006 FR-003: Retrieval statistics in streaming events
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct QueryStreamStats {
    /// Embedding time in ms.
    pub embedding_time_ms: u64,

    /// Keyword extraction time in ms.
    #[serde(default)]
    pub keyword_time_ms: u64,

    /// Retrieval time in ms.
    pub retrieval_time_ms: u64,

    /// Generation time in ms.
    pub generation_time_ms: u64,

    /// LLM time-to-first-token from generation start (ms). 064 UX metric.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttft_ms: Option<u64>,

    /// User-felt TTFT: retrieve start → first token (ms). 064 UX metric.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ux_ttft_ms: Option<u64>,

    /// Total time in ms.
    pub total_time_ms: u64,

    /// Number of sources retrieved.
    pub sources_retrieved: usize,

    /// Rerank time in ms (if reranking was applied).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rerank_time_ms: Option<u64>,

    /// Tokens used for generation (SSE non-optional historical field).
    pub tokens_used: u32,

    /// Tokens per second generation speed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_per_second: Option<f32>,

    /// Query mode used (after adaptive selection).
    pub query_mode: String,

    /// True when answer served from product answer cache.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub answer_cache_hit: bool,

    /// True when keywords served from LLM response cache (SPEC-103).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub keyword_cache_hit: bool,

    /// LLM provider used for generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_provider: Option<String>,

    /// LLM model name used for generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,

    /// True when retrieval returned no chunks/entities/relationships.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub context_empty: bool,

    /// True when post-retrieval truncation removed context items.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub context_truncated: bool,

    /// Chunks the SPEC-184 relevance floor removed (stream mirror of the sync field).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relevance_dropped: Option<usize>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm_local_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm_global_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm_naive_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm_local_chunks: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm_global_chunks: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm_naive_chunks: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arms_run: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arms_gated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_intent: Option<String>,
}

impl QueryStreamStats {
    /// Project sync [`QueryStats`] into stream stats (D-40 SSOT).
    pub fn from_query_stats(
        stats: QueryStats,
        query_mode: String,
        ux_ttft_ms: Option<u64>,
        tokens_used: u32,
    ) -> Self {
        Self {
            embedding_time_ms: stats.embedding_time_ms,
            keyword_time_ms: stats.keyword_time_ms,
            retrieval_time_ms: stats.retrieval_time_ms,
            generation_time_ms: stats.generation_time_ms,
            ttft_ms: stats.ttft_ms,
            ux_ttft_ms,
            total_time_ms: stats.total_time_ms,
            sources_retrieved: stats.sources_retrieved,
            rerank_time_ms: stats.rerank_time_ms,
            tokens_used: if tokens_used > 0 {
                tokens_used
            } else {
                stats.tokens_used.unwrap_or(0) as u32
            },
            tokens_per_second: stats.tokens_per_second,
            query_mode,
            answer_cache_hit: stats.answer_cache_hit,
            keyword_cache_hit: stats.keyword_cache_hit,
            llm_provider: stats.llm_provider,
            llm_model: stats.llm_model,
            context_empty: stats.context_empty,
            context_truncated: stats.context_truncated,
            relevance_dropped: stats.relevance_dropped,
            arm_local_ms: stats.arm_local_ms,
            arm_global_ms: stats.arm_global_ms,
            arm_naive_ms: stats.arm_naive_ms,
            arm_local_chunks: stats.arm_local_chunks,
            arm_global_chunks: stats.arm_global_chunks,
            arm_naive_chunks: stats.arm_naive_chunks,
            arms_run: stats.arms_run,
            arms_gated: stats.arms_gated,
            query_intent: stats.query_intent,
        }
    }
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

    /// Query-matched knowledge graph (entities + relationships).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subgraph: Option<crate::handlers::context_types::SubgraphBundle>,

    /// Query statistics.
    pub stats: QueryStats,

    /// Conversation ID for multi-turn context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,

    /// Whether reranking was applied.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub reranked: bool,

    /// SPEC-083 X-21: retrieval explainability (arms / sparse / intent).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explain: Option<ExplainTraceDto>,

    /// SPEC-124: W3C-style trace id derived from request correlation (Open in Langfuse).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

/// API projection of engine [`edgequake_query::ExplainTrace`].
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ExplainTraceDto {
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arms_run: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sparse_outcome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_intent: Option<String>,
}

impl From<edgequake_query::ExplainTrace> for ExplainTraceDto {
    fn from(t: edgequake_query::ExplainTrace) -> Self {
        Self {
            mode: t.mode,
            arms_run: t.arms_run,
            sparse_outcome: t.sparse_outcome,
            query_intent: t.query_intent,
        }
    }
}

/// A source reference.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SourceReference {
    /// Source type (chunk, entity, relationship).
    pub source_type: String,

    /// Source ID.
    pub id: String,

    /// Relevance score.
    ///
    /// On a Normal workspace this is a **reciprocal rank fusion** value, so it encodes ordering
    /// only: it is roughly `1/(60+rank)` and a perfect match starts no higher than a wild miss.
    /// Use `similarity` to judge absolute relevance; use this only to compare positions.
    pub score: f32,

    /// Absolute similarity to the query (0..1), when one was measured.
    ///
    /// SPEC-183: the dense leg computes this and fusion used to throw it away, which left every
    /// consumer of `score` unable to tell a strong match from a weak one. `None` means "not
    /// measured" (a lexical-only hit, or a conversation stored before this field existed), not
    /// zero.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similarity: Option<f32>,

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

    /// PDF page number (1-indexed) where this chunk starts.
    /// Present only when the source is a PDF with page-aware chunking (SPEC-032).
    /// The UI deep-links with `?page=N` (SPEC-033 / SPEC-142; not `#page=N`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_start: Option<u32>,

    /// PDF page number where this chunk ends (SPEC-135: may exceed page_start).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_end: Option<u32>,

    // ========================================================================
    // SPEC-006: Entity metadata enrichment (FR-002)
    // ========================================================================
    /// Entity type (e.g., "PERSON", "ORGANIZATION"). Only set for source_type="entity".
    /// @implements SPEC-006: Entity metadata enrichment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,

    /// Number of graph connections. Only set for source_type="entity".
    /// @implements SPEC-006: Entity degree in source references
    #[serde(skip_serializing_if = "Option::is_none")]
    pub degree: Option<usize>,

    /// Source chunk IDs where entity was mentioned (provenance). Only set for source_type="entity".
    /// @implements SPEC-006: Entity provenance tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_chunk_ids: Option<Vec<String>>,

    /// Family of the cited document: `pdf`, `image`, `markdown`, `text`, `office`, `other`.
    ///
    /// WHY this is on the source and not derived by the client: `file_path` carries the
    /// document *title* (see `resolve_chunk_file_paths`), so it usually has no extension.
    /// A client that sniffed a suffix to decide "this citation is a picture" therefore
    /// never matched, and the thumbnail grid that depended on that decision never rendered
    /// against a real backend. The document kind is already stored, so the server states it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_kind: Option<String>,

    /// MIME type of the cited document (`image/jpeg`, `application/pdf`, …).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    /// What the chunk was embedded from: `text` or `image` (SPEC-180).
    ///
    /// The precise form of "this citation is a picture": an image document produces one
    /// chunk with `source_modality = image`, and that is what retrieval matched on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_modality: Option<String>,
}

/// Query statistics.
///
/// @implements SPEC-032 Item 18, 22: Token metrics and model lineage
#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct QueryStats {
    /// Embedding time in ms (pure embed path; excludes keyword LLM — 059).
    pub embedding_time_ms: u64,

    /// Keyword extraction time in ms (059 C1b stage honesty).
    #[serde(default)]
    pub keyword_time_ms: u64,

    /// Retrieval time in ms.
    pub retrieval_time_ms: u64,

    /// Generation time in ms.
    pub generation_time_ms: u64,

    /// Time to first token from generation start (ms), when measured (064).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttft_ms: Option<u64>,

    /// True when answer served from product answer cache (064 / SPEC-103).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub answer_cache_hit: bool,

    /// True when keywords served from LLM response cache (SPEC-103).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub keyword_cache_hit: bool,

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

    // ========================================================================
    // SPEC-047 W0b: retrieval diagnostics (engine QueryStats projection)
    // ========================================================================
    /// True when retrieval returned no chunks/entities/relationships.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub context_empty: bool,

    /// True when post-retrieval truncation removed context items.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub context_truncated: bool,

    /// Chunks the SPEC-184 relevance floor removed on a normal workspace.
    ///
    /// WHY it matters to a client: `context_empty: true` together with this set means the corpus
    /// *was* searched and nothing cleared the floor — an honest "this workspace has nothing on
    /// that", which is a different answer from an empty corpus. Absent when the gate did not run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relevance_dropped: Option<usize>,

    /// Per-arm wall time for Hybrid/Mix local retrieval (ms).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm_local_ms: Option<u64>,

    /// Per-arm wall time for Hybrid/Mix global retrieval (ms).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm_global_ms: Option<u64>,

    /// Per-arm wall time for Hybrid/Mix naive retrieval (ms).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm_naive_ms: Option<u64>,

    /// Chunks from the local arm before merge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm_local_chunks: Option<usize>,

    /// Chunks from the global arm before merge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm_global_chunks: Option<usize>,

    /// Chunks from the naive arm before merge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arm_naive_chunks: Option<usize>,

    /// Comma-separated arms that ran (e.g. `"local,global,naive"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arms_run: Option<String>,

    /// True when intent/weight gating skipped at least one arm.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arms_gated: Option<bool>,

    /// LLM / heuristic query intent (022 P3a Summarize truncation audit).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_intent: Option<String>,
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
            similarity: None,
            rerank_score: Some(0.98),
            snippet: Some("This is a test snippet".to_string()),
            reference_id: Some(1),
            document_id: Some("doc_456".to_string()),
            file_path: Some("docs/test.md".to_string()),
            start_line: Some(10),
            end_line: Some(20),
            chunk_index: Some(2),
            entity_type: None,
            degree: None,
            source_chunk_ids: None,
            doc_kind: None,
            mime_type: None,
            source_modality: None,
            page_start: None,
            page_end: None,
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
            similarity: None,
            rerank_score: None,
            snippet: None,
            reference_id: None,
            document_id: None,
            file_path: None,
            start_line: None,
            end_line: None,
            chunk_index: None,
            entity_type: Some("ORGANIZATION".to_string()),
            degree: Some(5),
            source_chunk_ids: Some(vec!["chunk-1".to_string()]),
            doc_kind: None,
            mime_type: None,
            source_modality: None,
            page_start: None,
            page_end: None,
        };
        let json = serde_json::to_value(&source).unwrap();
        assert!(json.get("rerank_score").is_none());
        assert!(json.get("reference_id").is_none());
        // SPEC-006: Verify entity metadata fields are serialized
        assert_eq!(json["entity_type"], "ORGANIZATION");
        assert_eq!(json["degree"], 5);
        assert_eq!(json["source_chunk_ids"], serde_json::json!(["chunk-1"]));
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
            llm_model: Some("gemma4:latest".to_string()),
            context_empty: false,
            arms_run: Some("local,global,naive".into()),
            ..Default::default()
        };
        let json = serde_json::to_value(&stats).unwrap();
        assert_eq!(json["total_time_ms"], 650);
        assert_eq!(json["sources_retrieved"], 5);
        assert_eq!(json["rerank_time_ms"], 25);
        // SPEC-032: Verify new fields
        assert_eq!(json["tokens_used"], 124);
        assert_eq!(json["tokens_per_second"], 248.0);
        assert_eq!(json["llm_provider"], "ollama");
        assert_eq!(json["llm_model"], "gemma4:latest");
        assert_eq!(json["arms_run"], "local,global,naive");
        assert!(json.get("context_empty").is_none()); // skip_serializing_if false
    }

    #[test]
    fn contract_x_22_thinking_stream_event() {
        let event = QueryStreamEvent::Thinking {
            content: "Retrieved context via mix".into(),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "thinking");
        assert!(json["content"].as_str().unwrap().contains("mix"));

        let stream = QueryStreamStats::from_query_stats(
            QueryStats {
                arms_run: Some("local,naive".into()),
                ..Default::default()
            },
            "mix".into(),
            Some(12),
            7,
        );
        assert_eq!(stream.arms_run.as_deref(), Some("local,naive"));
        assert_eq!(stream.query_mode, "mix");
        assert_eq!(stream.ux_ttft_ms, Some(12));
        assert_eq!(stream.tokens_used, 7);
    }

    #[test]
    fn test_query_response_serialization() {
        let response = QueryResponse {
            answer: "RAG is Retrieval Augmented Generation".to_string(),
            mode: "hybrid".to_string(),
            sources: vec![],
            subgraph: None,
            stats: QueryStats {
                embedding_time_ms: 10,
                retrieval_time_ms: 20,
                generation_time_ms: 100,
                total_time_ms: 130,
                sources_retrieved: 0,
                ..Default::default()
            },
            conversation_id: None,
            reranked: false,
            explain: None,
            trace_id: None,
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["mode"], "hybrid");
        assert!(json.get("conversation_id").is_none());
        assert!(json.get("reranked").is_none()); // skip_serializing_if
    }
}
