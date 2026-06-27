//! BM25 sparse retrieval fused with dense vector ranks (SPEC-023 I10 / default-on).
//!
//! Production path: PostgreSQL native FTS (`ts_rank_cd` over GIN `content_tsv`).
//! Fallback: in-memory BM25 reranker over vector ANN candidates (memory adapter / tests).
//!
//! Used by naive, local, and global chunk stages (SPEC-024 2.3).

use std::collections::HashMap;
use std::sync::Arc;

use edgequake_llm::Reranker;
use edgequake_storage::traits::{KVStorage, MetadataFilter, VectorSearchResult, VectorStorage};

use crate::chunk_hydration::chunk_documents_for_rerank;
use crate::context::RetrievedChunk;
use crate::engine_impl::QueryEngineConfig;
use crate::fusion::{self, MixFusionMode};
use crate::helpers::build_chunk_from_result;

/// Whether vector+sparse chunk fusion uses RRF (default: weighted sparse-first).
///
/// Mix mode has its own [`fusion::mix_fusion_mode_from_env`]; this controls only
/// naive/local/global chunk-stage fusion (SPEC-024 2.3).
pub fn sparse_fusion_mode_from_env() -> MixFusionMode {
    match std::env::var("EDGEQUAKE_SPARSE_FUSION")
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "rrf" => MixFusionMode::Rrf,
        _ => MixFusionMode::Weighted,
    }
}

/// Whether BM25 retrieval fusion is active (default: true).
///
/// Set `EDGEQUAKE_BM25_RETRIEVAL=false` to restore vector-only naive ordering.
pub fn bm25_retrieval_enabled(config: &QueryEngineConfig) -> bool {
    if !config.enable_bm25_retrieval {
        return false;
    }
    std::env::var("EDGEQUAKE_BM25_RETRIEVAL")
        .map(|v| !matches!(v.to_ascii_lowercase().as_str(), "false" | "0" | "off"))
        .unwrap_or(true)
}

/// Fuse vector ANN hits with sparse retrieval; returns top `max_chunks`.
pub async fn fuse_vector_and_bm25_chunks(
    query: &str,
    vector_results: &[VectorSearchResult],
    vector_storage: &Arc<dyn VectorStorage>,
    metadata_filter: Option<&MetadataFilter>,
    reranker: Option<&dyn Reranker>,
    kv_storage: Option<&dyn KVStorage>,
    config: &QueryEngineConfig,
) -> Vec<RetrievedChunk> {
    let max_chunks = config.max_chunks;
    let min_score = config.min_score;

    if vector_results.is_empty() {
        return Vec::new();
    }

    if !bm25_retrieval_enabled(config) {
        return vector_results
            .iter()
            .filter(|r| r.score >= min_score)
            .take(max_chunks)
            .map(build_chunk_from_result)
            .collect();
    }

    let vector_ranked: Vec<String> = vector_results.iter().map(|r| r.id.clone()).collect();
    let mut lookup = chunk_lookup(vector_results);

    let sparse_ranked = if vector_storage.supports_native_text_search() {
        let candidate_k = max_chunks.saturating_mul(config.bm25_candidate_multiplier);
        match vector_storage
            .text_search_filtered(query, candidate_k, None, metadata_filter)
            .await
        {
            Ok(fts_hits) if !fts_hits.is_empty() => {
                for hit in &fts_hits {
                    lookup
                        .entry(hit.id.clone())
                        .or_insert_with(|| build_chunk_from_result(hit));
                }
                fts_hits.iter().map(|r| r.id.clone()).collect()
            }
            Ok(_) => Vec::new(),
            Err(e) => {
                tracing::warn!(error = %e, "Postgres FTS failed — falling back to in-memory BM25");
                in_memory_bm25_ranked(query, vector_results, reranker, kv_storage).await
            }
        }
    } else {
        in_memory_bm25_ranked(query, vector_results, reranker, kv_storage).await
    };

    if sparse_ranked.is_empty() {
        return vector_results
            .iter()
            .filter(|r| r.score >= min_score)
            .take(max_chunks)
            .map(build_chunk_from_result)
            .collect();
    }

    if sparse_fusion_mode_from_env() == MixFusionMode::Rrf {
        let fused = fusion::reciprocal_rank_fusion(
            &[vector_ranked, sparse_ranked],
            &[1.0, 1.25],
            fusion::RRF_K,
        );
        return fusion::chunks_from_rrf_ranking(&fused, &lookup, max_chunks);
    }

    // Weighted fallback: prefer sparse order when available.
    sparse_ranked
        .into_iter()
        .filter_map(|id| lookup.get(&id).cloned())
        .take(max_chunks)
        .collect()
}

fn chunk_lookup(vector_results: &[VectorSearchResult]) -> HashMap<String, RetrievedChunk> {
    vector_results
        .iter()
        .map(|r| (r.id.clone(), build_chunk_from_result(r)))
        .collect()
}

async fn in_memory_bm25_ranked(
    query: &str,
    vector_results: &[VectorSearchResult],
    reranker: Option<&dyn Reranker>,
    kv_storage: Option<&dyn KVStorage>,
) -> Vec<String> {
    let Some(reranker) = reranker else {
        return vector_results.iter().map(|r| r.id.clone()).collect();
    };

    let documents = chunk_documents_for_rerank(kv_storage, vector_results).await;

    match reranker
        .rerank(query, &documents, Some(documents.len()))
        .await
    {
        Ok(ranked) => ranked
            .into_iter()
            .filter_map(|r| vector_results.get(r.index).map(|vr| vr.id.clone()))
            .collect(),
        Err(_) => vector_results.iter().map(|r| r.id.clone()).collect(),
    }
}
