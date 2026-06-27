//! Shared query-engine and ingestion pipeline construction (SPEC-017 API-DRY-003).
//!
//! DRY helper used by both memory and PostgreSQL `AppState` bootstraps.

use std::sync::Arc;

use edgequake_llm::traits::{EmbeddingProvider, LLMProvider};
use edgequake_pipeline::{LLMExtractor, Pipeline};
use edgequake_query::QueryEngine;

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

/// Build the production query engine (reranker resolved inside query bootstrap).
pub fn build_production_query_engine(
    vector_storage: Arc<dyn VectorStorage>,
    graph_storage: Arc<dyn GraphStorage>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    llm_provider: Arc<dyn LLMProvider>,
    kv_storage: Arc<dyn edgequake_storage::traits::KVStorage>,
) -> Arc<QueryEngine> {
    edgequake_query::build_production_query_engine(
        vector_storage,
        graph_storage,
        embedding_provider,
        llm_provider,
        Some(kv_storage),
    )
}
