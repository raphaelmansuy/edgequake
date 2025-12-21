//! Application state.

use std::sync::Arc;

use edgequake_llm::OpenAIProvider;
use edgequake_pipeline::Pipeline;
use edgequake_query::{QueryEngine, QueryEngineConfig};
use edgequake_storage::adapters::memory::{MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage};

/// Type alias for the query engine with memory storage.
pub type MemoryQueryEngine = QueryEngine<MemoryVectorStorage, MemoryGraphStorage, OpenAIProvider, OpenAIProvider>;

/// Application state shared across handlers.
#[derive(Clone)]
pub struct AppState {
    /// KV storage.
    pub kv_storage: Arc<MemoryKVStorage>,

    /// Vector storage.
    pub vector_storage: Arc<MemoryVectorStorage>,

    /// Graph storage.
    pub graph_storage: Arc<MemoryGraphStorage>,

    /// LLM provider.
    pub llm_provider: Arc<OpenAIProvider>,

    /// Query engine.
    pub query_engine: Arc<MemoryQueryEngine>,

    /// Processing pipeline.
    pub pipeline: Arc<Pipeline>,

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
    /// Create a new application state with memory storage.
    pub fn new_memory(llm_api_key: impl Into<String>) -> Self {
        let kv_storage = Arc::new(MemoryKVStorage::new("default"));
        let vector_storage = Arc::new(MemoryVectorStorage::new("default", 1536));
        let graph_storage = Arc::new(MemoryGraphStorage::new("default"));
        let llm_provider = Arc::new(OpenAIProvider::new(llm_api_key));
        let pipeline = Arc::new(Pipeline::default_pipeline());

        // Create query engine
        let query_engine = Arc::new(QueryEngine::new(
            QueryEngineConfig::default(),
            Arc::clone(&vector_storage),
            Arc::clone(&graph_storage),
            Arc::clone(&llm_provider),
            Arc::clone(&llm_provider),
        ));

        Self {
            kv_storage,
            vector_storage,
            graph_storage,
            llm_provider,
            query_engine,
            pipeline,
            config: AppConfig::default(),
        }
    }

    /// Create a minimal state for testing.
    pub fn test_state() -> Self {
        Self::new_memory("test-key")
    }
}
