//! EdgeQuake Query - Query Engine for RAG
//!
//! # Implements
//!
//! - **FEAT0007**: Multi-Mode Query Execution
//! - **FEAT0101-0106**: All query mode strategies
//! - **FEAT0107**: LLM-Based Keyword Extraction
//! - **FEAT0108**: Smart Context Truncation
//!
//! # Enforces
//!
//! - **BR0101**: Token budget enforcement (configurable, default 4000)
//! - **BR0102**: Graph context priority over naive chunks
//! - **BR0104**: Conversation history in context
//!
//! This crate provides the query engine that combines:
//! - Vector similarity search
//! - Knowledge graph traversal
//! - LLM-based answer generation
//!
//! # Query Modes
//!
//! | Mode | FEAT | Description |
//! |------|------|-------------|
//! | Naive | FEAT0101 | Simple vector similarity search |
//! | Local | FEAT0102 | Entity-centric search with graph context |
//! | Global | FEAT0103 | Community-based search (relationship focus) |
//! | Hybrid | FEAT0104 | Combines local and global approaches |
//! | Mix | FEAT0105 | Max-after-minmax / RRF fusion of naive + graph |
//! | Bypass | FEAT0106 | Direct LLM, no RAG retrieval |
//!
//! # Architecture
//!
//! The query engine uses a multi-stage retrieval pipeline:
//! 1. Query embedding generation
//! 2. Keyword extraction (FEAT0107)
//! 3. Candidate retrieval (vector + graph)
//! 4. Context aggregation + truncation (FEAT0108)
//! 5. LLM answer generation
//!
//! # Key Components
//!
//! - [`QueryEngine`]: Main engine implementing LightRAG algorithm
//! - [`QueryMode`]: Enum of all supported query modes
//! - [`QueryContext`]: Retrieved context (entities, relationships, chunks)
//! - [`TruncationConfig`]: Token budget configuration
//!
//! # See Also
//!
//! - [`crate::engine_impl`] for the engine implementation
//! - [`crate::keywords`] for keyword extraction
//! - [`crate::truncation`] for token budgeting

pub mod bootstrap;
pub mod cache;
pub mod chunk_hydration;
pub mod citation_verify;
pub mod community_global;
pub mod context;
pub mod context_filter;
pub mod context_format;
pub mod conversation_context;
pub mod engine;
pub mod engine_impl;
pub mod entity_rank;
pub mod error;
pub mod eval;
pub mod fusion;
pub mod graph_expand;
pub mod graph_hops;
pub mod graph_ppr;
pub mod graph_seed_admit;
pub mod graph_walk_compress;
pub mod grounding;
pub mod helpers;
pub mod hybrid_merge;
pub mod intent_rerank;
pub mod keyword_boost;
pub mod keywords;
pub mod kg_chunk_pick;
pub mod l2_bm25_union;
pub mod l2_sources_union;
pub mod lineage_scope;
pub mod math_span;
pub mod mix_weights;
pub mod modality_retrieve;
pub mod modes;
pub mod path_prune;
pub mod query_reliability;
pub mod relation_select;
pub mod relevancy_prune;
pub mod rerank_protect;
pub mod retrieval_telemetry;
pub mod score_scale;
pub mod sparse_retrieval;
pub mod tokenizer;
pub mod topic_entity_admit;
pub mod truncation;
pub mod types;
pub mod vector_filter;

pub use citation_verify::{
    build_document_page_url, rewrite_verified_citations, short_document_label, CitationCatalog,
    CitationEntry, CitationSourceRow, RewriteReport,
};
pub use context::{
    QueryContext, RetrievedChunk, RetrievedContext, RetrievedEntity, RetrievedRelationship,
};
pub use engine::{ConversationMessage, ExplainTrace, QueryRequest, QueryResponse, QueryStats};
pub use error::{QueryError, Result};
// Re-export keywords module types
pub use bootstrap::{
    build_production_query_engine, create_production_reranker,
    create_production_reranker_with_embedding,
};
pub use cache::{QueryResultCache, QueryResultCacheInvalidator};
pub use context_format::{
    assign_stable_citation_ids, chunk_citation_ref, format_chunk_block, format_chunk_meta,
    format_entity_line, format_query_context, format_query_context_flat,
    format_query_context_passage_pack, format_query_context_path, format_query_context_with_mode,
    format_relationship_line, passage_pack_enabled, ContextFormatMode,
};
pub use engine_impl::{QueryEmbeddings, QueryEngine, QueryEngineConfig};
pub use entity_rank::{rank_entities_for_prompt, EntityRankMode};
pub use fusion::{
    chunks_from_rrf_ranking, mix_fusion_mode_from_env, mix_fusion_mode_label,
    reciprocal_rank_fusion, MixFusionMode, RRF_K,
};
pub use graph_ppr::{parse_graph_walk_mode, GraphWalkMode, PprConfig};
pub use grounding::{
    allows_honest_refusal, grounding_instructions, is_entailment_first, is_gold_answer_extension,
    strip_gold_citation_artifacts,
};
pub use keywords::{
    CachedKeywordExtractor, ExtractedKeywords, InMemoryKeywordCache, KeywordCache,
    KeywordExtractor, Keywords, LLMKeywordExtractor, MockKeywordExtractor, QueryIntent,
};
pub use kg_chunk_pick::{collect_kg_chunk_ids, collect_kg_chunk_ids_scoped, KgChunkPickMethod};
pub use lineage_scope::{
    document_ids_from_chunk_ids, filter_chunk_ids_by_allowed_docs, lineage_intersects_allowed,
    resolve_lineage_document_ids,
};
pub use math_span::{math_span_end, normalize_math_spans};
pub use mix_weights::{mix_arm_weight_from_env, MixWeightOverride};
pub use modality_retrieve::{
    chart_modality_filter_enabled, plan_modality_retrieval,
    query_filtered_with_modality_preference, query_prefers_chart_modality,
    text_search_with_modality_preference, with_chart_modality_filter, ModalityFilterPlan,
    MODALITY_CHART,
};
pub use modes::QueryMode;
pub use path_prune::{
    prune_orphan_entities, prune_relationships, prune_relationships_for_query, PathPruneConfig,
};
pub use query_reliability::{
    classify_query_failure, is_typed_timeout_message, query_failure_diagnostic, QueryFailureClass,
};
pub use relevancy_prune::{RelevancyPruneConfig, RelevancyScoreMode};
pub use rerank_protect::{blend_protect_first, blend_protect_ids, protect_first_from_env};
pub use score_scale::{
    max_minmax, min_max_normalize_to_fusion_scale, weighted_minmax_contribution, ScaleMismatch,
    ScaledScore, ScoreScale,
};
pub use tokenizer::{MockTokenizer, SimpleTokenizer, Tokenizer};
pub use truncation::{
    balance_context, min_chunk_token_budget, parse_min_chunk_budget_ratio, parse_token_cap,
    truncate_chunks, truncate_entities, truncate_relationships, truncation_config_for_intent,
    TruncationConfig, LR_MAX_ENTITY_TOKENS, LR_MAX_RELATION_TOKENS, LR_MAX_TOTAL_TOKENS,
};

// Re-export EmbeddingProvider and LLMProvider for workspace-specific query execution
pub use edgequake_llm::traits::EmbeddingProvider;
pub use edgequake_llm::traits::LLMProvider;
