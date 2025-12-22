//! EdgeQuake Orchestrator - Main RAG coordination module.
//!
//! This module provides the high-level EdgeQuake orchestrator, equivalent to
//! LightRAG's main class, that coordinates all RAG operations including:
//! - Document ingestion and processing
//! - Knowledge graph construction
//! - Multi-modal querying (local, global, hybrid)
//! - Workspace management for multi-tenancy

use std::collections::HashMap;
use std::sync::Arc;

use edgequake_llm::traits::{EmbeddingProvider, LLMProvider};
use edgequake_pipeline::{
    KnowledgeGraphMerger, LLMExtractor, MergerConfig, Pipeline, PipelineConfig,
};
use edgequake_storage::traits::{GraphStorage, KVStorage, VectorStorage};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::types::{
    ContextEntity, DocumentInfo, GraphStats, InsertResult, QueryContext, QueryParams, QueryResult,
};

/// EdgeQuake instance configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeQuakeConfig {
    /// Working directory for storage.
    pub working_dir: String,

    /// Namespace/workspace identifier.
    pub namespace: String,

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
            llm_model_name: "gpt-4o-mini".to_string(),
            response_model_name: None,
            embedding_model_name: "text-embedding-3-small".to_string(),
            embedding_dim: 1536,
            max_token_for_text_unit: 4000,
            max_token_for_global_context: 4000,
            max_token_for_local_context: 4000,
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

        let extractor = Arc::new(
            LLMExtractor::new(llm.clone()).with_entity_types(self.config.entity_types.clone()),
        );

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

    /// Insert a document for processing.
    pub async fn insert(&self, content: &str, document_id: Option<&str>) -> Result<InsertResult> {
        if !self.initialized {
            return Err(Error::not_initialized("EdgeQuake not initialized"));
        }

        let doc_id = document_id
            .map(String::from)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let start = std::time::Instant::now();

        let pipeline = self
            .pipeline
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Pipeline not initialized"))?;

        let graph_storage = self
            .graph_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Graph storage not initialized"))?;

        let vector_storage = self
            .vector_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Vector storage not initialized"))?;

        // 1. Process document through pipeline (Chunking + Extraction + Embedding)
        let processing_result = pipeline
            .process(&doc_id, content)
            .await
            .map_err(|e| Error::internal(format!("Pipeline error: {}", e)))?;

        // 2. Merge results into knowledge graph and vector store
        let merger = KnowledgeGraphMerger::new(
            MergerConfig::default(),
            graph_storage.clone(),
            vector_storage.clone(),
        );

        let merge_stats = merger
            .merge(processing_result.extractions.clone())
            .await
            .map_err(|e| Error::internal(format!("Merge error: {}", e)))?;

        // 3. Store chunk embeddings
        for chunk in &processing_result.chunks {
            if let Some(embedding) = &chunk.embedding {
                vector_storage
                    .upsert(&[(
                        chunk.id.clone(),
                        embedding.clone(),
                        serde_json::json!({
                            "document_id": doc_id,
                            "index": chunk.index,
                            "content": chunk.content
                        }),
                    )])
                    .await
                    .map_err(|e| Error::internal(format!("Vector storage error: {}", e)))?;
            }
        }

        let processing_time_ms = start.elapsed().as_millis() as u64;

        Ok(InsertResult {
            document_id: doc_id,
            success: true,
            chunks_created: processing_result.stats.chunk_count,
            entities_extracted: merge_stats.entities_created + merge_stats.entities_updated,
            relationships_extracted: merge_stats.relationships_created
                + merge_stats.relationships_updated,
            processing_time_ms,
            error: None,
        })
    }

    /// Insert multiple documents.
    pub async fn insert_batch(
        &self,
        documents: Vec<(&str, Option<&str>)>,
    ) -> Result<Vec<InsertResult>> {
        let mut results = Vec::with_capacity(documents.len());

        for (content, doc_id) in documents {
            let result = self.insert(content, doc_id).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Query the knowledge base.
    pub async fn query(&self, query: &str, params: Option<QueryParams>) -> Result<QueryResult> {
        if !self.initialized {
            return Err(Error::not_initialized("EdgeQuake not initialized"));
        }

        let params = params.unwrap_or_default();

        let query_engine = self
            .query_engine
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Query engine not initialized"))?;

        query_engine.query(query, params).await
    }

    /// Delete a document and all its associated data.
    pub async fn delete_document(&self, document_id: &str) -> Result<bool> {
        if !self.initialized {
            return Err(Error::not_initialized("EdgeQuake not initialized"));
        }

        tracing::info!("Deleting document: {}", document_id);

        // TODO: Delete from all storages

        Ok(true)
    }

    /// Delete an entity and its relationships.
    pub async fn delete_entity(&self, entity_name: &str) -> Result<bool> {
        if !self.initialized {
            return Err(Error::not_initialized("EdgeQuake not initialized"));
        }

        tracing::info!("Deleting entity: {}", entity_name);

        // TODO: Delete from graph and vector stores

        Ok(true)
    }

    /// Get graph statistics.
    pub async fn get_graph_stats(&self) -> Result<GraphStats> {
        if !self.initialized {
            return Err(Error::not_initialized("EdgeQuake not initialized"));
        }

        let graph_storage = self
            .graph_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Graph storage not initialized"))?;

        let node_count = graph_storage.node_count().await?;
        let edge_count = graph_storage.edge_count().await?;

        Ok(GraphStats {
            node_count,
            edge_count,
            ..Default::default()
        })
    }

    /// Get document information.
    pub async fn get_document(&self, _document_id: &str) -> Result<Option<DocumentInfo>> {
        if !self.initialized {
            return Err(Error::not_initialized("EdgeQuake not initialized"));
        }

        // TODO: Retrieve from KV store
        Ok(None)
    }

    /// List all documents.
    pub async fn list_documents(&self) -> Result<Vec<DocumentInfo>> {
        if !self.initialized {
            return Err(Error::not_initialized("EdgeQuake not initialized"));
        }

        Ok(Vec::new())
    }

    /// Search entities by name.
    pub async fn search_entities(&self, query: &str, limit: usize) -> Result<Vec<ContextEntity>> {
        if !self.initialized {
            return Err(Error::not_initialized("EdgeQuake not initialized"));
        }

        let embedding_provider = self
            .embedding_provider
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Embedding provider not initialized"))?;

        let vector_storage = self
            .vector_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Vector storage not initialized"))?;

        let graph_storage = self
            .graph_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Graph storage not initialized"))?;

        // 1. Embed query
        let embeddings = embedding_provider.embed(&[query.to_string()]).await?;
        let query_embedding = embeddings
            .first()
            .ok_or_else(|| Error::internal("No embedding generated"))?;

        // 2. Search vector store
        let results = vector_storage.query(query_embedding, limit, None).await?;

        // 3. Map to ContextEntity
        let mut entities = Vec::new();
        for result in results {
            if let Some(node) = graph_storage.get_node(&result.id).await? {
                entities.push(ContextEntity {
                    name: node
                        .properties
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&result.id)
                        .to_string(),
                    entity_type: node
                        .properties
                        .get("entity_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("UNKNOWN")
                        .to_string(),
                    description: node
                        .properties
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    score: result.score,
                });
            }
        }

        Ok(entities)
    }

    /// Get knowledge graph subgraph around an entity.
    pub async fn get_entity_graph(
        &self,
        entity_name: &str,
        _max_depth: usize,
        _max_nodes: usize,
    ) -> Result<QueryContext> {
        if !self.initialized {
            return Err(Error::not_initialized("EdgeQuake not initialized"));
        }

        let graph_storage = self
            .graph_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Graph storage not initialized"))?;

        // For now, just get the entity and its immediate neighbors
        let mut entities = Vec::new();
        let mut relationships = Vec::new();

        if let Some(node) = graph_storage.get_node(entity_name).await? {
            entities.push(ContextEntity {
                name: node
                    .properties
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(entity_name)
                    .to_string(),
                entity_type: node
                    .properties
                    .get("entity_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UNKNOWN")
                    .to_string(),
                description: node
                    .properties
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                score: 1.0,
            });

            let edges = graph_storage.get_node_edges(entity_name).await?;
            for edge in edges {
                relationships.push(crate::types::ContextRelationship {
                    source: edge.source.clone(),
                    target: edge.target.clone(),
                    relation_type: "RELATED".to_string(),
                    description: edge
                        .properties
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    score: 1.0,
                });

                // Also add the target entity if not already present
                if let Some(target_node) = graph_storage.get_node(&edge.target).await? {
                    entities.push(ContextEntity {
                        name: target_node
                            .properties
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&edge.target)
                            .to_string(),
                        entity_type: target_node
                            .properties
                            .get("entity_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("UNKNOWN")
                            .to_string(),
                        description: target_node
                            .properties
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        score: 1.0,
                    });
                }
            }
        }

        Ok(QueryContext {
            entities,
            relationships,
            ..Default::default()
        })
    }

    /// Check if the instance is healthy.
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
}
