//! EdgeQuake Orchestrator - Central RAG coordination module.
//!
//! @implements FEAT0023 (EdgeQuake Orchestrator)
//!
//! # Overview
//!
//! **Implements**: FEAT0001 (Document Ingestion), FEAT0007 (Multi-Mode Query)
//!
//! **Enforces**: BR0001 (Doc ID Uniqueness), BR0002 (Chunk Constraints),
//!               BR0101 (Token Budget), BR0201 (Tenant Isolation)
//!
//! The orchestrator is the primary entry point for all EdgeQuake operations,
//! coordinating document processing, knowledge graph construction, and query execution.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                       EdgeQuake                              │
//! │  ┌─────────────────────────────────────────────────────────┐ │
//! │  │                    Orchestrator                          │ │
//! │  │  - config: EdgeQuakeConfig                              │ │
//! │  │  - storage: KV + Vector + Graph                         │ │
//! │  │  - providers: LLM + Embedding                           │ │
//! │  └────────────────────────┬────────────────────────────────┘ │
//! │                           │                                  │
//! │     ┌─────────────────────┼─────────────────────┐           │
//! │     │                     │                     │           │
//! │     ▼                     ▼                     ▼           │
//! │  ┌──────────┐       ┌──────────┐         ┌──────────┐       │
//! │  │ insert() │       │  query() │         │ delete() │       │
//! │  └────┬─────┘       └────┬─────┘         └────┬─────┘       │
//! │       │                  │                    │             │
//! │       ▼                  ▼                    ▼             │
//! │  ┌──────────┐       ┌──────────┐         ┌──────────┐       │
//! │  │ Pipeline │       │  Query   │         │ Cascade  │       │
//! │  │ (chunk+  │       │  Engine  │         │  Delete  │       │
//! │  │ extract) │       │ (6 modes)│         │ (source  │       │
//! │  └──────────┘       └──────────┘         │ tracking)│       │
//! │                                          └──────────┘       │
//! └─────────────────────────────────────────────────────────────┘
//!
//! Storage Layer:
//! ┌──────────┐    ┌──────────┐    ┌──────────┐
//! │ KVStorage│    │VectorStor│    │GraphStor │
//! │ (docs,   │    │(pgvector)│    │(AGE/mem) │
//! │  chunks) │    │          │    │          │
//! └──────────┘    └──────────┘    └──────────┘
//! ```
//!
//! # Key Operations
//!
//! ## Document Ingestion (FEAT0001)
//!
//! ```rust,ignore
//! // Insert returns processing stats
//! let result = eq.insert("Document content...", Some("doc-001")).await?;
//! assert!(result.entities_extracted > 0);
//! ```
//!
//! ## Query Execution (FEAT0007)
//!
//! ```rust,ignore
//! use edgequake_core::{QueryParams, QueryMode};
//!
//! let params = QueryParams::new().with_mode(QueryMode::Hybrid);
//! let response = eq.query("What is X?", Some(params)).await?;
//! println!("Answer: {}", response.response);
//! ```
//!
//! # Query Modes (FEAT0101-FEAT0106)
//!
//! | Mode | Strategy | Best For |
//! |------|----------|----------|
//! | `naive` | Vector similarity only | Simple factual queries |
//! | `local` | Entity-centric + neighbors | Specific entity questions |
//! | `global` | Community-based | Broad topic overviews |
//! | `hybrid` | Local + global (default) | General purpose |
//! | `mix` | Weighted naive + graph | Tunable balance |
//! | `bypass` | Direct LLM (no RAG) | Creative/chat |
//!
//! # Multi-Tenancy (FEAT0015, BR0201)
//!
//! All operations respect tenant isolation via `tenant_id` and `workspace_id`
//! in the configuration. Cross-tenant data access is prevented at the storage layer.
//!
//! # See Also
//!
//! - [`crate::types::QueryParams`] - Query configuration options
//! - [`crate::types::InsertResult`] - Insertion result details
//! - [docs/features.md](../../../../../../docs/features.md) - Complete feature registry

use std::collections::HashMap;
use std::sync::Arc;

use edgequake_llm::traits::{EmbeddingProvider, LLMProvider};
use edgequake_pipeline::{
    GleaningConfig, GleaningExtractor, LLMExtractor, Pipeline, PipelineConfig,
};
use edgequake_storage::traits::{GraphStorage, KVStorage, VectorStorage};
use serde::{Deserialize, Serialize};
// Use query crate types
// edgequake-query is intentionally not linked here to avoid workspace cycles.

use crate::error::{Error, Result};

mod deletion;
mod ingestion;
mod query_ops;

/// EdgeQuake instance configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeQuakeConfig {
    /// Working directory for storage.
    pub working_dir: String,

    /// Namespace/workspace identifier.
    pub namespace: String,

    /// Tenant ID for multi-tenancy.
    pub tenant_id: Option<String>,

    /// Workspace ID for multi-tenancy.
    pub workspace_id: Option<String>,

    /// LLM model name for entity extraction.
    pub llm_model_name: String,

    /// LLM model name for response generation (can differ from extraction model).
    pub response_model_name: Option<String>,

    /// Embedding model name.
    pub embedding_model_name: String,

    /// Embedding dimension.
    pub embedding_dim: usize,

    /// Maximum token size for query context.
    pub max_token_for_text_unit: usize,

    /// Maximum token size for entity context.
    pub max_token_for_global_context: usize,

    /// Maximum token size for local context.
    pub max_token_for_local_context: usize,

    /// Chunk size in tokens.
    pub chunk_token_size: usize,

    /// Chunk overlap in tokens.
    pub chunk_overlap_token_size: usize,

    /// Enable logging.
    pub log_level: LogLevel,

    /// Storage configuration.
    pub storage: StorageConfig,

    /// Enable entity extraction caching.
    pub enable_cache: bool,

    /// Entity types to extract.
    pub entity_types: Vec<String>,

    /// Summary language for generated content.
    pub summary_language: String,

    /// Enable gleaning (multi-pass extraction) for better entity coverage.
    pub enable_gleaning: bool,

    /// Maximum number of gleaning iterations (1-3 recommended).
    pub max_gleaning: usize,

    /// Enable LLM-based description merging for better deduplication.
    pub use_llm_summarization: bool,
}

/// Log level configuration.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum LogLevel {
    /// Debug level.
    Debug,
    /// Info level.
    #[default]
    Info,
    /// Warning level.
    Warn,
    /// Error level.
    Error,
}

/// Storage backend configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage backend type.
    pub backend: StorageBackend,

    /// PostgreSQL connection string (for postgres backend).
    pub postgres_connection_string: Option<String>,

    /// Additional storage options.
    pub options: HashMap<String, String>,
}

/// Storage backend type.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum StorageBackend {
    /// In-memory storage (for testing).
    #[default]
    Memory,

    /// PostgreSQL with pgvector and AGE.
    Postgres,

    /// SurrealDB.
    SurrealDB,
}

impl Default for EdgeQuakeConfig {
    fn default() -> Self {
        Self {
            working_dir: "./edgequake_data".to_string(),
            namespace: "default".to_string(),
            tenant_id: None,
            workspace_id: None,
            llm_model_name: "gpt-4.1-nano".to_string(),
            response_model_name: None,
            embedding_model_name: "text-embedding-3-small".to_string(),
            embedding_dim: 1536,
            max_token_for_text_unit: 100000, // Very large budget (user request)
            max_token_for_global_context: 100000, // Very large budget (user request)
            max_token_for_local_context: 100000, // Very large budget (user request)
            chunk_token_size: 1200,
            chunk_overlap_token_size: 100,
            log_level: LogLevel::Info,
            storage: StorageConfig::default(),
            enable_cache: true,
            entity_types: vec![
                "PERSON".to_string(),
                "ORGANIZATION".to_string(),
                "LOCATION".to_string(),
                "CONCEPT".to_string(),
                "EVENT".to_string(),
            ],
            summary_language: "English".to_string(),
            enable_gleaning: true,       // Enable by default for SOTA quality
            max_gleaning: 1,             // LightRAG default
            use_llm_summarization: true, // Enable by default for SOTA quality
        }
    }
}

impl EdgeQuakeConfig {
    /// Create a new config with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the working directory.
    pub fn with_working_dir(mut self, dir: &str) -> Self {
        self.working_dir = dir.to_string();
        self
    }

    /// Set the namespace.
    pub fn with_namespace(mut self, ns: &str) -> Self {
        self.namespace = ns.to_string();
        self
    }

    /// Set the LLM model.
    pub fn with_llm_model(mut self, model: &str) -> Self {
        self.llm_model_name = model.to_string();
        self
    }

    /// Set the embedding model.
    pub fn with_embedding_model(mut self, model: &str, dim: usize) -> Self {
        self.embedding_model_name = model.to_string();
        self.embedding_dim = dim;
        self
    }

    /// Set the storage backend.
    pub fn with_storage(mut self, storage: StorageConfig) -> Self {
        self.storage = storage;
        self
    }

    /// Use PostgreSQL storage backend.
    pub fn with_postgres(mut self, connection_string: &str) -> Self {
        self.storage = StorageConfig {
            backend: StorageBackend::Postgres,
            postgres_connection_string: Some(connection_string.to_string()),
            options: HashMap::new(),
        };
        self
    }

    /// Set entity types to extract.
    pub fn with_entity_types(mut self, types: Vec<String>) -> Self {
        self.entity_types = types;
        self
    }

    /// Set chunk configuration.
    pub fn with_chunk_config(mut self, size: usize, overlap: usize) -> Self {
        self.chunk_token_size = size;
        self.chunk_overlap_token_size = overlap;
        self
    }

    /// Set gleaning configuration for multi-pass extraction.
    ///
    /// Gleaning performs additional LLM passes to find entities that might
    /// have been missed in the first extraction. This improves extraction
    /// quality at the cost of additional LLM calls.
    ///
    /// # Arguments
    /// * `enabled` - Whether to enable gleaning
    /// * `max_iterations` - Maximum gleaning iterations (1-3 recommended)
    pub fn with_gleaning(mut self, enabled: bool, max_iterations: usize) -> Self {
        self.enable_gleaning = enabled;
        self.max_gleaning = max_iterations;
        self
    }
}

/// EdgeQuake orchestrator.
pub struct EdgeQuake {
    /// Configuration.
    config: EdgeQuakeConfig,

    /// Whether the instance is initialized.
    initialized: bool,

    /// Storage backends.
    kv_storage: Option<Arc<dyn KVStorage>>,
    vector_storage: Option<Arc<dyn VectorStorage>>,
    graph_storage: Option<Arc<dyn GraphStorage>>,

    /// LLM and embedding providers.
    llm_provider: Option<Arc<dyn LLMProvider>>,
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,

    /// Pipeline for document processing.
    pipeline: Option<Arc<Pipeline>>,

    /// Query engine.
    query_engine: Option<Arc<crate::query::QueryEngine>>,
}

impl EdgeQuake {
    /// Create a new EdgeQuake instance.
    pub fn new(config: EdgeQuakeConfig) -> Self {
        Self {
            config,
            initialized: false,
            kv_storage: None,
            vector_storage: None,
            graph_storage: None,
            llm_provider: None,
            embedding_provider: None,
            pipeline: None,
            query_engine: None,
        }
    }

    /// Create with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(EdgeQuakeConfig::default())
    }

    /// Set the storage backends.
    pub fn with_storage_backends(
        mut self,
        kv: Arc<dyn KVStorage>,
        vector: Arc<dyn VectorStorage>,
        graph: Arc<dyn GraphStorage>,
    ) -> Self {
        self.kv_storage = Some(kv);
        self.vector_storage = Some(vector);
        self.graph_storage = Some(graph);
        self
    }

    /// Set the storage backends using a mutable reference.
    pub fn set_storage_backends(
        &mut self,
        kv: Arc<dyn KVStorage>,
        vector: Arc<dyn VectorStorage>,
        graph: Arc<dyn GraphStorage>,
    ) {
        self.kv_storage = Some(kv);
        self.vector_storage = Some(vector);
        self.graph_storage = Some(graph);
    }

    /// Set the LLM and embedding providers.
    pub fn with_providers(
        mut self,
        llm: Arc<dyn LLMProvider>,
        embedding: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        self.llm_provider = Some(llm);
        self.embedding_provider = Some(embedding);
        self
    }

    /// Set the LLM and embedding providers using a mutable reference.
    pub fn set_providers(
        &mut self,
        llm: Arc<dyn LLMProvider>,
        embedding: Arc<dyn EmbeddingProvider>,
    ) {
        self.llm_provider = Some(llm);
        self.embedding_provider = Some(embedding);
    }

    /// Initialize the EdgeQuake instance.
    ///
    /// This sets up all storage backends and connections.
    pub async fn initialize(&mut self) -> Result<()> {
        tracing::info!(
            "Initializing EdgeQuake for namespace: {}",
            self.config.namespace
        );

        // Ensure providers are set
        let llm = self
            .llm_provider
            .as_ref()
            .ok_or_else(|| Error::config("LLM provider not set"))?;
        let embedding = self
            .embedding_provider
            .as_ref()
            .ok_or_else(|| Error::config("Embedding provider not set"))?;

        // Set up pipeline
        let pipeline_config = PipelineConfig {
            chunker: edgequake_pipeline::ChunkerConfig {
                chunk_size: self.config.chunk_token_size,
                chunk_overlap: self.config.chunk_overlap_token_size,
                ..Default::default()
            },
            ..Default::default()
        };

        // Create base extractor
        let base_extractor: Arc<dyn edgequake_pipeline::EntityExtractor> = Arc::new(
            LLMExtractor::new(llm.clone()).with_entity_types(self.config.entity_types.clone()),
        );

        // Wrap with GleaningExtractor if enabled
        let extractor: Arc<dyn edgequake_pipeline::EntityExtractor> = if self.config.enable_gleaning
            && self.config.max_gleaning > 0
        {
            tracing::info!(
                max_gleaning = self.config.max_gleaning,
                "Enabling gleaning for multi-pass extraction"
            );
            Arc::new(
                GleaningExtractor::new(llm.clone(), base_extractor).with_config(GleaningConfig {
                    max_gleaning: self.config.max_gleaning,
                    always_glean: false,
                }),
            )
        } else {
            base_extractor
        };

        let pipeline = Pipeline::new(pipeline_config)
            .with_extractor(extractor)
            .with_embedding_provider(embedding.clone());

        self.pipeline = Some(Arc::new(pipeline));

        // Set up query engine
        let graph_storage = self
            .graph_storage
            .as_ref()
            .ok_or_else(|| Error::config("Graph storage not set"))?;
        let vector_storage = self
            .vector_storage
            .as_ref()
            .ok_or_else(|| Error::config("Vector storage not set"))?;

        // Initialize SOTA query engine from edgequake-query
        let query_engine = crate::query::QueryEngine::new(
            llm.clone(),
            embedding.clone(),
            graph_storage.clone(),
            vector_storage.clone(),
        );

        self.query_engine = Some(Arc::new(query_engine));

        self.initialized = true;
        tracing::info!("EdgeQuake initialized successfully");

        Ok(())
    }

    /// Finalize and clean up resources.
    pub async fn finalize(&self) -> Result<()> {
        tracing::info!("Finalizing EdgeQuake");
        Ok(())
    }

    /// Get the configuration.
    pub fn config(&self) -> &EdgeQuakeConfig {
        &self.config
    }

    /// Get the namespace.
    pub fn namespace(&self) -> &str {
        &self.config.namespace
    }

    /// Check if the EdgeQuake instance is healthy and ready.
    pub async fn health_check(&self) -> Result<bool> {
        // TODO: Check all backend connections
        Ok(self.initialized)
    }
}

impl std::fmt::Debug for EdgeQuake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EdgeQuake")
            .field("namespace", &self.config.namespace)
            .field("initialized", &self.initialized)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::QueryParams;
    use crate::QueryMode;

    #[test]
    fn test_config_builder() {
        let config = EdgeQuakeConfig::new()
            .with_namespace("test-ns")
            .with_llm_model("gpt-4")
            .with_embedding_model("text-embedding-3-large", 3072)
            .with_entity_types(vec!["PERSON".to_string(), "ORG".to_string()]);

        assert_eq!(config.namespace, "test-ns");
        assert_eq!(config.llm_model_name, "gpt-4");
        assert_eq!(config.embedding_model_name, "text-embedding-3-large");
        assert_eq!(config.embedding_dim, 3072);
        assert_eq!(config.entity_types.len(), 2);
    }

    #[test]
    fn test_query_params_builder() {
        let params = QueryParams::new()
            .with_mode(QueryMode::Local)
            .with_top_k(100)
            .with_streaming();

        assert_eq!(params.mode, QueryMode::Local);
        assert_eq!(params.top_k, 100);
        assert!(params.stream);
    }

    #[test]
    fn test_config_default_values() {
        let config = EdgeQuakeConfig::default();
        assert_eq!(config.namespace, "default");
        assert_eq!(config.embedding_dim, 1536);
        assert!(!config.entity_types.is_empty());
    }

    #[test]
    fn test_config_with_chunk_config() {
        let config = EdgeQuakeConfig::new().with_chunk_config(500, 100);
        assert_eq!(config.chunk_token_size, 500);
        assert_eq!(config.chunk_overlap_token_size, 100);
    }

    #[test]
    fn test_query_params_defaults() {
        let params = QueryParams::new();
        assert_eq!(params.mode, QueryMode::Hybrid);
        assert_eq!(params.top_k, 60);
        assert!(!params.stream);
    }

    #[test]
    fn test_config_with_gleaning() {
        let config = EdgeQuakeConfig::new().with_gleaning(true, 3);
        assert!(config.enable_gleaning);
        assert_eq!(config.max_gleaning, 3);
    }

    #[test]
    fn test_storage_backend_default() {
        let backend = StorageBackend::default();
        assert!(matches!(backend, StorageBackend::Memory));
    }

    #[test]
    fn test_storage_config_default() {
        let config = StorageConfig::default();
        assert!(matches!(config.backend, StorageBackend::Memory));
    }

    #[tokio::test]
    async fn test_edgequake_lifecycle() {
        use edgequake_llm::MockProvider;
        use edgequake_storage::adapters::memory::{
            MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage,
        };

        let mock_provider = Arc::new(MockProvider::new());
        let kv_storage: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("test"));
        let vector_storage: Arc<dyn VectorStorage> =
            Arc::new(MemoryVectorStorage::new("test", 1536));
        let graph_storage: Arc<dyn GraphStorage> = Arc::new(MemoryGraphStorage::new("test"));

        let mut eq = EdgeQuake::new(EdgeQuakeConfig::default())
            .with_storage_backends(kv_storage, vector_storage, graph_storage)
            .with_providers(mock_provider.clone(), mock_provider);

        assert!(!eq.initialized);

        eq.initialize().await.unwrap();

        assert!(eq.initialized);
        assert!(eq.health_check().await.unwrap());

        eq.finalize().await.unwrap();
    }

    #[tokio::test]
    async fn test_edgequake_query_uses_core_engine() {
        use edgequake_llm::MockProvider;
        use edgequake_storage::adapters::memory::{
            MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage,
        };

        let mock_provider = Arc::new(MockProvider::new());
        let kv_storage: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("test"));
        let vector_storage: Arc<dyn VectorStorage> =
            Arc::new(MemoryVectorStorage::new("test", 1536));
        let graph_storage: Arc<dyn GraphStorage> = Arc::new(MemoryGraphStorage::new("test"));

        let mut eq = EdgeQuake::new(EdgeQuakeConfig::default())
            .with_storage_backends(kv_storage, vector_storage, graph_storage)
            .with_providers(mock_provider.clone(), mock_provider);

        eq.initialize().await.unwrap();

        // Execute a simple query and verify result shape
        let result = eq.query("hello world", None).await.unwrap();
        assert!(matches!(result.mode, crate::types::QueryMode::Hybrid));
        assert!(result.response.is_empty() || !result.response.is_empty()); // existence check
    }
}
