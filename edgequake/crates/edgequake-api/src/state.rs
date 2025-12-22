//! Application state.

use std::sync::Arc;

use edgequake_llm::OpenAIProvider;
use edgequake_pipeline::Pipeline;
use edgequake_query::{QueryEngine, QueryEngineConfig};
use edgequake_storage::adapters::memory::{
    MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage,
};
use edgequake_tasks::{SharedTaskQueue, SharedTaskStorage};

/// Application state shared across handlers.
#[derive(Clone)]
pub struct AppState {
    /// KV storage.
    pub kv_storage: Arc<dyn edgequake_storage::traits::KVStorage>,

    /// Vector storage.
    pub vector_storage: Arc<dyn edgequake_storage::traits::VectorStorage>,

    /// Graph storage.
    pub graph_storage: Arc<dyn edgequake_storage::traits::GraphStorage>,

    /// LLM provider.
    pub llm_provider: Arc<dyn edgequake_llm::traits::LLMProvider>,

    /// Embedding provider.
    pub embedding_provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider>,

    /// Query engine.
    pub query_engine: Arc<QueryEngine>,

    /// Processing pipeline.
    pub pipeline: Arc<Pipeline>,

    /// Task storage.
    pub task_storage: SharedTaskStorage,

    /// Task queue.
    pub task_queue: SharedTaskQueue,

    /// Configuration.
    pub config: AppConfig,
}

/// Application configuration.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Workspace/tenant ID.
    pub workspace_id: String,

    /// Maximum document size in bytes.
    pub max_document_size: usize,

    /// Maximum query length.
    pub max_query_length: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            workspace_id: "default".to_string(),
            max_document_size: 10 * 1024 * 1024, // 10 MB
            max_query_length: 10000,
        }
    }
}

impl AppState {
    /// Create a new application state.
    pub fn new(
        kv_storage: Arc<dyn edgequake_storage::traits::KVStorage>,
        vector_storage: Arc<dyn edgequake_storage::traits::VectorStorage>,
        graph_storage: Arc<dyn edgequake_storage::traits::GraphStorage>,
        llm_provider: Arc<dyn edgequake_llm::traits::LLMProvider>,
        embedding_provider: Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
        query_engine: Arc<QueryEngine>,
        pipeline: Arc<Pipeline>,
        task_storage: SharedTaskStorage,
        task_queue: SharedTaskQueue,
    ) -> Self {
        Self {
            kv_storage,
            vector_storage,
            graph_storage,
            llm_provider,
            embedding_provider,
            query_engine,
            pipeline,
            task_storage,
            task_queue,
            config: AppConfig::default(),
        }
    }

    /// Create a new application state with memory storage.
    pub fn new_memory(llm_api_key: impl Into<String>) -> Self {
        let kv_storage = Arc::new(MemoryKVStorage::new("default"));
        let vector_storage = Arc::new(MemoryVectorStorage::new("default", 1536));
        let graph_storage = Arc::new(MemoryGraphStorage::new("default"));
        let llm_provider = Arc::new(OpenAIProvider::new(llm_api_key));
        let pipeline = Arc::new(Pipeline::default_pipeline());

        // Create task infrastructure
        let task_storage = Arc::new(edgequake_tasks::memory::MemoryTaskStorage::new());
        let task_queue = Arc::new(edgequake_tasks::queue::ChannelTaskQueue::new(100));

        // Create query engine
        let query_engine = Arc::new(QueryEngine::new(
            QueryEngineConfig::default(),
            Arc::clone(&vector_storage) as Arc<dyn edgequake_storage::traits::VectorStorage>,
            Arc::clone(&graph_storage) as Arc<dyn edgequake_storage::traits::GraphStorage>,
            Arc::clone(&llm_provider) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
            Arc::clone(&llm_provider) as Arc<dyn edgequake_llm::traits::LLMProvider>,
        ));

        Self {
            kv_storage: Arc::clone(&kv_storage) as Arc<dyn edgequake_storage::traits::KVStorage>,
            vector_storage: Arc::clone(&vector_storage)
                as Arc<dyn edgequake_storage::traits::VectorStorage>,
            graph_storage: Arc::clone(&graph_storage)
                as Arc<dyn edgequake_storage::traits::GraphStorage>,
            llm_provider: Arc::clone(&llm_provider) as Arc<dyn edgequake_llm::traits::LLMProvider>,
            embedding_provider: Arc::clone(&llm_provider)
                as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
            query_engine,
            pipeline,
            task_storage,
            task_queue,
            config: AppConfig::default(),
        }
    }

    /// Create a minimal state for testing.
    pub fn test_state() -> Self {
        use edgequake_llm::MockProvider;

        let mock_provider = Arc::new(MockProvider::new());
        let kv_storage = Arc::new(MemoryKVStorage::new("test"));
        let vector_storage = Arc::new(MemoryVectorStorage::new("test", 1536)); // Match MockProvider dimension
        let graph_storage = Arc::new(MemoryGraphStorage::new("test"));
        let pipeline = Arc::new(Pipeline::default_pipeline());

        // Create task infrastructure
        let task_storage = Arc::new(edgequake_tasks::memory::MemoryTaskStorage::new());
        let task_queue = Arc::new(edgequake_tasks::queue::ChannelTaskQueue::new(100));

        let query_config = QueryEngineConfig::default();
        let query_engine = Arc::new(QueryEngine::new(
            query_config,
            Arc::clone(&vector_storage) as Arc<dyn edgequake_storage::traits::VectorStorage>,
            Arc::clone(&graph_storage) as Arc<dyn edgequake_storage::traits::GraphStorage>,
            Arc::clone(&mock_provider) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
            Arc::clone(&mock_provider) as Arc<dyn edgequake_llm::traits::LLMProvider>,
        ));

        Self {
            kv_storage: Arc::clone(&kv_storage) as Arc<dyn edgequake_storage::traits::KVStorage>,
            vector_storage: Arc::clone(&vector_storage)
                as Arc<dyn edgequake_storage::traits::VectorStorage>,
            graph_storage: Arc::clone(&graph_storage)
                as Arc<dyn edgequake_storage::traits::GraphStorage>,
            llm_provider: Arc::clone(&mock_provider) as Arc<dyn edgequake_llm::traits::LLMProvider>,
            embedding_provider: Arc::clone(&mock_provider)
                as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
            query_engine,
            pipeline,
            task_storage,
            task_queue,
            config: AppConfig::default(),
        }
    }
}
