//! SOTA Query Engine - LightRAG-inspired implementation.
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

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::context::{QueryContext, RetrievedChunk, RetrievedEntity, RetrievedRelationship};
use crate::error::{QueryError, Result};
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

/// Extract document UUID from chunk ID.
///
/// Chunk IDs are formatted as "uuid-chunk-N" (e.g., "f0291a69-8b63-46d5-b44b-24095b3a8283-chunk-0").
/// This function extracts the UUID portion for document linking.
fn extract_document_id(chunk_id: &str) -> Option<String> {
    if let Some(suffix_idx) = chunk_id.rfind("-chunk-") {
        if suffix_idx > 0 {
            return Some(chunk_id[..suffix_idx].to_string());
        }
    }
    None
}

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
            max_entities: 20,
            max_relationships: 20,
            max_chunks: 10,
            max_context_tokens: 4000,
            graph_depth: 2,
            min_score: 0.1,
            use_keyword_extraction: true,
            use_adaptive_mode: true,
            truncation: TruncationConfig::default(),
            keyword_cache_ttl_secs: 24 * 60 * 60, // 24 hours
            enable_rerank: true,                  // Enable by default for SOTA quality
            min_rerank_score: 0.3,
            rerank_top_k: 10,
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
            keyword_validation_cache: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
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
            keyword_validation_cache: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
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
        let raw_keywords = if self.config.use_keyword_extraction {
            self.keyword_extractor
                .extract_extended(&request.query)
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

        self.llm_provider
            .stream(&prompt)
            .await
            .map(|stream| stream.map(|res| res.map_err(QueryError::from)).boxed())
            .map_err(QueryError::from)
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

        let stream = self
            .llm_provider
            .stream(&prompt)
            .await
            .map(|stream| stream.map(|res| res.map_err(QueryError::from)).boxed())
            .map_err(QueryError::from)?;

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
        let raw_keywords = if self.config.use_keyword_extraction {
            self.keyword_extractor
                .extract_extended(&request.query)
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
            .filter_map(|r| {
                let entity_name = r
                    .metadata
                    .get("entity_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| r.id.clone());
                Some((entity_name, r.score))
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
        for (id, node) in &nodes_map {
            let degree = degrees.get(id).copied().unwrap_or(0);
            let entity_type = node
                .properties
                .get("entity_type")
                .and_then(|v| v.as_str())
                .unwrap_or("UNKNOWN")
                .to_string();
            let description = node
                .properties
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Extract source tracking info (LightRAG parity)
            let source_chunk_ids: Vec<String> = node
                .properties
                .get("source_chunk_ids")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            let source_document_id = node
                .properties
                .get("source_document_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let source_file_path = node
                .properties
                .get("source_file_path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // Use preserved similarity score from vector search (fixes score=0.0 bug)
            let entity_score = entity_scores.get(id).copied().unwrap_or(0.0);
            let mut entity = RetrievedEntity::new(id, entity_type, description)
                .with_degree(degree)
                .with_score(entity_score);
            if !source_chunk_ids.is_empty() {
                entity = entity.with_source_chunk_ids(source_chunk_ids);
            }
            if let Some(doc_id) = source_document_id {
                entity = entity.with_source_document_id(doc_id);
            }
            if let Some(file_path) = source_file_path {
                entity = entity.with_source_file_path(file_path);
            }
            context.add_entity(entity);
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

            let rel_type = edge
                .properties
                .get("relation_type")
                .and_then(|v| v.as_str())
                .unwrap_or("RELATED_TO")
                .to_string();

            // Extract source tracking for relationships
            let source_chunk_id = edge
                .properties
                .get("source_chunk_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let source_document_id = edge
                .properties
                .get("source_document_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let source_file_path = edge
                .properties
                .get("source_file_path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let mut rel = RetrievedRelationship::new(&edge.source, &edge.target, rel_type);
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
            let chunk_ids_vec: Vec<String> =
                chunk_ids.into_iter().take(self.config.max_chunks).collect();

            // Use the low-level keyword embedding to query for these specific chunks
            // Query with filter to retrieve only the specific chunks
            let results = self
                .vector_storage
                .query(
                    &embeddings.low_level,
                    chunk_ids_vec.len(),
                    Some(&chunk_ids_vec),
                )
                .await?;

            for result in results {
                if !self.matches_tenant_filter(&result.metadata, &tenant_id, &workspace_id) {
                    continue;
                }

                let content = result
                    .metadata
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let mut chunk = RetrievedChunk::new(&result.id, content, result.score);

                // Extract document_id from chunk_id (format: "uuid-chunk-N")
                if let Some(doc_id) = extract_document_id(&result.id) {
                    chunk = chunk.with_document_id(doc_id);
                }

                // Extract line number information if available
                if let Some(start) = result.metadata.get("start_line").and_then(|v| v.as_u64()) {
                    if let Some(end) = result.metadata.get("end_line").and_then(|v| v.as_u64()) {
                        chunk = chunk.with_lines(start as usize, end as usize);
                    }
                }
                if let Some(idx) = result.metadata.get("chunk_index").and_then(|v| v.as_u64()) {
                    chunk = chunk.with_chunk_index(idx as usize);
                }
                context.add_chunk(chunk);
            }
        }

        Ok(context)
    }

    /// Global mode: Relationship-centric search with high-level keywords.
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
                let entity_type = node
                    .properties
                    .get("entity_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UNKNOWN")
                    .to_string();
                let description = node
                    .properties
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                // Extract source tracking
                let source_chunk_ids: Vec<String> = node
                    .properties
                    .get("source_chunk_ids")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();
                let source_document_id = node
                    .properties
                    .get("source_document_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let source_file_path = node
                    .properties
                    .get("source_file_path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let mut entity =
                    RetrievedEntity::new(&node.id, entity_type, description).with_degree(degree);
                if !source_chunk_ids.is_empty() {
                    entity = entity.with_source_chunk_ids(source_chunk_ids);
                }
                if let Some(doc_id) = source_document_id {
                    entity = entity.with_source_document_id(doc_id);
                }
                if let Some(file_path) = source_file_path {
                    entity = entity.with_source_file_path(file_path);
                }
                context.add_entity(entity);
            }

            // Get edges between popular entities
            if !entity_ids.is_empty() {
                let edges = self
                    .graph_storage
                    .get_edges_for_nodes_batch(&entity_ids)
                    .await?;
                for edge in edges.iter().take(self.config.max_relationships) {
                    let rel_type = edge
                        .properties
                        .get("relation_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("RELATED_TO")
                        .to_string();

                    // Extract source tracking
                    let source_chunk_id = edge
                        .properties
                        .get("source_chunk_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let source_document_id = edge
                        .properties
                        .get("source_document_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let source_file_path = edge
                        .properties
                        .get("source_file_path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let mut rel = RetrievedRelationship::new(&edge.source, &edge.target, rel_type);
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

            for (id, node) in &nodes_map {
                let degree = degrees.get(id).copied().unwrap_or(0);
                let entity_type = node
                    .properties
                    .get("entity_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UNKNOWN")
                    .to_string();
                let description = node
                    .properties
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                // Extract source tracking
                let source_chunk_ids: Vec<String> = node
                    .properties
                    .get("source_chunk_ids")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();
                let source_document_id = node
                    .properties
                    .get("source_document_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let source_file_path = node
                    .properties
                    .get("source_file_path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let mut entity =
                    RetrievedEntity::new(id, entity_type, description).with_degree(degree);
                if !source_chunk_ids.is_empty() {
                    entity = entity.with_source_chunk_ids(source_chunk_ids);
                }
                if let Some(doc_id) = source_document_id {
                    entity = entity.with_source_document_id(doc_id);
                }
                if let Some(file_path) = source_file_path {
                    entity = entity.with_source_file_path(file_path);
                }
                context.add_entity(entity);
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
            let content = result
                .metadata
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let mut chunk = RetrievedChunk::new(&result.id, content, result.score);
            if let Some(doc_id) = extract_document_id(&result.id) {
                chunk = chunk.with_document_id(doc_id);
            }
            // Extract line number information if available
            if let Some(start) = result.metadata.get("start_line").and_then(|v| v.as_u64()) {
                if let Some(end) = result.metadata.get("end_line").and_then(|v| v.as_u64()) {
                    chunk = chunk.with_lines(start as usize, end as usize);
                }
            }
            if let Some(idx) = result.metadata.get("chunk_index").and_then(|v| v.as_u64()) {
                chunk = chunk.with_chunk_index(idx as usize);
            }
            context.add_chunk(chunk);
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
            let chunk_ids_vec: Vec<String> =
                source_chunk_ids.into_iter().take(remaining_slots).collect();

            // Use the high-level keyword embedding to query for these specific chunks
            // Query with filter to retrieve only the specific chunks
            let results = self
                .vector_storage
                .query(
                    &embeddings.high_level,
                    chunk_ids_vec.len(),
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

                let content = result
                    .metadata
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let mut chunk = RetrievedChunk::new(&result.id, content, result.score);

                if let Some(doc_id) = extract_document_id(&result.id) {
                    chunk = chunk.with_document_id(doc_id);
                }

                if let Some(start) = result.metadata.get("start_line").and_then(|v| v.as_u64()) {
                    if let Some(end) = result.metadata.get("end_line").and_then(|v| v.as_u64()) {
                        chunk = chunk.with_lines(start as usize, end as usize);
                    }
                }
                if let Some(idx) = result.metadata.get("chunk_index").and_then(|v| v.as_u64()) {
                    chunk = chunk.with_chunk_index(idx as usize);
                }
                context.add_chunk(chunk);
            }
        }

        Ok(context)
    }

    /// Hybrid mode: Combine local and global with round-robin merging.
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

        // Combine chunks (deduplicated)
        let mut seen_chunks = std::collections::HashSet::new();
        for c in local.chunks.iter().chain(global.chunks.iter()) {
            if seen_chunks.insert(c.id.clone()) {
                context.add_chunk(c.clone());
            }
        }

        Ok(context)
    }

    /// Mix mode: Hybrid plus direct chunk search.
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
                let content = result
                    .metadata
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let mut chunk = RetrievedChunk::new(&result.id, content, result.score);
                if let Some(doc_id) = extract_document_id(&result.id) {
                    chunk = chunk.with_document_id(doc_id);
                }
                // Extract line number information if available
                if let Some(start) = result.metadata.get("start_line").and_then(|v| v.as_u64()) {
                    if let Some(end) = result.metadata.get("end_line").and_then(|v| v.as_u64()) {
                        chunk = chunk.with_lines(start as usize, end as usize);
                    }
                }
                if let Some(idx) = result.metadata.get("chunk_index").and_then(|v| v.as_u64()) {
                    chunk = chunk.with_chunk_index(idx as usize);
                }
                context.add_chunk(chunk);
            }
        }

        Ok(context)
    }

    /// Naive mode: Direct chunk vector search only.
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
            let content = result
                .metadata
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let mut chunk = RetrievedChunk::new(&result.id, content, result.score);
            if let Some(doc_id) = extract_document_id(&result.id) {
                chunk = chunk.with_document_id(doc_id);
            }
            // Extract line number information if available
            if let Some(start) = result.metadata.get("start_line").and_then(|v| v.as_u64()) {
                if let Some(end) = result.metadata.get("end_line").and_then(|v| v.as_u64()) {
                    chunk = chunk.with_lines(start as usize, end as usize);
                }
            }
            if let Some(idx) = result.metadata.get("chunk_index").and_then(|v| v.as_u64()) {
                chunk = chunk.with_chunk_index(idx as usize);
            }
            context.add_chunk(chunk);
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
            let entity_type = node
                .properties
                .get("entity_type")
                .and_then(|v| v.as_str())
                .unwrap_or("UNKNOWN")
                .to_string();
            let description = node
                .properties
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Extract source tracking
            let source_chunk_ids: Vec<String> = node
                .properties
                .get("source_chunk_ids")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            let source_document_id = node
                .properties
                .get("source_document_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let source_file_path = node
                .properties
                .get("source_file_path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let mut entity =
                RetrievedEntity::new(&node.id, entity_type, description).with_degree(degree);
            if !source_chunk_ids.is_empty() {
                entity = entity.with_source_chunk_ids(source_chunk_ids);
            }
            if let Some(doc_id) = source_document_id {
                entity = entity.with_source_document_id(doc_id);
            }
            if let Some(file_path) = source_file_path {
                entity = entity.with_source_file_path(file_path);
            }
            context.add_entity(entity);
        }

        // Get edges
        if !entity_ids.is_empty() {
            let edges = self
                .graph_storage
                .get_edges_for_nodes_batch(&entity_ids)
                .await?;
            for edge in edges.iter().take(self.config.max_relationships) {
                let rel_type = edge
                    .properties
                    .get("relation_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("RELATED_TO")
                    .to_string();

                // Extract source tracking
                let source_chunk_id = edge
                    .properties
                    .get("source_chunk_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let source_document_id = edge
                    .properties
                    .get("source_document_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let source_file_path = edge
                    .properties
                    .get("source_file_path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let mut rel = RetrievedRelationship::new(&edge.source, &edge.target, rel_type);
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
            r#"You are a helpful assistant. Answer the user's question based ONLY on the context below.

## Context
{context_text}

## Question
{query}

## CRITICAL Instructions
1. **EXTRACT MAXIMUM VALUE**: Even if the question asks about items not fully covered, provide ALL available information about related items in the context.
2. **COMPARISON HANDLING**: For comparison queries (X vs Y):
   - If you have data for BOTH items: Compare them directly with specific numbers.
   - If you have data for only ONE item: Provide detailed specs for that item, then briefly note the other item lacks data.
   - NEVER respond with just "no information" - always share what you found.
3. **TECHNICAL DETAILS REQUIRED**: Include battery capacity (kWh), charging speed (kW), autonomy (km), efficiency metrics.
4. **LANGUAGE**: Respond in the SAME language as the question.
5. **BE HELPFUL**: The user needs actionable information. A partial answer with specific data is better than a generic "insufficient information" response.

## Answer"#
        )
    }

    /// Generate answer using LLM.
    async fn generate_answer(
        &self,
        query: &str,
        context: &QueryContext,
    ) -> Result<(String, usize)> {
        if context.is_empty() {
            return Ok((
                "I'm sorry, but I couldn't find any relevant information in my knowledge base to answer your question.".to_string(),
                0,
            ));
        }

        let prompt = self.build_prompt(query, context);
        let response = self.llm_provider.complete(&prompt).await?;

        Ok((response.content, response.completion_tokens))
    }

    /// Get the engine configuration.
    pub fn config(&self) -> &SOTAQueryConfig {
        &self.config
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
