//! Production query-engine bootstrap (SPEC-022 P-H4 / DRY SSOT).
//!
//! Shared by `edgequake-api` HTTP bootstrap and `edgequake-core` orchestrator so
//! SDK and API query quality stay identical (BM25 + caches).

use std::sync::Arc;

use edgequake_llm::traits::{EmbeddingProvider, LLMProvider};
use edgequake_llm::Reranker;
use edgequake_storage::traits::{GraphStorage, VectorStorage};

use crate::{QueryEngine, QueryEngineConfig};

/// Create the configured reranker for production (BM25 by default).
///
/// Set `EDGEQUAKE_RERANKER=cross_encoder` to opt into neural reranking when an
/// implementation is available (SPEC-023 I4). Until then, falls back to BM25.
pub fn create_production_reranker() -> Arc<dyn Reranker> {
    match std::env::var("EDGEQUAKE_RERANKER")
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "cross_encoder" => {
            tracing::warn!(
                "EDGEQUAKE_RERANKER=cross_encoder requested but not yet implemented; using BM25"
            );
            create_bm25_reranker()
        }
        _ => create_bm25_reranker(),
    }
}

fn create_bm25_reranker() -> Arc<dyn Reranker> {
    if std::env::var("BM25_ENHANCED").unwrap_or_default() == "false" {
        tracing::info!("Using minimal BM25 reranker (BM25_ENHANCED=false)");
        Arc::new(edgequake_llm::reranker::BM25Reranker::new())
    } else {
        tracing::info!("Using enhanced BM25 reranker with stemming and Unicode normalization");
        Arc::new(edgequake_llm::reranker::BM25Reranker::new_enhanced())
    }
}

/// Build the production query engine: BM25 reranker + embedding cache + result cache.
pub fn build_production_query_engine(
    vector_storage: Arc<dyn VectorStorage>,
    graph_storage: Arc<dyn GraphStorage>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    llm_provider: Arc<dyn LLMProvider>,
) -> Arc<QueryEngine> {
    Arc::new(
        QueryEngine::new(
            QueryEngineConfig::default(),
            vector_storage,
            graph_storage,
            embedding_provider,
            llm_provider,
        )
        .with_reranker(create_production_reranker())
        .with_embedding_cache()
        .with_result_cache(),
    )
}
