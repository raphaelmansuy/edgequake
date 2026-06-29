//! Mix query mode — weighted or RRF blend of local, global, and naive arms.

use std::collections::HashMap;
use std::sync::Arc;

use crate::context::{QueryContext, RetrievedChunk};
use crate::error::Result;
use crate::keywords::ExtractedKeywords;

use edgequake_storage::traits::VectorStorage;

use super::super::{QueryEmbeddings, QueryEngine};

impl QueryEngine {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::engine_impl) async fn query_mix_with_vector_storage(
        &self,
        query_text: &str,
        keywords: &ExtractedKeywords,
        embeddings: &QueryEmbeddings,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
        // Document IDs to restrict vector search to (SPEC-031 Tier 1 pre-filter).
        allowed_document_ids: Option<&[String]>,
        vector_storage: &Arc<dyn VectorStorage>,
        mix_weights: Option<&crate::mix_weights::MixWeightOverride>,
        max_chunks: usize,
    ) -> Result<QueryContext> {
        let (local_context, global_context, naive_context) = tokio::join!(
            self.query_local_with_vector_storage(
                query_text,
                keywords,
                embeddings,
                tenant_id.clone(),
                workspace_id.clone(),
                allowed_document_ids,
                vector_storage,
                max_chunks,
            ),
            self.query_global_with_vector_storage(
                query_text,
                keywords,
                embeddings,
                tenant_id.clone(),
                workspace_id.clone(),
                allowed_document_ids,
                vector_storage,
                max_chunks,
            ),
            self.query_naive_with_vector_storage(
                query_text,
                embeddings,
                tenant_id,
                workspace_id,
                allowed_document_ids,
                vector_storage,
                max_chunks,
            ),
        );

        let local_context = local_context?;
        let global_context = global_context?;
        let naive_context = naive_context?;

        let (w_local, w_global, w_naive) =
            crate::mix_weights::normalized_mix_weights(&self.config, mix_weights);

        let mut merged = QueryContext::new();

        if crate::fusion::mix_fusion_mode_from_env() == crate::fusion::MixFusionMode::Rrf {
            let mut chunk_lookup: HashMap<String, RetrievedChunk> = HashMap::new();
            for ctx in [&local_context, &global_context, &naive_context] {
                for chunk in &ctx.chunks {
                    chunk_lookup
                        .entry(chunk.id.clone())
                        .or_insert_with(|| chunk.clone());
                }
            }

            let ranked_lists = [
                local_context.chunks.iter().map(|c| c.id.clone()).collect(),
                global_context.chunks.iter().map(|c| c.id.clone()).collect(),
                naive_context.chunks.iter().map(|c| c.id.clone()).collect(),
            ];
            let weights = [w_local, w_global, w_naive];
            let fused = crate::fusion::reciprocal_rank_fusion(
                &ranked_lists,
                &weights,
                crate::fusion::RRF_K,
            );
            for chunk in crate::fusion::chunks_from_rrf_ranking(&fused, &chunk_lookup, max_chunks) {
                merged.add_chunk(chunk);
            }
        } else {
            let mut blended: HashMap<String, (RetrievedChunk, f32)> = HashMap::new();
            for (ctx, weight) in [
                (&local_context, w_local),
                (&global_context, w_global),
                (&naive_context, w_naive),
            ] {
                let norm = min_max_normalize_scores(&ctx.chunks);
                for (chunk, &norm_score) in ctx.chunks.iter().zip(norm.iter()) {
                    let contribution = weight * norm_score;
                    blended
                        .entry(chunk.id.clone())
                        .and_modify(|(_, score)| {
                            if contribution > *score {
                                *score = contribution;
                            }
                        })
                        .or_insert_with(|| (chunk.clone(), contribution));
                }
            }

            let mut chunks: Vec<(RetrievedChunk, f32)> = blended.into_values().collect();
            chunks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            for (mut chunk, score) in chunks.into_iter().take(max_chunks) {
                chunk.score = score.max(0.0);
                merged.add_chunk(chunk);
            }
        }

        let mut seen_entities = std::collections::HashSet::new();
        for e in local_context
            .entities
            .iter()
            .chain(global_context.entities.iter())
        {
            if seen_entities.insert(e.name.clone()) {
                merged.add_entity(e.clone());
            }
        }
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
            w_local,
            w_global,
            w_naive,
            "Mix merge complete (weighted blend)"
        );

        Ok(merged)
    }
}

fn min_max_normalize_scores(chunks: &[crate::context::RetrievedChunk]) -> Vec<f32> {
    if chunks.is_empty() {
        return Vec::new();
    }
    let (min, max) = chunks
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(mn, mx), c| {
            (mn.min(c.score), mx.max(c.score))
        });
    let range = max - min;
    chunks
        .iter()
        .map(|c| {
            if range <= 0.0 {
                1.0
            } else {
                (c.score - min) / range
            }
        })
        .collect()
}
