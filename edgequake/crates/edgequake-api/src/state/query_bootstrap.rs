//! Shared query-engine and ingestion pipeline construction (SPEC-017 API-DRY-003).
//!
//! DRY helper used by both memory and PostgreSQL `AppState` bootstraps.

use std::sync::Arc;

use edgequake_llm::traits::{EmbeddingProvider, LLMProvider};
use edgequake_pipeline::{LLMExtractor, Pipeline};
use edgequake_query::{QueryEngine, QueryEngineConfig};
use edgequake_storage::traits::{GraphStorage, VectorStorage};

/// Build the default ingestion pipeline with workspace-configurable providers.
pub fn build_ingestion_pipeline(
    llm_provider: Arc<dyn LLMProvider>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
) -> Arc<Pipeline> {
    let extractor = Arc::new(LLMExtractor::new(Arc::clone(&llm_provider)));
    Arc::new(
        Pipeline::default_pipeline()
            .with_extractor(extractor)
            .with_embedding_provider(embedding_provider),
    )
}

/// Build the production query engine with BM25 reranker.
///
/// P-G6a (RC-11): returns only the SOTA engine — the legacy `QueryEngine`
/// was dead (no handler read it) and is deleted. There is now exactly one
/// query engine implementation in the crate.
pub fn build_production_query_engine(
    vector_storage: Arc<dyn VectorStorage>,
    graph_storage: Arc<dyn GraphStorage>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    llm_provider: Arc<dyn LLMProvider>,
    reranker: Arc<dyn edgequake_llm::Reranker>,
) -> Arc<QueryEngine> {
    Arc::new(
        QueryEngine::new(
            QueryEngineConfig::default(),
            vector_storage,
            graph_storage,
            embedding_provider,
            llm_provider,
        )
        .with_reranker(reranker)
        // P-G9 (RC-14): memoize query embeddings to skip redundant embedding
        // round-trips for repeated queries. Ingestion `embed` (batch) is
        // delegated unchanged, so this is query-path only.
        .with_embedding_cache()
        // P-G9 result half: cache context_only retrieval contexts.
        .with_result_cache(),
    )
}
