//! Query engines and LLM provider runtime bundle (SPEC-017 P1-04).

use std::sync::Arc;

use edgequake_llm::ModelsConfig;
use edgequake_pipeline::Pipeline;
use edgequake_query::QueryEngine;

/// SOTA query engine, ingestion pipeline, and default providers.
///
/// P-G6a (RC-11): the dead legacy `QueryEngine` field was removed. Production
/// routes exclusively through `QueryEngine`; no handler read the legacy
/// engine, so carrying it was pure overhead and a misleading "two engines"
/// affordance.
#[derive(Clone)]
pub struct QueryRuntime {
    pub llm_provider: Arc<dyn edgequake_llm::traits::LLMProvider>,
    pub vision_llm_provider: Option<Arc<dyn edgequake_llm::traits::LLMProvider>>,
    pub embedding_provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
    pub engine_impl: Arc<QueryEngine>,
    pub pipeline: Arc<Pipeline>,
    pub models_config: Arc<ModelsConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_llm::MockProvider;
    use edgequake_query::{QueryEngine, QueryEngineConfig};
    use edgequake_storage::adapters::memory::{MemoryGraphStorage, MemoryVectorStorage};

    #[test]
    fn query_runtime_wires_engines() {
        let mock = Arc::new(MockProvider::new());
        let vector = Arc::new(MemoryVectorStorage::new("test", 1536));
        let graph = Arc::new(MemoryGraphStorage::new("test"));

        let engine_impl = Arc::new(QueryEngine::with_mock_keywords(
            QueryEngineConfig::default(),
            Arc::clone(&vector) as Arc<dyn edgequake_storage::traits::VectorStorage>,
            Arc::clone(&graph) as Arc<dyn edgequake_storage::traits::GraphStorage>,
            Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
            Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::LLMProvider>,
        ));

        let runtime = QueryRuntime {
            llm_provider: Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::LLMProvider>,
            vision_llm_provider: None,
            embedding_provider: Arc::clone(&mock)
                as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
            engine_impl,
            pipeline: Arc::new(Pipeline::default_pipeline()),
            models_config: Arc::new(ModelsConfig::builtin_defaults()),
        };

        assert_eq!(runtime.embedding_provider.dimension(), 1536);
    }
}
