//! Shared query pipeline phases (SPEC-017 P1-01).
//!
//! Single prepare → retrieve → finalize implementation used by all non-stream
//! query entry points. Eliminates triplicated ~600 LOC blocks across basic,
//! workspace, and LLM-override paths.

use std::sync::Arc;
use std::time::Instant;

use edgequake_storage::traits::VectorStorage;

use crate::context::QueryContext;
use crate::error::Result;
use crate::keywords::{ExtractedKeywords, QueryIntent};
use crate::modes::QueryMode;
use crate::truncation::balance_context;
use crate::types::{QueryRequest, QueryResponse, QueryStats};
use crate::{EmbeddingProvider, LLMProvider};

use super::super::{QueryEmbeddings, QueryEngine};

/// Provider bundle for one pipeline run.
pub(crate) struct QueryProviders<'a> {
    pub embedding: &'a dyn EmbeddingProvider,
    pub vector_storage: Option<&'a Arc<dyn VectorStorage>>,
    pub keyword_llm: Option<Arc<dyn LLMProvider>>,
    pub answer_llm: Option<Arc<dyn LLMProvider>>,
}

/// State after prepare phase (keywords, embeddings, mode).
pub(crate) struct PreparedQuery {
    pub keywords: ExtractedKeywords,
    pub embeddings: QueryEmbeddings,
    pub mode: QueryMode,
    pub embedding_time_ms: u64,
    /// Effective chunk cap (`max_results` API override or config default).
    pub max_chunks: usize,
}

impl QueryEngine {
    /// Run the full non-streaming query pipeline with explicit providers.
    #[tracing::instrument(
        name = "query_pipeline",
        skip(self, request, providers),
        err(Debug, level = "warn"),
        fields(
            mode = ?request.mode,
            context_only = request.context_only,
            query_len = request.query.len(),
            error.code = tracing::field::Empty,
            error.message = tracing::field::Empty,
        )
    )]
    pub(crate) async fn run_query_pipeline(
        &self,
        request: QueryRequest,
        providers: QueryProviders<'_>,
    ) -> Result<QueryResponse> {
        let start = Instant::now();
        let mut stats = QueryStats::default();

        if request.mode.is_some_and(|m| m.is_bypass()) {
            return self
                .pipeline_finalize(
                    request,
                    QueryContext::default(),
                    QueryMode::Bypass,
                    &mut stats,
                    &providers,
                    start,
                )
                .await;
        }

        let prepared = self.pipeline_prepare(&request, &providers).await?;
        stats.embedding_time_ms = prepared.embedding_time_ms;

        if request.context_only {
            if let Some(cache) = &self.result_cache {
                if let Some(cached) = cache.get(&request, prepared.mode) {
                    stats.retrieval_time_ms = 0;
                    return self
                        .pipeline_finalize(
                            request,
                            cached,
                            prepared.mode,
                            &mut stats,
                            &providers,
                            start,
                        )
                        .await;
                }
                cache.record_miss();
            }
        }

        let retrieval_start = Instant::now();
        let context = self
            .pipeline_retrieve(&prepared, &request, &providers)
            .await?;
        stats.retrieval_time_ms = retrieval_start.elapsed().as_millis() as u64;
        stats.context_tokens = context.token_count;

        if request.context_only {
            if let Some(cache) = &self.result_cache {
                cache.put(&request, prepared.mode, context.clone());
            }
        }

        self.pipeline_finalize(
            request,
            context,
            prepared.mode,
            &mut stats,
            &providers,
            start,
        )
        .await
    }

    /// Retrieve context only (streaming / inspection paths).
    pub(crate) async fn run_context_pipeline(
        &self,
        request: &QueryRequest,
        providers: QueryProviders<'_>,
    ) -> Result<(QueryContext, QueryMode)> {
        let prepared = self.pipeline_prepare(request, &providers).await?;
        let context = self
            .pipeline_retrieve(&prepared, request, &providers)
            .await?;
        Ok((context, prepared.mode))
    }

    /// Post-retrieval enrichment: filter, rerank, sort, truncate (no LLM generation).
    pub(crate) async fn enrich_retrieved_context(
        &self,
        request: &QueryRequest,
        mut context: QueryContext,
    ) -> QueryContext {
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

        self.sort_entities_by_degree(&mut context.entities);

        let (truncated_entities, truncated_relationships, truncated_chunks) = balance_context(
            context.entities.clone(),
            context.relationships.clone(),
            context.chunks.clone(),
            &self.config.truncation,
            self.tokenizer.as_ref(),
        );

        context.entities = truncated_entities;
        context.relationships = truncated_relationships;
        context.chunks = truncated_chunks;
        context
    }

    async fn pipeline_prepare(
        &self,
        request: &QueryRequest,
        providers: &QueryProviders<'_>,
    ) -> Result<PreparedQuery> {
        let keyword_query = crate::conversation_context::query_with_conversation_context(
            &request.query,
            &request.conversation_history,
            crate::conversation_context::DEFAULT_CONVERSATION_TURN_LIMIT,
        );

        let par_start = Instant::now();
        let keyword_llm = providers.keyword_llm.clone();
        let (raw_keywords_result, query_vec_result) = tokio::join!(
            async {
                if self.config.use_keyword_extraction {
                    if let Some(llm) = keyword_llm {
                        self.keyword_extractor
                            .extract_with_llm_override(&keyword_query, Some(llm))
                            .await
                    } else {
                        self.keyword_extractor
                            .extract_extended(&keyword_query)
                            .await
                    }
                } else {
                    Ok(ExtractedKeywords::new(
                        vec![],
                        vec![],
                        QueryIntent::Exploratory,
                    ))
                }
            },
            async {
                // Skip embed_one when keywords are disabled — compute_with_query_vec
                // batch-embeds three levels (MockProvider / LightRAG parity).
                if self.config.use_keyword_extraction {
                    providers.embedding.embed_one(&keyword_query).await
                } else {
                    Ok(vec![])
                }
            }
        );

        let raw_keywords = raw_keywords_result?;
        let query_vec = query_vec_result.map_err(crate::error::QueryError::from)?;
        let mut embedding_time_ms = par_start.elapsed().as_millis() as u64;

        let keywords = self.validate_keywords(&raw_keywords).await;

        let mode = if let Some(m) = request.mode {
            m
        } else if self.config.use_adaptive_mode {
            keywords.query_intent.recommended_mode()
        } else {
            self.config.default_mode
        };

        let embed_start = Instant::now();
        let embeddings = QueryEmbeddings::compute_with_query_vec(
            &request.query,
            query_vec,
            &keywords,
            providers.embedding,
        )
        .await?;
        embedding_time_ms += embed_start.elapsed().as_millis() as u64;

        Ok(PreparedQuery {
            keywords,
            embeddings,
            mode,
            embedding_time_ms,
            max_chunks: request.max_results.unwrap_or(self.config.max_chunks),
        })
    }

    async fn pipeline_retrieve(
        &self,
        prepared: &PreparedQuery,
        request: &QueryRequest,
        providers: &QueryProviders<'_>,
    ) -> Result<QueryContext> {
        let tenant = request.tenant_id();
        let workspace = request.workspace_id();
        let mode = prepared.mode;
        let keywords = &prepared.keywords;
        let embeddings = &prepared.embeddings;

        let max_chunks = prepared.max_chunks;
        // SPEC-031: pass allowed_document_ids into every mode for Tier 1 pre-filter
        let allowed_doc_ids = request.allowed_document_ids.as_deref();

        match providers.vector_storage {
            Some(vector_storage) => match mode {
                QueryMode::Local => {
                    self.query_local_with_vector_storage(
                        &request.query,
                        keywords,
                        embeddings,
                        tenant,
                        workspace,
                        allowed_doc_ids,
                        vector_storage,
                        max_chunks,
                    )
                    .await
                }
                QueryMode::Global => {
                    self.query_global_with_vector_storage(
                        &request.query,
                        keywords,
                        embeddings,
                        tenant,
                        workspace,
                        allowed_doc_ids,
                        vector_storage,
                        max_chunks,
                    )
                    .await
                }
                QueryMode::Hybrid => {
                    self.query_hybrid_with_vector_storage(
                        &request.query,
                        keywords,
                        embeddings,
                        tenant,
                        workspace,
                        allowed_doc_ids,
                        vector_storage,
                        max_chunks,
                    )
                    .await
                }
                QueryMode::Mix => {
                    self.query_mix_with_vector_storage(
                        &request.query,
                        keywords,
                        embeddings,
                        tenant,
                        workspace,
                        allowed_doc_ids,
                        vector_storage,
                        request.mix_weights.as_ref(),
                        max_chunks,
                    )
                    .await
                }
                QueryMode::Naive => {
                    self.query_naive_with_vector_storage(
                        &request.query,
                        embeddings,
                        tenant,
                        workspace,
                        allowed_doc_ids,
                        vector_storage,
                        max_chunks,
                    )
                    .await
                }
                QueryMode::Bypass => Ok(QueryContext::default()),
            },
            None => match mode {
                QueryMode::Local => {
                    self.query_local(
                        &request.query,
                        keywords,
                        embeddings,
                        tenant,
                        workspace,
                        max_chunks,
                    )
                    .await
                }
                QueryMode::Global => {
                    self.query_global(
                        &request.query,
                        keywords,
                        embeddings,
                        tenant,
                        workspace,
                        max_chunks,
                    )
                    .await
                }
                QueryMode::Hybrid => {
                    self.query_hybrid(
                        &request.query,
                        keywords,
                        embeddings,
                        tenant,
                        workspace,
                        max_chunks,
                    )
                    .await
                }
                QueryMode::Mix => {
                    self.query_mix(
                        &request.query,
                        keywords,
                        embeddings,
                        tenant,
                        workspace,
                        request.mix_weights.as_ref(),
                        max_chunks,
                    )
                    .await
                }
                QueryMode::Naive => {
                    self.query_naive(&request.query, embeddings, tenant, workspace, max_chunks)
                        .await
                }
                QueryMode::Bypass => Ok(QueryContext::default()),
            },
        }
    }

    async fn pipeline_finalize(
        &self,
        request: QueryRequest,
        mut context: QueryContext,
        mode: QueryMode,
        stats: &mut QueryStats,
        providers: &QueryProviders<'_>,
        pipeline_start: Instant,
    ) -> Result<QueryResponse> {
        crate::context_filter::filter_context_by_document_ids(
            &mut context,
            request.allowed_document_ids.as_deref(),
        );

        let should_rerank = request.enable_rerank.unwrap_or(self.config.enable_rerank);
        if should_rerank && self.reranker.is_some() {
            let rerank_start = Instant::now();
            let reranked_chunks = self
                .rerank_chunks(
                    &request.query,
                    context.chunks,
                    request.enable_rerank,
                    request.rerank_top_k,
                )
                .await;
            context.chunks = reranked_chunks;
            stats.rerank_time_ms = Some(rerank_start.elapsed().as_millis() as u64);
        }

        self.sort_entities_by_degree(&mut context.entities);

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

        let (answer, generated_tokens) = if request.context_only {
            (String::new(), 0)
        } else if request.prompt_only {
            (
                self.build_prompt(
                    &request.query,
                    &final_context,
                    request.system_prompt.as_deref(),
                    &request.conversation_history,
                ),
                0,
            )
        } else if mode.is_bypass() {
            // P-G8 / RC-13: Bypass = direct LLM, no RAG template, no apology.
            // `final_context` is intentionally empty for Bypass (the retrieval
            // step is skipped in `run_query_pipeline`); the RAG `generate_answer`
            // would misinterpret that emptiness as a retrieval miss and return
            // the apology string. Use the dedicated direct-LLM path instead.
            let gen_start = Instant::now();
            let result = self
                .generate_bypass_answer(
                    &request.query,
                    providers.answer_llm.as_ref(),
                    request.system_prompt.as_deref(),
                    request.images.as_deref(),
                )
                .await?;
            stats.generation_time_ms = gen_start.elapsed().as_millis() as u64;
            result
        } else {
            let gen_start = Instant::now();
            let result = if let Some(ref llm) = providers.answer_llm {
                self.generate_answer_with_provider(
                    &request.query,
                    &final_context,
                    Some(llm),
                    request.system_prompt.as_deref(),
                    request.images.as_deref(),
                    &request.conversation_history,
                )
                .await?
            } else {
                self.generate_answer(
                    &request.query,
                    &final_context,
                    request.system_prompt.as_deref(),
                    &request.conversation_history,
                )
                .await?
            };
            stats.generation_time_ms = gen_start.elapsed().as_millis() as u64;
            result
        };

        stats.generated_tokens = generated_tokens;
        stats.total_time_ms = pipeline_start.elapsed().as_millis() as u64;

        Ok(QueryResponse {
            answer,
            context: final_context,
            mode,
            stats: stats.clone(),
        })
    }
}
