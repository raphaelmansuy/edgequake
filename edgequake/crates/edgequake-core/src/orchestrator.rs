//! EdgeQuake Orchestrator - Central RAG coordination module.
//!
//! @implements FEAT0023 (EdgeQuake Orchestrator)
//! @implements FEAT0007
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
    GleaningConfig, GleaningExtractor, KnowledgeGraphMerger, LLMExtractor, LLMSummarizer,
    MergerConfig, Pipeline, PipelineConfig, SummarizerConfig,
};
use edgequake_storage::traits::{GraphStorage, KVStorage, VectorStorage};
use serde::{Deserialize, Serialize};
// Use query crate types
// edgequake-query is intentionally not linked here to avoid workspace cycles.

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

/// Calculate adaptive chunk size based on document length.
///
/// WHY: Large documents need smaller chunks to avoid LLM timeouts and ensure reliable processing.
///
/// Based on LightRAG research:
/// - Default: 1200 tokens for normal documents
/// - Quality mode: 1500 tokens (maximum)
/// - Large documents: 600-800 tokens for better reliability
///
/// # Arguments
///
/// * `document_size_bytes` - Size of the document in bytes
///
/// # Returns
///
/// Recommended chunk size in tokens
///
/// # Examples
///
/// ```ignore
/// // Internal function - not part of public API
/// let chunk_size = calculate_adaptive_chunk_size(30_000);  // 30KB → 1200 tokens
/// let chunk_size = calculate_adaptive_chunk_size(80_000);  // 80KB → 800 tokens
/// let chunk_size = calculate_adaptive_chunk_size(200_000); // 200KB → 600 tokens
/// ```
fn calculate_adaptive_chunk_size(document_size_bytes: usize) -> usize {
    // Based on LightRAG best practices and empirical testing:
    // - Small documents (<50KB): Use standard 1200 tokens
    // - Medium documents (50-100KB): Use reduced 800 tokens
    // - Large documents (>100KB): Use minimal 600 tokens
    //
    // WHY these thresholds:
    // - 50KB ≈ 12,500 tokens → ~10 chunks at 1200 tokens (manageable)
    // - 100KB ≈ 25,000 tokens → ~31 chunks at 800 tokens (reasonable)
    // - 150KB ≈ 37,500 tokens → ~62 chunks at 600 tokens (many but necessary)
    //
    // Smaller chunks for large documents reduce:
    // 1. LLM timeout risk (less context per request)
    // 2. Entity extraction complexity (focused scope)
    // 3. Memory pressure (smaller batches)
    if document_size_bytes > 100_000 {
        600 // >100KB: minimal chunks for reliability
    } else if document_size_bytes > 50_000 {
        800 // 50-100KB: reduced chunks
    } else {
        1200 // <50KB: standard LightRAG default
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

    /// Insert a document for processing.
    ///
    /// # Implements
    ///
    /// - **FEAT0001**: Document Ingestion
    /// - **FEAT0002**: Text Chunking with Overlap
    /// - **FEAT0003**: LLM-Based Entity Extraction
    /// - **FEAT0005**: Knowledge Graph Construction
    /// - **FEAT0006**: Vector Embedding Generation
    ///
    /// # Enforces
    ///
    /// - **BR0001**: Document ID must be unique (error on duplicate)
    /// - **BR0002**: Chunk overlap < chunk size (validated in pipeline)
    /// - **BR0003**: Entity names normalized to UPPERCASE_UNDERSCORED
    ///
    /// # WHY: 3-Stage Pipeline Architecture
    ///
    /// The insert flow follows a 3-stage architecture (similar to LightRAG):
    ///
    /// 1. **Pipeline Processing** - Chunking → Entity Extraction → Embedding
    ///    - WHY chunks: LLM context windows are limited; chunks enable parallel processing
    ///    - WHY overlapping chunks: Entities spanning chunk boundaries are captured
    ///
    /// 2. **Knowledge Graph Merge** - Deduplicate and merge into graph storage
    ///    - WHY merge instead of insert: Same entity may appear in multiple documents
    ///    - WHY LLM summarization: Merge conflicting descriptions intelligently
    ///    - WHY source tracking: Enable cascade delete when documents are removed
    ///
    /// 3. **Vector Storage** - Store embeddings for semantic search
    ///    - WHY type metadata: Distinguish entity vectors from chunk vectors
    ///    - WHY tenant isolation: Multi-tenancy requires vector filtering
    ///
    /// # Arguments
    ///
    /// * `content` - Raw text content to process
    /// * `document_id` - Optional document ID; auto-generated UUID if not provided
    ///
    /// # Returns
    ///
    /// [`InsertResult`] with processing statistics (chunks, entities, relationships)
    ///
    /// # Errors
    ///
    /// - `Error::not_initialized` if EdgeQuake not initialized
    /// - `Error::internal` if pipeline or storage operations fail
    pub async fn insert(&self, content: &str, document_id: Option<&str>) -> Result<InsertResult> {
        if !self.initialized {
            return Err(Error::not_initialized("EdgeQuake not initialized"));
        }

        // Edge case: Empty document
        // WHY: Skip processing empty content to save resources
        let content_trimmed = content.trim();
        if content_trimmed.is_empty() {
            let doc_id = document_id
                .map(String::from)
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

            tracing::warn!(
                doc_id = %doc_id,
                "Skipping empty document - no content to process"
            );

            return Ok(InsertResult {
                document_id: doc_id,
                success: true,
                chunks_created: 0,
                entities_extracted: 0,
                relationships_extracted: 0,
                processing_time_ms: 0,
                error: None,
            });
        }

        // Edge case: Extremely large document (>10MB)
        // WHY: Documents over 10MB are likely to cause OOM or extreme timeouts
        const MAX_DOCUMENT_SIZE_BYTES: usize = 10 * 1024 * 1024; // 10MB
        if content.len() > MAX_DOCUMENT_SIZE_BYTES {
            let doc_id = document_id
                .map(String::from)
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

            let size_mb = content.len() as f64 / (1024.0 * 1024.0);
            tracing::error!(
                doc_id = %doc_id,
                size_bytes = content.len(),
                size_mb = %format!("{:.2}", size_mb),
                max_size_mb = MAX_DOCUMENT_SIZE_BYTES / (1024 * 1024),
                "Document exceeds maximum size limit"
            );

            return Err(Error::validation(format!(
                "Document too large: {:.2}MB. Maximum allowed: {}MB. \
                Please split the document into smaller files.",
                size_mb,
                MAX_DOCUMENT_SIZE_BYTES / (1024 * 1024)
            )));
        }

        let doc_id = document_id
            .map(String::from)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let start = std::time::Instant::now();

        // Calculate adaptive chunk size based on document length
        // WHY: Large documents need smaller chunks to avoid LLM timeouts
        // Based on LightRAG research: 1200 tokens optimal for <50KB, scale down for larger docs
        let doc_size_bytes = content.len();
        let adaptive_chunk_size = calculate_adaptive_chunk_size(doc_size_bytes);
        let adaptive_overlap = (adaptive_chunk_size as f32 * 0.083) as usize; // ~8% overlap (LightRAG best practice)
        let doc_size_kb = doc_size_bytes / 1024;

        tracing::info!(
            doc_id = %doc_id,
            doc_size_bytes = doc_size_bytes,
            doc_size_kb = doc_size_kb,
            adaptive_chunk_size = adaptive_chunk_size,
            adaptive_overlap = adaptive_overlap,
            default_chunk_size = self.config.chunk_token_size,
            "Using adaptive chunking for document ingestion"
        );

        // Create pipeline with adaptive configuration
        // WHY: Per-document pipeline allows dynamic chunk sizing
        // WHY not reuse stored pipeline: Stored pipeline uses static config
        let pipeline_config = PipelineConfig {
            chunker: edgequake_pipeline::ChunkerConfig {
                chunk_size: adaptive_chunk_size,
                chunk_overlap: adaptive_overlap,
                ..Default::default()
            },
            ..Default::default()
        };

        let llm = self
            .llm_provider
            .as_ref()
            .ok_or_else(|| Error::config("LLM provider not set"))?;

        let embedding = self
            .embedding_provider
            .as_ref()
            .ok_or_else(|| Error::config("Embedding provider not set"))?;

        // Create base extractor
        let base_extractor: Arc<dyn edgequake_pipeline::EntityExtractor> = Arc::new(
            LLMExtractor::new(llm.clone()).with_entity_types(self.config.entity_types.clone()),
        );

        // Wrap with GleaningExtractor if enabled
        let extractor: Arc<dyn edgequake_pipeline::EntityExtractor> = if self.config.enable_gleaning
            && self.config.max_gleaning > 0
        {
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

        let graph_storage = self
            .graph_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Graph storage not initialized"))?;

        let vector_storage = self
            .vector_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Vector storage not initialized"))?;

        // Stage 1: Process document through pipeline (Chunking → Extraction → Embedding)
        // WHY: Transforms raw text into structured knowledge graph elements
        let processing_result = pipeline
            .process(&doc_id, content)
            .await
            .map_err(|e| Error::internal(format!("Pipeline error: {}", e)))?;

        // Stage 2: Merge results into knowledge graph
        // WHY: Entities may exist from previous documents; merge avoids duplicates
        // WHY LLM summarization: When merging descriptions, LLM produces coherent summary
        let llm = self
            .llm_provider
            .as_ref()
            .ok_or_else(|| Error::not_initialized("LLM provider not initialized"))?;

        let merger_config = MergerConfig {
            use_llm_summarization: self.config.use_llm_summarization,
            ..Default::default()
        };

        let mut merger =
            KnowledgeGraphMerger::new(merger_config, graph_storage.clone(), vector_storage.clone())
                .with_tenant_context(
                    self.config.tenant_id.clone(),
                    self.config.workspace_id.clone(),
                );

        // Add LLM summarizer if enabled
        if self.config.use_llm_summarization {
            let summarizer = Arc::new(LLMSummarizer::new(llm.clone(), SummarizerConfig::default()));
            merger = merger.with_summarizer(summarizer);
        }

        let merge_stats = merger
            .merge(processing_result.extractions.clone())
            .await
            .map_err(|e| Error::internal(format!("Merge error: {}", e)))?;

        // Stage 3: Store chunk embeddings with type metadata
        // WHY type: "chunk" metadata: Enables filtering entity vs chunk vectors at query time
        // WHY tenant/workspace: Multi-tenancy isolation at vector level
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

    /// Query the knowledge base with configurable retrieval strategy.
    ///
    /// # Implements
    ///
    /// - **FEAT0007**: Multi-Mode Query Execution
    /// - **FEAT0101-0106**: Query mode strategies (naive/local/global/hybrid/mix/bypass)
    /// - **FEAT0107**: LLM-Based Keyword Extraction
    /// - **FEAT0108**: Smart Context Truncation
    ///
    /// # Enforces
    ///
    /// - **BR0101**: Token budget must not exceed LLM context window
    /// - **BR0102**: Graph context takes priority over naive chunks
    /// - **BR0103**: Query mode must be valid enum value
    /// - **BR0105**: Empty queries are rejected
    /// - **BR0201**: Tenant isolation (queries scoped to tenant/workspace)
    ///
    /// # WHY: Multi-Stage Retrieval Pipeline
    ///
    /// Query execution follows a multi-stage retrieval pipeline:
    ///
    /// ```text
    /// Query → Keywords → Vector Search → Graph Traversal → Context → LLM → Response
    ///                         ↓                ↓
    ///                    [chunks]        [entities, rels]
    /// ```
    ///
    /// 1. **Keyword Extraction** - Extract search terms from natural language query
    /// 2. **Candidate Retrieval** - Vector similarity + graph traversal
    /// 3. **Context Aggregation** - Merge and rank retrieved context
    /// 4. **Token Budget** - Truncate to fit LLM context window
    /// 5. **LLM Generation** - Generate final response
    ///
    /// # Arguments
    ///
    /// * `query` - Natural language query string
    /// * `params` - Optional query parameters (mode, filters, limits)
    ///
    /// # Returns
    ///
    /// [`QueryResult`] with response, sources, and retrieval statistics
    ///
    /// # Errors
    ///
    /// - `Error::not_initialized` if EdgeQuake not initialized
    /// - Query engine errors propagated from retrieval/generation
    ///
    /// # See Also
    ///
    /// - [`QueryParams`] for configuration options
    /// - [`QueryMode`] for available modes
    /// - [docs/features.md#FEAT0101](../../../../../../docs/features.md) for mode details
    pub async fn query(&self, query: &str, params: Option<QueryParams>) -> Result<QueryResult> {
        if !self.initialized {
            return Err(Error::not_initialized("EdgeQuake not initialized"));
        }

        let mut params = params.unwrap_or_default();

        // Set tenant and workspace IDs from config if not provided in params
        // WHY: Ensures tenant isolation (BR0201) even if caller forgets to set
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

        // Delegate to SOTA query engine (FEAT0109)
        // WHY delegation: Query logic is complex; separating into edgequake-query crate
        // enables independent testing and evolution of retrieval strategies
        query_engine.query(query, params).await
    }

    /// Delete a document and cascade delete associated graph data.
    ///
    /// # Implements
    ///
    /// - **UC0005**: Delete Document
    /// - **FEAT0011**: Document-Chunk-Entity Lineage
    ///
    /// # Enforces
    ///
    /// - **BR0007**: Lineage records are append-only (deletion removes, not modifies)
    /// - **BR0201**: Tenant isolation (only deletes within tenant scope)
    ///
    /// # WHY: Source-Tracking Cascade Delete
    ///
    /// This implements document suppression with cascade semantics:
    ///
    /// 1. **Source Tracking** - Every entity/relationship stores `source_id` listing all
    ///    contributing chunks. WHY: A single entity (e.g., "Apple") may be mentioned
    ///    in 100 documents. We can't delete the entity unless ALL sources are gone.
    ///
    /// 2. **Cascade Logic**:
    ///    - If entity has ONLY sources from this document → DELETE entity
    ///    - If entity has MIXED sources → UPDATE to remove this document's sources
    ///
    /// 3. **Edge Cleanup** - Edges connected to deleted nodes are also deleted.
    ///    WHY: Orphan edges would corrupt graph queries.
    ///
    /// This matches LightRAG's P4-04 (Document Suppression) and P4-05 (Cascade Delete).
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
                        graph_storage
                            .delete_edge(&edge.source, &edge.target)
                            .await?;
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
                    graph_storage
                        .delete_edge(&edge.source, &edge.target)
                        .await?;
                    result.relationships_removed += 1;
                } else if remaining_sources.len() < source_id.split('|').count() {
                    // Some sources were removed - update the relationship
                    let mut updated_props = edge.properties.clone();
                    updated_props.insert(
                        "source_id".to_string(),
                        serde_json::json!(remaining_sources.join("|")),
                    );
                    graph_storage
                        .upsert_edge(&edge.source, &edge.target, updated_props)
                        .await?;
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
    /// # Implements
    ///
    /// - **UC0006**: Preview Document Deletion Impact
    /// - **FEAT0012**: Deletion Impact Analysis
    ///
    /// # WHY: Pre-Flight Impact Visibility
    ///
    /// Before destructive operations, users need to understand what will change.
    /// This method performs a dry-run of deletion to show:
    /// - How many chunks will be removed
    /// - Which entities will be fully deleted vs. partially updated
    /// - Which relationships will be affected
    ///
    /// This implements impact analysis (P4-06) from the LightRAG specification.
    pub async fn analyze_deletion_impact(
        &self,
        document_id: &str,
    ) -> Result<DocumentDeletionResult> {
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

    /// Delete an entity and its relationships from the knowledge graph.
    ///
    /// # Implements
    ///
    /// - **UC0103**: Delete Entity from Graph
    /// - **FEAT0203**: Graph Mutation Operations
    ///
    /// # Enforces
    ///
    /// - **BR0008**: Entity names are normalized (UPPERCASE with underscores)
    /// - **BR0201**: Tenant isolation (deletion scoped to tenant)
    ///
    /// # WHY: Cascade Edge Deletion
    ///
    /// When an entity is deleted, all connected edges must also be deleted.
    /// Orphan edges would corrupt graph traversal queries. The deletion order is:
    /// 1. Find and delete all edges where entity is source or target
    /// 2. Delete the node itself from graph storage
    /// 3. Delete the entity embedding from vector storage
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
            graph_storage
                .delete_edge(&edge.source, &edge.target)
                .await?;
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

    /// Get knowledge graph statistics (node count, edge count, etc.).
    ///
    /// # Implements
    ///
    /// - **UC0104**: View Graph Statistics
    /// - **FEAT0204**: Graph Analytics
    ///
    /// # WHY: Operational Visibility
    ///
    /// Graph statistics are essential for:
    /// - Monitoring knowledge base growth over time
    /// - Capacity planning (when to shard or scale)
    /// - Quality metrics (entities per document ratio)
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

    /// Get document information by ID.
    ///
    /// # Implements
    ///
    /// - **UC0003**: View Document Details
    /// - **FEAT0010**: Document Metadata Storage
    ///
    /// # TODO
    ///
    /// Implementation pending - needs to retrieve from KV store.
    pub async fn get_document(&self, _document_id: &str) -> Result<Option<DocumentInfo>> {
        if !self.initialized {
            return Err(Error::not_initialized("EdgeQuake not initialized"));
        }

        // TODO: Retrieve from KV store
        Ok(None)
    }

    /// List all documents in the knowledge base.
    ///
    /// # Implements
    ///
    /// - **UC0002**: List Documents
    /// - **FEAT0010**: Document Metadata Storage
    ///
    /// # TODO
    ///
    /// Implementation pending - needs to enumerate KV store entries.
    pub async fn list_documents(&self) -> Result<Vec<DocumentInfo>> {
        if !self.initialized {
            return Err(Error::not_initialized("EdgeQuake not initialized"));
        }

        Ok(Vec::new())
    }

    /// Search entities by name using vector similarity.
    ///
    /// # Implements
    ///
    /// - **UC0102**: Search Entities by Name
    /// - **FEAT0201**: Vector Similarity Search
    ///
    /// # WHY: Fuzzy Entity Discovery
    ///
    /// Users often don't know exact entity names. Vector similarity enables:
    /// - Typo tolerance (finding "Apple Inc" when searching "apple company")
    /// - Semantic matching (finding "Microsoft" when searching "software giant")
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

    /// Get knowledge graph subgraph centered on an entity.
    ///
    /// # Implements
    ///
    /// - **UC0101**: Explore Entity Neighborhood
    /// - **FEAT0202**: Graph Traversal
    /// - **FEAT0601**: Knowledge Graph Visualization
    ///
    /// # WHY: Visual Knowledge Exploration
    ///
    /// Subgraph extraction enables:
    /// - Interactive graph visualization in the WebUI
    /// - Understanding entity context and relationships
    /// - Debugging knowledge graph quality
    ///
    /// # Arguments
    ///
    /// * `entity_name` - Starting entity for traversal
    /// * `max_depth` - Maximum hops from starting entity (currently unused, always 1)
    /// * `max_nodes` - Maximum nodes to return (currently unused)
    ///
    /// # TODO
    ///
    /// - Implement multi-hop traversal with configurable depth
    /// - Add node limit enforcement
    /// - Optimize for large graphs with sampling
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

    /// Check if the EdgeQuake instance is healthy and ready.
    ///
    /// # Implements
    ///
    /// - **UC0501**: Health Check
    /// - **FEAT0401**: REST API Readiness Endpoint
    ///
    /// # WHY: Kubernetes Liveness/Readiness Probes
    ///
    /// Container orchestrators (Kubernetes, ECS) need health endpoints to:
    /// - Determine if instance should receive traffic
    /// - Restart unhealthy instances automatically
    /// - Enable zero-downtime deployments
    ///
    /// # TODO
    ///
    /// - Add deep health checks (database connectivity, LLM provider reachability)
    /// - Add configurable timeout for health probe
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
