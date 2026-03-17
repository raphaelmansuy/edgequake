//! Core query entry points using the engine's default vector storage.
//!
//! Contains: `query`, `query_with_embedding_provider`, `query_with_llm_provider`,
//! and `get_context`.

use crate::context::QueryContext;
use crate::error::Result;
use crate::keywords::{ExtractedKeywords, QueryIntent};
use crate::modes::QueryMode;
use crate::truncation::balance_context;

use super::super::{QueryEmbeddings, SOTAQueryEngine};

impl SOTAQueryEngine {
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
        // SPEC-005: Filter context by allowed document IDs
        crate::context_filter::filter_context_by_document_ids(
            &mut context,
            request.allowed_document_ids.as_deref(),
        );
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
            (
                self.build_prompt(
                    &request.query,
                    &final_context,
                    request.system_prompt.as_deref(),
                ),
                0,
            )
        } else {
            let gen_start = std::time::Instant::now();
            let result = self
                .generate_answer(
                    &request.query,
                    &final_context,
                    request.system_prompt.as_deref(),
                )
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
        // SPEC-005: Filter context by allowed document IDs
        crate::context_filter::filter_context_by_document_ids(
            &mut context,
            request.allowed_document_ids.as_deref(),
        );
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
            (
                self.build_prompt(
                    &request.query,
                    &final_context,
                    request.system_prompt.as_deref(),
                ),
                0,
            )
        } else {
            let gen_start = std::time::Instant::now();
            let result = self
                .generate_answer(
                    &request.query,
                    &final_context,
                    request.system_prompt.as_deref(),
                )
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
        // SPEC-005: Filter context by allowed document IDs
        crate::context_filter::filter_context_by_document_ids(
            &mut context,
            request.allowed_document_ids.as_deref(),
        );
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
            (
                self.build_prompt(
                    &request.query,
                    &final_context,
                    request.system_prompt.as_deref(),
                ),
                0,
            )
        } else {
            let gen_start = std::time::Instant::now();
            // SPEC-032: Use the override LLM provider
            let result = self
                .generate_answer_with_provider(
                    &request.query,
                    &final_context,
                    Some(&llm_provider),
                    request.system_prompt.as_deref(),
                )
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
        // SPEC-005: Filter context by allowed document IDs
        crate::context_filter::filter_context_by_document_ids(
            &mut context,
            request.allowed_document_ids.as_deref(),
        );
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
}
