//! Query entry points using workspace-specific vector storage.
//!
//! Contains: `query_with_workspace_config`, `query_with_full_config`,
//!           `query_with_vector_storage`.
//! These methods override the default vector storage with a workspace-specific one.

use crate::error::Result;
use crate::keywords::{ExtractedKeywords, QueryIntent};
use crate::modes::QueryMode;
use crate::truncation::balance_context;

use edgequake_storage::traits::VectorStorage;

use super::super::{QueryEmbeddings, SOTAQueryEngine};

impl SOTAQueryEngine {
    /// Execute a query with workspace-specific vector storage using the engine's default embedding.
    ///
    /// WHY: Workspaces that share the same embedding model as the server default still need
    /// queries to search their workspace-specific vector table.  Without this method the
    /// caller would have to duplicate `self.embedding_provider` knowledge.
    ///
    /// @implements SPEC-033 (Workspace vector isolation)
    /// @implements SPEC-0002 (Knowledge injection — vectors stored in workspace table are found)
    ///
    /// # Arguments
    ///
    /// * `request` - The query request
    /// * `vector_storage` - The workspace-specific vector storage to search
    pub async fn query_with_vector_storage(
        &self,
        request: crate::engine::QueryRequest,
        vector_storage: std::sync::Arc<dyn VectorStorage>,
    ) -> Result<crate::engine::QueryResponse> {
        // Delegate to query_with_workspace_config using the engine's own embedding provider.
        // WHY: The workspace may use the same embedding model as the server default but still
        // needs its vectors searched in the workspace-specific table.
        self.query_with_workspace_config(request, self.embedding_provider.clone(), vector_storage)
            .await
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

        // Steps 1 + 3 (partial): parallel keyword extraction + query embedding.
        // WHY: Same rationale as query() — LLM and embedding calls are independent;
        // run both concurrently to eliminate the embedding round-trip from the critical path.
        let par_start = std::time::Instant::now();
        let (raw_keywords_result, query_vec_result) = tokio::join!(
            async {
                if self.config.use_keyword_extraction {
                    self.keyword_extractor
                        .extract_with_llm_override(&request.query, None)
                        .await
                } else {
                    Ok(ExtractedKeywords::new(
                        vec![],
                        vec![],
                        QueryIntent::Exploratory,
                    ))
                }
            },
            embedding_provider.embed_one(&request.query)
        );

        let raw_keywords = raw_keywords_result?;
        let query_vec = query_vec_result.map_err(crate::error::QueryError::from)?;
        stats.embedding_time_ms += par_start.elapsed().as_millis() as u64;

        tracing::debug!(
            query = %request.query,
            high_level = ?raw_keywords.high_level,
            low_level = ?raw_keywords.low_level,
            intent = %raw_keywords.query_intent,
            "Extracted keywords (workspace config, parallel)"
        );

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

        // Step 3: Compute keyword embeddings using WORKSPACE-SPECIFIC embedding provider.
        let embed_start = std::time::Instant::now();
        let embeddings = QueryEmbeddings::compute_with_query_vec(
            &request.query,
            query_vec,
            &keywords,
            embedding_provider.as_ref(),
        )
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
        // SPEC-005: Filter context by allowed document IDs
        crate::context_filter::filter_context_by_document_ids(
            &mut context,
            request.allowed_document_ids.as_deref(),
        );
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

        // Steps 1 + 3 (partial): parallel keyword extraction + query embedding.
        // WHY: FIX #168 still applies — keyword extraction uses the same LLM as answer
        // generation. Embedding is independent; run both concurrently.
        let par_start = std::time::Instant::now();
        let has_llm_override = llm_provider.is_some();
        let (raw_keywords_result, query_vec_result) = tokio::join!(
            async {
                if self.config.use_keyword_extraction {
                    self.keyword_extractor
                        .extract_with_llm_override(&request.query, llm_provider.clone())
                        .await
                } else {
                    Ok(ExtractedKeywords::new(
                        vec![],
                        vec![],
                        QueryIntent::Exploratory,
                    ))
                }
            },
            embedding_provider.embed_one(&request.query)
        );

        let raw_keywords = raw_keywords_result?;
        let query_vec = query_vec_result.map_err(crate::error::QueryError::from)?;
        stats.embedding_time_ms += par_start.elapsed().as_millis() as u64;

        tracing::debug!(
            query = %request.query,
            high_level = ?raw_keywords.high_level,
            low_level = ?raw_keywords.low_level,
            intent = %raw_keywords.query_intent,
            has_llm_override,
            "Extracted keywords (full config, parallel)"
        );

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

        // Step 3: Compute keyword embeddings using WORKSPACE-SPECIFIC embedding provider.
        let embed_start = std::time::Instant::now();
        let embeddings = QueryEmbeddings::compute_with_query_vec(
            &request.query,
            query_vec,
            &keywords,
            embedding_provider.as_ref(),
        )
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

        // Step 6: Generate answer using OVERRIDE LLM or default
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
            let result = if let Some(ref llm) = llm_provider {
                // Use override LLM provider
                self.generate_answer_with_provider(
                    &request.query,
                    &final_context,
                    Some(llm),
                    request.system_prompt.as_deref(),
                )
                .await?
            } else {
                // Use default LLM provider
                self.generate_answer(
                    &request.query,
                    &final_context,
                    request.system_prompt.as_deref(),
                )
                .await?
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
}
