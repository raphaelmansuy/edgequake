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

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::error::{QueryError, Result};
use crate::keywords::{
    CachedKeywordExtractor, ExtractedKeywords, InMemoryKeywordCache, KeywordExtractor,
    LLMKeywordExtractor, MockKeywordExtractor,
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
    /// arms in parallel and blends them with these weights after min-max
    /// normalizing each arm's scores. Weights need not sum to 1 (they are
    /// normalized at use). When all three are equal, Mix ordering matches the
    /// round-robin Hybrid ordering on identical fixtures (backward compatible).
    /// A weight of 0 for an arm means it contributes 0 to the blend (E25).
    pub mix_local_weight: f32,
    pub mix_global_weight: f32,
    pub mix_naive_weight: f32,

    /// Enable BM25 sparse retrieval fused with vector ANN (SPEC-023 I10, default on).
    pub enable_bm25_retrieval: bool,

    /// Expand global mode with co-community entities (SPEC-023 I6, default on).
    pub enable_community_global: bool,

    /// Vector candidate pool multiplier for BM25 fusion in naive mode.
    pub bm25_candidate_multiplier: usize,
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
            // WHY derived from max_context_tokens: The truncation budget MUST match
            // the context token budget, otherwise the system fetches chunks it then
            // throws away. LightRAG splits: 50% entities, 50% relationships, chunks
            // fill the remainder. With 30K total: entities=10K, rels=10K, chunks=10K.
            truncation: TruncationConfig {
                max_entity_tokens: 10000,
                max_relation_tokens: 10000,
                max_total_tokens: 30000,
            },
            keyword_cache_ttl_secs: 24 * 60 * 60, // 24 hours
            enable_rerank: true,                  // Enable by default for retrieval quality
            // WHY 0.1: BM25 scores can be low for short documents or simple queries.
            // 0.3 was too aggressive and filtered out valid chunks. 0.1 matches min_score.
            min_rerank_score: 0.1,
            // WHY 20: Match max_chunks to keep all chunk candidates after reranking.
            rerank_top_k: 20,
            // P-G8: equal weights preserve Hybrid ordering on identical fixtures.
            mix_local_weight: 1.0,
            mix_global_weight: 1.0,
            mix_naive_weight: 1.0,
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
}

impl QueryEmbeddings {
    /// Compute all embeddings in a single batch.
    pub async fn compute(
        query: &str,
        keywords: &ExtractedKeywords,
        embedder: &dyn EmbeddingProvider,
    ) -> Result<Self> {
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

        // Batch embed all three texts
        let texts = vec![query.to_string(), high_level_text, low_level_text];

        let embeddings = embedder.embed(&texts).await.map_err(QueryError::from)?;

        if embeddings.len() != 3 {
            return Err(QueryError::Internal(format!(
                "Expected 3 embeddings, got {}",
                embeddings.len()
            )));
        }

        Ok(Self {
            query: embeddings[0].clone(),
            high_level: embeddings[1].clone(),
            low_level: embeddings[2].clone(),
        })
    }

    /// Simple embedding (same for all levels).
    pub fn uniform(embedding: Vec<f32>) -> Self {
        Self {
            query: embedding.clone(),
            high_level: embedding.clone(),
            low_level: embedding,
        }
    }

    /// Compute keyword-level embeddings when the query vector is already available.
    ///
    /// WHY: In the parallel query pipeline, the query embedding is computed
    /// concurrently with keyword extraction. Once both are ready, this method
    /// embeds only the keyword texts, avoiding a redundant re-embedding of the
    /// query and reducing total embedding calls.
    ///
    /// If both keyword texts fall back to the query string (empty keywords),
    /// the pre-computed `query_vec` is reused for all three levels — no extra
    /// embedding call is made at all.
    pub async fn compute_with_query_vec(
        query: &str,
        query_vec: Vec<f32>,
        keywords: &ExtractedKeywords,
        embedder: &dyn EmbeddingProvider,
    ) -> Result<Self> {
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

        // When keyword extraction is off, high/low texts equal the query string.
        // Batch-embed three slots so providers (e.g. MockProvider queue) can supply
        // distinct query / high_level / low_level vectors — required for Local/Global
        // mode ranking (SPEC-017 / e2e_engine_impl chunk-ranking contract).
        if high_level_text == query && low_level_text == query {
            let texts = vec![query.to_string(), query.to_string(), query.to_string()];
            let embeds = embedder.embed(&texts).await.map_err(QueryError::from)?;
            if embeds.len() >= 3 {
                return Ok(Self {
                    query: embeds[0].clone(),
                    high_level: embeds[1].clone(),
                    low_level: embeds[2].clone(),
                });
            }
            // Fallback: reuse parallel embed_one result when provider returns fewer.
            return Ok(Self {
                query: query_vec.clone(),
                high_level: query_vec.clone(),
                low_level: query_vec,
            });
        }

        // Embed only the keyword texts (query_vec is already computed).
        let texts = vec![high_level_text, low_level_text];
        let embeds = embedder.embed(&texts).await.map_err(QueryError::from)?;
        if embeds.len() != 2 {
            return Err(QueryError::Internal(format!(
                "Expected 2 keyword embeddings, got {}",
                embeds.len()
            )));
        }

        Ok(Self {
            query: query_vec,
            high_level: embeds[0].clone(),
            low_level: embeds[1].clone(),
        })
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
        // Create cached keyword extractor
        let base_extractor = Arc::new(LLMKeywordExtractor::new(llm_provider.clone()));
        let cache = Arc::new(InMemoryKeywordCache::new(1000));
        let keyword_extractor: Arc<dyn KeywordExtractor> = Arc::new(CachedKeywordExtractor::new(
            base_extractor,
            cache,
            std::time::Duration::from_secs(config.keyword_cache_ttl_secs),
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
        }
    }

    /// Create with a reranker for improved retrieval precision.
    pub fn with_reranker(mut self, reranker: Arc<dyn Reranker>) -> Self {
        self.reranker = Some(reranker);
        self
    }

    /// Attach KV storage for chunk content hydration (SPEC-024 2.5).
    pub fn with_kv_storage(mut self, kv: Arc<dyn edgequake_storage::traits::KVStorage>) -> Self {
        self.kv_storage = Some(kv);
        self
    }

    /// Wrap the engine's embedding provider in an LRU+TTL embedding cache
    /// (P-G9 / RC-14). Repeated `embed_one` calls for the same query text skip
    /// the embedding round-trip. Batch `embed` (ingestion) is delegated
    /// unchanged, so this never affects ingestion semantics.
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

    /// Get the engine's default vector storage.
    ///
    /// WHY: Same rationale as `default_embedding_provider` — callers with partial
    /// overrides need the default vector storage to construct full config calls.
    /// @implements FIX-168
    pub fn default_vector_storage(&self) -> Arc<dyn VectorStorage> {
        self.vector_storage.clone()
    }
}

mod modes;
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

    /// @implements SPEC-004: build_prompt with system_prompt_extension
    mod system_prompt_tests {
        use super::*;
        use crate::context::{QueryContext, RetrievedChunk};
        use edgequake_llm::MockProvider;
        use edgequake_storage::{MemoryGraphStorage, MemoryVectorStorage};
        use std::sync::Arc;

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

            let prompt = engine.build_prompt("What is Rust?", &context, None, &[]);

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
            let prompt = engine.build_prompt("What is Rust?", &context, Some(""), &[]);
            assert!(!prompt.contains("---Additional Instructions---"));

            // Whitespace-only should also behave like None
            let prompt = engine.build_prompt("What is Rust?", &context, Some("   \n\t  "), &[]);
            assert!(!prompt.contains("---Additional Instructions---"));
        }

        #[test]
        fn test_build_prompt_empty_context() {
            let engine = create_prompt_test_engine();
            let empty_context = QueryContext::default();

            // Empty context should return a "no information" message regardless of system_prompt
            let prompt = engine.build_prompt("query", &empty_context, Some("Be concise"), &[]);
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
    }
}
