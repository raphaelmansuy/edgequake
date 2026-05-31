//! Query engines and LLM provider runtime bundle (SPEC-017 P1-04).

use std::sync::Arc;

use edgequake_llm::ModelsConfig;
use edgequake_pipeline::Pipeline;
use edgequake_query::{QueryEngine, SOTAQueryEngine};

/// SOTA/legacy query engines, ingestion pipeline, and default providers.
#[derive(Clone)]
pub struct QueryRuntime {
    pub llm_provider: Arc<dyn edgequake_llm::traits::LLMProvider>,
    pub vision_llm_provider: Option<Arc<dyn edgequake_llm::traits::LLMProvider>>,
    pub embedding_provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
    pub query_engine: Arc<QueryEngine>,
    pub sota_engine: Arc<SOTAQueryEngine>,
    pub pipeline: Arc<Pipeline>,
    pub models_config: Arc<ModelsConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_llm::MockProvider;
    use edgequake_query::{QueryEngineConfig, SOTAQueryConfig, SOTAQueryEngine};
    use edgequake_storage::adapters::memory::{MemoryGraphStorage, MemoryVectorStorage};

    #[test]
    fn query_runtime_wires_engines() {
        let mock = Arc::new(MockProvider::new());
        let vector = Arc::new(MemoryVectorStorage::new("test", 1536));
        let graph = Arc::new(MemoryGraphStorage::new("test"));

        let query_engine = Arc::new(QueryEngine::new(
            QueryEngineConfig::default(),
            Arc::clone(&vector) as Arc<dyn edgequake_storage::traits::VectorStorage>,
            Arc::clone(&graph) as Arc<dyn edgequake_storage::traits::GraphStorage>,
            Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
            Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::LLMProvider>,
        ));
        let sota_engine = Arc::new(SOTAQueryEngine::with_mock_keywords(
            SOTAQueryConfig::default(),
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
            query_engine,
            sota_engine,
            pipeline: Arc::new(Pipeline::default_pipeline()),
            models_config: Arc::new(ModelsConfig::builtin_defaults()),
        };

        assert_eq!(runtime.embedding_provider.dimension(), 1536);
    }
}
