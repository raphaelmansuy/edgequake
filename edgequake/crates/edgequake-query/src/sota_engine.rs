//! SOTA Query Engine - LightRAG-inspired implementation.
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
//! - **FEAT0109**: SOTA Query Delegation
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

use crate::context::{QueryContext, RetrievedRelationship};
use crate::error::{QueryError, Result};
use crate::helpers::{
    build_chunk_from_result, build_entity_from_node, build_relationship_from_edge,
};
use crate::keywords::{
    CachedKeywordExtractor, ExtractedKeywords, InMemoryKeywordCache, KeywordExtractor,
    LLMKeywordExtractor, MockKeywordExtractor, QueryIntent,
};
use crate::modes::QueryMode;
use crate::tokenizer::{SimpleTokenizer, Tokenizer};
use crate::truncation::{balance_context, TruncationConfig};
use crate::vector_filter::{filter_by_type, VectorType};

use edgequake_llm::traits::{EmbeddingProvider, LLMProvider};
use edgequake_llm::Reranker;
use edgequake_storage::traits::{GraphStorage, VectorStorage};

/// Configuration for the SOTA query engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SOTAQueryConfig {
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
}

impl Default for SOTAQueryConfig {
    fn default() -> Self {
        Self {
            default_mode: QueryMode::Hybrid,
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
            min_score: 0.1,
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
            enable_rerank: true,                  // Enable by default for SOTA quality
            // WHY 0.1: BM25 scores can be low for short documents or simple queries.
            // 0.3 was too aggressive and filtered out valid chunks. 0.1 matches min_score.
            min_rerank_score: 0.1,
            // WHY 20: Match max_chunks to keep all chunk candidates after reranking.
            rerank_top_k: 20,
        }
    }
}

/// Query embeddings for different keyword levels.
///
/// LightRAG uses different embeddings for different modes:
/// - low_level: Entity search (Local mode)
/// - high_level: Relationship search (Global mode)
/// - query: Direct chunk search (Naive mode)
#[derive(Debug, Clone)]
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
}

/// SOTA Query Engine with LightRAG-inspired enhancements.
pub struct SOTAQueryEngine {
    config: SOTAQueryConfig,
    vector_storage: Arc<dyn VectorStorage>,
    graph_storage: Arc<dyn GraphStorage>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    llm_provider: Arc<dyn LLMProvider>,
    keyword_extractor: Arc<dyn KeywordExtractor>,
    tokenizer: Arc<dyn Tokenizer>,
    /// Optional reranker for improved retrieval precision.
    reranker: Option<Arc<dyn Reranker>>,
    /// Cache for keyword validation (keyword -> exists_in_graph).
    /// WHY: Avoids repeated graph lookups for the same keywords.
    keyword_validation_cache: Arc<tokio::sync::RwLock<std::collections::HashMap<String, bool>>>,
}

impl SOTAQueryEngine {
    /// Create a new SOTA query engine.
    pub fn new(
        config: SOTAQueryConfig,
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
            reranker: None, // No reranker by default
            keyword_validation_cache: Arc::new(tokio::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
        }
    }

    /// Create with a reranker for improved retrieval precision.
    pub fn with_reranker(mut self, reranker: Arc<dyn Reranker>) -> Self {
        self.reranker = Some(reranker);
        self
    }

    /// Create with mock keyword extractor (for testing).
    pub fn with_mock_keywords(
        config: SOTAQueryConfig,
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
            reranker: None,
            keyword_validation_cache: Arc::new(tokio::sync::RwLock::new(
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

    /// Rerank chunks using the configured reranker.
    ///
    /// Applies reranking to improve retrieval precision:
    /// 1. Calls the reranker with query and chunk contents
    /// 2. Filters chunks by min_rerank_score
    /// 3. Returns top_k chunks sorted by rerank score
    async fn rerank_chunks(
        &self,
        query: &str,
        mut chunks: Vec<crate::context::RetrievedChunk>,
        enable_override: Option<bool>,
        top_k_override: Option<usize>,
    ) -> Vec<crate::context::RetrievedChunk> {
        // Check if reranking is enabled (use request override if provided)
        let enable_rerank = enable_override.unwrap_or(self.config.enable_rerank);
        let rerank_top_k = top_k_override.unwrap_or(self.config.rerank_top_k);

        // Skip if reranking is disabled or no reranker configured
        if !enable_rerank || self.reranker.is_none() || chunks.is_empty() {
            return chunks;
        }

        let reranker = self.reranker.as_ref().unwrap();

        // Extract contents for reranking
        let documents: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();

        // Call the reranker
        match reranker.rerank(query, &documents, Some(rerank_top_k)).await {
            Ok(results) => {
                tracing::debug!(
                    query = %query,
                    chunk_count = chunks.len(),
                    result_count = results.len(),
                    "Reranked chunks"
                );

                // Log all rerank scores for debugging
                for r in &results {
                    tracing::debug!(
                        index = r.index,
                        score = r.relevance_score,
                        min_required = self.config.min_rerank_score,
                        passes = r.relevance_score >= self.config.min_rerank_score as f64,
                        "OODA-231: Rerank result score check"
                    );
                }

                // Build index -> score map
                let score_map: std::collections::HashMap<usize, f64> = results
                    .iter()
                    .map(|r| (r.index, r.relevance_score))
                    .collect();

                // Update scores and filter by min score
                let mut reranked: Vec<_> = chunks
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, chunk)| {
                        score_map.get(&idx).and_then(|&score| {
                            if score >= self.config.min_rerank_score as f64 {
                                let mut c = chunk.clone();
                                c.score = score as f32;
                                Some(c)
                            } else {
                                None
                            }
                        })
                    })
                    .collect();

                // OODA-231: Fallback - if ALL chunks were filtered by min_rerank_score,
                // return top_k original chunks to preserve source context.
                // WHY: BM25 reranker scores 0.0 for terms that don't appear in chunks,
                // but those chunks may still be relevant (e.g., found via entity graph).
                if reranked.is_empty() && !chunks.is_empty() {
                    tracing::warn!(
                        query = %query,
                        original_chunks = chunks.len(),
                        min_rerank_score = self.config.min_rerank_score,
                        "OODA-231: All chunks filtered by reranking, falling back to original chunks"
                    );
                    chunks.truncate(rerank_top_k);
                    return chunks;
                }

                // Sort by score descending
                reranked.sort_by(|a, b| {
                    b.score
                        .partial_cmp(&a.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

                // Return top_k
                reranked.truncate(rerank_top_k);
                reranked
            }
            Err(e) => {
                tracing::warn!(error = %e, "Reranking failed, returning original chunks");
                chunks.truncate(rerank_top_k);
                chunks
            }
        }
    }

    /// Sort entities by degree (descending) for importance-based ranking.
    ///
    /// High-degree entities are more connected in the knowledge graph
    /// and typically represent more important/central concepts.
    fn sort_entities_by_degree(&self, entities: &mut [crate::context::RetrievedEntity]) {
        entities.sort_by(|a, b| {
            // Sort by degree descending (higher degree = more important)
            b.degree.cmp(&a.degree)
        });
        tracing::debug!(
            entity_count = entities.len(),
            top_degree = entities.first().map(|e| e.degree).unwrap_or(0),
            "Sorted entities by degree"
        );
    }

    /// Validate keywords against the knowledge graph.
    ///
    /// WHY: When a query contains terms that don't exist in the knowledge base
    /// (e.g., "STLA Medium"), including them in the embedding computation dilutes
    /// the semantic search and reduces retrieval quality for terms that DO exist.
    ///
    /// This method checks each low-level keyword against the graph and drops
    /// those with zero entity matches, preventing embedding dilution.
    async fn validate_keywords(&self, keywords: &ExtractedKeywords) -> ExtractedKeywords {
        if keywords.low_level.is_empty() {
            return keywords.clone();
        }

        let mut validated_low_level = Vec::new();
        let mut dropped_keywords = Vec::new();

        for keyword in &keywords.low_level {
            // Check cache first
            let cache_key = keyword.to_lowercase();
            let cached_result = {
                let cache = self.keyword_validation_cache.read().await;
                cache.get(&cache_key).copied()
            };

            let exists = if let Some(exists) = cached_result {
                // Cache hit
                exists
            } else {
                // Cache miss - check graph
                let matches = self.graph_storage.search_labels(keyword, 1).await;
                let exists = matches.map(|labels| !labels.is_empty()).unwrap_or(false);

                // Update cache
                {
                    let mut cache = self.keyword_validation_cache.write().await;
                    // Limit cache size to prevent unbounded growth
                    if cache.len() < 10000 {
                        cache.insert(cache_key, exists);
                    }
                }
                exists
            };

            if exists {
                validated_low_level.push(keyword.clone());
            } else {
                dropped_keywords.push(keyword.clone());
            }
        }

        if !dropped_keywords.is_empty() {
            tracing::info!(
                dropped = ?dropped_keywords,
                kept = ?validated_low_level,
                "Dropped keywords with no graph matches"
            );
        }

        // If ALL keywords were dropped, fall back to original to avoid empty search
        if validated_low_level.is_empty() {
            tracing::warn!(
                original = ?keywords.low_level,
                "All keywords dropped - falling back to original keywords"
            );
            return keywords.clone();
        }

        ExtractedKeywords::new(
            keywords.high_level.clone(),
            validated_low_level,
            keywords.query_intent,
        )
    }

    /// Execute a query with full SOTA pipeline.
    ///
    /// # WHY: 5-Stage Query Pipeline
    ///
    /// The query flow follows LightRAG's proven architecture:
    ///
    /// 1. **Keyword Extraction** - Extract high/low-level keywords using LLM
    ///    - WHY high-level: Relationships (e.g., "partnership", "acquired")
    ///    - WHY low-level: Entities (e.g., "Apple", "Microsoft")
    ///    - WHY caching: Same queries reuse extraction results (24h TTL)
    ///
    /// 2. **Keyword Validation** - Check keywords exist in knowledge graph
    ///    - WHY: Non-existent keywords dilute embedding computation
    ///    - Example: "STLA Medium" not in graph → drop it
    ///
    /// 3. **Mode Selection** - Choose retrieval strategy
    ///    - Local: Entities + 1-hop neighbors (specific questions)
    ///    - Global: Relationships + community summaries (broad themes)
    ///    - Hybrid: Both local + global (best quality, higher cost)
    ///    - Naive: Chunks only (keyword search fallback)
    ///
    /// 4. **Vector Retrieval** - Semantic search with mode-specific embedding
    ///    - WHY different embeddings: low_level → entity search, high_level → relationship search
    ///
    /// 5. **Token Budgeting** - Fit context within LLM limits
    ///    - WHY: LLM context windows are limited; we prioritize high-scoring content
    ///
    /// @implements FEAT0109 (SOTA Query Engine)
    pub async fn query(
        &self,
        request: crate::engine::QueryRequest,
    ) -> Result<crate::engine::QueryResponse> {
        let start = std::time::Instant::now();
        let mut stats = crate::engine::QueryStats::default();

        // Step 1: Extract keywords (with caching)
        let raw_keywords = if self.config.use_keyword_extraction {
            let kw_start = std::time::Instant::now();
            let kw = self
                .keyword_extractor
                .extract_extended(&request.query)
                .await?;
            tracing::debug!(
                query = %request.query,
                high_level = ?kw.high_level,
                low_level = ?kw.low_level,
                intent = %kw.query_intent,
                "Extracted keywords"
            );
            stats.embedding_time_ms += kw_start.elapsed().as_millis() as u64;
            kw
        } else {
            ExtractedKeywords::new(vec![], vec![], QueryIntent::Exploratory)
        };

        // Step 1.5: Validate keywords against knowledge graph
        // WHY: Drop keywords with no graph matches to prevent embedding dilution
        let keywords = self.validate_keywords(&raw_keywords).await;

        // Step 2: Determine query mode
        let mode = if let Some(m) = request.mode {
            m
        } else if self.config.use_adaptive_mode {
            keywords.query_intent.recommended_mode()
        } else {
            self.config.default_mode
        };

        tracing::debug!(mode = %mode, "Selected query mode");

        // Step 3: Compute embeddings
        let embed_start = std::time::Instant::now();
        let embeddings =
            QueryEmbeddings::compute(&request.query, &keywords, self.embedding_provider.as_ref())
                .await?;
        stats.embedding_time_ms += embed_start.elapsed().as_millis() as u64;

        // Step 4: Mode-specific retrieval
        let retrieval_start = std::time::Instant::now();
        let context = match mode {
            QueryMode::Local => {
                self.query_local(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                )
                .await?
            }
            QueryMode::Global => {
                self.query_global(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                )
                .await?
            }
            QueryMode::Hybrid => {
                self.query_hybrid(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                )
                .await?
            }
            QueryMode::Mix => {
                self.query_mix(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                )
                .await?
            }
            QueryMode::Naive => {
                self.query_naive(&embeddings, request.tenant_id(), request.workspace_id())
                    .await?
            }
        };
        stats.retrieval_time_ms = retrieval_start.elapsed().as_millis() as u64;
        stats.context_tokens = context.token_count;

        // Step 4.5: Rerank chunks for improved precision
        let mut context = context;
        let should_rerank = request.enable_rerank.unwrap_or(self.config.enable_rerank);
        if should_rerank && self.reranker.is_some() {
            let rerank_start = std::time::Instant::now();
            let reranked_chunks = self
                .rerank_chunks(
                    &request.query,
                    context.chunks,
                    request.enable_rerank,
                    request.rerank_top_k,
                )
                .await;
            context.chunks = reranked_chunks;
            let rerank_time = rerank_start.elapsed().as_millis() as u64;
            tracing::debug!(rerank_time_ms = rerank_time, "Reranking completed");
            // Include rerank time in retrieval
            stats.retrieval_time_ms += rerank_time;
        }

        // Step 4.6: Sort entities by degree for importance-based ranking
        self.sort_entities_by_degree(&mut context.entities);

        // Step 5: Apply truncation
        let (truncated_entities, truncated_relationships, truncated_chunks) = balance_context(
            context.entities.clone(),
            context.relationships.clone(),
            context.chunks.clone(),
            &self.config.truncation,
            self.tokenizer.as_ref(),
        );

        let mut final_context = context;
        final_context.entities = truncated_entities;
        final_context.relationships = truncated_relationships;
        final_context.chunks = truncated_chunks;

        // Step 6: Generate answer (if not context-only)
        let (answer, generated_tokens) = if request.context_only {
            (String::new(), 0)
        } else if request.prompt_only {
            (self.build_prompt(&request.query, &final_context), 0)
        } else {
            let gen_start = std::time::Instant::now();
            let result = self.generate_answer(&request.query, &final_context).await?;
            stats.generation_time_ms = gen_start.elapsed().as_millis() as u64;
            result
        };

        stats.generated_tokens = generated_tokens;
        stats.total_time_ms = start.elapsed().as_millis() as u64;

        Ok(crate::engine::QueryResponse {
            answer,
            context: final_context,
            mode,
            stats,
        })
    }

    /// Execute a query with a workspace-specific embedding provider override.
    ///
    /// This method is used when the workspace has a different embedding configuration
    /// than the default engine provider. The override provider is used ONLY for
    /// computing query embeddings, not for document ingestion.
    ///
    /// @implements SPEC-032: Workspace-specific embedding in query process
    ///
    /// # Arguments
    ///
    /// * `request` - The query request
    /// * `embedding_provider` - The workspace-specific embedding provider
    ///
    /// # Returns
    ///
    /// Query response with answer, context, and stats.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let workspace_provider = ProviderFactory::create_embedding_provider(
    ///     "ollama", "embeddinggemma:latest", 768,
    /// )?;
    /// let response = engine.query_with_embedding_provider(request, workspace_provider).await?;
    /// ```
    pub async fn query_with_embedding_provider(
        &self,
        request: crate::engine::QueryRequest,
        embedding_provider: std::sync::Arc<dyn crate::EmbeddingProvider>,
    ) -> Result<crate::engine::QueryResponse> {
        let start = std::time::Instant::now();
        let mut stats = crate::engine::QueryStats::default();

        // Step 1: Extract keywords (with caching)
        let raw_keywords = if self.config.use_keyword_extraction {
            let kw_start = std::time::Instant::now();
            let kw = self
                .keyword_extractor
                .extract_extended(&request.query)
                .await?;
            tracing::debug!(
                query = %request.query,
                high_level = ?kw.high_level,
                low_level = ?kw.low_level,
                intent = %kw.query_intent,
                "Extracted keywords (workspace embedding)"
            );
            stats.embedding_time_ms += kw_start.elapsed().as_millis() as u64;
            kw
        } else {
            ExtractedKeywords::new(vec![], vec![], QueryIntent::Exploratory)
        };

        // Step 1.5: Validate keywords against knowledge graph
        let keywords = self.validate_keywords(&raw_keywords).await;

        // Step 2: Determine query mode
        let mode = if let Some(m) = request.mode {
            m
        } else if self.config.use_adaptive_mode {
            keywords.query_intent.recommended_mode()
        } else {
            self.config.default_mode
        };

        tracing::debug!(mode = %mode, "Selected query mode (workspace embedding)");

        // Step 3: Compute embeddings using WORKSPACE-SPECIFIC provider
        let embed_start = std::time::Instant::now();
        let embeddings =
            QueryEmbeddings::compute(&request.query, &keywords, embedding_provider.as_ref())
                .await?;
        stats.embedding_time_ms += embed_start.elapsed().as_millis() as u64;

        // Step 4: Mode-specific retrieval (same as query method)
        let retrieval_start = std::time::Instant::now();
        let context = match mode {
            QueryMode::Local => {
                self.query_local(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                )
                .await?
            }
            QueryMode::Global => {
                self.query_global(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                )
                .await?
            }
            QueryMode::Hybrid => {
                self.query_hybrid(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                )
                .await?
            }
            QueryMode::Mix => {
                self.query_mix(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                )
                .await?
            }
            QueryMode::Naive => {
                self.query_naive(&embeddings, request.tenant_id(), request.workspace_id())
                    .await?
            }
        };
        stats.retrieval_time_ms = retrieval_start.elapsed().as_millis() as u64;
        stats.context_tokens = context.token_count;

        // Step 4.5: Rerank chunks
        let mut context = context;
        let should_rerank = request.enable_rerank.unwrap_or(self.config.enable_rerank);
        if should_rerank && self.reranker.is_some() {
            let rerank_start = std::time::Instant::now();
            let reranked_chunks = self
                .rerank_chunks(
                    &request.query,
                    context.chunks,
                    request.enable_rerank,
                    request.rerank_top_k,
                )
                .await;
            context.chunks = reranked_chunks;
            let rerank_time = rerank_start.elapsed().as_millis() as u64;
            stats.retrieval_time_ms += rerank_time;
        }

        // Step 4.6: Sort entities by degree
        self.sort_entities_by_degree(&mut context.entities);

        // Step 5: Apply truncation
        let (truncated_entities, truncated_relationships, truncated_chunks) = balance_context(
            context.entities.clone(),
            context.relationships.clone(),
            context.chunks.clone(),
            &self.config.truncation,
            self.tokenizer.as_ref(),
        );

        let mut final_context = context;
        final_context.entities = truncated_entities;
        final_context.relationships = truncated_relationships;
        final_context.chunks = truncated_chunks;

        // Step 6: Generate answer
        let (answer, generated_tokens) = if request.context_only {
            (String::new(), 0)
        } else if request.prompt_only {
            (self.build_prompt(&request.query, &final_context), 0)
        } else {
            let gen_start = std::time::Instant::now();
            let result = self.generate_answer(&request.query, &final_context).await?;
            stats.generation_time_ms = gen_start.elapsed().as_millis() as u64;
            result
        };

        stats.generated_tokens = generated_tokens;
        stats.total_time_ms = start.elapsed().as_millis() as u64;

        Ok(crate::engine::QueryResponse {
            answer,
            context: final_context,
            mode,
            stats,
        })
    }

    /// Execute a query with workspace-specific vector storage and embedding provider.
    ///
    /// SPEC-033: Full workspace isolation for vector storage.
    ///
    /// This method enables complete workspace isolation by using:
    /// - Workspace-specific embedding provider (for computing query embeddings)
    /// - Workspace-specific vector storage (for similarity search)
    ///
    /// WHY: Different workspaces may use different embedding models with different
    /// dimensions (e.g., OpenAI 1536 vs Ollama 768). The vector storage must match
    /// the embedding dimension for correct similarity search.
    ///
    /// # Arguments
    ///
    /// * `request` - The query request
    /// * `embedding_provider` - The workspace-specific embedding provider
    /// * `vector_storage` - The workspace-specific vector storage
    ///
    /// # Example
    ///
    /// ```ignore
    /// let ws_embedding = ProviderFactory::create_embedding_provider("ollama", "nomic-embed-text", 768)?;
    /// let ws_vector = registry.get_or_create(workspace_config).await?;
    /// let response = engine.query_with_workspace_config(request, ws_embedding, ws_vector).await?;
    /// ```
    pub async fn query_with_workspace_config(
        &self,
        request: crate::engine::QueryRequest,
        embedding_provider: std::sync::Arc<dyn crate::EmbeddingProvider>,
        vector_storage: std::sync::Arc<dyn VectorStorage>,
    ) -> Result<crate::engine::QueryResponse> {
        let start = std::time::Instant::now();
        let mut stats = crate::engine::QueryStats::default();

        // Step 1: Extract keywords (with caching)
        let raw_keywords = if self.config.use_keyword_extraction {
            let kw_start = std::time::Instant::now();
            let kw = self
                .keyword_extractor
                .extract_extended(&request.query)
                .await?;
            tracing::debug!(
                query = %request.query,
                high_level = ?kw.high_level,
                low_level = ?kw.low_level,
                intent = %kw.query_intent,
                "Extracted keywords (workspace config)"
            );
            stats.embedding_time_ms += kw_start.elapsed().as_millis() as u64;
            kw
        } else {
            ExtractedKeywords::new(vec![], vec![], QueryIntent::Exploratory)
        };

        // Step 1.5: Validate keywords against knowledge graph
        let keywords = self.validate_keywords(&raw_keywords).await;

        // Step 2: Determine query mode
        let mode = if let Some(m) = request.mode {
            m
        } else if self.config.use_adaptive_mode {
            keywords.query_intent.recommended_mode()
        } else {
            self.config.default_mode
        };

        tracing::debug!(mode = %mode, "Selected query mode (workspace config)");

        // Step 3: Compute embeddings using WORKSPACE-SPECIFIC embedding provider
        let embed_start = std::time::Instant::now();
        let embeddings =
            QueryEmbeddings::compute(&request.query, &keywords, embedding_provider.as_ref())
                .await?;
        stats.embedding_time_ms += embed_start.elapsed().as_millis() as u64;

        // Step 4: Mode-specific retrieval using WORKSPACE-SPECIFIC vector storage
        let retrieval_start = std::time::Instant::now();
        let context = match mode {
            QueryMode::Local => {
                self.query_local_with_vector_storage(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                    &vector_storage,
                )
                .await?
            }
            QueryMode::Global => {
                self.query_global_with_vector_storage(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                    &vector_storage,
                )
                .await?
            }
            QueryMode::Hybrid => {
                self.query_hybrid_with_vector_storage(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                    &vector_storage,
                )
                .await?
            }
            QueryMode::Mix => {
                self.query_mix_with_vector_storage(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                    &vector_storage,
                )
                .await?
            }
            QueryMode::Naive => {
                self.query_naive_with_vector_storage(
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                    &vector_storage,
                )
                .await?
            }
        };
        stats.retrieval_time_ms = retrieval_start.elapsed().as_millis() as u64;
        stats.context_tokens = context.token_count;

        tracing::debug!(
            chunks_from_retrieval = context.chunks.len(),
            entities_from_retrieval = context.entities.len(),
            "OODA-231: Context returned from mode-specific retrieval (query_with_workspace_config)"
        );

        // Step 4.5: Rerank chunks
        let mut context = context;
        let should_rerank = request.enable_rerank.unwrap_or(self.config.enable_rerank);
        tracing::debug!(
            chunks_before_rerank = context.chunks.len(),
            should_rerank = should_rerank,
            has_reranker = self.reranker.is_some(),
            "OODA-231: Before reranking step (query_with_workspace_config)"
        );
        if should_rerank && self.reranker.is_some() {
            let rerank_start = std::time::Instant::now();
            let reranked_chunks = self
                .rerank_chunks(
                    &request.query,
                    context.chunks,
                    request.enable_rerank,
                    request.rerank_top_k,
                )
                .await;
            context.chunks = reranked_chunks;
            let rerank_time = rerank_start.elapsed().as_millis() as u64;
            stats.retrieval_time_ms += rerank_time;
        }
        tracing::debug!(
            chunks_after_rerank = context.chunks.len(),
            "OODA-231: After reranking step (query_with_workspace_config)"
        );

        // Step 4.6: Sort entities by degree
        self.sort_entities_by_degree(&mut context.entities);

        // Step 5: Apply truncation
        let (truncated_entities, truncated_relationships, truncated_chunks) = balance_context(
            context.entities.clone(),
            context.relationships.clone(),
            context.chunks.clone(),
            &self.config.truncation,
            self.tokenizer.as_ref(),
        );

        let mut final_context = context;
        final_context.entities = truncated_entities;
        final_context.relationships = truncated_relationships;
        final_context.chunks = truncated_chunks;

        // Step 6: Generate answer
        let (answer, generated_tokens) = if request.context_only {
            (String::new(), 0)
        } else if request.prompt_only {
            (self.build_prompt(&request.query, &final_context), 0)
        } else {
            let gen_start = std::time::Instant::now();
            let result = self.generate_answer(&request.query, &final_context).await?;
            stats.generation_time_ms = gen_start.elapsed().as_millis() as u64;
            result
        };

        stats.generated_tokens = generated_tokens;
        stats.total_time_ms = start.elapsed().as_millis() as u64;

        Ok(crate::engine::QueryResponse {
            answer,
            context: final_context,
            mode,
            stats,
        })
    }

    /// Execute a query with full workspace configuration AND optional LLM override.
    ///
    /// This method combines workspace-specific embedding/vector storage for retrieval
    /// with an optional LLM provider override for answer generation. This is the
    /// recommended method for chat-style interfaces where users can select a different
    /// LLM model while still using workspace-specific embeddings.
    ///
    /// @implements SPEC-032: Workspace-specific embedding in query process
    /// @implements SPEC-033: Workspace vector isolation
    /// @implements OODA-228: Fix dimension mismatch in chat handler
    ///
    /// # Arguments
    ///
    /// * `request` - The query request
    /// * `embedding_provider` - The workspace-specific embedding provider
    /// * `vector_storage` - The workspace-specific vector storage
    /// * `llm_provider` - Optional LLM provider override for answer generation
    ///
    /// # Returns
    ///
    /// Query response using workspace embeddings and optionally custom LLM.
    pub async fn query_with_full_config(
        &self,
        request: crate::engine::QueryRequest,
        embedding_provider: std::sync::Arc<dyn crate::EmbeddingProvider>,
        vector_storage: std::sync::Arc<dyn VectorStorage>,
        llm_provider: Option<std::sync::Arc<dyn crate::LLMProvider>>,
    ) -> Result<crate::engine::QueryResponse> {
        let start = std::time::Instant::now();
        let mut stats = crate::engine::QueryStats::default();

        // Step 1: Extract keywords (with caching)
        // WHY: Use extract_with_llm_override when user selected a specific LLM provider.
        // This ensures keyword extraction uses the SAME LLM as answer generation.
        // Without this, keyword extraction would use the server default (often Ollama)
        // while answer generation uses the user's choice (e.g., OpenAI GPT-4).
        // This bug caused inconsistent behavior and unexpected costs.
        let raw_keywords = if self.config.use_keyword_extraction {
            let kw_start = std::time::Instant::now();
            let kw = self
                .keyword_extractor
                .extract_with_llm_override(&request.query, llm_provider.clone())
                .await?;
            tracing::debug!(
                query = %request.query,
                high_level = ?kw.high_level,
                low_level = ?kw.low_level,
                intent = %kw.query_intent,
                has_llm_override = llm_provider.is_some(),
                "Extracted keywords (full config)"
            );
            stats.embedding_time_ms += kw_start.elapsed().as_millis() as u64;
            kw
        } else {
            ExtractedKeywords::new(vec![], vec![], QueryIntent::Exploratory)
        };

        // Step 1.5: Validate keywords against knowledge graph
        let keywords = self.validate_keywords(&raw_keywords).await;

        // Step 2: Determine query mode
        let mode = if let Some(m) = request.mode {
            m
        } else if self.config.use_adaptive_mode {
            keywords.query_intent.recommended_mode()
        } else {
            self.config.default_mode
        };

        tracing::debug!(mode = %mode, "Selected query mode (full config)");

        // Step 3: Compute embeddings using WORKSPACE-SPECIFIC embedding provider
        let embed_start = std::time::Instant::now();
        let embeddings =
            QueryEmbeddings::compute(&request.query, &keywords, embedding_provider.as_ref())
                .await?;
        stats.embedding_time_ms += embed_start.elapsed().as_millis() as u64;

        // Step 4: Mode-specific retrieval using WORKSPACE-SPECIFIC vector storage
        let retrieval_start = std::time::Instant::now();
        let context = match mode {
            QueryMode::Local => {
                self.query_local_with_vector_storage(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                    &vector_storage,
                )
                .await?
            }
            QueryMode::Global => {
                self.query_global_with_vector_storage(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                    &vector_storage,
                )
                .await?
            }
            QueryMode::Hybrid => {
                self.query_hybrid_with_vector_storage(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                    &vector_storage,
                )
                .await?
            }
            QueryMode::Mix => {
                self.query_mix_with_vector_storage(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                    &vector_storage,
                )
                .await?
            }
            QueryMode::Naive => {
                self.query_naive_with_vector_storage(
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                    &vector_storage,
                )
                .await?
            }
        };
        stats.retrieval_time_ms = retrieval_start.elapsed().as_millis() as u64;
        stats.context_tokens = context.token_count;

        // Step 4.5: Rerank chunks
        let mut context = context;
        let should_rerank = request.enable_rerank.unwrap_or(self.config.enable_rerank);
        if should_rerank && self.reranker.is_some() {
            let rerank_start = std::time::Instant::now();
            let reranked_chunks = self
                .rerank_chunks(
                    &request.query,
                    context.chunks,
                    request.enable_rerank,
                    request.rerank_top_k,
                )
                .await;
            context.chunks = reranked_chunks;
            let rerank_time = rerank_start.elapsed().as_millis() as u64;
            stats.retrieval_time_ms += rerank_time;
        }

        // Step 4.6: Sort entities by degree
        self.sort_entities_by_degree(&mut context.entities);

        // Step 5: Apply truncation
        let (truncated_entities, truncated_relationships, truncated_chunks) = balance_context(
            context.entities.clone(),
            context.relationships.clone(),
            context.chunks.clone(),
            &self.config.truncation,
            self.tokenizer.as_ref(),
        );

        let mut final_context = context;
        final_context.entities = truncated_entities;
        final_context.relationships = truncated_relationships;
        final_context.chunks = truncated_chunks;

        // Step 6: Generate answer using OVERRIDE LLM or default
        let (answer, generated_tokens) = if request.context_only {
            (String::new(), 0)
        } else if request.prompt_only {
            (self.build_prompt(&request.query, &final_context), 0)
        } else {
            let gen_start = std::time::Instant::now();
            let result = if let Some(ref llm) = llm_provider {
                // Use override LLM provider
                self.generate_answer_with_provider(&request.query, &final_context, Some(llm))
                    .await?
            } else {
                // Use default LLM provider
                self.generate_answer(&request.query, &final_context).await?
            };
            stats.generation_time_ms = gen_start.elapsed().as_millis() as u64;
            result
        };

        stats.generated_tokens = generated_tokens;
        stats.total_time_ms = start.elapsed().as_millis() as u64;

        Ok(crate::engine::QueryResponse {
            answer,
            context: final_context,
            mode,
            stats,
        })
    }

    /// Execute a query streaming with full config (workspace embedding + storage + optional LLM override).
    ///
    /// This is the streaming equivalent of `query_with_full_config`. It returns context first,
    /// then streams tokens from the answer generation.
    ///
    /// @implements SPEC-032: Workspace-specific embedding in query process
    /// @implements SPEC-033: Workspace vector isolation
    /// @implements OODA-228: Fix dimension mismatch in chat handler (streaming variant)
    ///
    /// # Returns
    ///
    /// Tuple of (QueryContext, QueryMode, Token stream)
    pub async fn query_stream_with_full_config(
        &self,
        request: crate::engine::QueryRequest,
        embedding_provider: std::sync::Arc<dyn crate::EmbeddingProvider>,
        vector_storage: std::sync::Arc<dyn VectorStorage>,
        llm_provider: Option<std::sync::Arc<dyn crate::LLMProvider>>,
    ) -> Result<(
        QueryContext,
        QueryMode,
        futures::stream::BoxStream<'static, Result<String>>,
    )> {
        use futures::StreamExt;

        // Step 1: Extract keywords (with caching)
        // WHY: Use extract_with_llm_override when user selected a specific LLM provider.
        // This ensures keyword extraction uses the SAME LLM as answer generation.
        // Without this, keyword extraction would use the server default (often Ollama)
        // while answer generation uses the user's choice (e.g., OpenAI GPT-4).
        let raw_keywords = if self.config.use_keyword_extraction {
            self.keyword_extractor
                .extract_with_llm_override(&request.query, llm_provider.clone())
                .await?
        } else {
            ExtractedKeywords::new(vec![], vec![], QueryIntent::Exploratory)
        };

        // Step 1.5: Validate keywords against knowledge graph
        let keywords = self.validate_keywords(&raw_keywords).await;

        // Step 2: Determine query mode
        let mode = if let Some(m) = request.mode {
            m
        } else if self.config.use_adaptive_mode {
            keywords.query_intent.recommended_mode()
        } else {
            self.config.default_mode
        };

        tracing::debug!(mode = %mode, "Selected query mode (stream full config)");

        // Step 3: Compute embeddings using WORKSPACE-SPECIFIC embedding provider
        let embeddings =
            QueryEmbeddings::compute(&request.query, &keywords, embedding_provider.as_ref())
                .await?;

        // Step 4: Mode-specific retrieval using WORKSPACE-SPECIFIC vector storage
        let context = match mode {
            QueryMode::Local => {
                self.query_local_with_vector_storage(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                    &vector_storage,
                )
                .await?
            }
            QueryMode::Global => {
                self.query_global_with_vector_storage(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                    &vector_storage,
                )
                .await?
            }
            QueryMode::Hybrid => {
                self.query_hybrid_with_vector_storage(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                    &vector_storage,
                )
                .await?
            }
            QueryMode::Mix => {
                self.query_mix_with_vector_storage(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                    &vector_storage,
                )
                .await?
            }
            QueryMode::Naive => {
                self.query_naive_with_vector_storage(
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                    &vector_storage,
                )
                .await?
            }
        };

        // Step 4.5: Rerank chunks
        let mut context = context;
        let should_rerank = request.enable_rerank.unwrap_or(self.config.enable_rerank);
        if should_rerank && self.reranker.is_some() {
            let reranked_chunks = self
                .rerank_chunks(
                    &request.query,
                    context.chunks,
                    request.enable_rerank,
                    request.rerank_top_k,
                )
                .await;
            context.chunks = reranked_chunks;
        }

        // Step 4.6: Sort entities by degree
        self.sort_entities_by_degree(&mut context.entities);

        // Step 5: Apply truncation
        let (truncated_entities, truncated_relationships, truncated_chunks) = balance_context(
            context.entities.clone(),
            context.relationships.clone(),
            context.chunks.clone(),
            &self.config.truncation,
            self.tokenizer.as_ref(),
        );

        let mut final_context = context;
        final_context.entities = truncated_entities;
        final_context.relationships = truncated_relationships;
        final_context.chunks = truncated_chunks;

        // Step 6: Handle empty context
        if final_context.is_empty() {
            return Ok((
                final_context,
                mode,
                futures::stream::once(async {
                    Ok("I'm sorry, but I couldn't find any relevant information in my knowledge base to answer your question.".to_string())
                })
                .boxed(),
            ));
        }

        // Step 7: Build prompt and stream using LLM override or default
        let prompt = self.build_prompt(&request.query, &final_context);

        // Determine which LLM provider to use for streaming
        let llm_to_use = llm_provider
            .clone()
            .or_else(|| Some(self.llm_provider.clone()));

        let stream = if let Some(ref llm) = llm_to_use {
            // Check if provider supports streaming
            if llm.supports_streaming() {
                tracing::debug!("Using streaming mode for LLM provider (full config)");
                llm.stream(&prompt)
                    .await
                    .map(|stream| stream.map(|res| res.map_err(QueryError::from)).boxed())
                    .map_err(QueryError::from)?
            } else {
                // Fallback to non-streaming and wrap in a stream
                tracing::warn!(
                    provider = llm.name(),
                    "Provider doesn't support streaming (full config), falling back to non-streaming mode"
                );

                let prompt_clone = prompt.clone();
                let llm_clone = llm.clone();

                let response = llm_clone
                    .complete(&prompt_clone)
                    .await
                    .map_err(QueryError::from)?;

                futures::stream::once(async move { Ok(response.content) }).boxed()
            }
        } else {
            return Err(QueryError::ConfigError(
                "No LLM provider available for streaming".to_string(),
            ));
        };

        tracing::debug!("Using full config for streaming response (embedding + vector storage + optional LLM override)");

        Ok((final_context, mode, stream))
    }

    /// Execute a query with an LLM provider override.
    ///
    /// This method is used when the user selects a different LLM provider/model
    /// in the query interface. The override provider is used ONLY for generating
    /// the answer, not for keyword extraction.
    ///
    /// @implements SPEC-032: Provider selection at query time
    ///
    /// # Arguments
    ///
    /// * `request` - The query request (may contain llm_provider/llm_model hints)
    /// * `llm_provider` - The LLM provider to use for answer generation
    ///
    /// # Returns
    ///
    /// Query response with answer generated using the override provider.
    pub async fn query_with_llm_provider(
        &self,
        request: crate::engine::QueryRequest,
        llm_provider: std::sync::Arc<dyn crate::LLMProvider>,
    ) -> Result<crate::engine::QueryResponse> {
        let start = std::time::Instant::now();
        let mut stats = crate::engine::QueryStats::default();

        // Step 1: Extract keywords (with caching)
        let raw_keywords = if self.config.use_keyword_extraction {
            let kw_start = std::time::Instant::now();
            let kw = self
                .keyword_extractor
                .extract_extended(&request.query)
                .await?;
            tracing::debug!(
                query = %request.query,
                high_level = ?kw.high_level,
                low_level = ?kw.low_level,
                intent = %kw.query_intent,
                "Extracted keywords (LLM override)"
            );
            stats.embedding_time_ms += kw_start.elapsed().as_millis() as u64;
            kw
        } else {
            ExtractedKeywords::new(vec![], vec![], QueryIntent::Exploratory)
        };

        // Step 1.5: Validate keywords against knowledge graph
        let keywords = self.validate_keywords(&raw_keywords).await;

        // Step 2: Determine query mode
        let mode = if let Some(m) = request.mode {
            m
        } else if self.config.use_adaptive_mode {
            keywords.query_intent.recommended_mode()
        } else {
            self.config.default_mode
        };

        tracing::debug!(mode = %mode, "Selected query mode (LLM override)");

        // Step 3: Compute embeddings (uses default embedding provider)
        let embed_start = std::time::Instant::now();
        let embeddings =
            QueryEmbeddings::compute(&request.query, &keywords, self.embedding_provider.as_ref())
                .await?;
        stats.embedding_time_ms += embed_start.elapsed().as_millis() as u64;

        // Step 4: Mode-specific retrieval
        let retrieval_start = std::time::Instant::now();
        let context = match mode {
            QueryMode::Local => {
                self.query_local(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                )
                .await?
            }
            QueryMode::Global => {
                self.query_global(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                )
                .await?
            }
            QueryMode::Hybrid => {
                self.query_hybrid(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                )
                .await?
            }
            QueryMode::Mix => {
                self.query_mix(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                )
                .await?
            }
            QueryMode::Naive => {
                self.query_naive(&embeddings, request.tenant_id(), request.workspace_id())
                    .await?
            }
        };
        stats.retrieval_time_ms = retrieval_start.elapsed().as_millis() as u64;
        stats.context_tokens = context.token_count;

        // Step 4.5: Rerank chunks for improved precision
        let mut context = context;
        let should_rerank = request.enable_rerank.unwrap_or(self.config.enable_rerank);
        if should_rerank && self.reranker.is_some() {
            let rerank_start = std::time::Instant::now();
            let reranked_chunks = self
                .rerank_chunks(
                    &request.query,
                    context.chunks,
                    request.enable_rerank,
                    request.rerank_top_k,
                )
                .await;
            context.chunks = reranked_chunks;
            let rerank_time = rerank_start.elapsed().as_millis() as u64;
            tracing::debug!(
                rerank_time_ms = rerank_time,
                "Reranking completed (LLM override)"
            );
            stats.retrieval_time_ms += rerank_time;
        }

        // Step 4.6: Sort entities by degree for importance-based ranking
        self.sort_entities_by_degree(&mut context.entities);

        // Step 5: Apply truncation
        let (truncated_entities, truncated_relationships, truncated_chunks) = balance_context(
            context.entities.clone(),
            context.relationships.clone(),
            context.chunks.clone(),
            &self.config.truncation,
            self.tokenizer.as_ref(),
        );

        let mut final_context = context;
        final_context.entities = truncated_entities;
        final_context.relationships = truncated_relationships;
        final_context.chunks = truncated_chunks;

        // Step 6: Generate answer using OVERRIDE LLM provider
        let (answer, generated_tokens) = if request.context_only {
            (String::new(), 0)
        } else if request.prompt_only {
            (self.build_prompt(&request.query, &final_context), 0)
        } else {
            let gen_start = std::time::Instant::now();
            // SPEC-032: Use the override LLM provider
            let result = self
                .generate_answer_with_provider(&request.query, &final_context, Some(&llm_provider))
                .await?;
            stats.generation_time_ms = gen_start.elapsed().as_millis() as u64;
            result
        };

        stats.generated_tokens = generated_tokens;
        stats.total_time_ms = start.elapsed().as_millis() as u64;

        Ok(crate::engine::QueryResponse {
            answer,
            context: final_context,
            mode,
            stats,
        })
    }

    /// Execute a streaming query with full SOTA pipeline.
    ///
    /// This method applies all SOTA enhancements (keyword extraction, adaptive mode,
    /// mode-specific retrieval) and then streams the LLM response.
    pub async fn query_stream(
        &self,
        request: crate::engine::QueryRequest,
    ) -> Result<futures::stream::BoxStream<'static, Result<String>>> {
        use futures::StreamExt;

        // Step 1: Extract keywords (with caching)
        // WHY: These methods (query, query_stream) don't have an LLM override parameter.
        // They always use the engine's default LLM provider (self.llm_provider).
        // Pass None to extract_with_llm_override to use the default LLM.
        // For workspace-specific LLM selection, use query_with_full_config or query_stream_with_full_config.
        let raw_keywords = if self.config.use_keyword_extraction {
            self.keyword_extractor
                .extract_with_llm_override(&request.query, None)
                .await?
        } else {
            ExtractedKeywords::new(vec![], vec![], QueryIntent::Exploratory)
        };

        // Step 1.5: Validate keywords against knowledge graph
        // WHY: Drop keywords with no graph matches to prevent embedding dilution
        let keywords = self.validate_keywords(&raw_keywords).await;

        // Step 2: Determine query mode
        let mode = if let Some(m) = request.mode {
            m
        } else if self.config.use_adaptive_mode {
            keywords.query_intent.recommended_mode()
        } else {
            self.config.default_mode
        };

        tracing::debug!(mode = %mode, streaming = true, "Selected query mode for streaming");

        // Step 3: Compute embeddings
        let embeddings =
            QueryEmbeddings::compute(&request.query, &keywords, self.embedding_provider.as_ref())
                .await?;

        // Step 4: Mode-specific retrieval
        let context = match mode {
            QueryMode::Local => {
                self.query_local(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                )
                .await?
            }
            QueryMode::Global => {
                self.query_global(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                )
                .await?
            }
            QueryMode::Hybrid => {
                self.query_hybrid(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                )
                .await?
            }
            QueryMode::Mix => {
                self.query_mix(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                )
                .await?
            }
            QueryMode::Naive => {
                self.query_naive(&embeddings, request.tenant_id(), request.workspace_id())
                    .await?
            }
        };

        // Step 4.5: Rerank chunks for improved precision (streaming version)
        let mut context = context;
        let should_rerank = request.enable_rerank.unwrap_or(self.config.enable_rerank);
        if should_rerank && self.reranker.is_some() {
            let reranked_chunks = self
                .rerank_chunks(
                    &request.query,
                    context.chunks,
                    request.enable_rerank,
                    request.rerank_top_k,
                )
                .await;
            context.chunks = reranked_chunks;
            tracing::debug!(streaming = true, "Reranking completed for streaming query");
        }

        // Step 4.6: Sort entities by degree for importance-based ranking
        self.sort_entities_by_degree(&mut context.entities);

        // Step 5: Apply truncation
        let (truncated_entities, truncated_relationships, truncated_chunks) = balance_context(
            context.entities.clone(),
            context.relationships.clone(),
            context.chunks.clone(),
            &self.config.truncation,
            self.tokenizer.as_ref(),
        );

        let mut final_context = context;
        final_context.entities = truncated_entities;
        final_context.relationships = truncated_relationships;
        final_context.chunks = truncated_chunks;

        // Step 6: Handle empty context
        if final_context.is_empty() {
            return Ok(futures::stream::once(async {
                Ok("I'm sorry, but I couldn't find any relevant information in my knowledge base to answer your question.".to_string())
            }).boxed());
        }

        // Step 7: Build prompt and stream response
        let prompt = self.build_prompt(&request.query, &final_context);

        // Check if provider supports streaming
        if self.llm_provider.supports_streaming() {
            self.llm_provider
                .stream(&prompt)
                .await
                .map(|stream| stream.map(|res| res.map_err(QueryError::from)).boxed())
                .map_err(QueryError::from)
        } else {
            // Fallback: Use non-streaming and convert to single-chunk stream
            tracing::warn!(
                provider = self.llm_provider.name(),
                "Provider doesn't support streaming, falling back to non-streaming mode"
            );

            let response = self
                .llm_provider
                .complete(&prompt)
                .await
                .map_err(QueryError::from)?;
            Ok(futures::stream::once(async move { Ok(response.content) }).boxed())
        }
    }

    /// Execute a streaming query and return both context and stream.
    ///
    /// This is the preferred method for UI scenarios where sources need to be
    /// displayed alongside the streaming response.
    ///
    /// Returns:
    /// - QueryContext: The retrieved entities, relationships, and chunks
    /// - QueryMode: The mode used for retrieval
    /// - BoxStream: The LLM response stream
    ///
    /// # Streaming Fallback
    ///
    /// If the default LLM provider doesn't support streaming, this method will
    /// fall back to non-streaming mode and convert the full response into a
    /// single-chunk stream. This ensures compatibility with all providers.
    ///
    /// Returns:
    /// - QueryContext: The retrieved entities, relationships, and chunks
    /// - QueryMode: The mode used for retrieval
    /// - BoxStream: The LLM response stream
    pub async fn query_stream_with_context(
        &self,
        request: crate::engine::QueryRequest,
    ) -> Result<(
        QueryContext,
        QueryMode,
        futures::stream::BoxStream<'static, Result<String>>,
    )> {
        use futures::StreamExt;

        // Step 1: Get context (this handles keywords, mode selection, retrieval, truncation)
        let (context, mode) = self.get_context(&request).await?;

        // Step 2: Handle empty context
        if context.is_empty() {
            return Ok((
                context,
                mode,
                futures::stream::once(async {
                    Ok("I'm sorry, but I couldn't find any relevant information in my knowledge base to answer your question.".to_string())
                })
                .boxed(),
            ));
        }

        // Step 3: Build prompt and get stream
        let prompt = self.build_prompt(&request.query, &context);

        // Check if provider supports streaming
        let stream = if self.llm_provider.supports_streaming() {
            // Use streaming mode
            self.llm_provider
                .stream(&prompt)
                .await
                .map(|stream| stream.map(|res| res.map_err(QueryError::from)).boxed())
                .map_err(QueryError::from)?
        } else {
            // Fallback: Use non-streaming and convert to single-chunk stream
            tracing::warn!(
                provider = self.llm_provider.name(),
                "Provider doesn't support streaming, falling back to non-streaming mode"
            );

            let response = self
                .llm_provider
                .complete(&prompt)
                .await
                .map_err(QueryError::from)?;
            futures::stream::once(async move { Ok(response.content) }).boxed()
        };

        Ok((context, mode, stream))
    }

    /// Execute a streaming query with an LLM provider override.
    ///
    /// This method is used when the user selects a different LLM provider/model
    /// in the query interface. The override provider is used for streaming the answer.
    ///
    /// @implements SPEC-032: Provider selection at query time (streaming)
    ///
    /// # Arguments
    ///
    /// * `request` - The query request
    /// * `llm_provider` - The LLM provider to use for streaming the answer
    ///
    /// # Returns
    ///
    /// - QueryContext: The retrieved entities, relationships, and chunks
    /// - QueryMode: The mode used for retrieval
    /// - BoxStream: The LLM response stream using the override provider
    ///
    /// # Streaming Fallback
    ///
    /// If the provider doesn't support streaming (`supports_streaming() == false`),
    /// this method will fall back to non-streaming mode and convert the full response
    /// into a single-chunk stream. This ensures compatibility with all providers.
    pub async fn query_stream_with_context_and_llm(
        &self,
        request: crate::engine::QueryRequest,
        llm_provider: std::sync::Arc<dyn crate::LLMProvider>,
    ) -> Result<(
        QueryContext,
        QueryMode,
        futures::stream::BoxStream<'static, Result<String>>,
    )> {
        use futures::StreamExt;

        // Step 1: Get context (this handles keywords, mode selection, retrieval, truncation)
        let (context, mode) = self.get_context(&request).await?;

        // Step 2: Handle empty context
        if context.is_empty() {
            return Ok((
                context,
                mode,
                futures::stream::once(async {
                    Ok("I'm sorry, but I couldn't find any relevant information in my knowledge base to answer your question.".to_string())
                })
                .boxed(),
            ));
        }

        // Step 3: Build prompt and get stream using OVERRIDE LLM provider
        let prompt = self.build_prompt(&request.query, &context);

        // SPEC-032: Check if provider supports streaming
        // If not, fall back to non-streaming mode
        let stream = if llm_provider.supports_streaming() {
            // Use streaming mode
            tracing::debug!("Using streaming mode for LLM provider override");
            llm_provider
                .stream(&prompt)
                .await
                .map(|stream| stream.map(|res| res.map_err(QueryError::from)).boxed())
                .map_err(QueryError::from)?
        } else {
            // Fallback: Use non-streaming and convert to single-chunk stream
            tracing::warn!(
                provider = llm_provider.name(),
                "Provider doesn't support streaming, falling back to non-streaming mode"
            );

            // Clone prompt for the async block
            let prompt_clone = prompt.clone();
            let llm_clone = llm_provider.clone();

            // Use non-streaming completion and wrap in a stream
            let response = llm_clone
                .complete(&prompt_clone)
                .await
                .map_err(QueryError::from)?;

            // Return as a single-chunk stream
            futures::stream::once(async move { Ok(response.content) }).boxed()
        };

        tracing::debug!("Using LLM provider override for response");

        Ok((context, mode, stream))
    }

    /// Get the retrieved context without generating an answer.
    ///
    /// Useful for streaming scenarios where context is sent first.
    pub async fn get_context(
        &self,
        request: &crate::engine::QueryRequest,
    ) -> Result<(QueryContext, QueryMode)> {
        // Step 1: Extract keywords (with caching)
        // WHY: These methods (query, query_stream) don't have an LLM override parameter.
        // They always use the engine's default LLM provider (self.llm_provider).
        // Pass None to extract_with_llm_override to use the default LLM.
        // For workspace-specific LLM selection, use query_with_full_config or query_stream_with_full_config.
        let raw_keywords = if self.config.use_keyword_extraction {
            self.keyword_extractor
                .extract_with_llm_override(&request.query, None)
                .await?
        } else {
            ExtractedKeywords::new(vec![], vec![], QueryIntent::Exploratory)
        };

        // Step 1.5: Validate keywords against knowledge graph
        // WHY: Drop keywords with no graph matches to prevent embedding dilution
        let keywords = self.validate_keywords(&raw_keywords).await;

        // Step 2: Determine query mode
        let mode = if let Some(m) = request.mode {
            m
        } else if self.config.use_adaptive_mode {
            keywords.query_intent.recommended_mode()
        } else {
            self.config.default_mode
        };

        // Step 3: Compute embeddings
        let embeddings =
            QueryEmbeddings::compute(&request.query, &keywords, self.embedding_provider.as_ref())
                .await?;

        // Step 4: Mode-specific retrieval
        let context = match mode {
            QueryMode::Local => {
                self.query_local(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                )
                .await?
            }
            QueryMode::Global => {
                self.query_global(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                )
                .await?
            }
            QueryMode::Hybrid => {
                self.query_hybrid(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                )
                .await?
            }
            QueryMode::Mix => {
                self.query_mix(
                    &keywords,
                    &embeddings,
                    request.tenant_id(),
                    request.workspace_id(),
                )
                .await?
            }
            QueryMode::Naive => {
                self.query_naive(&embeddings, request.tenant_id(), request.workspace_id())
                    .await?
            }
        };

        // Step 4.5: Rerank chunks for improved precision
        let mut context = context;
        let should_rerank = request.enable_rerank.unwrap_or(self.config.enable_rerank);
        if should_rerank && self.reranker.is_some() {
            let reranked_chunks = self
                .rerank_chunks(
                    &request.query,
                    context.chunks,
                    request.enable_rerank,
                    request.rerank_top_k,
                )
                .await;
            context.chunks = reranked_chunks;
        }

        // Step 4.6: Sort entities by degree for importance-based ranking
        self.sort_entities_by_degree(&mut context.entities);

        // Step 5: Apply truncation
        let (truncated_entities, truncated_relationships, truncated_chunks) = balance_context(
            context.entities.clone(),
            context.relationships.clone(),
            context.chunks.clone(),
            &self.config.truncation,
            self.tokenizer.as_ref(),
        );

        let mut final_context = context;
        final_context.entities = truncated_entities;
        final_context.relationships = truncated_relationships;
        final_context.chunks = truncated_chunks;

        Ok((final_context, mode))
    }

    /// Local mode: Entity-centric search with low-level keywords.
    ///
    /// # WHY: Local Mode Strategy
    ///
    /// Local mode answers specific factual questions (e.g., "Who is the CEO of Apple?"):
    ///
    /// 1. **Low-level embedding** - Uses entity-focused keywords ("Apple", "CEO")
    ///    WHY: These keywords match entity descriptions, not relationships
    ///
    /// 2. **Entity vector filter** - Only search entity vectors, ignore relationships
    ///    WHY: Reduces noise; relationships are for Global mode
    ///
    /// 3. **1-hop graph expansion** - Fetch connected entities/relationships
    ///    WHY: Immediate neighbors provide supporting context
    ///
    /// 4. **Degree-based ranking** - Higher-degree entities ranked first
    ///    WHY: Well-connected entities are typically more important
    ///
    /// @implements FEAT0101 (Local Search Mode - entity-focused retrieval)
    async fn query_local(
        &self,
        _keywords: &ExtractedKeywords,
        embeddings: &QueryEmbeddings,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
    ) -> Result<QueryContext> {
        let mut context = QueryContext::new();

        // Step 1: Vector search with LOW-level keyword embedding
        // This finds entities relevant to specific terms
        let vector_results = self
            .vector_storage
            .query(&embeddings.low_level, self.config.max_entities * 3, None)
            .await?;

        // Step 2: Filter to entity vectors only (LightRAG Local mode)
        let entity_vectors = filter_by_type(vector_results, VectorType::Entity);

        // Step 2.5: Build entity scores map to preserve vector similarity scores
        let entity_scores: HashMap<String, f32> = entity_vectors
            .iter()
            .filter(|r| r.score >= self.config.min_score)
            .filter(|r| self.matches_tenant_filter(&r.metadata, &tenant_id, &workspace_id))
            .map(|r| {
                let entity_name = r
                    .metadata
                    .get("entity_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| r.id.clone());
                (entity_name, r.score)
            })
            .collect();

        // Step 3: Extract entity IDs from vector results
        let entity_ids: Vec<String> = entity_vectors
            .iter()
            .filter(|r| r.score >= self.config.min_score)
            .filter(|r| self.matches_tenant_filter(&r.metadata, &tenant_id, &workspace_id))
            .filter_map(|r| {
                // Try to get entity_name from metadata, fallback to id
                r.metadata
                    .get("entity_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| Some(r.id.clone()))
            })
            .take(self.config.max_entities)
            .collect();

        if entity_ids.is_empty() {
            // Fallback to popular entities
            return self.fallback_to_popular(tenant_id, workspace_id).await;
        }

        // Step 4: Batch fetch nodes and degrees (LightRAG optimization)
        let (nodes_map, degrees) = tokio::join!(
            self.graph_storage.get_nodes_batch(&entity_ids),
            self.graph_storage.node_degrees_batch(&entity_ids),
        );

        let nodes_map = nodes_map?;
        let degrees: HashMap<String, usize> = degrees?.into_iter().collect();

        // Step 5: Build entity context with source tracking
        //
        // WHY: Iterate in vector search score order (entity_ids) instead of HashMap iteration.
        // HashMap iteration is non-deterministic, causing same query → different results.
        // By preserving entity_ids order (Vec), we maintain deterministic entity ordering.
        //
        //   Before (Random):              After (Deterministic):
        //   Vector Search                 Vector Search
        //      ↓                             ↓
        //   entity_ids=[A,B,C]           entity_ids=[A,B,C]
        //   (score order)                (score order)
        //      ↓                             ↓
        //   nodes_map={A,C,B}            nodes_map={A,C,B}
        //   (HashMap - random)           (HashMap - lookup only)
        //      ↓                             ↓
        //   for (id,node) in map         for id in entity_ids
        //      ↓                             ↓
        //   [C,A,B] ← RANDOM!            [A,B,C] ← STABLE!
        //
        for id in &entity_ids {
            if let Some(node) = nodes_map.get(id) {
                let degree = degrees.get(id).copied().unwrap_or(0);
                // Use preserved similarity score from vector search (fixes score=0.0 bug)
                let entity_score = entity_scores.get(id).copied().unwrap_or(0.0);
                let entity = build_entity_from_node(id, &node.properties, degree, entity_score);
                context.add_entity(entity);
            }
        }

        // Step 6: Batch fetch edges for these entities
        let edges = self
            .graph_storage
            .get_edges_for_nodes_batch(&entity_ids)
            .await?;

        for edge in edges.iter().take(self.config.max_relationships) {
            if !self.matches_tenant_filter_props(&edge.properties, &tenant_id, &workspace_id) {
                continue;
            }

            let rel = build_relationship_from_edge(&edge.source, &edge.target, &edge.properties);
            context.add_relationship(rel);
        }

        // Step 7: Retrieve chunks from source_chunk_ids
        let mut chunk_ids = std::collections::HashSet::new();

        // Collect chunk IDs from entities
        for entity in &context.entities {
            for chunk_id in &entity.source_chunk_ids {
                chunk_ids.insert(chunk_id.clone());
            }
        }

        // Collect chunk IDs from relationships
        for rel in &context.relationships {
            if let Some(chunk_id) = &rel.source_chunk_id {
                chunk_ids.insert(chunk_id.clone());
            }
        }

        tracing::info!(
            total_chunk_ids = chunk_ids.len(),
            entity_count = context.entities.len(),
            "Local mode chunk collection"
        );

        // Retrieve chunks from vector storage if any chunk IDs were collected
        if !chunk_ids.is_empty() {
            // WHY: Pass ALL candidate chunk IDs to vector storage and let cosine similarity
            // determine the best max_chunks. The old approach sorted alphabetically and
            // truncated before scoring, which could discard high-relevance chunks.
            // VectorStorage.query() returns results sorted by score descending (contract).
            let chunk_ids_vec: Vec<String> = chunk_ids.into_iter().collect();

            let results = self
                .vector_storage
                .query(
                    &embeddings.low_level,
                    self.config.max_chunks,
                    Some(&chunk_ids_vec),
                )
                .await?;

            for result in results {
                if !self.matches_tenant_filter(&result.metadata, &tenant_id, &workspace_id) {
                    continue;
                }

                context.add_chunk(build_chunk_from_result(&result));
            }
        }

        Ok(context)
    }

    /// Global mode: Relationship-centric search with high-level keywords.
    ///
    /// # WHY: Global Mode Strategy
    ///
    /// Global mode answers thematic/analytical questions (e.g., "How do tech companies compete?"):
    ///
    /// 1. **High-level embedding** - Uses relationship-focused keywords ("compete", "partnership")
    ///    WHY: These keywords match relationship descriptions, not entities
    ///
    /// 2. **Relationship vector filter** - Only search relationship vectors
    ///    WHY: Relationships capture "how" and "why" connections between entities
    ///
    /// 3. **Entity hydration** - Fetch source/target entities for each relationship
    ///    WHY: Relationships are meaningless without their endpoint context
    ///
    /// 4. **Community summaries** - Include pre-computed graph cluster summaries
    ///    WHY: Provides high-level thematic context for broad questions
    ///
    /// @implements FEAT0102 (Global Search Mode - relationship-focused retrieval)
    async fn query_global(
        &self,
        _keywords: &ExtractedKeywords,
        embeddings: &QueryEmbeddings,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
    ) -> Result<QueryContext> {
        let mut context = QueryContext::new();
        let mut entity_ids: Vec<String> = Vec::new();
        let mut seen_relationships = std::collections::HashSet::new();

        // Step 1: Vector search with HIGH-level keyword embedding
        // This finds relationships relevant to broader concepts
        let vector_results = self
            .vector_storage
            .query(
                &embeddings.high_level,
                self.config.max_relationships * 3,
                None,
            )
            .await?;

        // Step 2: Filter to relationship vectors only (LightRAG Global mode)
        let relationship_vectors = filter_by_type(vector_results.clone(), VectorType::Relationship);

        // Step 3: Extract relationships from vector results
        for result in relationship_vectors
            .iter()
            .filter(|r| r.score >= self.config.min_score)
            .filter(|r| self.matches_tenant_filter(&r.metadata, &tenant_id, &workspace_id))
            .take(self.config.max_relationships)
        {
            let src_id = result
                .metadata
                .get("src_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let tgt_id = result
                .metadata
                .get("tgt_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let rel_type = result
                .metadata
                .get("relation_type")
                .and_then(|v| v.as_str())
                .unwrap_or("RELATED_TO");
            let description = result
                .metadata
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if !src_id.is_empty() && !tgt_id.is_empty() {
                let rel_key = format!("{}->{}:{}", src_id, tgt_id, rel_type);
                if seen_relationships.insert(rel_key) {
                    // Extract source tracking from vector metadata
                    let source_chunk_id = result
                        .metadata
                        .get("source_chunk_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let source_document_id = result
                        .metadata
                        .get("source_document_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let source_file_path = result
                        .metadata
                        .get("source_file_path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let mut rel = RetrievedRelationship::new(src_id, tgt_id, rel_type.to_string())
                        .with_description(description.to_string())
                        .with_score(result.score);
                    if let Some(chunk_id) = source_chunk_id {
                        rel = rel.with_source_chunk_id(chunk_id);
                    }
                    if let Some(doc_id) = source_document_id {
                        rel = rel.with_source_document_id(doc_id);
                    }
                    if let Some(file_path) = source_file_path {
                        rel = rel.with_source_file_path(file_path);
                    }
                    context.add_relationship(rel);
                    // Collect entity IDs from relationships
                    if !entity_ids.contains(&src_id.to_string()) {
                        entity_ids.push(src_id.to_string());
                    }
                    if !entity_ids.contains(&tgt_id.to_string()) {
                        entity_ids.push(tgt_id.to_string());
                    }
                }
            }
        }

        // Step 4: Fallback to popular entities if no relationship vectors found
        if entity_ids.is_empty() {
            let popular = self
                .graph_storage
                .get_popular_nodes_with_degree(
                    self.config.max_entities,
                    Some(2), // Min degree
                    None,
                    tenant_id.as_deref(),
                    workspace_id.as_deref(),
                )
                .await?;

            entity_ids = popular.iter().map(|(n, _)| n.id.clone()).collect();

            for (node, degree) in popular {
                let entity = build_entity_from_node(&node.id, &node.properties, degree, 0.0);
                context.add_entity(entity);
            }

            // Get edges between popular entities
            if !entity_ids.is_empty() {
                let edges = self
                    .graph_storage
                    .get_edges_for_nodes_batch(&entity_ids)
                    .await?;
                for edge in edges.iter().take(self.config.max_relationships) {
                    let rel =
                        build_relationship_from_edge(&edge.source, &edge.target, &edge.properties);
                    context.add_relationship(rel);
                }
            }
        } else {
            // Step 5: Batch fetch entities from relationship endpoints
            let (nodes_map, degrees) = tokio::join!(
                self.graph_storage.get_nodes_batch(&entity_ids),
                self.graph_storage.node_degrees_batch(&entity_ids),
            );

            let nodes_map = nodes_map?;
            let degrees: HashMap<String, usize> = degrees?.into_iter().collect();

            // WHY: Iterate in relationship discovery order (entity_ids) instead of HashMap.
            // Global mode discovers entities from relationship endpoints in vector search score order.
            // Preserving that order ensures deterministic retrieval (same query → same results).
            for id in &entity_ids {
                if let Some(node) = nodes_map.get(id) {
                    let degree = degrees.get(id).copied().unwrap_or(0);
                    let entity = build_entity_from_node(id, &node.properties, degree, 0.0);
                    context.add_entity(entity);
                }
            }
        }

        // Step 6: Add chunks from vector search (filter to chunks)
        let chunk_vectors = filter_by_type(vector_results, VectorType::Chunk);
        for result in chunk_vectors
            .iter()
            .filter(|r| r.score >= self.config.min_score)
            .filter(|r| self.matches_tenant_filter(&r.metadata, &tenant_id, &workspace_id))
            .take(self.config.max_chunks)
        {
            context.add_chunk(build_chunk_from_result(result));
        }

        // Step 7: Also retrieve chunks from source_chunk_ids tracked in entities/relationships
        let mut source_chunk_ids = std::collections::HashSet::new();

        // Collect chunk IDs from entities
        for entity in &context.entities {
            for chunk_id in &entity.source_chunk_ids {
                source_chunk_ids.insert(chunk_id.clone());
            }
        }

        // Collect chunk IDs from relationships
        for rel in &context.relationships {
            if let Some(chunk_id) = &rel.source_chunk_id {
                source_chunk_ids.insert(chunk_id.clone());
            }
        }

        // Retrieve source chunks if any were collected and we haven't hit max chunks
        if !source_chunk_ids.is_empty() && context.chunks.len() < self.config.max_chunks {
            let remaining_slots = self.config.max_chunks - context.chunks.len();

            // WHY: Pass ALL candidate chunk IDs to vector storage and let cosine similarity
            // rank them. VectorStorage.query() returns results sorted by score descending.
            let chunk_ids_vec: Vec<String> = source_chunk_ids.into_iter().collect();

            let results = self
                .vector_storage
                .query(
                    &embeddings.high_level,
                    remaining_slots,
                    Some(&chunk_ids_vec),
                )
                .await?;

            // Track which chunks we already have to avoid duplicates
            let existing_chunk_ids: std::collections::HashSet<_> =
                context.chunks.iter().map(|c| c.id.clone()).collect();

            for result in results {
                if existing_chunk_ids.contains(&result.id) {
                    continue;
                }
                if !self.matches_tenant_filter(&result.metadata, &tenant_id, &workspace_id) {
                    continue;
                }

                context.add_chunk(build_chunk_from_result(&result));
            }
        }

        Ok(context)
    }

    /// Hybrid mode: Combine local and global with round-robin merging.
    ///
    /// @implements FEAT0103 (Hybrid Search Mode - combined local+global)
    async fn query_hybrid(
        &self,
        keywords: &ExtractedKeywords,
        embeddings: &QueryEmbeddings,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
    ) -> Result<QueryContext> {
        // Run local and global in parallel
        let (local_result, global_result) = tokio::join!(
            self.query_local(
                keywords,
                embeddings,
                tenant_id.clone(),
                workspace_id.clone()
            ),
            self.query_global(
                keywords,
                embeddings,
                tenant_id.clone(),
                workspace_id.clone()
            ),
        );

        let local = local_result?;
        let global = global_result?;

        // Round-robin merge with deduplication
        let mut context = QueryContext::new();
        let mut seen_entities = std::collections::HashSet::new();
        let mut seen_relationships = std::collections::HashSet::new();

        // Interleave entities
        let max_len = local.entities.len().max(global.entities.len());
        for i in 0..max_len {
            if let Some(e) = local.entities.get(i) {
                if seen_entities.insert(e.name.clone()) {
                    context.add_entity(e.clone());
                }
            }
            if let Some(e) = global.entities.get(i) {
                if seen_entities.insert(e.name.clone()) {
                    context.add_entity(e.clone());
                }
            }
        }

        // Interleave relationships
        let max_len = local.relationships.len().max(global.relationships.len());
        for i in 0..max_len {
            if let Some(r) = local.relationships.get(i) {
                let key = format!("{}-{}-{}", r.source, r.relation_type, r.target);
                if seen_relationships.insert(key) {
                    context.add_relationship(r.clone());
                }
            }
            if let Some(r) = global.relationships.get(i) {
                let key = format!("{}-{}-{}", r.source, r.relation_type, r.target);
                if seen_relationships.insert(key) {
                    context.add_relationship(r.clone());
                }
            }
        }

        // WHY: Round-robin interleave chunks for balanced source diversity.
        // The old approach chained local-then-global, giving local chunks priority.
        // Round-robin ensures the top chunk from each source is represented first,
        // matching the entity/relationship interleaving pattern above.
        let mut seen_chunks = std::collections::HashSet::new();
        let max_chunk_len = local.chunks.len().max(global.chunks.len());
        for i in 0..max_chunk_len {
            if let Some(c) = local.chunks.get(i) {
                if seen_chunks.insert(c.id.clone()) {
                    context.add_chunk(c.clone());
                }
            }
            if let Some(c) = global.chunks.get(i) {
                if seen_chunks.insert(c.id.clone()) {
                    context.add_chunk(c.clone());
                }
            }
        }

        Ok(context)
    }

    /// Mix mode: Hybrid plus direct chunk search.
    ///
    /// @implements FEAT0105 (Mix Weighted Search - hybrid + direct chunks)
    async fn query_mix(
        &self,
        keywords: &ExtractedKeywords,
        embeddings: &QueryEmbeddings,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
    ) -> Result<QueryContext> {
        // Run hybrid and direct chunk search in parallel
        let (hybrid_result, chunk_results) = tokio::join!(
            self.query_hybrid(
                keywords,
                embeddings,
                tenant_id.clone(),
                workspace_id.clone()
            ),
            self.vector_storage
                .query(&embeddings.query, self.config.max_chunks * 2, None),
        );

        let mut context = hybrid_result?;
        let chunk_results = chunk_results?;

        // Filter to chunk vectors only
        let chunk_vectors = filter_by_type(chunk_results, VectorType::Chunk);

        // Add direct chunks (deduplicated)
        let existing_chunk_ids: std::collections::HashSet<_> =
            context.chunks.iter().map(|c| c.id.clone()).collect();

        for result in chunk_vectors
            .iter()
            .filter(|r| r.score >= self.config.min_score)
            .filter(|r| self.matches_tenant_filter(&r.metadata, &tenant_id, &workspace_id))
            .take(self.config.max_chunks)
        {
            if !existing_chunk_ids.contains(&result.id) {
                context.add_chunk(build_chunk_from_result(result));
            }
        }

        Ok(context)
    }

    /// Naive mode: Direct chunk vector search only.
    ///
    /// @implements FEAT0106 (Bypass Mode - direct vector search without graph)
    async fn query_naive(
        &self,
        embeddings: &QueryEmbeddings,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
    ) -> Result<QueryContext> {
        let mut context = QueryContext::new();

        let results = self
            .vector_storage
            .query(&embeddings.query, self.config.max_chunks * 2, None)
            .await?;

        // Filter to chunk vectors only
        let chunk_results = filter_by_type(results, VectorType::Chunk);

        for result in chunk_results
            .iter()
            .filter(|r| r.score >= self.config.min_score)
            .filter(|r| self.matches_tenant_filter(&r.metadata, &tenant_id, &workspace_id))
            .take(self.config.max_chunks)
        {
            context.add_chunk(build_chunk_from_result(result));
        }

        Ok(context)
    }

    /// Fallback to popular entities when no vector matches.
    async fn fallback_to_popular(
        &self,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
    ) -> Result<QueryContext> {
        let mut context = QueryContext::new();

        let popular = self
            .graph_storage
            .get_popular_nodes_with_degree(
                self.config.max_entities,
                None,
                None,
                tenant_id.as_deref(),
                workspace_id.as_deref(),
            )
            .await?;

        let entity_ids: Vec<String> = popular.iter().map(|(n, _)| n.id.clone()).collect();

        for (node, degree) in popular {
            let entity = build_entity_from_node(&node.id, &node.properties, degree, 0.0);
            context.add_entity(entity);
        }

        // Get edges
        if !entity_ids.is_empty() {
            let edges = self
                .graph_storage
                .get_edges_for_nodes_batch(&entity_ids)
                .await?;
            for edge in edges.iter().take(self.config.max_relationships) {
                let rel =
                    build_relationship_from_edge(&edge.source, &edge.target, &edge.properties);
                context.add_relationship(rel);
            }
        }

        Ok(context)
    }

    /// Check if metadata matches tenant filter.
    fn matches_tenant_filter(
        &self,
        metadata: &serde_json::Value,
        tenant_id: &Option<String>,
        workspace_id: &Option<String>,
    ) -> bool {
        if tenant_id.is_none() && workspace_id.is_none() {
            return true;
        }

        if let Some(tid) = tenant_id {
            if let Some(meta_tid) = metadata.get("tenant_id").and_then(|v| v.as_str()) {
                if meta_tid != tid {
                    return false;
                }
            }
        }

        if let Some(wid) = workspace_id {
            if let Some(meta_wid) = metadata.get("workspace_id").and_then(|v| v.as_str()) {
                if meta_wid != wid {
                    return false;
                }
            }
        }

        true
    }

    /// Check if properties match tenant filter.
    fn matches_tenant_filter_props(
        &self,
        properties: &HashMap<String, serde_json::Value>,
        tenant_id: &Option<String>,
        workspace_id: &Option<String>,
    ) -> bool {
        if tenant_id.is_none() && workspace_id.is_none() {
            return true;
        }

        if let Some(tid) = tenant_id {
            if let Some(prop_tid) = properties.get("tenant_id").and_then(|v| v.as_str()) {
                if prop_tid != tid {
                    return false;
                }
            }
        }

        if let Some(wid) = workspace_id {
            if let Some(prop_wid) = properties.get("workspace_id").and_then(|v| v.as_str()) {
                if prop_wid != wid {
                    return false;
                }
            }
        }

        true
    }

    /// Build prompt for LLM.
    ///
    /// WHY: The prompt is designed to maximize information extraction from available context.
    /// When comparing products where one term doesn't exist in the knowledge base, we still
    /// want to provide useful information about what IS available, rather than just saying
    /// "no information found."
    fn build_prompt(&self, query: &str, context: &QueryContext) -> String {
        if context.is_empty() {
            return "I'm sorry, but I couldn't find any relevant information in my knowledge base to answer your question.".to_string();
        }

        let context_text = context.to_context_string();

        format!(
            r#"---Role---

You are an expert AI assistant specializing in synthesizing information from a provided knowledge base. Your primary function is to answer user queries accurately by ONLY using the information within the provided **Context**.

---Goal---

Generate a comprehensive, well-structured answer to the user query.
The answer must integrate relevant facts from the Knowledge Graph and Document Chunks found in the **Context**.

---Instructions---

1. Step-by-Step Reasoning:
  - Carefully determine the user's query intent to fully understand the information need.
  - Scrutinize both Knowledge Graph Data (Entities and Relationships) and Document Chunks in the **Context**. Identify and extract all pieces of information that are directly relevant to answering the user query.
  - Weave the extracted facts into a coherent and logical response. Your own knowledge must ONLY be used to formulate fluent sentences and connect ideas, NOT to introduce any external information.

2. Content & Grounding:
  - Strictly adhere to the provided context; DO NOT invent, assume, or infer any information not explicitly stated.
  - If the answer cannot be fully determined from the **Context**, state what information IS available and note what is missing. A partial answer with specific data is better than a generic "insufficient information" response.

3. Formatting & Language:
  - The response MUST be in the same language as the user query.
  - Use Markdown formatting for clarity (headings, bold text, bullet points).

---Context---

{context_text}

---User Query---

{query}"#
        )
    }

    /// Generate answer using LLM.
    ///
    /// If `llm_override` is provided, uses that provider instead of the default.
    /// This enables per-request provider selection (SPEC-032).
    async fn generate_answer_with_provider(
        &self,
        query: &str,
        context: &QueryContext,
        llm_override: Option<&Arc<dyn crate::LLMProvider>>,
    ) -> Result<(String, usize)> {
        if context.is_empty() {
            return Ok((
                "I'm sorry, but I couldn't find any relevant information in my knowledge base to answer your question.".to_string(),
                0,
            ));
        }

        let prompt = self.build_prompt(query, context);

        // SPEC-032: Use override provider if provided, else default
        let response = if let Some(provider) = llm_override {
            provider.complete(&prompt).await?
        } else {
            self.llm_provider.complete(&prompt).await?
        };

        Ok((response.content, response.completion_tokens))
    }

    /// Generate answer using the default LLM.
    async fn generate_answer(
        &self,
        query: &str,
        context: &QueryContext,
    ) -> Result<(String, usize)> {
        self.generate_answer_with_provider(query, context, None)
            .await
    }

    /// Get the engine configuration.
    pub fn config(&self) -> &SOTAQueryConfig {
        &self.config
    }

    // =========================================================================
    // Workspace-specific vector storage methods (SPEC-033)
    // =========================================================================
    //
    // These methods are variants of the query mode methods that accept an
    // external vector storage instance instead of using self.vector_storage.
    // This enables per-workspace vector isolation with different dimensions.

    /// Naive mode with workspace-specific vector storage.
    async fn query_naive_with_vector_storage(
        &self,
        embeddings: &QueryEmbeddings,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
        vector_storage: &Arc<dyn VectorStorage>,
    ) -> Result<QueryContext> {
        let mut context = QueryContext::new();

        // WHY 2x oversampling: Vector storage returns all types (entities, relationships, chunks).
        // We retrieve 2x max_chunks to compensate for non-chunk results in top results.
        let results = vector_storage
            .query(&embeddings.query, self.config.max_chunks * 2, None)
            .await?;

        let chunk_results = filter_by_type(results, VectorType::Chunk);

        for result in chunk_results
            .iter()
            .filter(|r| r.score >= self.config.min_score)
            .filter(|r| self.matches_tenant_filter(&r.metadata, &tenant_id, &workspace_id))
            .take(self.config.max_chunks)
        {
            context.add_chunk(build_chunk_from_result(result));
        }

        Ok(context)
    }

    /// Local mode with workspace-specific vector storage.
    async fn query_local_with_vector_storage(
        &self,
        _keywords: &ExtractedKeywords,
        embeddings: &QueryEmbeddings,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
        vector_storage: &Arc<dyn VectorStorage>,
    ) -> Result<QueryContext> {
        let mut context = QueryContext::new();

        // Step 1: Vector search with LOW-level keyword embedding
        let vector_results = vector_storage
            .query(&embeddings.low_level, self.config.max_entities * 3, None)
            .await?;

        // Step 2: Filter to entity vectors only
        let entity_vectors = filter_by_type(vector_results, VectorType::Entity);

        // Step 2.5: Build entity scores map
        let entity_scores: HashMap<String, f32> = entity_vectors
            .iter()
            .filter(|r| r.score >= self.config.min_score)
            .filter(|r| self.matches_tenant_filter(&r.metadata, &tenant_id, &workspace_id))
            .map(|r| {
                let entity_name = r
                    .metadata
                    .get("entity_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| r.id.clone());
                (entity_name, r.score)
            })
            .collect();

        // Step 3: Extract entity IDs
        let entity_ids: Vec<String> = entity_vectors
            .iter()
            .filter(|r| r.score >= self.config.min_score)
            .filter(|r| self.matches_tenant_filter(&r.metadata, &tenant_id, &workspace_id))
            .filter_map(|r| {
                r.metadata
                    .get("entity_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| Some(r.id.clone()))
            })
            .take(self.config.max_entities)
            .collect();

        // OODA-231: When no entity vectors exist (workspace-isolated storage often only has chunks),
        // fall back to popular entities from the graph, then continue to collect chunks.
        // WHY: Early return skipped chunk collection, causing 0 sources in response.
        if entity_ids.is_empty() {
            tracing::debug!(
                workspace_id = ?workspace_id,
                "OODA-231: No entity vectors found, falling back to popular entities from graph"
            );
            // Populate context with popular entities from graph
            let popular = self
                .graph_storage
                .get_popular_nodes_with_degree(
                    self.config.max_entities,
                    None,
                    None,
                    tenant_id.as_deref(),
                    workspace_id.as_deref(),
                )
                .await?;

            let fallback_entity_ids: Vec<String> =
                popular.iter().map(|(n, _)| n.id.clone()).collect();

            for (node, degree) in popular {
                let entity = build_entity_from_node(&node.id, &node.properties, degree, 0.0);
                context.add_entity(entity);
            }

            // Get edges for fallback entities
            if !fallback_entity_ids.is_empty() {
                let edges = self
                    .graph_storage
                    .get_edges_for_nodes_batch(&fallback_entity_ids)
                    .await?;
                for edge in edges.iter().take(self.config.max_relationships) {
                    let rel =
                        build_relationship_from_edge(&edge.source, &edge.target, &edge.properties);
                    context.add_relationship(rel);
                }
            }
            // NOTE: Don't return early - continue to chunk collection below
        } else {
            // Step 4: Batch fetch nodes and degrees
            let (nodes_map, degrees) = tokio::join!(
                self.graph_storage.get_nodes_batch(&entity_ids),
                self.graph_storage.node_degrees_batch(&entity_ids),
            );

            let nodes_map = nodes_map?;
            let degrees: HashMap<String, usize> = degrees?.into_iter().collect();

            // Step 5: Build entity context
            // WHY: Use entity_ids (Vec) for deterministic ordering, not HashMap iteration.
            for id in &entity_ids {
                if let Some(node) = nodes_map.get(id) {
                    let degree = degrees.get(id).copied().unwrap_or(0);
                    let entity_score = entity_scores.get(id).copied().unwrap_or(0.0);
                    let entity = build_entity_from_node(id, &node.properties, degree, entity_score);
                    context.add_entity(entity);
                }
            }

            // Step 6: Batch fetch edges
            let edges = self
                .graph_storage
                .get_edges_for_nodes_batch(&entity_ids)
                .await?;

            for edge in edges.iter().take(self.config.max_relationships) {
                let rel =
                    build_relationship_from_edge(&edge.source, &edge.target, &edge.properties);
                context.add_relationship(rel);
            }
        }

        // Step 7: Collect source_chunk_ids from entities and relationships
        // WHY-OODA230: Must retrieve chunks via their IDs, not by semantic similarity.
        // The old approach (semantic search + filter_by_type) returned 0 chunks because
        // entity/relationship vectors often score higher than chunks for concept queries.
        let mut chunk_ids = std::collections::HashSet::new();

        // Collect chunk IDs from entities
        for entity in &context.entities {
            for chunk_id in &entity.source_chunk_ids {
                chunk_ids.insert(chunk_id.clone());
            }
        }

        // Collect chunk IDs from relationships
        for rel in &context.relationships {
            if let Some(chunk_id) = &rel.source_chunk_id {
                chunk_ids.insert(chunk_id.clone());
            }
        }

        tracing::info!(
            total_chunk_ids = chunk_ids.len(),
            entity_count = context.entities.len(),
            relationship_count = context.relationships.len(),
            "OODA-230: Local mode chunk collection (workspace)"
        );

        // Retrieve chunks from workspace vector storage using chunk IDs
        if !chunk_ids.is_empty() {
            // WHY: Pass ALL candidate chunk IDs to vector storage and let cosine similarity
            // rank them. VectorStorage.query() returns results sorted by score descending.
            let chunk_ids_vec: Vec<String> = chunk_ids.into_iter().collect();

            tracing::debug!(
                chunk_ids_count = chunk_ids_vec.len(),
                max_chunks = self.config.max_chunks,
                "OODA-231: Requesting chunks by ID from vector storage (score-ranked)"
            );

            // Query with filter to retrieve only the specific chunks, score-ranked
            let results = vector_storage
                .query(
                    &embeddings.low_level,
                    self.config.max_chunks,
                    Some(&chunk_ids_vec),
                )
                .await?;

            tracing::debug!(
                candidates = chunk_ids_vec.len(),
                returned = results.len(),
                "OODA-231: Chunk retrieval result (top-k by cosine similarity)"
            );

            for result in results {
                if !self.matches_tenant_filter(&result.metadata, &tenant_id, &workspace_id) {
                    continue;
                }
                context.add_chunk(build_chunk_from_result(&result));
            }
        }

        Ok(context)
    }

    /// Global mode with workspace-specific vector storage.
    async fn query_global_with_vector_storage(
        &self,
        _keywords: &ExtractedKeywords,
        embeddings: &QueryEmbeddings,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
        vector_storage: &Arc<dyn VectorStorage>,
    ) -> Result<QueryContext> {
        let mut context = QueryContext::new();
        let mut entity_ids: Vec<String> = Vec::new();
        let mut seen_relationships = std::collections::HashSet::new();

        // Step 1: Vector search with HIGH-level keyword embedding
        let vector_results = vector_storage
            .query(
                &embeddings.high_level,
                self.config.max_relationships * 3,
                None,
            )
            .await?;

        // Step 2: Filter to relationship vectors only
        let relationship_vectors = filter_by_type(vector_results.clone(), VectorType::Relationship);

        // Step 3: Extract relationships from vector results
        for result in relationship_vectors
            .iter()
            .filter(|r| r.score >= self.config.min_score)
            .filter(|r| self.matches_tenant_filter(&r.metadata, &tenant_id, &workspace_id))
            .take(self.config.max_relationships)
        {
            let src_id = result
                .metadata
                .get("src_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let tgt_id = result
                .metadata
                .get("tgt_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let rel_type = result
                .metadata
                .get("relation_type")
                .and_then(|v| v.as_str())
                .unwrap_or("RELATED_TO");
            let description = result
                .metadata
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if !src_id.is_empty() && !tgt_id.is_empty() {
                let rel_key = format!("{}->{}:{}", src_id, tgt_id, rel_type);
                if seen_relationships.insert(rel_key) {
                    let source_chunk_id = result
                        .metadata
                        .get("source_chunk_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let source_document_id = result
                        .metadata
                        .get("source_document_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let source_file_path = result
                        .metadata
                        .get("source_file_path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let mut rel = RetrievedRelationship::new(src_id, tgt_id, rel_type.to_string())
                        .with_description(description.to_string())
                        .with_score(result.score);
                    if let Some(chunk_id) = source_chunk_id {
                        rel = rel.with_source_chunk_id(chunk_id);
                    }
                    if let Some(doc_id) = source_document_id {
                        rel = rel.with_source_document_id(doc_id);
                    }
                    if let Some(file_path) = source_file_path {
                        rel = rel.with_source_file_path(file_path);
                    }
                    context.add_relationship(rel);
                    if !entity_ids.contains(&src_id.to_string()) {
                        entity_ids.push(src_id.to_string());
                    }
                    if !entity_ids.contains(&tgt_id.to_string()) {
                        entity_ids.push(tgt_id.to_string());
                    }
                }
            }
        }

        // Step 4: OODA-231: When no relationship vectors exist, fall back to popular entities
        // WHY: Early return skipped chunk collection, causing 0 sources in response.
        if entity_ids.is_empty() {
            tracing::debug!(
                workspace_id = ?workspace_id,
                "OODA-231: No relationship vectors found, falling back to popular entities from graph"
            );
            // Populate context with popular entities from graph
            let popular = self
                .graph_storage
                .get_popular_nodes_with_degree(
                    self.config.max_entities,
                    None,
                    None,
                    tenant_id.as_deref(),
                    workspace_id.as_deref(),
                )
                .await?;

            for (node, degree) in &popular {
                let entity = build_entity_from_node(&node.id, &node.properties, *degree, 0.0);
                context.add_entity(entity);
                entity_ids.push(node.id.clone());
            }

            // Get edges for fallback entities
            if !entity_ids.is_empty() {
                let edges = self
                    .graph_storage
                    .get_edges_for_nodes_batch(&entity_ids)
                    .await?;
                for edge in edges.iter().take(self.config.max_relationships) {
                    let rel_key = format!("{}->{}:{}", edge.source, edge.target, "RELATED_TO");
                    if seen_relationships.insert(rel_key) {
                        let rel = build_relationship_from_edge(
                            &edge.source,
                            &edge.target,
                            &edge.properties,
                        );
                        context.add_relationship(rel);
                    }
                }
            }
            // NOTE: Don't return early - continue to chunk collection below
        } else {
            // Step 5: Batch fetch entity nodes
            let nodes_map = self.graph_storage.get_nodes_batch(&entity_ids).await?;

            // WHY: Iterate entity_ids (Vec) for deterministic ordering instead of HashMap.
            // HashMap iteration order is random, causing non-deterministic results.
            for id in &entity_ids {
                if let Some(node) = nodes_map.get(id) {
                    let degree = self.graph_storage.node_degree(id).await?;
                    let entity = build_entity_from_node(id, &node.properties, degree, 0.5);
                    context.add_entity(entity);
                }
            }
        }

        // Step 6: Collect source_chunk_ids from entities and relationships
        // WHY-OODA230: Must retrieve chunks via their IDs, not by semantic similarity.
        // The old approach (semantic search + filter_by_type) returned 0 chunks because
        // entity/relationship vectors often score higher than chunks for concept queries.
        let mut chunk_ids = std::collections::HashSet::new();

        // Collect chunk IDs from entities
        for entity in &context.entities {
            for chunk_id in &entity.source_chunk_ids {
                chunk_ids.insert(chunk_id.clone());
            }
        }

        // Collect chunk IDs from relationships
        for rel in &context.relationships {
            if let Some(chunk_id) = &rel.source_chunk_id {
                chunk_ids.insert(chunk_id.clone());
            }
        }

        tracing::info!(
            total_chunk_ids = chunk_ids.len(),
            entity_count = context.entities.len(),
            relationship_count = context.relationships.len(),
            "OODA-230: Global mode chunk collection (workspace)"
        );

        // Retrieve chunks from workspace vector storage using chunk IDs
        if !chunk_ids.is_empty() {
            // WHY: Pass ALL candidate chunk IDs to vector storage and let cosine similarity
            // rank them. VectorStorage.query() returns results sorted by score descending.
            let chunk_ids_vec: Vec<String> = chunk_ids.into_iter().collect();

            let results = vector_storage
                .query(
                    &embeddings.high_level,
                    self.config.max_chunks,
                    Some(&chunk_ids_vec),
                )
                .await?;

            for result in results {
                if !self.matches_tenant_filter(&result.metadata, &tenant_id, &workspace_id) {
                    continue;
                }
                context.add_chunk(build_chunk_from_result(&result));
            }
        }

        Ok(context)
    }

    /// Hybrid mode with workspace-specific vector storage.
    async fn query_hybrid_with_vector_storage(
        &self,
        keywords: &ExtractedKeywords,
        embeddings: &QueryEmbeddings,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
        vector_storage: &Arc<dyn VectorStorage>,
    ) -> Result<QueryContext> {
        // WHY: Run entity-based (local+global) AND naive chunk retrieval in parallel.
        // Entity-based retrieval provides graph context (entities, relationships),
        // but ONLY finds chunks linked to matching entities.
        // Naive retrieval finds chunks by direct semantic similarity to the query,
        // ensuring high recall even when entity extraction doesn't match.
        let (local_context, global_context, naive_context) = tokio::join!(
            self.query_local_with_vector_storage(
                keywords,
                embeddings,
                tenant_id.clone(),
                workspace_id.clone(),
                vector_storage,
            ),
            self.query_global_with_vector_storage(
                keywords,
                embeddings,
                tenant_id.clone(),
                workspace_id.clone(),
                vector_storage,
            ),
            self.query_naive_with_vector_storage(
                embeddings,
                tenant_id.clone(),
                workspace_id.clone(),
                vector_storage,
            ),
        );

        let local_context = local_context?;
        let global_context = global_context?;
        let naive_context = naive_context?;

        tracing::debug!(
            naive_chunks = naive_context.chunks.len(),
            local_chunks = local_context.chunks.len(),
            local_entities = local_context.entities.len(),
            global_chunks = global_context.chunks.len(),
            global_entities = global_context.entities.len(),
            "Hybrid merge: round-robin (local, global, naive)"
        );

        // WHY: Round-robin interleave chunks from local, global, and naive sources.
        // KG-derived chunks (local, global) go first at each position since they carry
        // entity/relationship context. The old approach gave naive all slots first,
        // which could starve KG-derived chunks even when they were more relevant.
        let mut merged = QueryContext::new();
        let mut seen_chunks = std::collections::HashSet::new();
        let max_chunk_len = local_context
            .chunks
            .len()
            .max(global_context.chunks.len())
            .max(naive_context.chunks.len());

        for i in 0..max_chunk_len {
            // KG-derived first (higher signal), then naive (broader recall)
            if let Some(c) = local_context.chunks.get(i) {
                if seen_chunks.insert(c.id.clone()) {
                    merged.add_chunk(c.clone());
                }
            }
            if let Some(c) = global_context.chunks.get(i) {
                if seen_chunks.insert(c.id.clone()) {
                    merged.add_chunk(c.clone());
                }
            }
            if let Some(c) = naive_context.chunks.get(i) {
                if seen_chunks.insert(c.id.clone()) {
                    merged.add_chunk(c.clone());
                }
            }
        }

        // Round-robin entities from local+global
        let mut seen_entities = std::collections::HashSet::new();
        let max_entity_len = local_context
            .entities
            .len()
            .max(global_context.entities.len());
        for i in 0..max_entity_len {
            if let Some(e) = local_context.entities.get(i) {
                if seen_entities.insert(e.name.clone()) {
                    merged.add_entity(e.clone());
                }
            }
            if let Some(e) = global_context.entities.get(i) {
                if seen_entities.insert(e.name.clone()) {
                    merged.add_entity(e.clone());
                }
            }
        }

        // Add relationships from local+global (dedup by key)
        let mut seen_rels = std::collections::HashSet::new();
        for rel in local_context
            .relationships
            .iter()
            .chain(global_context.relationships.iter())
        {
            let key = format!("{}-{}-{}", rel.source, rel.relation_type, rel.target);
            if seen_rels.insert(key) {
                merged.add_relationship(rel.clone());
            }
        }

        tracing::debug!(
            merged_chunks = merged.chunks.len(),
            merged_entities = merged.entities.len(),
            merged_relationships = merged.relationships.len(),
            "Hybrid merge complete (round-robin)"
        );

        Ok(merged)
    }

    /// Mix mode with workspace-specific vector storage.
    async fn query_mix_with_vector_storage(
        &self,
        keywords: &ExtractedKeywords,
        embeddings: &QueryEmbeddings,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
        vector_storage: &Arc<dyn VectorStorage>,
    ) -> Result<QueryContext> {
        // Adaptive blend - delegates to hybrid for now
        self.query_hybrid_with_vector_storage(
            keywords,
            embeddings,
            tenant_id,
            workspace_id,
            vector_storage,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sota_config_default() {
        let config = SOTAQueryConfig::default();
        assert_eq!(config.default_mode, QueryMode::Hybrid);
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
}
