use crate::keywords::ExtractedKeywords;

use super::SOTAQueryEngine;

// WHY: validate_keywords makes N graph search calls for N keywords.
// Using parallel execution eliminates the N×RTT sequential latency.

impl SOTAQueryEngine {
    pub(super) async fn rerank_chunks(
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
    pub(super) fn sort_entities_by_degree(&self, entities: &mut [crate::context::RetrievedEntity]) {
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
    ///
    /// WHY parallel: Each `search_labels` call is an independent DB round-trip.
    /// Running them sequentially paid N×RTT; `join_all` pays max(RTT) instead.
    /// Cache hits are separated first to avoid holding the lock during IO.
    pub(super) async fn validate_keywords(
        &self,
        keywords: &ExtractedKeywords,
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
        let miss_futures: Vec<_> = miss_keywords
            .iter()
            .map(|kw| {
                let storage = self.graph_storage.clone();
                let kw = kw.clone();
                async move {
                    let exists = storage
                        .search_labels(&kw, 1)
                        .await
                        .map(|labels| !labels.is_empty())
                        .unwrap_or(false);
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
