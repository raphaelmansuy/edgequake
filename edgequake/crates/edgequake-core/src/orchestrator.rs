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
    ContextEntity, DocumentDeletionResult, DocumentInfo, EntityDeletionResult, GraphStats,
    InsertResult, QueryContext, QueryParams, QueryResult,
};

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
        )
        .with_tenant_context(self.config.tenant_id.clone(), self.config.workspace_id.clone());

        let merge_stats = merger
            .merge(processing_result.extractions.clone())
            .await
            .map_err(|e| Error::internal(format!("Merge error: {}", e)))?;

        // 3. Store chunk embeddings with type metadata
        for chunk in &processing_result.chunks {
            if let Some(embedding) = &chunk.embedding {
                let mut metadata = serde_json::json!({
                    "type": "chunk",  // Mark as chunk for retrieval filtering
                    "document_id": doc_id,
                    "index": chunk.index,
                    "content": chunk.content
                });

                // Add tenant and workspace IDs if present
                if let Some(tenant_id) = &self.config.tenant_id {
                    metadata["tenant_id"] = serde_json::json!(tenant_id);
                }
                if let Some(workspace_id) = &self.config.workspace_id {
                    metadata["workspace_id"] = serde_json::json!(workspace_id);
                }

                vector_storage
                    .upsert(&[(chunk.id.clone(), embedding.clone(), metadata)])
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

        let mut params = params.unwrap_or_default();

        // Set tenant and workspace IDs from config if not provided in params
        if params.tenant_id.is_none() {
            params.tenant_id = self.config.tenant_id.clone();
        }
        if params.workspace_id.is_none() {
            params.workspace_id = self.config.workspace_id.clone();
        }

        let query_engine = self
            .query_engine
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Query engine not initialized"))?;

        query_engine.query(query, params).await
    }

    /// Delete a document and cascade delete associated graph data.
    ///
    /// This implements document suppression (P4-04) and cascade delete (P4-05)
    /// from the ingestion pipeline specification:
    /// 1. Finds all entities/relationships sourced from this document's chunks
    /// 2. Removes those sources from the entity's source_id list
    /// 3. Deletes entities/relationships that have no remaining sources
    pub async fn delete_document(&self, document_id: &str) -> Result<DocumentDeletionResult> {
        if !self.initialized {
            return Err(Error::not_initialized("EdgeQuake not initialized"));
        }

        tracing::info!(document_id = %document_id, "Starting document suppression");

        let mut result = DocumentDeletionResult {
            document_id: document_id.to_string(),
            chunks_deleted: 0,
            entities_removed: 0,
            entities_updated: 0,
            relationships_removed: 0,
            relationships_updated: 0,
        };

        let graph_storage = self
            .graph_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Graph storage not initialized"))?;

        let vector_storage = self
            .vector_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Vector storage not initialized"))?;

        let kv_storage = self
            .kv_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("KV storage not initialized"))?;

        // 1. Find and delete chunks belonging to this document
        let chunk_prefix = format!("{}-chunk-", document_id);
        let keys = kv_storage.keys().await?;
        let chunk_ids: Vec<String> = keys
            .iter()
            .filter(|k| k.starts_with(&chunk_prefix))
            .cloned()
            .collect();

        result.chunks_deleted = chunk_ids.len();

        // 2. Process graph entities - remove document sources
        let all_nodes = graph_storage.get_all_nodes().await?;
        for node in all_nodes {
            // Check if this node has any sources from the deleted document
            if let Some(source_id) = node.properties.get("source_id").and_then(|v| v.as_str()) {
                let sources: Vec<&str> = source_id.split('|').collect();
                let remaining_sources: Vec<&str> = sources
                    .into_iter()
                    .filter(|s| !s.starts_with(&chunk_prefix) && !s.starts_with(document_id))
                    .collect();

                if remaining_sources.is_empty() {
                    // No sources left - delete the entity entirely
                    // First delete all connected edges
                    let edges = graph_storage.get_node_edges(&node.id).await?;
                    for edge in edges {
                        graph_storage.delete_edge(&edge.source, &edge.target).await?;
                        result.relationships_removed += 1;
                    }
                    // Then delete the node
                    graph_storage.delete_node(&node.id).await?;
                    // Also delete from vector storage
                    let _ = vector_storage.delete_entity(&node.id).await;
                    result.entities_removed += 1;
                } else if remaining_sources.len() < source_id.split('|').count() {
                    // Some sources were removed - update the entity
                    let mut updated_props = node.properties.clone();
                    updated_props.insert(
                        "source_id".to_string(),
                        serde_json::json!(remaining_sources.join("|")),
                    );
                    graph_storage.upsert_node(&node.id, updated_props).await?;
                    result.entities_updated += 1;
                }
            }
        }

        // 3. Process graph edges - remove document sources
        let all_edges = graph_storage.get_all_edges().await?;
        for edge in all_edges {
            if let Some(source_id) = edge.properties.get("source_id").and_then(|v| v.as_str()) {
                let sources: Vec<&str> = source_id.split('|').collect();
                let remaining_sources: Vec<&str> = sources
                    .into_iter()
                    .filter(|s| !s.starts_with(&chunk_prefix) && !s.starts_with(document_id))
                    .collect();

                if remaining_sources.is_empty() {
                    // No sources left - delete the relationship
                    graph_storage.delete_edge(&edge.source, &edge.target).await?;
                    result.relationships_removed += 1;
                } else if remaining_sources.len() < source_id.split('|').count() {
                    // Some sources were removed - update the relationship
                    let mut updated_props = edge.properties.clone();
                    updated_props.insert(
                        "source_id".to_string(),
                        serde_json::json!(remaining_sources.join("|")),
                    );
                    graph_storage.upsert_edge(&edge.source, &edge.target, updated_props).await?;
                    result.relationships_updated += 1;
                }
            }
        }

        // 4. Delete chunks and document metadata from KV storage
        let mut keys_to_delete = chunk_ids;
        let metadata_key = format!("{}-metadata", document_id);
        let content_key = format!("{}-content", document_id);
        if keys.contains(&metadata_key) {
            keys_to_delete.push(metadata_key);
        }
        if keys.contains(&content_key) {
            keys_to_delete.push(content_key);
        }
        if !keys_to_delete.is_empty() {
            kv_storage.delete(&keys_to_delete).await?;
        }

        tracing::info!(
            document_id = %document_id,
            chunks = result.chunks_deleted,
            entities_removed = result.entities_removed,
            entities_updated = result.entities_updated,
            relationships_removed = result.relationships_removed,
            "Document suppression complete"
        );

        Ok(result)
    }

    /// Analyze the impact of deleting a document before actually deleting it.
    ///
    /// This implements impact analysis (P4-06) from the specification.
    pub async fn analyze_deletion_impact(&self, document_id: &str) -> Result<DocumentDeletionResult> {
        if !self.initialized {
            return Err(Error::not_initialized("EdgeQuake not initialized"));
        }

        let mut result = DocumentDeletionResult {
            document_id: document_id.to_string(),
            chunks_deleted: 0,
            entities_removed: 0,
            entities_updated: 0,
            relationships_removed: 0,
            relationships_updated: 0,
        };

        let graph_storage = self
            .graph_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Graph storage not initialized"))?;

        let kv_storage = self
            .kv_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("KV storage not initialized"))?;

        // Count chunks
        let chunk_prefix = format!("{}-chunk-", document_id);
        let keys = kv_storage.keys().await?;
        result.chunks_deleted = keys.iter().filter(|k| k.starts_with(&chunk_prefix)).count();

        // Analyze entities
        let all_nodes = graph_storage.get_all_nodes().await?;
        for node in all_nodes {
            if let Some(source_id) = node.properties.get("source_id").and_then(|v| v.as_str()) {
                let sources: Vec<&str> = source_id.split('|').collect();
                let remaining = sources
                    .iter()
                    .filter(|s| !s.starts_with(&chunk_prefix) && !s.starts_with(document_id))
                    .count();

                if remaining == 0 {
                    result.entities_removed += 1;
                } else if remaining < sources.len() {
                    result.entities_updated += 1;
                }
            }
        }

        // Analyze edges
        let all_edges = graph_storage.get_all_edges().await?;
        for edge in all_edges {
            if let Some(source_id) = edge.properties.get("source_id").and_then(|v| v.as_str()) {
                let sources: Vec<&str> = source_id.split('|').collect();
                let remaining = sources
                    .iter()
                    .filter(|s| !s.starts_with(&chunk_prefix) && !s.starts_with(document_id))
                    .count();

                if remaining == 0 {
                    result.relationships_removed += 1;
                } else if remaining < sources.len() {
                    result.relationships_updated += 1;
                }
            }
        }

        Ok(result)
    }

    /// Delete an entity and its relationships.
    pub async fn delete_entity(&self, entity_name: &str) -> Result<EntityDeletionResult> {
        if !self.initialized {
            return Err(Error::not_initialized("EdgeQuake not initialized"));
        }

        tracing::info!(entity = %entity_name, "Deleting entity");

        let graph_storage = self
            .graph_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Graph storage not initialized"))?;

        let vector_storage = self
            .vector_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Vector storage not initialized"))?;

        let normalized_name = crate::types::GraphEntity::normalize_name(entity_name);
        let mut relationships_deleted = 0;

        // First, delete all edges connected to this entity
        let edges = graph_storage.get_node_edges(&normalized_name).await?;
        for edge in edges {
            graph_storage.delete_edge(&edge.source, &edge.target).await?;
            relationships_deleted += 1;
        }

        // Delete the node from graph storage
        graph_storage.delete_node(&normalized_name).await?;

        // Delete from vector storage
        let _ = vector_storage.delete_entity(&normalized_name).await;

        Ok(EntityDeletionResult {
            entity_name: normalized_name,
            deleted: true,
            relationships_deleted,
        })
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
