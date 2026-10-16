//! Query Engine - LightRAG-inspired implementation.
//!
//! # Implements
//!
//! - **FEAT0007**: Multi-Mode Query Execution
//! - **FEAT0101**: Naive Mode (vector search only)
//! - **FEAT0102**: Local Mode (entity-centric)
//! - **FEAT0103**: Global Mode (community summaries)
//! - **FEAT0104**: Hybrid Mode (local + global)
//! - **FEAT0105**: Mix Mode (adaptive blend)
//! - **FEAT0106**: Bypass Mode (direct LLM)
//! - **FEAT0107**: LLM-Based Keyword Extraction
//! - **FEAT0108**: Smart Context Truncation
//! - **FEAT0109**: Query Delegation
//!
//! # Enforces
//!
//! - **BR0101**: Token budget must not exceed LLM context window
//! - **BR0102**: Graph context takes priority over naive chunks
//! - **BR0103**: Query mode must be valid enum value
//! - **BR0104**: Conversation history included in context
//! - **BR0106**: Keyword cache TTL 24 hours default
//!
//! This module provides the enhanced query engine with:
//! - LLM-based keyword extraction with caching
//! - Mode-specific vector search (entities vs relationships)
//! - Batch graph operations
//! - Query caching
//!
//! # Architecture
//!
//! ```text
//! Query → Keyword Extraction → Mode Router
//!                                 ↓
//!         ┌───────────────────────┼───────────────────────┐
//!         ↓                       ↓                       ↓
//!     Local Mode             Global Mode             Naive Mode
//!   (Entity VDB +          (Relationship VDB +      (Chunk VDB)
//!    low-level kw)          high-level kw)
//!         ↓                       ↓                       ↓
//!         └───────────────────────┼───────────────────────┘
//!                                 ↓
//!                         Context Building
//!                                 ↓
//!                         Token Budgeting
//!                                 ↓
//!                         LLM Generation
//! ```
//!
//! # WHY: LightRAG Algorithm
//!
//! This implements the LightRAG paper's multi-level retrieval strategy:
//!
//! 1. **Keyword Extraction**: LLM extracts high-level (themes) and low-level
//!    (entities) keywords from the query. WHY: Different keywords retrieve
//!    different context types optimally.
//!
//! 2. **Mode-Specific Search**:
//!    - Local: Uses low-level keywords to find entity nodes
//!    - Global: Uses high-level keywords to find relationship clusters
//!    - Naive: Direct query embedding against chunk vectors
//!
//! 3. **Token Budgeting**: Context is truncated to fit LLM window while
//!    maintaining the most relevant information. Graph context is prioritized
//!    over raw chunks because graph relationships are pre-summarized.
//!
//! # See Also
//!
//! - [`QueryMode`] for available modes
//! - [`QueryRequest`] for query parameters
//! - [docs/features.md](../../../../../../docs/features.md) for feature details

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::error::{QueryError, Result};
use crate::keywords::{
    CachedKeywordExtractor, ExtractedKeywords, KeywordExtractor, LLMKeywordExtractor,
    MockKeywordExtractor,
};
use crate::modes::QueryMode;
use crate::tokenizer::{SimpleTokenizer, Tokenizer};
use crate::truncation::TruncationConfig;

use edgequake_llm::traits::{EmbeddingProvider, LLMProvider};
use edgequake_llm::Reranker;
use edgequake_storage::traits::{GraphReadView, GraphStorage, VectorStorage};

/// Configuration for the query engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryEngineConfig {
    /// Default query mode.
    pub default_mode: QueryMode,

    /// Maximum entities to retrieve.
    pub max_entities: usize,

    /// Maximum relationships to retrieve.
    pub max_relationships: usize,

    /// Maximum chunks to retrieve.
    pub max_chunks: usize,

    /// Maximum context tokens.
    pub max_context_tokens: usize,

    /// Graph traversal depth.
    pub graph_depth: usize,

    /// Minimum similarity score threshold.
    pub min_score: f32,

    /// Whether to use keyword extraction.
    pub use_keyword_extraction: bool,

    /// Whether to use adaptive mode selection based on query intent.
    pub use_adaptive_mode: bool,

    /// Truncation configuration.
    pub truncation: TruncationConfig,

    /// Keyword cache TTL in seconds.
    pub keyword_cache_ttl_secs: u64,

    /// Enable reranking for improved retrieval precision.
    pub enable_rerank: bool,

    /// Minimum rerank score threshold (0.0 - 1.0).
    pub min_rerank_score: f32,

    /// Top K results to keep after reranking.
    pub rerank_top_k: usize,

    /// Mix-mode weights (P-G8 / RC-13). Mix runs the Local, Global, and Naive
    /// arms in parallel, min-max normalizes each arm, then takes the **max**
    /// weighted contribution per chunk (D-35: not a weighted sum). When
    /// `EDGEQUAKE_MIX_FUSION=rrf` (ablation; product default is round_robin /
    /// SPEC-086 E2-occ). A weight of 0 skips that arm (E25).
    pub mix_local_weight: f32,
    pub mix_global_weight: f32,
    pub mix_naive_weight: f32,

    /// Enable sparse retrieval (`ts_rank_cd` FTS / in-memory BM25 fallback)
    /// fused with vector ANN (SPEC-023 I10, default on). Env name remains
    /// `EDGEQUAKE_BM25_RETRIEVAL` for compatibility (X-05).
    pub enable_bm25_retrieval: bool,

    /// Expand global mode with co-community entities (SPEC-023 I6, default on).
    pub enable_community_global: bool,

    /// Vector candidate pool multiplier for BM25 fusion in naive mode.
    pub bm25_candidate_multiplier: usize,

    /// Max chunks contributed per entity/relation source (LightRAG
    /// `related_chunk_number`). `0` = unlimited. SPEC-046 P0.3.
    pub related_chunk_number: usize,

    /// KG→chunk pick strategy (`vector` | `weight`). SPEC-046 P0.3.
    pub kg_chunk_pick_method: crate::kg_chunk_pick::KgChunkPickMethod,

    /// Neighborhood expansion: BFS or Personalized PageRank. SPEC-046 P1.1.
    #[serde(skip)]
    pub graph_walk: crate::graph_ppr::GraphWalkMode,

    /// PathRAG-style relation prune before truncation. SPEC-046 P1.3.
    #[serde(skip)]
    pub path_prune: crate::path_prune::PathPruneConfig,

    /// Prompt entity order: `degree` (default) or `query_score` (Acc-win E2).
    #[serde(skip)]
    pub entity_rank: crate::entity_rank::EntityRankMode,
}

impl Default for QueryEngineConfig {
    fn default() -> Self {
        Self {
            default_mode: QueryMode::Mix,
            // WHY 60: LightRAG uses top_k=60 entities. More entity candidates = more
            // chunk candidates from the KG path, directly improving recall.
            max_entities: 60,
            // WHY 60: Match entity count for balanced KG context.
            // LightRAG allocates max_relation_tokens=8000 for relations.
            max_relationships: 60,
            // WHY 20: LightRAG uses chunk_top_k=20. More text chunks = more direct
            // evidence for the LLM, improving both recall and correctness.
            max_chunks: 20,
            // WHY 30000: LightRAG uses max_total_tokens=30000. With gpt-4o-mini
            // having 128K context, 4000 tokens was throwing away ~87% of usable context.
            // 30000 tokens uses only 23% of the context window — safe and effective.
            max_context_tokens: 30000,
            graph_depth: 2,
            // Configurable via EDGEQUAKE_MIN_ENTITY_SCORE env var (default: 0.1).
            // Lower this (e.g. 0.0) to retrieve low-frequency entities that score
            // below the default threshold on bare name queries.
            min_score: std::env::var("EDGEQUAKE_MIN_ENTITY_SCORE")
                .ok()
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(0.1),
            use_keyword_extraction: true,
            use_adaptive_mode: true,
            // WHY derived from max_context_tokens: truncation MUST match the
            // context token budget. LightRAG (constants.py): entity=6000,
            // relation=8000, total=30000; chunks get dynamic remainder.
            // Override via EDGEQUAKE_MAX_ENTITY_TOKENS / MAX_RELATION_TOKENS.
            truncation: TruncationConfig::default(),
            keyword_cache_ttl_secs: 24 * 60 * 60, // 24 hours
            enable_rerank: true,                  // Enable by default for retrieval quality
            // WHY 0.1 default: filters CE noise. Acc recall recovery (025): set
            // EDGEQUAKE_MIN_RERANK_SCORE=0 so Mix gold is not hard-dropped before protect.
            min_rerank_score: std::env::var("EDGEQUAKE_MIN_RERANK_SCORE")
                .ok()
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(0.1)
                .clamp(0.0, 1.0),
            // WHY 20: Match max_chunks to keep all chunk candidates after reranking.
            rerank_top_k: 20,
            // P-G8: equal weights preserve Hybrid ordering on identical fixtures.
            // Acc-win E3b: override via EDGEQUAKE_MIX_{LOCAL,GLOBAL,NAIVE}_WEIGHT.
            mix_local_weight: crate::mix_weights::mix_arm_weight_from_env(
                "EDGEQUAKE_MIX_LOCAL_WEIGHT",
                1.0,
            ),
            mix_global_weight: crate::mix_weights::mix_arm_weight_from_env(
                "EDGEQUAKE_MIX_GLOBAL_WEIGHT",
                1.0,
            ),
            mix_naive_weight: crate::mix_weights::mix_arm_weight_from_env(
                "EDGEQUAKE_MIX_NAIVE_WEIGHT",
                1.0,
            ),
            enable_bm25_retrieval: std::env::var("EDGEQUAKE_BM25_RETRIEVAL")
                .map(|v| !matches!(v.to_ascii_lowercase().as_str(), "false" | "0" | "off"))
                .unwrap_or(true),
            enable_community_global: std::env::var("EDGEQUAKE_COMMUNITY_GLOBAL")
                .map(|v| !matches!(v.to_ascii_lowercase().as_str(), "false" | "0" | "off"))
                .unwrap_or(true),
            bm25_candidate_multiplier: std::env::var("EDGEQUAKE_BM25_CANDIDATE_MULTIPLIER")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5)
                .clamp(2, 20),
            related_chunk_number: std::env::var("EDGEQUAKE_RELATED_CHUNK_NUMBER")
                .or_else(|_| std::env::var("RELATED_CHUNK_NUMBER"))
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
            kg_chunk_pick_method: crate::kg_chunk_pick::KgChunkPickMethod::from_env(),
            graph_walk: crate::graph_ppr::GraphWalkMode::from_env(),
            path_prune: crate::path_prune::PathPruneConfig::from_env(),
            entity_rank: crate::entity_rank::EntityRankMode::from_env(),
        }
    }
}

/// Query embeddings for different keyword levels.
///
/// LightRAG uses different embeddings for different modes:
/// - low_level: Entity search (Local mode)
/// - high_level: Relationship search (Global mode)
/// - query: Direct chunk search (Naive mode)
pub struct QueryEmbeddings {
    /// Original query embedding.
    pub query: Vec<f32>,

    /// High-level keywords embedding (for Global mode).
    pub high_level: Vec<f32>,

    /// Low-level keywords embedding (for Local mode).
    pub low_level: Vec<f32>,

    /// Logical model name that produced these vectors (typed ANN registry key).
    ///
    /// Must match workspace embedding override (e.g. `mistral-embed`), not the
    /// process `EDGEQUAKE_EMBEDDING_MODEL` default, or typed search misses rows.
    pub model: String,
}

impl QueryEmbeddings {
    /// High/low keyword strings used as Mix embed inputs (LightRAG parity).
    pub(crate) fn keyword_level_texts(
        query: &str,
        keywords: &ExtractedKeywords,
    ) -> (String, String) {
        let high_level_text = if keywords.high_level.is_empty() {
            query.to_string()
        } else {
            keywords.high_level.join(", ")
        };
        let low_level_text = if keywords.low_level.is_empty() {
            query.to_string()
        } else {
            keywords.low_level.join(", ")
        };
        (high_level_text, low_level_text)
    }

    fn take_level(known: &HashMap<String, Vec<f32>>, text: &str) -> Vec<f32> {
        known.get(text).cloned().unwrap_or_default()
    }

    /// One embed RTT for remaining unique texts; X-10 L2-normalize every vector.
    ///
    /// `query_vec` prefetch (cold path) is reused so we never re-embed the
    /// question. Empty prefetch + identical level texts keeps the SPEC-017
    /// triple batch so MockProvider queues still pop three slots.
    async fn assemble(
        query: &str,
        keywords: &ExtractedKeywords,
        query_vec: Vec<f32>,
        embedder: &dyn EmbeddingProvider,
    ) -> Result<Self> {
        let (high_level_text, low_level_text) = Self::keyword_level_texts(query, keywords);
        let all_equal = high_level_text == query && low_level_text == query;

        let model = embedder.model().to_string();

        if all_equal && !query_vec.is_empty() {
            let v = crate::cache::embedding_cache::l2_normalize_vec(query_vec);
            return Ok(Self {
                query: v.clone(),
                high_level: v.clone(),
                low_level: v,
                model,
            });
        }

        if all_equal && query_vec.is_empty() {
            // One unique RTT (warm peek / keyword-off). Scatter the same vector
            // to all Mix levels — do not send [q,q,q] (extra payload, same RTT).
            let embeds = embedder
                .embed(&[query.to_string()])
                .await
                .map_err(QueryError::from)?;
            let v = crate::cache::embedding_cache::l2_normalize_vec(
                embeds.into_iter().next().unwrap_or_default(),
            );
            return Ok(Self {
                query: v.clone(),
                high_level: v.clone(),
                low_level: v,
                model,
            });
        }

        let mut known: HashMap<String, Vec<f32>> = HashMap::new();
        if !query_vec.is_empty() {
            known.insert(
                query.to_string(),
                crate::cache::embedding_cache::l2_normalize_vec(query_vec),
            );
        }
        let needed = [
            query.to_string(),
            high_level_text.clone(),
            low_level_text.clone(),
        ];
        let mut missing: Vec<String> = Vec::new();
        for text in &needed {
            if !known.contains_key(text) && !missing.iter().any(|m| m == text) {
                missing.push(text.clone());
            }
        }
        if !missing.is_empty() {
            let embeds = embedder.embed(&missing).await.map_err(QueryError::from)?;
            if embeds.len() != missing.len() {
                return Err(QueryError::Internal(format!(
                    "Expected {} unique embeddings, got {}",
                    missing.len(),
                    embeds.len()
                )));
            }
            for (text, vec) in missing.into_iter().zip(embeds) {
                known.insert(text, crate::cache::embedding_cache::l2_normalize_vec(vec));
            }
        }

        Ok(Self {
            query: Self::take_level(&known, query),
            high_level: Self::take_level(&known, &high_level_text),
            low_level: Self::take_level(&known, &low_level_text),
            model,
        })
    }

    /// Compute all Mix embeddings in one unique `embed()` batch (LR #2728).
    pub async fn compute(
        query: &str,
        keywords: &ExtractedKeywords,
        embedder: &dyn EmbeddingProvider,
    ) -> Result<Self> {
        Self::assemble(query, keywords, Vec::new(), embedder).await
    }

    /// Simple embedding (same for all levels).
    pub fn uniform(embedding: Vec<f32>) -> Self {
        Self::uniform_with_model(embedding, String::new())
    }

    /// Uniform vectors stamped with the logical embedding model name.
    pub fn uniform_with_model(embedding: Vec<f32>, model: impl Into<String>) -> Self {
        Self {
            query: embedding.clone(),
            high_level: embedding.clone(),
            low_level: embedding,
            model: model.into(),
        }
    }

    /// Reuse a speculative query vector, then one unique batch for the rest.
    ///
    /// WHY: Cold Mix overlaps `embed_one(query)` with keyword LLM. Warm Mix
    /// (keyword cache hit) skips prefetch and calls [`Self::compute`] instead
    /// so hl/ll share the same RTT as the question (064 / LightRAG #2728).
    pub async fn compute_with_query_vec(
        query: &str,
        query_vec: Vec<f32>,
        keywords: &ExtractedKeywords,
        embedder: &dyn EmbeddingProvider,
    ) -> Result<Self> {
        Self::assemble(query, keywords, query_vec, embedder).await
    }
}

pub struct QueryEngine {
    config: QueryEngineConfig,
    vector_storage: Arc<dyn VectorStorage>,
    graph_storage: Arc<dyn GraphStorage>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    llm_provider: Arc<dyn LLMProvider>,
    keyword_extractor: Arc<dyn KeywordExtractor>,
    tokenizer: Arc<dyn Tokenizer>,
    /// Optional KV store for chunk content hydration (SPEC-024 2.5).
    pub(super) kv_storage: Option<Arc<dyn edgequake_storage::traits::KVStorage>>,
    /// Optional reranker for improved retrieval precision.
    pub(super) reranker: Option<Arc<dyn Reranker>>,
    /// Cache for keyword validation (keyword -> exists_in_graph).
    /// WHY: Avoids repeated graph lookups for the same keywords.
    keyword_validation_cache: Arc<tokio::sync::RwLock<std::collections::HashMap<String, bool>>>,
    /// Optional cache for `context_only` retrieval contexts (P-G9).
    result_cache: Option<Arc<crate::cache::QueryResultCache>>,
    /// Legacy in-memory answer cache (064); prefer [`Self::llm_response_cache`].
    answer_cache: Option<crate::cache::SharedAnswerCache>,
    /// SPEC-103 unified keywords+query LLM response cache (L1 and/or L2).
    llm_response_cache: Option<crate::cache::SharedLlmResponseCache>,
    /// Shared flag set by [`CachedKeywordExtractor`] on hit (LAW-C8).
    keyword_cache_hit_flag: Arc<std::sync::atomic::AtomicBool>,
    /// LRU wrappers for injected workspace embedders whose identity differs
    /// from the engine default (064 / Acc Mix). Same-model injects reuse
    /// [`Self::embedding_provider`] instead of a fresh uncached client.
    alternate_embed_caches:
        Arc<std::sync::RwLock<std::collections::HashMap<String, Arc<dyn EmbeddingProvider>>>>,
}

impl QueryEngine {
    /// Clone engine config with an effective chunk cap (API `max_results` override).
    pub(super) fn config_with_max_chunks(&self, max_chunks: usize) -> QueryEngineConfig {
        let mut cfg = self.config.clone();
        cfg.max_chunks = max_chunks;
        cfg
    }

    /// Create a new query engine.
    pub fn new(
        config: QueryEngineConfig,
        vector_storage: Arc<dyn VectorStorage>,
        graph_storage: Arc<dyn GraphStorage>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
        llm_provider: Arc<dyn LLMProvider>,
    ) -> Self {
        let keyword_cache_hit_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let base_extractor = Arc::new(LLMKeywordExtractor::new(llm_provider.clone()));
        let cache = Arc::new(crate::keywords::TieredKeywordCache::memory_only(1000));
        let keyword_extractor: Arc<dyn KeywordExtractor> =
            Arc::new(CachedKeywordExtractor::with_model(
                base_extractor,
                cache,
                std::time::Duration::from_secs(config.keyword_cache_ttl_secs),
                "default",
                llm_provider.model().to_string(),
                Arc::clone(&keyword_cache_hit_flag),
            ));

        Self {
            config,
            vector_storage,
            graph_storage,
            embedding_provider,
            llm_provider,
            keyword_extractor,
            tokenizer: Arc::new(SimpleTokenizer),
            kv_storage: None,
            reranker: None, // No reranker by default
            keyword_validation_cache: Arc::new(tokio::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
            result_cache: None,
            answer_cache: None,
            llm_response_cache: None,
            keyword_cache_hit_flag,
            alternate_embed_caches: Arc::new(std::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
        }
    }

    /// Create with a reranker for improved retrieval precision.
    pub fn with_reranker(mut self, reranker: Arc<dyn Reranker>) -> Self {
        self.reranker = Some(reranker);
        self
    }

    /// Enable Mix answer LLM cache (064 / SPEC-103). Also auto-enabled when
    /// master `EDGEQUAKE_LLM_CACHE` is on via [`Self::with_answer_cache_from_env`].
    pub fn with_answer_cache(self) -> Self {
        self.with_answer_cache_config(1_000, std::time::Duration::from_secs(3600))
    }

    pub fn with_answer_cache_config(mut self, max_size: usize, ttl: std::time::Duration) -> Self {
        self.answer_cache = Some(Arc::new(crate::cache::InMemoryAnswerCache::new(
            max_size, ttl,
        )));
        self.llm_response_cache = Some(Arc::new(crate::cache::MemoryLlmResponseCache::new(
            max_size, ttl,
        )));
        self.rewire_keyword_cache();
        self
    }

    /// Attach answer cache when SPEC-103 flags enable it (master ON by default).
    pub fn with_answer_cache_from_env(self) -> Self {
        if crate::cache::answer_cache_enabled_from_env() {
            self.with_answer_cache()
        } else {
            self
        }
    }

    /// Attach unified LLM response cache (tests / custom backends).
    pub fn with_llm_response_cache(mut self, cache: crate::cache::SharedLlmResponseCache) -> Self {
        self.llm_response_cache = Some(cache);
        if self.answer_cache.is_none() {
            self.answer_cache = Some(Arc::new(crate::cache::InMemoryAnswerCache::with_defaults()));
        }
        self.rewire_keyword_cache();
        self
    }

    /// Attach KV storage for chunk hydration + SPEC-103 L2 `public.llm_cache`.
    pub fn with_kv_storage(mut self, kv: Arc<dyn edgequake_storage::traits::KVStorage>) -> Self {
        self.kv_storage = Some(Arc::clone(&kv));
        let tiered = Arc::new(crate::cache::TieredLlmResponseCache::with_kv(kv));
        self.llm_response_cache = Some(tiered);
        if self.answer_cache.is_none() && crate::cache::answer_cache_enabled_from_env() {
            self.answer_cache = Some(Arc::new(crate::cache::InMemoryAnswerCache::with_defaults()));
        }
        self.rewire_keyword_cache();
        self
    }

    /// Rebuild keyword extractor L1/L2 against current `llm_response_cache`.
    fn rewire_keyword_cache(&mut self) {
        let base_extractor = Arc::new(LLMKeywordExtractor::new(self.llm_provider.clone()));
        let kw_cache: Arc<dyn crate::keywords::KeywordCache> =
            if let Some(durable) = &self.llm_response_cache {
                Arc::new(crate::keywords::TieredKeywordCache::with_durable(
                    1000,
                    Arc::clone(durable),
                ))
            } else {
                Arc::new(crate::keywords::TieredKeywordCache::memory_only(1000))
            };
        self.keyword_extractor = Arc::new(CachedKeywordExtractor::with_model(
            base_extractor,
            kw_cache,
            std::time::Duration::from_secs(self.config.keyword_cache_ttl_secs),
            "default",
            self.llm_provider.model().to_string(),
            Arc::clone(&self.keyword_cache_hit_flag),
        ));
    }

    /// Wrap the engine's embedding provider in an LRU+TTL embedding cache
    /// (P-G9 / RC-14 / 064). Repeated `embed_one` and per-text batch `embed`
    /// slots share keys so Mix keyword-level batches stay warm.
    ///
    /// Defaults: 10_000 entries, 1h TTL. Use [`Self::with_embedding_cache_config`]
    /// for custom sizing.
    pub fn with_embedding_cache(self) -> Self {
        self.with_embedding_cache_config(10_000, std::time::Duration::from_secs(3600))
    }

    /// Wrap the embedding provider in a cache with custom `max_size` and `ttl`.
    pub fn with_embedding_cache_config(
        mut self,
        max_size: usize,
        ttl: std::time::Duration,
    ) -> Self {
        self.embedding_provider = Arc::new(crate::cache::CachingEmbeddingProvider::new(
            self.embedding_provider.clone(),
            max_size,
            ttl,
        ));
        self
    }

    /// Enable LRU+TTL cache for `context_only` retrieval (P-G9 result half).
    pub fn with_result_cache(self) -> Self {
        self.with_result_cache_config(1_000, std::time::Duration::from_secs(300))
    }

    pub fn with_result_cache_config(mut self, max_size: usize, ttl: std::time::Duration) -> Self {
        self.result_cache = Some(Arc::new(crate::cache::QueryResultCache::new(max_size, ttl)));
        self
    }

    pub fn result_cache(&self) -> Option<&Arc<crate::cache::QueryResultCache>> {
        self.result_cache.as_ref()
    }

    /// Invalidate cached `context_only` retrieval after ingestion (P-G9 / E26).
    pub fn invalidate_result_cache(&self) {
        if let Some(cache) = &self.result_cache {
            cache.invalidate_all();
        }
        if let Some(cache) = &self.answer_cache {
            cache.clear();
        }
        if let Some(cache) = &self.llm_response_cache {
            cache.clear_l1();
        }
    }

    /// SPEC-103: unified LLM response cache handle (if wired).
    pub fn llm_response_cache(&self) -> Option<&crate::cache::SharedLlmResponseCache> {
        self.llm_response_cache.as_ref()
    }
}

impl crate::cache::QueryResultCacheInvalidator for QueryEngine {
    fn invalidate_query_result_cache(&self) {
        self.invalidate_result_cache();
    }

    fn invalidate_query_result_cache_for_workspace(&self, workspace_id: &str) {
        if let Some(cache) = &self.result_cache {
            cache.invalidate_workspace(workspace_id);
        }
    }
}

impl QueryEngine {
    #[inline]
    pub(super) fn graph_read(&self) -> GraphReadView<'_> {
        GraphReadView::new(self.graph_storage.as_ref())
    }

    /// Create with mock keyword extractor (for testing).
    pub fn with_mock_keywords(
        config: QueryEngineConfig,
        vector_storage: Arc<dyn VectorStorage>,
        graph_storage: Arc<dyn GraphStorage>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
        llm_provider: Arc<dyn LLMProvider>,
    ) -> Self {
        let keyword_extractor: Arc<dyn KeywordExtractor> = Arc::new(MockKeywordExtractor::new());

        Self {
            config,
            vector_storage,
            graph_storage,
            embedding_provider,
            llm_provider,
            keyword_extractor,
            tokenizer: Arc::new(SimpleTokenizer),
            kv_storage: None,
            reranker: None,
            keyword_validation_cache: Arc::new(tokio::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
            result_cache: None,
            answer_cache: None,
            llm_response_cache: None,
            keyword_cache_hit_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            alternate_embed_caches: Arc::new(std::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
        }
    }

    /// Set a custom keyword extractor.
    pub fn with_keyword_extractor(mut self, extractor: Arc<dyn KeywordExtractor>) -> Self {
        self.keyword_extractor = extractor;
        self
    }

    /// Set a custom tokenizer.
    pub fn with_tokenizer(mut self, tokenizer: Arc<dyn Tokenizer>) -> Self {
        self.tokenizer = tokenizer;
        self
    }
}

impl QueryEngine {
    /// Get the query configuration.
    pub fn config(&self) -> &QueryEngineConfig {
        &self.config
    }

    /// Whether a reranker is configured on this engine.
    ///
    /// WHY (P-G6c / RC-11): the sync `/query` handler must report `reranked`
    /// and `rerank_time_ms` truthfully. The real rerank happens inside
    /// `pipeline_finalize` only when `reranker.is_some()`; without this
    /// accessor the API layer could not distinguish "rerank requested but no
    /// reranker configured" from "rerank actually applied", and previously
    /// faked the scores instead.
    pub fn has_reranker(&self) -> bool {
        self.reranker.is_some()
    }

    /// Get the engine's default embedding provider.
    ///
    /// WHY: Callers that override only part of the query config (e.g., LLM provider
    /// but not embedding) need access to the default embedding provider to pass it
    /// to `query_with_full_config`. Without this accessor, callers cannot construct
    /// a full config call when they only have partial overrides.
    /// @implements FIX-168
    pub fn default_embedding_provider(&self) -> Arc<dyn EmbeddingProvider> {
        self.embedding_provider.clone()
    }

    /// Embed identity: same model ⇒ same vectors (pure function of text).
    fn embedding_identity(provider: &dyn EmbeddingProvider) -> String {
        format!(
            "{}/{}/{}",
            provider.name(),
            provider.model(),
            provider.dimension()
        )
    }

    /// Acc Mix injects a fresh workspace embedder per request. Reuse the engine
    /// LRU when identity matches; otherwise wrap once per identity so repeats
    /// skip the embed RTT (064 first principle: memoize a pure function).
    pub(crate) fn cached_embedding_for(
        &self,
        injected: Arc<dyn EmbeddingProvider>,
    ) -> Arc<dyn EmbeddingProvider> {
        let key = Self::embedding_identity(injected.as_ref());
        if key == Self::embedding_identity(self.embedding_provider.as_ref()) {
            return Arc::clone(&self.embedding_provider);
        }
        {
            let map = self
                .alternate_embed_caches
                .read()
                .unwrap_or_else(|e| e.into_inner());
            if let Some(cached) = map.get(&key) {
                return Arc::clone(cached);
            }
        }
        let wrapped: Arc<dyn EmbeddingProvider> = Arc::new(
            crate::cache::CachingEmbeddingProvider::with_defaults(injected),
        );
        let mut map = self
            .alternate_embed_caches
            .write()
            .unwrap_or_else(|e| e.into_inner());
        Arc::clone(map.entry(key).or_insert(wrapped))
    }

    /// Get the engine's default vector storage.
    ///
    /// WHY: Same rationale as `default_embedding_provider` — callers with partial
    /// overrides need the default vector storage to construct full config calls.
    /// @implements FIX-168
    pub fn default_vector_storage(&self) -> Arc<dyn VectorStorage> {
        self.vector_storage.clone()
    }
}

/// SPEC-059: public for arm concurrency load tests.
pub mod modes;
mod prompt;
mod query_entry;
mod query_modes;
mod reranking;

/// Shared token-stream type for streaming query answers (P-G11).
pub type TokenStream = futures::stream::BoxStream<'static, std::result::Result<String, QueryError>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_engine_config_default() {
        let config = QueryEngineConfig::default();
        assert_eq!(config.default_mode, QueryMode::Mix);
        assert!(config.use_keyword_extraction);
        assert!(config.use_adaptive_mode);
    }

    #[test]
    fn test_query_embeddings_uniform() {
        let embedding = vec![1.0, 2.0, 3.0];
        let embeddings = QueryEmbeddings::uniform(embedding.clone());

        assert_eq!(embeddings.query, embedding);
        assert_eq!(embeddings.high_level, embedding);
        assert_eq!(embeddings.low_level, embedding);
    }

    #[tokio::test]
    async fn compute_with_query_vec_reuses_when_keywords_equal_query() {
        // 058 C1c: non-empty query_vec + empty keywords → no second embed RTT.
        use crate::keywords::QueryIntent;
        use edgequake_llm::MockProvider;
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct CountingEmbed {
            inner: MockProvider,
            calls: AtomicUsize,
        }
        #[async_trait::async_trait]
        impl EmbeddingProvider for CountingEmbed {
            fn name(&self) -> &str {
                EmbeddingProvider::name(&self.inner)
            }
            fn model(&self) -> &str {
                EmbeddingProvider::model(&self.inner)
            }
            fn dimension(&self) -> usize {
                EmbeddingProvider::dimension(&self.inner)
            }
            fn max_tokens(&self) -> usize {
                EmbeddingProvider::max_tokens(&self.inner)
            }
            async fn embed(&self, texts: &[String]) -> edgequake_llm::Result<Vec<Vec<f32>>> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                self.inner.embed(texts).await
            }
            async fn embed_one(&self, text: &str) -> edgequake_llm::Result<Vec<f32>> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                self.inner.embed_one(text).await
            }
        }

        let provider = CountingEmbed {
            inner: MockProvider::default(),
            calls: AtomicUsize::new(0),
        };
        let query_vec = vec![0.5_f32; EmbeddingProvider::dimension(&provider)];
        let keywords = ExtractedKeywords::new(vec![], vec![], QueryIntent::Factual);
        let out = QueryEmbeddings::compute_with_query_vec(
            "what is BRCA1?",
            query_vec.clone(),
            &keywords,
            &provider,
        )
        .await
        .unwrap();
        assert_eq!(out.query, out.high_level);
        assert_eq!(out.high_level, out.low_level);
        assert_eq!(out.query.len(), query_vec.len());
        assert_eq!(
            provider.calls.load(Ordering::SeqCst),
            0,
            "must reuse query_vec — no embed() batch"
        );
    }

    #[tokio::test]
    async fn compute_unique_batch_is_one_embed_rtt_for_distinct_levels() {
        use crate::keywords::QueryIntent;
        use edgequake_llm::MockProvider;
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct CountingEmbed {
            inner: MockProvider,
            calls: AtomicUsize,
            last_len: AtomicUsize,
        }
        #[async_trait::async_trait]
        impl EmbeddingProvider for CountingEmbed {
            fn name(&self) -> &str {
                EmbeddingProvider::name(&self.inner)
            }
            fn model(&self) -> &str {
                EmbeddingProvider::model(&self.inner)
            }
            fn dimension(&self) -> usize {
                EmbeddingProvider::dimension(&self.inner)
            }
            fn max_tokens(&self) -> usize {
                EmbeddingProvider::max_tokens(&self.inner)
            }
            async fn embed(&self, texts: &[String]) -> edgequake_llm::Result<Vec<Vec<f32>>> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                self.last_len.store(texts.len(), Ordering::SeqCst);
                self.inner.embed(texts).await
            }
            async fn embed_one(&self, text: &str) -> edgequake_llm::Result<Vec<f32>> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                self.inner.embed_one(text).await
            }
        }

        let provider = CountingEmbed {
            inner: MockProvider::default(),
            calls: AtomicUsize::new(0),
            last_len: AtomicUsize::new(0),
        };
        let keywords = ExtractedKeywords::new(
            vec!["oncology".into()],
            vec!["BRCA1".into()],
            QueryIntent::Factual,
        );
        let _ = QueryEmbeddings::compute("what is BRCA1?", &keywords, &provider)
            .await
            .unwrap();
        assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
        assert_eq!(provider.last_len.load(Ordering::SeqCst), 3);
    }

    /// @implements SPEC-004: build_prompt with system_prompt_extension
    mod system_prompt_tests {
        use super::*;
        use crate::context::{QueryContext, RetrievedChunk};
        use edgequake_llm::MockProvider;
        use edgequake_storage::{MemoryGraphStorage, MemoryVectorStorage};
        use std::sync::{Arc, Mutex};

        /// Serialize env-var prompt tests (parallel cargo test races on process env).
        static ANSWER_PROMPT_ENV_LOCK: Mutex<()> = Mutex::new(());

        /// Helper to create a minimal QueryEngine for prompt tests.
        fn create_prompt_test_engine() -> QueryEngine {
            let vector_storage = Arc::new(MemoryVectorStorage::new("test", 384));
            let graph_storage = Arc::new(MemoryGraphStorage::new("test"));
            let embedding_provider: Arc<dyn crate::EmbeddingProvider> =
                Arc::new(MockProvider::default());
            let llm_provider: Arc<dyn crate::LLMProvider> = Arc::new(MockProvider::default());

            QueryEngine::new(
                QueryEngineConfig::default(),
                vector_storage,
                graph_storage,
                embedding_provider,
                llm_provider,
            )
        }

        /// Helper to create a non-empty context for prompt testing.
        fn test_context() -> QueryContext {
            let mut ctx = QueryContext::default();
            ctx.chunks.push(RetrievedChunk::new(
                "chunk-1",
                "Rust is a systems programming language.",
                0.9,
            ));
            ctx
        }

        #[test]
        fn test_build_prompt_without_system_prompt() {
            let engine = create_prompt_test_engine();
            let context = test_context();

            let prompt = engine.build_prompt("What is Rust?", &context, None, &[], None, None);

            assert!(prompt.contains("---Role---"));
            assert!(prompt.contains("---Instructions---"));
            assert!(prompt.contains("---Context---"));
            assert!(prompt.contains("What is Rust?"));
            // Should NOT contain additional instructions section
            assert!(!prompt.contains("---Additional Instructions---"));
        }

        #[test]
        fn test_build_prompt_with_system_prompt() {
            let engine = create_prompt_test_engine();
            let context = test_context();

            let prompt = engine.build_prompt(
                "What is Rust?",
                &context,
                Some("Always respond in French. Be concise."),
                &[],
                None,
                None,
            );

            assert!(prompt.contains("---Role---"));
            assert!(prompt.contains("---Instructions---"));
            assert!(prompt.contains("---Additional Instructions---"));
            assert!(prompt.contains("Always respond in French. Be concise."));
            assert!(prompt.contains("---Context---"));
            assert!(prompt.contains("What is Rust?"));

            // Additional instructions should appear between instructions and context
            let instructions_pos = prompt.find("---Instructions---").unwrap();
            let additional_pos = prompt.find("---Additional Instructions---").unwrap();
            let context_pos = prompt.find("---Context---").unwrap();
            assert!(
                instructions_pos < additional_pos,
                "Additional instructions should come after base instructions"
            );
            assert!(
                additional_pos < context_pos,
                "Additional instructions should come before context"
            );
        }

        #[test]
        fn test_build_prompt_with_empty_system_prompt() {
            let engine = create_prompt_test_engine();
            let context = test_context();

            // Empty string should behave like None
            let prompt = engine.build_prompt("What is Rust?", &context, Some(""), &[], None, None);
            assert!(!prompt.contains("---Additional Instructions---"));

            // Whitespace-only should also behave like None
            let prompt = engine.build_prompt(
                "What is Rust?",
                &context,
                Some("   \n\t  "),
                &[],
                None,
                None,
            );
            assert!(!prompt.contains("---Additional Instructions---"));
        }

        #[test]
        fn test_build_prompt_specific_style_names_entities() {
            let _guard = ANSWER_PROMPT_ENV_LOCK
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            std::env::set_var("EDGEQUAKE_ANSWER_PROMPT", "specific");
            std::env::remove_var("EDGEQUAKE_ANSWER_SPECIFIC_TYPES");
            let engine = create_prompt_test_engine();
            let context = test_context();
            let prompt = engine.build_prompt(
                "Which PARP inhibitors are recommended?",
                &context,
                None,
                &[],
                None,
                None,
            );
            assert!(
                prompt.contains("specific named items") || prompt.contains("name those members"),
                "specific prompt missing specificity instructions"
            );
            assert!(
                !prompt.contains("Do not attempt to guess"),
                "specific must not use LR abstain wording"
            );
            std::env::remove_var("EDGEQUAKE_ANSWER_PROMPT");
        }

        #[test]
        fn test_build_prompt_specific_types_scopes_to_complex() {
            let _guard = ANSWER_PROMPT_ENV_LOCK
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            std::env::set_var("EDGEQUAKE_ANSWER_PROMPT", "specific");
            std::env::set_var("EDGEQUAKE_ANSWER_SPECIFIC_TYPES", "complex");
            let engine = create_prompt_test_engine();
            let context = test_context();

            let complex = engine.build_prompt(
                "Which PARP inhibitors are recommended?",
                &context,
                None,
                &[],
                Some("Complex Reasoning"),
                None,
            );
            assert!(
                complex.contains("name those members") || complex.contains("specific named items"),
                "Complex question_type must get specific prompt"
            );

            let fact = engine.build_prompt(
                "What is the capital?",
                &context,
                None,
                &[],
                Some("Fact Retrieval"),
                None,
            );
            assert!(
                !fact.contains("name those members") && !fact.contains("specific named items"),
                "Fact question_type must keep default prompt under SPECIFIC_TYPES=complex"
            );

            let missing = engine.build_prompt("q", &context, None, &[], None, None);
            assert!(
                !missing.contains("name those members")
                    && !missing.contains("specific named items"),
                "missing question_type must keep default when types scoped"
            );

            std::env::remove_var("EDGEQUAKE_ANSWER_PROMPT");
            std::env::remove_var("EDGEQUAKE_ANSWER_SPECIFIC_TYPES");
        }

        #[test]
        fn test_build_prompt_empty_context() {
            let engine = create_prompt_test_engine();
            let empty_context = QueryContext::default();

            // Empty context should return a "no information" message regardless of system_prompt
            let prompt =
                engine.build_prompt("query", &empty_context, Some("Be concise"), &[], None, None);
            assert!(prompt.contains("couldn't find any relevant information"));
            assert!(!prompt.contains("---Additional Instructions---"));
        }

        /// P-G6c (RC-11): a default engine has no reranker, so `has_reranker`
        /// must report `false`. The sync `/query` handler uses this to report
        /// `reranked` truthfully instead of faking a rerank.
        #[test]
        fn test_has_reranker_false_by_default() {
            let engine = create_prompt_test_engine();
            assert!(!engine.has_reranker());
        }

        #[test]
        fn test_response_type_in_system_prompt() {
            let engine = create_prompt_test_engine();
            let context = test_context();
            let system =
                engine.build_system_prompt(&context, None, &[], None, Some("Bullet Points"));
            assert!(system.contains("Structure the answer as: Bullet Points."));
            assert!(!system.contains("---User Query---"));
            let combined = engine.build_prompt(
                "What is Rust?",
                &context,
                None,
                &[],
                None,
                Some("Bullet Points"),
            );
            assert!(combined.contains("---User Query---"));
            assert!(combined.contains("What is Rust?"));
        }

        #[test]
        fn gold_prompt_keeps_later_supporting_clauses() {
            let engine = create_prompt_test_engine();
            let system = engine.build_system_prompt(
                &test_context(),
                Some("Do NOT append citation markers. Plain answer text only."),
                &[],
                None,
                None,
            );
            assert!(system.contains("Do not stop after the first supporting fact"));
            assert!(!system.contains("partial answer with specific data is better"));
        }

        #[test]
        fn product_prompt_still_allows_a_partial_answer() {
            let engine = create_prompt_test_engine();
            let system = engine.build_system_prompt(&test_context(), None, &[], None, None);
            assert!(system.contains("partial answer with specific data is better"));
        }

        #[tokio::test]
        async fn test_generate_uses_chat_system_user_by_default() {
            use async_trait::async_trait;
            use edgequake_llm::traits::{
                ChatMessage, CompletionOptions, LLMProvider, LLMResponse, ToolDefinition,
            };
            use std::sync::atomic::{AtomicUsize, Ordering};

            struct ChatCountingProvider {
                chat_calls: AtomicUsize,
                complete_calls: AtomicUsize,
                last_roles: std::sync::Mutex<Vec<String>>,
            }

            #[async_trait]
            impl LLMProvider for ChatCountingProvider {
                fn name(&self) -> &str {
                    "chat-count"
                }
                fn model(&self) -> &str {
                    "chat-count"
                }
                fn max_context_length(&self) -> usize {
                    8192
                }
                async fn complete(&self, _prompt: &str) -> edgequake_llm::Result<LLMResponse> {
                    self.complete_calls.fetch_add(1, Ordering::SeqCst);
                    Ok(LLMResponse::new("blob", "chat-count"))
                }
                async fn complete_with_options(
                    &self,
                    prompt: &str,
                    _options: &CompletionOptions,
                ) -> edgequake_llm::Result<LLMResponse> {
                    self.complete(prompt).await
                }
                async fn chat(
                    &self,
                    messages: &[ChatMessage],
                    _options: Option<&CompletionOptions>,
                ) -> edgequake_llm::Result<LLMResponse> {
                    self.chat_calls.fetch_add(1, Ordering::SeqCst);
                    *self.last_roles.lock().unwrap() = messages
                        .iter()
                        .map(|m| m.role.as_str().to_string())
                        .collect();
                    Ok(LLMResponse::new(
                        "Rust is a systems language [1].",
                        "chat-count",
                    ))
                }
                async fn chat_with_tools(
                    &self,
                    messages: &[ChatMessage],
                    _tools: &[ToolDefinition],
                    _tool_choice: Option<edgequake_llm::traits::ToolChoice>,
                    options: Option<&CompletionOptions>,
                ) -> edgequake_llm::Result<LLMResponse> {
                    self.chat(messages, options).await
                }
            }

            let provider = Arc::new(ChatCountingProvider {
                chat_calls: AtomicUsize::new(0),
                complete_calls: AtomicUsize::new(0),
                last_roles: std::sync::Mutex::new(Vec::new()),
            });
            let vector_storage = Arc::new(MemoryVectorStorage::new("chat-split", 384));
            let graph_storage = Arc::new(MemoryGraphStorage::new("chat-split"));
            let embedding_provider: Arc<dyn crate::EmbeddingProvider> =
                Arc::new(MockProvider::default());
            let engine = {
                let _guard = ANSWER_PROMPT_ENV_LOCK
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                std::env::remove_var("EDGEQUAKE_ANSWER_COMPLETE_BLOB");
                QueryEngine::new(
                    QueryEngineConfig::default(),
                    vector_storage,
                    graph_storage,
                    embedding_provider,
                    provider.clone() as Arc<dyn crate::LLMProvider>,
                )
            };
            let context = test_context();
            let (answer, _) = engine
                .generate_answer_with_provider(
                    "What is Rust?",
                    &context,
                    None,
                    None,
                    None,
                    &[],
                    None,
                    Some("Multiple Paragraphs"),
                    None,
                )
                .await
                .unwrap();
            assert_eq!(provider.chat_calls.load(Ordering::SeqCst), 1);
            assert_eq!(provider.complete_calls.load(Ordering::SeqCst), 0);
            assert_eq!(
                *provider.last_roles.lock().unwrap(),
                vec!["system".to_string(), "user".to_string()]
            );
            // 082 gold-compat not active → citations may remain; non-empty answer.
            assert!(!answer.is_empty());
        }
    }
}
