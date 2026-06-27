//! Production query-engine bootstrap (SPEC-022 P-H4 / DRY SSOT).
//!
//! Shared by `edgequake-api` HTTP bootstrap and `edgequake-core` orchestrator so
//! SDK and API query quality stay identical. Reranker resolution delegates to
//! `edgequake-llm` factory (SPEC-024 2.4).

use std::sync::Arc;

use edgequake_llm::traits::{EmbeddingProvider, LLMProvider};
use edgequake_llm::Reranker;

use edgequake_storage::traits::{GraphStorage, KVStorage, VectorStorage};

use crate::{QueryEngine, QueryEngineConfig};

/// Create the configured reranker for production (BM25 by default).
///
/// Set `EDGEQUAKE_RERANKER=cross_encoder` for neural reranking — see
/// [`edgequake_llm::create_production_reranker`].
pub fn create_production_reranker() -> Arc<dyn Reranker> {
    edgequake_llm::create_production_reranker(None)
}

/// Same as [`create_production_reranker`] but passes embedding provider for bi-encoder fallback.
pub fn create_production_reranker_with_embedding(
    embedding: Option<Arc<dyn EmbeddingProvider>>,
) -> Arc<dyn Reranker> {
    edgequake_llm::create_production_reranker(embedding)
}

/// Build the production query engine: reranker + embedding cache + result cache.
pub fn build_production_query_engine(
    vector_storage: Arc<dyn VectorStorage>,
    graph_storage: Arc<dyn GraphStorage>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    llm_provider: Arc<dyn LLMProvider>,
    kv_storage: Option<Arc<dyn KVStorage>>,
) -> Arc<QueryEngine> {
    let reranker = create_production_reranker_with_embedding(Some(Arc::clone(&embedding_provider)));
    let mut engine = QueryEngine::new(
        QueryEngineConfig::default(),
        vector_storage,
        graph_storage,
        embedding_provider,
        llm_provider,
    )
    .with_reranker(reranker)
    .with_embedding_cache()
    .with_result_cache();

    if let Some(kv) = kv_storage {
        engine = engine.with_kv_storage(kv);
    }

    Arc::new(engine)
}
