//! Shared query-engine and ingestion pipeline construction (SPEC-017 API-DRY-003).
//!
//! DRY helper used by both memory and PostgreSQL `AppState` bootstraps.

use std::sync::Arc;

use edgequake_llm::traits::{EmbeddingProvider, LLMProvider};
use edgequake_pipeline::{LLMExtractor, Pipeline};
use edgequake_query::{QueryEngine, QueryEngineConfig, SOTAQueryConfig, SOTAQueryEngine};
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

/// Build legacy + SOTA query engines with BM25 reranker (production bootstrap path).
pub fn build_production_query_engines(
    vector_storage: Arc<dyn VectorStorage>,
    graph_storage: Arc<dyn GraphStorage>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    llm_provider: Arc<dyn LLMProvider>,
    reranker: Arc<dyn edgequake_llm::Reranker>,
) -> (Arc<QueryEngine>, Arc<SOTAQueryEngine>) {
    let query_engine = Arc::new(QueryEngine::new(
        QueryEngineConfig::default(),
        Arc::clone(&vector_storage),
        Arc::clone(&graph_storage),
        Arc::clone(&embedding_provider),
        Arc::clone(&llm_provider),
    ));
    let sota_engine = Arc::new(
        SOTAQueryEngine::new(
            SOTAQueryConfig::default(),
            vector_storage,
            graph_storage,
            embedding_provider,
            llm_provider,
        )
        .with_reranker(reranker),
    );
    (query_engine, sota_engine)
}
