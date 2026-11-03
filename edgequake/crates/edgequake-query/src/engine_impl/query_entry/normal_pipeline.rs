//! SPEC-180: the query pipeline of a `normal` workspace.
//!
//! A request that carries a [`NormalQueryPlan`] skips keyword extraction, graph seeding
//! and entity expansion (LAW-180-6). It embeds the question once, runs the two retrieval
//! legs, and rejoins the shared finalize step (rerank, truncate, answer) as `naive`.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use edgequake_storage::traits::{MetadataFilter, VectorStorage};

use crate::context::QueryContext;
use crate::error::Result;
use crate::modes::QueryMode;
use crate::normal::{
    meta, mmr, retriever::retrieve_normal, rewrite, NormalOutcome, NormalQueryPlan, NormalSearch,
    NormalWeights,
};
use crate::truncation::{balance_context, truncation_config_for_intent};
use crate::types::{QueryRequest, QueryResponse, QueryStats};

use super::super::QueryEngine;
use super::query_pipeline::QueryProviders;

impl QueryEngine {
    /// Full normal query: retrieve, then the shared finalize step.
    pub(crate) async fn run_normal_pipeline(
        &self,
        request: QueryRequest,
        plan: NormalQueryPlan,
        providers: QueryProviders<'_>,
    ) -> Result<QueryResponse> {
        let start = Instant::now();
        let mut stats = QueryStats::default();
        let prepared = self
            .prepare_normal_retrieval(&request, &plan, &providers)
            .await?;

        let retrieval_start = Instant::now();
        let context = self
            .retrieve_normal_context(&request, &plan, &prepared, &providers)
            .await?;
        stats.embedding_time_ms = prepared.embed_ms;
        stats.retrieval_time_ms = retrieval_start.elapsed().as_millis() as u64;
        stats.context_tokens = context.token_count;
        stats.absorb_arm_metadata(&context);
        self.pipeline_finalize(
            request,
            context,
            QueryMode::Naive,
            &mut stats,
            &providers,
            start,
        )
        .await
    }

    /// Retrieval only, for the streaming path.
    pub(crate) async fn run_normal_context(
        &self,
        request: &QueryRequest,
        plan: &NormalQueryPlan,
        providers: &QueryProviders<'_>,
    ) -> Result<(QueryContext, QueryMode)> {
        let prepared = self
            .prepare_normal_retrieval(request, plan, providers)
            .await?;
        let context = self
            .retrieve_normal_context(request, plan, &prepared, providers)
            .await?;
        Ok((context, QueryMode::Naive))
    }

    /// Rewrite (fail-open), then embed the retrieval text once.
    ///
    /// The chat handler emits stage `retrieving` before it calls the engine, so
    /// this LLM call does not leave the stream silent. The answer prompt keeps
    /// the user's question; only the two retrieval legs see the rewrite.
    async fn prepare_normal_retrieval(
        &self,
        request: &QueryRequest,
        plan: &NormalQueryPlan,
        providers: &QueryProviders<'_>,
    ) -> Result<PreparedNormalRetrieval> {
        let llm = providers
            .answer_llm
            .clone()
            .unwrap_or_else(|| std::sync::Arc::clone(&self.llm_provider));
        let policy = rewrite::RewritePolicy::from_env();
        let outcome = if rewrite::should_rewrite(policy.enabled, &request.query, true) {
            rewrite::rewrite_question(llm.as_ref(), &request.query, policy).await
        } else if request.query.trim().is_empty() {
            rewrite::RewriteOutcome {
                query: request.query.clone(),
                applied: false,
                reason: "blank",
                llm_calls: 0,
                elapsed_ms: 0,
            }
        } else {
            rewrite::RewriteOutcome {
                query: request.query.clone(),
                applied: false,
                reason: "disabled",
                llm_calls: 0,
                elapsed_ms: 0,
            }
        };
        // A precomputed text vector belongs to the original question. Rewriting
        // and then searching with the old vector would mix two queries.
        let embed_start = Instant::now();
        let text_vector = if outcome.applied {
            let vector = providers.embedding.embed_one(&outcome.query).await?;
            Some(crate::cache::embedding_cache::l2_normalize_vec(vector))
        } else if let Some(vector) = &plan.text_vector {
            Some(vector.clone())
        } else if request.query.trim().is_empty() {
            None
        } else {
            let vector = providers.embedding.embed_one(&request.query).await?;
            Some(crate::cache::embedding_cache::l2_normalize_vec(vector))
        };
        Ok(PreparedNormalRetrieval {
            query: outcome.query,
            text_vector,
            rewrite_applied: outcome.applied,
            rewrite_ms: outcome.elapsed_ms,
            rewrite_llm_calls: outcome.llm_calls,
            embed_ms: embed_start.elapsed().as_millis() as u64,
        })
    }

    async fn retrieve_normal_context(
        &self,
        request: &QueryRequest,
        plan: &NormalQueryPlan,
        prepared: &PreparedNormalRetrieval,
        providers: &QueryProviders<'_>,
    ) -> Result<QueryContext> {
        let vectors: Arc<dyn VectorStorage> = providers
            .vector_storage
            .cloned()
            .unwrap_or_else(|| Arc::clone(&self.vector_storage));
        let max_chunks = request.max_results.unwrap_or(self.config.max_chunks);
        // SPEC-180 09: a bad weight is a configuration error, never a silent fallback to the
        // defaults — otherwise an operator who tuned recall believes a value that never applied.
        let weights = NormalWeights::from_env_cached()
            .map_err(|message| crate::error::QueryError::ConfigError(message.to_string()))?;
        let scope = normal_scope(request, plan, providers.embedding.model());
        let search = NormalSearch {
            query_text: &prepared.query,
            text_vector: prepared.text_vector.as_deref(),
            image_vector: plan.image_vector.as_deref(),
            scope,
            max_chunks,
            candidate_multiplier: self.config.bm25_candidate_multiplier,
            weights,
        };
        let NormalOutcome { chunks, trace } = retrieve_normal(&vectors, &search).await?;
        let mut context = QueryContext::new();
        stamp_trace(&mut context, &trace);
        stamp_rewrite(&mut context, prepared);
        for chunk in chunks {
            context.add_chunk(chunk);
        }
        crate::chunk_hydration::hydrate_retrieved_chunks(
            self.kv_storage.as_deref(),
            &mut context.chunks,
        )
        .await;
        Ok(context)
    }

    /// Post-retrieval steps a chunk-only context needs: scope filter, optional rerank,
    /// token budget. Nothing graph-shaped runs (no entity expansion, no path prune).
    pub(crate) async fn postprocess_normal_context(
        &self,
        request: &QueryRequest,
        mut context: QueryContext,
        mut stats: Option<&mut QueryStats>,
    ) -> QueryContext {
        crate::context_filter::filter_context_by_document_ids(
            &mut context,
            request.allowed_document_ids.as_deref(),
        );
        let rerank = request.enable_rerank.unwrap_or(self.config.enable_rerank);
        if rerank && self.reranker.is_some() {
            let started = Instant::now();
            context.chunks = self
                .rerank_chunks(
                    &request.query,
                    context.chunks,
                    request.enable_rerank,
                    request.rerank_top_k,
                    false,
                    0,
                    &[],
                )
                .await;
            if let Some(s) = stats.as_mut() {
                s.rerank_time_ms = Some(started.elapsed().as_millis() as u64);
            }
        }
        let dropped = self.apply_normal_mmr(request, &mut context).await;
        context
            .metadata
            .insert(meta::MMR_DROPPED.to_string(), serde_json::json!(dropped));
        let intent = crate::keywords::QueryIntent::classify_heuristic(&request.query);
        let budget = truncation_config_for_intent(&self.config.truncation, intent);
        let before = context.chunks.len();
        let (_, _, chunks) = balance_context(
            Vec::new(),
            Vec::new(),
            std::mem::take(&mut context.chunks),
            &budget,
            self.tokenizer.as_ref(),
        );
        context.chunks = chunks;
        context.is_truncated = context.chunks.len() < before;
        context.ensure_stable_citation_ids();
        if let Some(s) = stats.as_mut() {
            s.context_truncated = context.is_truncated;
            s.context_empty = context.chunks.is_empty();
        }
        context
    }

    /// Drop near-duplicate chunks before the token budget. A read failure or a
    /// list with no vectors leaves the chunks alone.
    async fn apply_normal_mmr(&self, request: &QueryRequest, context: &mut QueryContext) -> usize {
        if !mmr::mmr_enabled_from_env() || context.chunks.len() < 2 {
            return 0;
        }
        let ids: Vec<String> = context.chunks.iter().map(|c| c.id.clone()).collect();
        let fetched = match self.vector_storage.get_embeddings_by_ids(&ids).await {
            Ok(rows) => rows,
            Err(error) => {
                tracing::warn!(error = %error, "normal MMR skipped; chunk vectors could not be read");
                return 0;
            }
        };
        if fetched.is_empty() {
            return 0;
        }
        let vectors: HashMap<String, Vec<f32>> = fetched.into_iter().collect();
        let before = context.chunks.len();
        let keep = request.max_results.unwrap_or(self.config.max_chunks).max(1);
        let candidates = context
            .chunks
            .iter()
            .map(|chunk| mmr::MmrCandidate {
                id: chunk.id.clone(),
                relevance: chunk.score,
                vector: if chunk.is_image_chunk() {
                    None
                } else {
                    vectors.get(&chunk.id).cloned()
                },
            })
            .collect();
        let selected = mmr::mmr_select(candidates, mmr::mmr_lambda_from_env(), keep);
        let mut by_id: HashMap<String, crate::context::RetrievedChunk> = context
            .chunks
            .drain(..)
            .map(|chunk| (chunk.id.clone(), chunk))
            .collect();
        context.chunks = selected
            .into_iter()
            .filter_map(|candidate| by_id.remove(&candidate.id))
            .collect();
        before.saturating_sub(context.chunks.len())
    }
}

struct PreparedNormalRetrieval {
    query: String,
    text_vector: Option<Vec<f32>>,
    rewrite_applied: bool,
    rewrite_ms: u64,
    rewrite_llm_calls: u64,
    embed_ms: u64,
}

/// Scope filter of a normal query: tenant, workspace, document ids, chunk vectors only,
/// the workspace embedding model, and the kind predicates (all pushed into SQL).
fn normal_scope(
    request: &QueryRequest,
    plan: &NormalQueryPlan,
    embedding_model: &str,
) -> MetadataFilter {
    let mut scope = MetadataFilter {
        tenant_id: request.tenant_id(),
        workspace_id: request.workspace_id(),
        document_ids: request
            .allowed_document_ids
            .clone()
            .filter(|ids| !ids.is_empty()),
        vector_type: Some("chunk".into()),
        embedding_model: Some(embedding_model.trim().to_string()).filter(|m| !m.is_empty()),
        ..MetadataFilter::default()
    };
    scope.kinds = plan.kinds.clone();
    scope
}

fn stamp_rewrite(context: &mut QueryContext, prepared: &PreparedNormalRetrieval) {
    context.metadata.insert(
        meta::QUERY_REWRITTEN.to_string(),
        serde_json::json!(prepared.rewrite_applied),
    );
    context.metadata.insert(
        meta::REWRITE_MS.to_string(),
        serde_json::json!(prepared.rewrite_ms),
    );
    context.metadata.insert(
        meta::REWRITE_LLM_CALLS.to_string(),
        serde_json::json!(prepared.rewrite_llm_calls),
    );
}

fn stamp_trace(context: &mut QueryContext, trace: &crate::normal::NormalTrace) {
    let put = |ctx: &mut QueryContext, key: &str, value: serde_json::Value| {
        ctx.metadata.insert(key.to_string(), value);
    };
    put(context, meta::PROFILE, serde_json::json!("normal"));
    put(
        context,
        meta::DENSE_HITS,
        serde_json::json!(trace.dense_hits),
    );
    put(context, meta::BM25_HITS, serde_json::json!(trace.bm25_hits));
    put(
        context,
        meta::FUSED_HITS,
        serde_json::json!(trace.fused_hits),
    );
    put(
        context,
        meta::BM25_BACKEND,
        serde_json::json!(trace.bm25_backend),
    );
    put(
        context,
        meta::QUERY_MODALITY,
        serde_json::json!(trace.modality.as_str()),
    );
    // SPEC-184: record what the relevance floor removed, so a caller can tell "this corpus has
    // nothing on that" apart from "nothing was retrieved" — and say so honestly.
    put(
        context,
        meta::RELEVANCE_DROPPED,
        serde_json::json!(trace.relevance.dropped),
    );
    put(
        context,
        meta::RELEVANCE_EMPTIED,
        serde_json::json!(trace.relevance.emptied()),
    );
}
