use crate::context::QueryContext;
use crate::keywords::ExtractedKeywords;
use crate::relevancy_prune::{
    apply_cosine_rescored_prune, capped_chunk_text, RelevancyPruneConfig,
};

use super::QueryEngine;

/// Forms of a keyword that can appear in a graph node id.
///
/// Stored ids are [`edgequake_storage::normalize_entity_name`]: spaces become
/// underscores and the result is uppercased (`intrathecal chemotherapy` →
/// `INTRATHECAL_CHEMOTHERAPY`). Label search matches that id text. Searching
/// only the spaced phrase drops the entity and can keep a question word that
/// is a substring of a different id.
pub(crate) fn keyword_graph_probes(keyword: &str) -> Vec<String> {
    let mut probes = Vec::with_capacity(2);
    let normalized = edgequake_storage::normalize_entity_name(keyword);
    if !normalized.is_empty() {
        probes.push(normalized);
    }
    let raw = keyword.trim();
    if !raw.is_empty() && !probes.iter().any(|probe| probe.eq_ignore_ascii_case(raw)) {
        probes.push(raw.to_string());
    }
    probes
}

// WHY: validate_keywords makes N graph search calls for N keywords.
// Using parallel execution eliminates the N×RTT sequential latency.

impl QueryEngine {
    #[allow(clippy::too_many_arguments)] // CE / BM25 / protect knobs stay explicit
    pub(super) async fn rerank_chunks(
        &self,
        query: &str,
        mut chunks: Vec<crate::context::RetrievedChunk>,
        enable_override: Option<bool>,
        top_k_override: Option<usize>,
        // 027: Fact→BM25 when intent-gated (skip CE + protect).
        prefer_bm25: bool,
        // 036: intent-aware protect slots (Exploratory coverage override).
        protect_first: usize,
        // 039: topic-admitted chunk ids — CE set membership protect.
        topic_protect_ids: &[String],
    ) -> Vec<crate::context::RetrievedChunk> {
        // Check if reranking is enabled (use request override if provided)
        let enable_rerank = enable_override.unwrap_or(self.config.enable_rerank);
        let rerank_top_k = top_k_override.unwrap_or(self.config.rerank_top_k);

        // Skip if reranking is disabled or no reranker configured
        if !enable_rerank || self.reranker.is_none() || chunks.is_empty() {
            return chunks;
        }

        // 027: hold a BM25 instance when Fact routing is active so the trait
        // object lives for the full match.
        let bm25_holder;
        let reranker: &dyn edgequake_llm::Reranker = if prefer_bm25 {
            bm25_holder = edgequake_llm::BM25Reranker::for_rag();
            tracing::debug!("027 Fact intent → BM25 rerank (skip CE protect)");
            &bm25_holder
        } else {
            self.reranker.as_ref().unwrap().as_ref()
        };

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

                // 027: BM25 often scores 0.0 on non-overlapping terms — do not hard-drop.
                let min_score = if prefer_bm25 {
                    0.0
                } else {
                    self.config.min_rerank_score as f64
                };

                // Update scores and filter by min score
                let mut reranked: Vec<_> = chunks
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, chunk)| {
                        score_map.get(&idx).and_then(|&score| {
                            if score >= min_score {
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
                        min_rerank_score = min_score,
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

                // SPEC-001 / 036: first-stage protect slots (intent-aware).
                // 027: skip protect on Fact→BM25 (lexical order is the Acc win).
                let protect_n = if prefer_bm25 { 0 } else { protect_first };
                let mut out = if protect_n > 0 {
                    tracing::debug!(
                        protect_n,
                        rerank_top_k,
                        "Blending CE ranks with first-stage protect slots"
                    );
                    crate::rerank_protect::blend_protect_first(
                        &chunks,
                        reranked,
                        protect_n,
                        rerank_top_k,
                    )
                } else {
                    reranked.truncate(rerank_top_k);
                    reranked
                };

                // 039/042: topic-admit / materialized ids survive CE hard-drop.
                // Still skip on Fact→BM25 — that path owns Fact Acc.
                if !prefer_bm25
                    && crate::topic_entity_admit::topic_survival_enabled()
                    && !topic_protect_ids.is_empty()
                {
                    tracing::debug!(
                        topic_n = topic_protect_ids.len(),
                        rerank_top_k,
                        "039 topic_ce_protect: blending CE ranks with topic chunk ids"
                    );
                    out = crate::rerank_protect::blend_protect_ids(
                        &chunks,
                        out,
                        topic_protect_ids,
                        rerank_top_k,
                    );
                }

                out
            }
            Err(e) => {
                tracing::warn!(error = %e, "Reranking failed, returning original chunks");
                chunks.truncate(rerank_top_k);
                chunks
            }
        }
    }

    /// SPEC-001: re-score fused chunks by query↔chunk embedding cosine, then keep-m.
    ///
    /// Fail-open: on embed errors, returns the original context unchanged.
    pub(crate) async fn apply_query_embed_cosine_prune(
        &self,
        query: &str,
        context: QueryContext,
        config: &RelevancyPruneConfig,
    ) -> QueryContext {
        if !config.uses_query_embed_cosine() || context.chunks.is_empty() {
            return context;
        }

        let mut texts = Vec::with_capacity(context.chunks.len() + 1);
        texts.push(query.to_string());
        for chunk in &context.chunks {
            texts.push(capped_chunk_text(&chunk.content, config.embed_char_cap));
        }

        let embeddings = match self.default_embedding_provider().embed(&texts).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "relevancy_prune cosine: embed failed — keeping unpruned context"
                );
                return context;
            }
        };

        if embeddings.len() != texts.len() {
            tracing::warn!(
                expected = texts.len(),
                got = embeddings.len(),
                "relevancy_prune cosine: unexpected embed batch size — keeping unpruned"
            );
            return context;
        }

        let query_vec = &embeddings[0];
        let chunk_vecs = embeddings[1..].to_vec();
        apply_cosine_rescored_prune(context, query_vec, &chunk_vecs, config)
    }

    /// Sort entities by degree (descending). Prefer
    /// [`crate::entity_rank::rank_entities_for_prompt`] via `entity_rank` config.
    #[allow(dead_code)]
    pub(super) fn sort_entities_by_degree(&self, entities: &mut [crate::context::RetrievedEntity]) {
        crate::entity_rank::rank_entities_for_prompt(
            entities,
            crate::entity_rank::EntityRankMode::Degree,
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
    ///
    /// WHY parallel: Each `search_labels` call is an independent DB round-trip.
    /// Running them sequentially paid N×RTT; `join_all` pays max(RTT) instead.
    /// Cache hits are separated first to avoid holding the lock during IO.
    pub(super) async fn validate_keywords(
        &self,
        keywords: &ExtractedKeywords,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> ExtractedKeywords {
        if keywords.low_level.is_empty() {
            return keywords.clone();
        }

        // Step 1: Separate cache hits from misses under a short-lived read lock.
        let mut hit_results: std::collections::HashMap<String, bool> =
            std::collections::HashMap::new();
        let mut miss_keywords: Vec<String> = Vec::new();
        {
            let cache = self.keyword_validation_cache.read().await;
            for kw in &keywords.low_level {
                match cache.get(&kw.to_lowercase()) {
                    Some(&exists) => {
                        hit_results.insert(kw.clone(), exists);
                    }
                    None => miss_keywords.push(kw.clone()),
                }
            }
        }

        // Step 2: Fan-out all cache misses in parallel (no sequential RTT stacking).
        // 032: scope search to workspace so foreign AGE vertices cannot keep noise keywords.
        let tenant_owned = tenant_id.map(str::to_owned);
        let workspace_owned = workspace_id.map(str::to_owned);
        let miss_futures: Vec<_> = miss_keywords
            .iter()
            .map(|kw| {
                let graph = self.graph_storage.clone();
                let kw = kw.clone();
                let tenant = tenant_owned.clone();
                let workspace = workspace_owned.clone();
                async move {
                    let view = edgequake_storage::GraphReadView::from_arc(&graph);
                    let mut exists = false;
                    // Node ids store the normalized name. The spaced keyword is
                    // not a substring of that id, so try the normalized form
                    // and keep the raw text when it already matches.
                    for probe in keyword_graph_probes(&kw) {
                        exists = view
                            .search_labels(&probe, 1, tenant.as_deref(), workspace.as_deref())
                            .await
                            .map(|labels| !labels.is_empty())
                            .unwrap_or(false);
                        if exists {
                            break;
                        }
                    }
                    (kw, exists)
                }
            })
            .collect();

        let miss_results: Vec<(String, bool)> = futures::future::join_all(miss_futures).await;

        // Step 3: Write results to cache (single lock acquisition).
        {
            let mut cache = self.keyword_validation_cache.write().await;
            for (kw, exists) in &miss_results {
                if cache.len() < 10000 {
                    cache.insert(kw.to_lowercase(), *exists);
                }
            }
        }

        // Step 4: Build validated list preserving original keyword order.
        let mut validated_low_level = Vec::new();
        let mut dropped_keywords = Vec::new();
        for kw in &keywords.low_level {
            let exists = hit_results
                .get(kw)
                .copied()
                .or_else(|| miss_results.iter().find(|(k, _)| k == kw).map(|(_, e)| *e))
                .unwrap_or(false);
            if exists {
                validated_low_level.push(kw.clone());
            } else {
                dropped_keywords.push(kw.clone());
            }
        }

        if !dropped_keywords.is_empty() {
            tracing::info!(
                dropped = ?dropped_keywords,
                kept = ?validated_low_level,
                "Dropped keywords with no graph matches"
            );
        }

        // If ALL keywords were dropped, fall back to original to avoid empty search.
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
}

#[cfg(test)]
mod tests {
    use super::keyword_graph_probes;

    #[test]
    fn spaced_entity_keyword_probes_the_stored_node_id() {
        assert_eq!(
            keyword_graph_probes("intrathecal chemotherapy")[0],
            "INTRATHECAL_CHEMOTHERAPY"
        );
        assert_eq!(
            keyword_graph_probes("cerebrospinal fluid (CSF)")[0],
            "CEREBROSPINAL_FLUID_(CSF)"
        );
        assert_eq!(keyword_graph_probes("bladder cancer")[0], "BLADDER_CANCER");
        assert_eq!(keyword_graph_probes("ALL"), vec!["ALL".to_string()]);
    }
}
