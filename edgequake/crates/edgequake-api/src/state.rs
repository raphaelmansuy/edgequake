//! Application state.

use std::sync::Arc;

use edgequake_llm::OpenAIProvider;
use edgequake_pipeline::Pipeline;
use edgequake_storage::adapters::memory::{MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage};

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

        Self {
            kv_storage,
            vector_storage,
            graph_storage,
            llm_provider,
            pipeline,
            config: AppConfig::default(),
        }
    }

    /// Create a minimal state for testing.
    pub fn test_state() -> Self {
        Self::new_memory("test-key")
    }
}
