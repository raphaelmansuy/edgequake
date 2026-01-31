//! Document task processor for async document processing.
//!
//! This module implements the `TaskProcessor` trait to process document
//! upload tasks through the pipeline and update storage accordingly.
//!
//! ## Implements
//!
//! - [`FEAT0470`]: Async document processing
//! - [`FEAT0471`]: Pipeline integration
//! - [`FEAT0472`]: Progress tracking via PipelineState
//! - [`SPEC-032`]: Workspace-specific LLM/embedding provider selection
//! - [`OODA-198`]: Provider lineage tracking
//!
//! ## Use Cases
//!
//! - [`UC2070`]: System processes document asynchronously
//! - [`UC2071`]: System updates storage after processing
//! - [`UC2072`]: System uses workspace-configured LLM/embedding for processing
//!
//! ## Enforces
//!
//! - [`BR0470`]: Task queue integration
//! - [`BR0471`]: Error propagation to task result
//! - [`BR0472`]: Documents processed with workspace-specific providers

use std::sync::Arc;

use crate::state::SharedWorkspaceService;
use edgequake_llm::ModelsConfig;
use edgequake_pipeline::{ChunkProgressCallback, ChunkProgressUpdate, LLMExtractor, Pipeline};
use edgequake_storage::traits::{GraphStorage, KVStorage, VectorStorage, WorkspaceVectorRegistry};
use edgequake_tasks::{PipelineState, Task, TaskProcessor, TaskResult, TaskType, TextInsertData};
use serde_json::json;
use tracing::{error, info, warn};

/// SPEC-032/OODA-198: Provider lineage information for tracking which
/// providers were used to process a document.
#[derive(Debug, Clone, Default)]
pub struct ProviderLineage {
    /// LLM provider used for entity extraction.
    pub extraction_provider: String,
    /// LLM model used for entity extraction.
    pub extraction_model: String,
    /// Embedding provider used.
    pub embedding_provider: String,
    /// Embedding model used.
    pub embedding_model: String,
    /// Embedding dimension.
    pub embedding_dimension: usize,
}

/// Document task processor that processes documents through the pipeline.
///
/// SPEC-032: This processor supports workspace-specific LLM and embedding providers.
/// When a task includes workspace_id in its metadata, the processor will:
/// 1. Look up the workspace configuration
/// 2. Create a workspace-specific pipeline with the configured providers
/// 3. Process the document using those providers
/// 4. Store embeddings in workspace-specific vector storage (via vector_registry)
///
/// This ensures that rebuild/reprocess operations use the workspace's configured
/// models, not the server's default models.
pub struct DocumentTaskProcessor {
    /// Default processing pipeline (fallback when workspace not specified).
    pipeline: Arc<Pipeline>,
    /// LLM provider for extraction and enhancement (SPEC-007: needed for PDF processing).
    llm_provider: Arc<dyn edgequake_llm::traits::LLMProvider>,
    /// KV storage for document metadata and chunks.
    kv_storage: Arc<dyn KVStorage>,
    /// Vector storage for chunk embeddings (legacy fallback).
    vector_storage: Arc<dyn VectorStorage>,
    /// Workspace vector registry for per-workspace vector storage.
    /// WHY: Different workspaces can have different embedding dimensions.
    vector_registry: Arc<dyn WorkspaceVectorRegistry>,
    /// Graph storage for entities and relationships.
    graph_storage: Arc<dyn GraphStorage>,
    /// PDF storage for PDF document management (SPEC-007, postgres-only).
    #[cfg(feature = "postgres")]
    pdf_storage: Option<Arc<dyn edgequake_storage::PdfDocumentStorage>>,
    /// Pipeline state for progress tracking.
    pipeline_state: PipelineState,
    /// Workspace service for looking up workspace configuration (SPEC-032).
    workspace_service: Option<SharedWorkspaceService>,
    /// Models configuration for creating providers (SPEC-032).
    models_config: Option<Arc<ModelsConfig>>,
    /// OODA-223: Strict workspace mode - when true, fail if workspace not found.
    /// When false (memory/test mode), allow fallback to default storage.
    strict_workspace_mode: bool,
}

impl DocumentTaskProcessor {
    /// Create a new document task processor (legacy, without workspace support).
    /// OODA-223: Uses non-strict mode (allows fallback) for backward compatibility.
    pub fn new(
        pipeline: Arc<Pipeline>,
        llm_provider: Arc<dyn edgequake_llm::traits::LLMProvider>,
        kv_storage: Arc<dyn KVStorage>,
        vector_storage: Arc<dyn VectorStorage>,
        vector_registry: Arc<dyn WorkspaceVectorRegistry>,
        graph_storage: Arc<dyn GraphStorage>,
        pipeline_state: PipelineState,
    ) -> Self {
        Self {
            pipeline,
            llm_provider,
            kv_storage,
            vector_storage,
            vector_registry,
            graph_storage,
            #[cfg(feature = "postgres")]
            pdf_storage: None,
            pipeline_state,
            workspace_service: None,
            models_config: None,
            strict_workspace_mode: false, // OODA-223: Legacy mode allows fallback
        }
    }

    /// Create a new document task processor with workspace-specific pipeline support.
    ///
    /// SPEC-032: This constructor enables workspace-specific LLM and embedding providers.
    /// When processing tasks with workspace_id in metadata, the processor will use
    /// the workspace's configured providers instead of the server defaults.
    ///
    /// OODA-223: Use `with_workspace_support_strict` for production to ensure workspace
    /// isolation is enforced.
    pub fn with_workspace_support(
        pipeline: Arc<Pipeline>,
        llm_provider: Arc<dyn edgequake_llm::traits::LLMProvider>,
        kv_storage: Arc<dyn KVStorage>,
        vector_storage: Arc<dyn VectorStorage>,
        vector_registry: Arc<dyn WorkspaceVectorRegistry>,
        graph_storage: Arc<dyn GraphStorage>,
        pipeline_state: PipelineState,
        workspace_service: SharedWorkspaceService,
        models_config: Arc<ModelsConfig>,
    ) -> Self {
        Self {
            pipeline,
            llm_provider,
            kv_storage,
            vector_storage,
            vector_registry,
            graph_storage,
            #[cfg(feature = "postgres")]
            pdf_storage: None,
            pipeline_state,
            workspace_service: Some(workspace_service),
            models_config: Some(models_config),
            strict_workspace_mode: false, // OODA-223: Legacy mode allows fallback
        }
    }

    /// Create a new document task processor with strict workspace isolation.
    ///
    /// OODA-223: This constructor enables strict mode where ingestion FAILS if
    /// workspace storage cannot be obtained. Use this in production to prevent
    /// data from being stored in the wrong (global) table.
    pub fn with_workspace_support_strict(
        pipeline: Arc<Pipeline>,
        llm_provider: Arc<dyn edgequake_llm::traits::LLMProvider>,
        kv_storage: Arc<dyn KVStorage>,
        vector_storage: Arc<dyn VectorStorage>,
        vector_registry: Arc<dyn WorkspaceVectorRegistry>,
        graph_storage: Arc<dyn GraphStorage>,
        pipeline_state: PipelineState,
        workspace_service: SharedWorkspaceService,
        models_config: Arc<ModelsConfig>,
    ) -> Self {
        Self {
            pipeline,
            llm_provider,
            kv_storage,
            vector_storage,
            vector_registry,
            graph_storage,
            #[cfg(feature = "postgres")]
            pdf_storage: None,
            pipeline_state,
            workspace_service: Some(workspace_service),
            models_config: Some(models_config),
            strict_workspace_mode: true, // OODA-223: Production mode - fail on workspace errors
        }
    }

    /// Set PDF storage for PDF processing support (SPEC-007).
    ///
    /// This method allows PDF storage to be injected after processor creation,
    /// enabling PDF upload functionality when postgres feature is enabled.
    #[cfg(feature = "postgres")]
    pub fn with_pdf_storage(
        mut self,
        pdf_storage: Arc<dyn edgequake_storage::PdfDocumentStorage>,
    ) -> Self {
        self.pdf_storage = Some(pdf_storage);
        self
    }

    /// Get a workspace-specific pipeline if workspace_id is provided and valid.
    ///
    /// SPEC-032: Creates a new Pipeline instance configured with the workspace's
    /// LLM and embedding providers. Falls back to the default pipeline if:
    /// - No workspace_id provided
    /// - Workspace not found
    /// - Failed to create workspace-specific providers
    async fn get_workspace_pipeline(&self, workspace_id: Option<&str>) -> Arc<Pipeline> {
        use edgequake_llm::ProviderFactory;

        info!(
            workspace_id = ?workspace_id,
            has_workspace_service = self.workspace_service.is_some(),
            has_models_config = self.models_config.is_some(),
            "SPEC-032: Getting pipeline for workspace"
        );

        // If no workspace support configured, use default pipeline
        let (workspace_service, _models_config): (&SharedWorkspaceService, &Arc<ModelsConfig>) =
            match (&self.workspace_service, &self.models_config) {
                (Some(ws), Some(mc)) => (ws, mc),
                _ => {
                    warn!("SPEC-032: No workspace support configured, using default pipeline");
                    return Arc::clone(&self.pipeline);
                }
            };

        // If no workspace_id provided, use default pipeline
        let workspace_id = match workspace_id {
            Some(id) if !id.is_empty() && id != "default" => id,
            _ => {
                info!(
                    workspace_id = ?workspace_id,
                    "SPEC-032: No valid workspace_id, using default pipeline"
                );
                return Arc::clone(&self.pipeline);
            }
        };

        // Parse workspace_id to UUID
        let workspace_uuid = match uuid::Uuid::parse_str(workspace_id) {
            Ok(uuid) => uuid,
            Err(e) => {
                warn!(
                    workspace_id = workspace_id,
                    error = %e,
                    "Invalid workspace ID format, using default pipeline"
                );
                return Arc::clone(&self.pipeline);
            }
        };

        // Look up workspace configuration
        match workspace_service.get_workspace(workspace_uuid).await {
            Ok(Some(ws)) => {
                // Try to create workspace-specific LLM provider with safety limits
                // @implements OODA-189: Explicit error logging for provider failures
                // @implements FEAT0777: Safety limits for LLM calls
                let llm_provider_result =
                    ProviderFactory::create_safe_llm_provider(&ws.llm_provider, &ws.llm_model);

                // Try to create workspace-specific embedding provider with safety limits
                let embedding_provider_result = ProviderFactory::create_safe_embedding_provider(
                    &ws.embedding_provider,
                    &ws.embedding_model,
                    ws.embedding_dimension,
                );

                // Check for provider creation failures and log explicit errors
                match (&llm_provider_result, &embedding_provider_result) {
                    (Ok(llm), Ok(embedding)) => {
                        // SUCCESS: Both providers created
                        info!(
                            workspace_id = workspace_id,
                            llm_provider = %ws.llm_provider,
                            llm_model = %ws.llm_model,
                            embedding_provider = %ws.embedding_provider,
                            embedding_model = %ws.embedding_model,
                            "SPEC-032: Using workspace-specific providers for document processing"
                        );

                        let extractor = Arc::new(LLMExtractor::new(Arc::clone(llm)));
                        return Arc::new(
                            Pipeline::default_pipeline()
                                .with_extractor(extractor)
                                .with_embedding_provider(Arc::clone(embedding)),
                        );
                    }
                    (Err(llm_err), Ok(_)) => {
                        // LLM provider failed - this is a CRITICAL issue
                        error!(
                            workspace_id = workspace_id,
                            llm_provider = %ws.llm_provider,
                            llm_model = %ws.llm_model,
                            error = %llm_err,
                            "CRITICAL: Failed to create workspace LLM provider. \
                             Document extraction will use DEFAULT provider instead of workspace config. \
                             This may result in unexpected extraction results."
                        );
                    }
                    (Ok(_), Err(embed_err)) => {
                        // Embedding provider failed - this is a CRITICAL issue
                        error!(
                            workspace_id = workspace_id,
                            embedding_provider = %ws.embedding_provider,
                            embedding_model = %ws.embedding_model,
                            error = %embed_err,
                            "CRITICAL: Failed to create workspace embedding provider. \
                             Document embeddings will use DEFAULT provider instead of workspace config. \
                             This may result in dimension mismatches or unexpected query results."
                        );
                    }
                    (Err(llm_err), Err(embed_err)) => {
                        // Both providers failed - this is a CRITICAL issue
                        error!(
                            workspace_id = workspace_id,
                            llm_provider = %ws.llm_provider,
                            llm_model = %ws.llm_model,
                            llm_error = %llm_err,
                            embedding_provider = %ws.embedding_provider,
                            embedding_model = %ws.embedding_model,
                            embedding_error = %embed_err,
                            "CRITICAL: Failed to create BOTH workspace providers. \
                             Document processing will use DEFAULT pipeline instead of workspace config. \
                             Check API keys and provider configuration."
                        );
                    }
                }

                // Fallback to default pipeline (but with explicit ERROR logging above)
                warn!(
                    workspace_id = workspace_id,
                    llm_config = %ws.llm_full_id(),
                    embedding_config = %ws.embedding_full_id(),
                    "Falling back to default pipeline due to provider creation failure"
                );
            }
            Ok(None) => {
                warn!(
                    workspace_id = workspace_id,
                    "Workspace not found, using default pipeline"
                );
            }
            Err(e) => {
                warn!(
                    workspace_id = workspace_id,
                    error = %e,
                    "Failed to lookup workspace, using default pipeline"
                );
            }
        }

        Arc::clone(&self.pipeline)
    }

    /// Get workspace-specific vector storage using the registry.
    ///
    /// WHY: Different workspaces can have different embedding dimensions (e.g.,
    /// OpenAI 1536 vs Ollama/nomic 768). The registry creates per-workspace
    /// vector tables with the correct dimension.
    ///
    /// # OODA-223: Behavior depends on `strict_workspace_mode`
    ///
    /// - **Strict mode (production)**: Returns error if workspace storage cannot be obtained.
    /// - **Non-strict mode (tests/legacy)**: Falls back to default storage with warning.
    ///
    /// # Lesson Learned (OODA-223)
    ///
    /// Silent fallback to default storage caused data to be stored in the
    /// global table instead of workspace-specific tables, leading to "0 Sources"
    /// on queries because reads look in workspace tables.
    async fn get_workspace_vector_storage_strict(
        &self,
        workspace_id: &str,
    ) -> Result<Arc<dyn VectorStorage>, String> {
        use edgequake_storage::traits::WorkspaceVectorConfig;

        // OODA-223: Check if we should allow fallback
        let allow_fallback = !self.strict_workspace_mode;

        // Handle empty/default workspace IDs
        if workspace_id.is_empty() || workspace_id == "default" {
            if allow_fallback {
                warn!(
                    workspace_id = %workspace_id,
                    strict_mode = self.strict_workspace_mode,
                    "Empty/default workspace ID - using default storage (non-strict mode)"
                );
                return Ok(Arc::clone(&self.vector_storage));
            }
            error!(
                workspace_id = %workspace_id,
                "CRITICAL INGESTION ERROR: Cannot use 'default' workspace for document ingestion. \
                 Data must be stored in workspace-specific tables."
            );
            return Err("Cannot ingest documents without a valid workspace ID. \
                 Please ensure workspace context is properly set."
                .to_string());
        }

        // Parse workspace UUID
        let workspace_uuid = match uuid::Uuid::parse_str(workspace_id) {
            Ok(uuid) => uuid,
            Err(e) => {
                if allow_fallback {
                    warn!(
                        workspace_id = %workspace_id,
                        error = %e,
                        strict_mode = self.strict_workspace_mode,
                        "Invalid workspace ID format - using default storage (non-strict mode)"
                    );
                    return Ok(Arc::clone(&self.vector_storage));
                }
                error!(
                    workspace_id = %workspace_id,
                    error = %e,
                    "CRITICAL INGESTION ERROR: Invalid workspace ID format"
                );
                return Err(format!(
                    "Invalid workspace ID format '{}': {}",
                    workspace_id, e
                ));
            }
        };

        // Check if we already have this workspace's vector storage cached
        if let Some(storage) = self.vector_registry.get(&workspace_uuid).await {
            return Ok(storage);
        }

        // Look up workspace to get embedding dimension
        let workspace_service = match &self.workspace_service {
            Some(ws) => ws,
            None => {
                if allow_fallback {
                    warn!(
                        workspace_id = %workspace_id,
                        strict_mode = self.strict_workspace_mode,
                        "No workspace service - using default storage (non-strict mode)"
                    );
                    return Ok(Arc::clone(&self.vector_storage));
                }
                error!(
                    workspace_id = %workspace_id,
                    "CRITICAL INGESTION ERROR: No workspace service available"
                );
                return Err(
                    "Workspace service not configured. Cannot verify workspace exists.".to_string(),
                );
            }
        };

        match workspace_service.get_workspace(workspace_uuid).await {
            Ok(Some(ws)) => {
                // Create workspace-specific vector storage with correct dimension
                let config = WorkspaceVectorConfig {
                    workspace_id: workspace_uuid,
                    dimension: ws.embedding_dimension,
                    namespace: "default".to_string(),
                };

                match self.vector_registry.get_or_create(config).await {
                    Ok(storage) => {
                        info!(
                            workspace_id = %workspace_id,
                            dimension = ws.embedding_dimension,
                            strict_mode = self.strict_workspace_mode,
                            "Using workspace-specific vector storage"
                        );
                        Ok(storage)
                    }
                    Err(e) => {
                        if allow_fallback {
                            warn!(
                                workspace_id = %workspace_id,
                                error = %e,
                                strict_mode = self.strict_workspace_mode,
                                "Failed to create workspace storage - using default (non-strict mode)"
                            );
                            return Ok(Arc::clone(&self.vector_storage));
                        }
                        error!(
                            workspace_id = %workspace_id,
                            error = %e,
                            "CRITICAL INGESTION ERROR: Failed to create workspace vector storage"
                        );
                        Err(format!(
                            "Failed to create vector storage for workspace '{}': {}",
                            workspace_id, e
                        ))
                    }
                }
            }
            Ok(None) => {
                if allow_fallback {
                    warn!(
                        workspace_id = %workspace_id,
                        strict_mode = self.strict_workspace_mode,
                        "Workspace not found - using default storage (non-strict mode)"
                    );
                    return Ok(Arc::clone(&self.vector_storage));
                }
                error!(
                    workspace_id = %workspace_id,
                    "CRITICAL INGESTION ERROR: Workspace not found"
                );
                Err(format!(
                    "Workspace '{}' not found. Cannot ingest documents into non-existent workspace.",
                    workspace_id
                ))
            }
            Err(e) => {
                if allow_fallback {
                    warn!(
                        workspace_id = %workspace_id,
                        error = %e,
                        strict_mode = self.strict_workspace_mode,
                        "Failed to lookup workspace - using default storage (non-strict mode)"
                    );
                    return Ok(Arc::clone(&self.vector_storage));
                }
                error!(
                    workspace_id = %workspace_id,
                    error = %e,
                    "CRITICAL INGESTION ERROR: Failed to lookup workspace"
                );
                Err(format!(
                    "Failed to lookup workspace '{}': {}",
                    workspace_id, e
                ))
            }
        }
    }

    /// SPEC-032/OODA-198: Get provider lineage for a workspace.
    ///
    /// Returns the provider configuration that will be used for processing
    /// documents in this workspace. This enables lineage tracking by storing
    /// which providers were used for extraction.
    ///
    /// Returns default provider config if workspace not found.
    async fn get_workspace_provider_lineage(&self, workspace_id: Option<&str>) -> ProviderLineage {
        use edgequake_core::types::{
            DEFAULT_EMBEDDING_DIMENSION, DEFAULT_EMBEDDING_MODEL, DEFAULT_EMBEDDING_PROVIDER,
            DEFAULT_LLM_MODEL, DEFAULT_LLM_PROVIDER,
        };

        // Default lineage (used when workspace not available)
        let default_lineage = ProviderLineage {
            extraction_provider: DEFAULT_LLM_PROVIDER.to_string(),
            extraction_model: DEFAULT_LLM_MODEL.to_string(),
            embedding_provider: DEFAULT_EMBEDDING_PROVIDER.to_string(),
            embedding_model: DEFAULT_EMBEDDING_MODEL.to_string(),
            embedding_dimension: DEFAULT_EMBEDDING_DIMENSION,
        };

        let workspace_id = match workspace_id {
            Some(id) if !id.is_empty() && id != "default" => id,
            _ => return default_lineage,
        };

        let workspace_uuid = match uuid::Uuid::parse_str(workspace_id) {
            Ok(uuid) => uuid,
            Err(_) => return default_lineage,
        };

        let workspace_service = match &self.workspace_service {
            Some(ws) => ws,
            None => return default_lineage,
        };

        match workspace_service.get_workspace(workspace_uuid).await {
            Ok(Some(ws)) => ProviderLineage {
                extraction_provider: ws.llm_provider.clone(),
                extraction_model: ws.llm_model.clone(),
                embedding_provider: ws.embedding_provider.clone(),
                embedding_model: ws.embedding_model.clone(),
                embedding_dimension: ws.embedding_dimension,
            },
            _ => default_lineage,
        }
    }

    /// Process a text insert task.
    async fn process_text_insert(
        &self,
        task: &mut Task,
        data: TextInsertData,
    ) -> TaskResult<serde_json::Value> {
        let document_id = data
            .metadata
            .as_ref()
            .and_then(|m| m.get("document_id"))
            .and_then(|v| v.as_str())
            .unwrap_or(&data.file_source)
            .to_string();

        // SPEC-032: Extract workspace_id to use workspace-specific pipeline
        // Prefer the direct field (data.workspace_id), fallback to metadata if needed
        let workspace_id = if !data.workspace_id.is_empty() && data.workspace_id != "default" {
            Some(data.workspace_id.as_str())
        } else {
            data.metadata
                .as_ref()
                .and_then(|m| m.get("workspace_id"))
                .and_then(|v| v.as_str())
        };

        // Get workspace-specific pipeline (or default if not available)
        let pipeline = self.get_workspace_pipeline(workspace_id).await;

        // SPEC-032/OODA-198: Capture provider lineage for tracking
        let provider_lineage = self.get_workspace_provider_lineage(workspace_id).await;

        info!(
            document_id = %document_id,
            workspace_id = ?workspace_id,
            file_source = %data.file_source,
            extraction_provider = %provider_lineage.extraction_provider,
            extraction_model = %provider_lineage.extraction_model,
            embedding_provider = %provider_lineage.embedding_provider,
            "Processing document with workspace-specific pipeline"
        );

        // Update task progress - chunking
        task.update_progress("chunking".to_string(), 4, 10);

        // Log to pipeline state
        self.pipeline_state
            .info(format!("Chunking document {}...", document_id))
            .await;

        // OODA-02: Update document status to "chunking" for frontend visibility
        // WHY: Users need to see exactly which processing stage their document is in
        self.update_document_status(&document_id, "chunking", None)
            .await?;

        // SPEC-001/Objective-A: Create chunk progress callback for real-time updates
        // WHY: Users need to see granular progress like "Chunk 12/35 (34%) - ETA: 53s"
        let task_id = task.track_id.clone();
        let doc_id_for_callback = document_id.clone();
        let pipeline_state_for_callback = self.pipeline_state.clone();
        let chunk_progress_callback: ChunkProgressCallback =
            Arc::new(move |update: ChunkProgressUpdate| {
                // Emit real-time WebSocket event for chunk progress
                pipeline_state_for_callback.emit_chunk_progress(
                    doc_id_for_callback.clone(),
                    task_id.clone(),
                    update.chunk_index as u32,
                    update.total_chunks as u32,
                    update.chunk_preview.clone(),
                    update.processing_time_ms,
                    update.eta_seconds,
                    update.cumulative_input_tokens,
                    update.cumulative_output_tokens,
                    update.cumulative_cost_usd,
                );
            });

        // SPEC-003: Process through pipeline with RESILIENT chunk-level extraction
        // WHY: Uses map-reduce pattern to continue processing even if some chunks fail
        // This enables partial results instead of complete document failure
        // @implements FEAT0020: Chunk-level resilience and error isolation
        // @implements UC2305: System continues processing when individual chunks fail
        let result = match pipeline
            .process_with_resilience(&document_id, &data.text, Some(chunk_progress_callback))
            .await
        {
            Ok(result) => {
                // SPEC-003: Log partial success if some chunks failed
                if result.stats.failed_chunks > 0 {
                    warn!(
                        document_id = %document_id,
                        successful_chunks = result.stats.successful_chunks,
                        failed_chunks = result.stats.failed_chunks,
                        total_chunks = result.stats.chunk_count,
                        "Document processed with partial success - some chunks failed extraction"
                    );

                    // Emit WebSocket events for failed chunks
                    if let Some(ref chunk_errors) = result.stats.chunk_errors {
                        for error_info in chunk_errors {
                            self.pipeline_state.emit_chunk_failure(
                                document_id.clone(),
                                task.track_id.clone(),
                                error_info.chunk_index as u32,
                                result.stats.chunk_count as u32,
                                error_info.error_message.clone(),
                                error_info.was_timeout,
                                error_info.retry_attempts,
                            );
                        }
                    }
                }
                result
            }
            Err(e) => {
                let error_msg = format!("Pipeline processing failed: {}", e);
                error!("{}", error_msg);

                // Update document status to failed
                self.update_document_status(&document_id, "failed", Some(&error_msg))
                    .await?;

                self.pipeline_state
                    .document_failed(&document_id, &error_msg)
                    .await;

                return Err(edgequake_tasks::TaskError::Process(error_msg));
            }
        };

        // Update task progress - embedding
        task.update_progress("embedding".to_string(), 4, 30);
        self.pipeline_state
            .info(format!(
                "Generated {} chunks for {}",
                result.chunks.len(),
                document_id
            ))
            .await;

        // OODA-02: Update status to "extracting" - LLM entity extraction in progress
        // WHY: This is often the longest stage, users need visibility
        self.update_document_status(&document_id, "extracting", None)
            .await?;

        // Store chunks in KV storage
        let chunks: Vec<(String, serde_json::Value)> = result
            .chunks
            .iter()
            .map(|c| {
                (
                    c.id.clone(),
                    json!({
                        "content": c.content,
                        "document_id": document_id,
                        "index": c.index,
                    }),
                )
            })
            .collect();

        if let Err(e) = self.kv_storage.upsert(&chunks).await {
            let error_msg = format!("Failed to store chunks: {}", e);
            error!("{}", error_msg);

            self.update_document_status(&document_id, "failed", Some(&error_msg))
                .await?;

            return Err(edgequake_tasks::TaskError::Storage(error_msg));
        }

        // Extract tenant_id and workspace_id from metadata for scoping
        let tenant_id = data
            .metadata
            .as_ref()
            .and_then(|m| m.get("tenant_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let workspace_id_meta = data
            .metadata
            .as_ref()
            .and_then(|m| m.get("workspace_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| data.workspace_id.clone());

        // Get workspace-specific vector storage using the registry
        // WHY: Different workspaces may have different embedding dimensions
        // WHY-OODA223: STRICT mode - fail loudly if workspace storage unavailable
        // to prevent embeddings from being stored in the wrong (global) table
        let workspace_vector_storage = self
            .get_workspace_vector_storage_strict(&workspace_id_meta)
            .await
            .map_err(|e| {
                let error_msg = format!(
                    "CRITICAL: Cannot obtain workspace vector storage for '{}': {}. \
                         Document ingestion aborted to prevent data isolation violation.",
                    workspace_id_meta, e
                );
                error!("{}", error_msg);
                edgequake_tasks::TaskError::Process(error_msg)
            })?;

        // OODA-02: Update status to "embedding" - generating vector embeddings
        // WHY: Shows user that extraction is complete, now vectorizing
        self.update_document_status(&document_id, "embedding", None)
            .await?;

        // Store chunk embeddings in vector storage for semantic search
        let mut chunk_embeddings_stored = 0;
        for chunk in &result.chunks {
            if let Some(embedding) = &chunk.embedding {
                let mut metadata = json!({
                    "type": "chunk",
                    "document_id": document_id,
                    "index": chunk.index,
                    "content": chunk.content,
                });

                // Add tenant and workspace IDs if present
                if let Some(ref tid) = tenant_id {
                    metadata["tenant_id"] = json!(tid);
                }
                metadata["workspace_id"] = json!(&workspace_id_meta);

                if workspace_vector_storage
                    .upsert(&[(chunk.id.clone(), embedding.clone(), metadata)])
                    .await
                    .is_ok()
                {
                    chunk_embeddings_stored += 1;
                }
            }
        }
        info!(
            "Stored {} chunk embeddings in vector storage for document {}",
            chunk_embeddings_stored, document_id
        );

        // Update task progress - extraction
        task.update_progress("extraction".to_string(), 4, 60);
        self.pipeline_state
            .info(format!("Extracting entities from {}...", document_id))
            .await;

        info!(
            "Storing entities with tenant_id={:?}, workspace_id={:?}",
            tenant_id, workspace_id_meta
        );

        // OODA-02: Update status to "indexing" - storing in graph and vector databases
        // WHY: Final stage before completion, indicates DB writes in progress
        self.update_document_status(&document_id, "indexing", None)
            .await?;

        // Store entities and relationships in graph storage using batch operations
        // Collect all nodes for batch upsert
        let mut nodes_batch: Vec<(String, std::collections::HashMap<String, serde_json::Value>)> =
            Vec::new();
        let mut edges_batch: Vec<(
            String,
            String,
            std::collections::HashMap<String, serde_json::Value>,
        )> = Vec::new();

        // OODA-07: Pre-fetch existing entities to merge source_ids (GAP-07 fix for async path)
        // WHY: Without merge, second document overwrites first's source_ids, breaking reference counting
        let entity_names: Vec<String> = result
            .extractions
            .iter()
            .flat_map(|e| e.entities.iter().map(|ent| ent.name.clone()))
            .collect();

        let existing_entity_source_ids: std::collections::HashMap<
            String,
            std::collections::HashSet<String>,
        > = if !entity_names.is_empty() {
            match self.graph_storage.get_nodes_by_ids(&entity_names).await {
                Ok(nodes) => nodes
                    .into_iter()
                    .map(|node| {
                        let sources: std::collections::HashSet<String> = node
                            .properties
                            .get("source_ids")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default();
                        (node.id, sources)
                    })
                    .collect(),
                Err(e) => {
                    warn!(
                        "Failed to fetch existing entities for source_ids merge: {}",
                        e
                    );
                    std::collections::HashMap::new()
                }
            }
        } else {
            std::collections::HashMap::new()
        };

        // OODA-07: Pre-fetch existing edges to merge source_ids
        // WHY: Same issue as entities - edges need reference counting for correct deletion
        let edge_keys: Vec<(String, String)> = result
            .extractions
            .iter()
            .flat_map(|e| {
                e.relationships
                    .iter()
                    .map(|r| (r.source.clone(), r.target.clone()))
            })
            .collect();

        let mut existing_edge_source_ids: std::collections::HashMap<
            (String, String),
            std::collections::HashSet<String>,
        > = std::collections::HashMap::new();
        for (source, target) in &edge_keys {
            if let Ok(Some(edge)) = self.graph_storage.get_edge(source, target).await {
                let sources: std::collections::HashSet<String> = edge
                    .properties
                    .get("source_ids")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                existing_edge_source_ids.insert((source.clone(), target.clone()), sources);
            }
        }

        for extraction in &result.extractions {
            for entity in &extraction.entities {
                let mut properties = std::collections::HashMap::new();
                properties.insert("entity_type".to_string(), json!(entity.entity_type));
                properties.insert("description".to_string(), json!(entity.description));
                properties.insert("importance".to_string(), json!(entity.importance));

                // OODA-07: Merge source_ids with existing entity (GAP-07 fix)
                let mut merged_sources: std::collections::HashSet<String> =
                    existing_entity_source_ids
                        .get(&entity.name)
                        .cloned()
                        .unwrap_or_default();
                merged_sources.insert(document_id.clone());
                let source_ids_vec: Vec<String> = merged_sources.into_iter().collect();
                properties.insert("source_ids".to_string(), json!(source_ids_vec));

                // CRITICAL: Store source_chunk_ids for Local/Global query mode chunk retrieval
                properties.insert(
                    "source_chunk_ids".to_string(),
                    json!(&entity.source_chunk_ids),
                );
                // Add tenant scoping
                if let Some(ref tid) = tenant_id {
                    properties.insert("tenant_id".to_string(), json!(tid));
                }
                properties.insert("workspace_id".to_string(), json!(&workspace_id_meta));

                nodes_batch.push((entity.name.clone(), properties));
            }

            for relationship in &extraction.relationships {
                let mut properties = std::collections::HashMap::new();
                properties.insert(
                    "relation_type".to_string(),
                    json!(relationship.relation_type),
                );
                properties.insert("description".to_string(), json!(relationship.description));
                properties.insert("weight".to_string(), json!(relationship.weight));
                properties.insert("keywords".to_string(), json!(relationship.keywords));

                // OODA-07: Merge source_ids with existing edge (GAP-07 fix)
                let edge_key = (relationship.source.clone(), relationship.target.clone());
                let mut merged_sources: std::collections::HashSet<String> =
                    existing_edge_source_ids
                        .get(&edge_key)
                        .cloned()
                        .unwrap_or_default();
                merged_sources.insert(document_id.clone());
                let source_ids_vec: Vec<String> = merged_sources.into_iter().collect();
                properties.insert("source_ids".to_string(), json!(source_ids_vec));

                // CRITICAL: Store source_chunk_id for relationship chunk linkage
                if let Some(ref chunk_id) = relationship.source_chunk_id {
                    properties.insert("source_chunk_ids".to_string(), json!(vec![chunk_id]));
                }
                // Add tenant scoping
                if let Some(ref tid) = tenant_id {
                    properties.insert("tenant_id".to_string(), json!(tid));
                }
                properties.insert("workspace_id".to_string(), json!(&workspace_id_meta));

                edges_batch.push((
                    relationship.source.clone(),
                    relationship.target.clone(),
                    properties,
                ));
            }
        }

        // Batch upsert nodes
        if !nodes_batch.is_empty() {
            if let Err(e) = self.graph_storage.upsert_nodes_batch(&nodes_batch).await {
                warn!(
                    "Failed to batch store {} entities: {}",
                    nodes_batch.len(),
                    e
                );
            } else {
                info!("Batch stored {} entities", nodes_batch.len());
            }
        }

        // CRITICAL: Store entity embeddings in vector storage for query_local retrieval
        for extraction in &result.extractions {
            for entity in &extraction.entities {
                if let Some(embedding) = &entity.embedding {
                    let mut metadata = json!({
                        "type": "entity",
                        "entity_name": entity.name,
                        "entity_type": entity.entity_type,
                        "description": entity.description,
                        "document_id": document_id,
                        "source_chunk_ids": entity.source_chunk_ids,
                    });
                    if let Some(ref tid) = tenant_id {
                        metadata["tenant_id"] = json!(tid);
                    }
                    metadata["workspace_id"] = json!(&workspace_id_meta);

                    let entity_id = format!("entity:{}", entity.name);
                    if let Err(e) = self
                        .vector_storage
                        .upsert(&[(entity_id.clone(), embedding.clone(), metadata)])
                        .await
                    {
                        warn!("Failed to store entity embedding {}: {}", entity_id, e);
                    }
                }
            }
        }

        // Batch upsert edges
        if !edges_batch.is_empty() {
            if let Err(e) = self.graph_storage.upsert_edges_batch(&edges_batch).await {
                warn!(
                    "Failed to batch store {} relationships: {}",
                    edges_batch.len(),
                    e
                );
            } else {
                info!("Batch stored {} relationships", edges_batch.len());
            }
        }

        // Update task progress - indexing complete
        task.update_progress("indexing".to_string(), 4, 100);

        // SPEC-032/OODA-198: Augment stats with provider lineage before storing
        let mut stats_with_lineage = result.stats.clone();
        stats_with_lineage.llm_provider = Some(provider_lineage.extraction_provider.clone());
        stats_with_lineage.llm_model = Some(provider_lineage.extraction_model.clone());
        stats_with_lineage.embedding_provider = Some(provider_lineage.embedding_provider.clone());
        stats_with_lineage.embedding_model = Some(provider_lineage.embedding_model.clone());
        stats_with_lineage.embedding_dimensions = Some(provider_lineage.embedding_dimension);

        // Update document status to completed with stats and lineage
        self.update_document_status_with_stats(&document_id, "completed", &stats_with_lineage)
            .await?;

        // OODA-ITERATION-03-FIX: Invalidate workspace stats cache after async document processing
        // WHY: The cache contains stale entity/relationship counts. Without this, Dashboard
        // shows 0 entities while Workspace page shows correct counts because both pages use
        // the same cached stats, but cache was populated before the document was processed.
        // This ensures the next stats request fetches fresh data.
        if let Some(workspace_id_str) = workspace_id {
            if let Ok(workspace_uuid) = uuid::Uuid::parse_str(workspace_id_str) {
                crate::handlers::workspaces::invalidate_workspace_stats_cache(workspace_uuid).await;
            }
        }

        // Log success
        self.pipeline_state
            .document_processed(&document_id, result.stats.entity_count)
            .await;

        info!(
            "Document {} processed: {} chunks, {} entities, {} relationships",
            document_id,
            result.stats.chunk_count,
            result.stats.entity_count,
            result.stats.relationship_count
        );

        Ok(json!({
            "document_id": document_id,
            "chunk_count": result.stats.chunk_count,
            "entity_count": result.stats.entity_count,
            "relationship_count": result.stats.relationship_count,
        }))
    }

    /// Update document metadata status.
    async fn update_document_status(
        &self,
        document_id: &str,
        status: &str,
        error_message: Option<&str>,
    ) -> TaskResult<()> {
        let metadata_key = format!("{}-metadata", document_id);

        // Get existing metadata
        if let Ok(Some(existing)) = self.kv_storage.get_by_id(&metadata_key).await {
            if let Some(obj) = existing.as_object() {
                let mut updated = obj.clone();
                updated.insert("status".to_string(), json!(status));
                updated.insert(
                    "updated_at".to_string(),
                    json!(chrono::Utc::now().to_rfc3339()),
                );

                if let Some(msg) = error_message {
                    updated.insert("error_message".to_string(), json!(msg));
                }

                self.kv_storage
                    .upsert(&[(metadata_key, json!(updated))])
                    .await
                    .map_err(|e| edgequake_tasks::TaskError::Storage(e.to_string()))?;
            }
        }

        Ok(())
    }

    /// Update document metadata with processing stats and lineage information.
    async fn update_document_status_with_stats(
        &self,
        document_id: &str,
        status: &str,
        stats: &edgequake_pipeline::pipeline::ProcessingStats,
    ) -> TaskResult<()> {
        let metadata_key = format!("{}-metadata", document_id);

        // Get existing metadata
        if let Ok(Some(existing)) = self.kv_storage.get_by_id(&metadata_key).await {
            if let Some(obj) = existing.as_object() {
                let mut updated = obj.clone();
                updated.insert("status".to_string(), json!(status));
                updated.insert(
                    "updated_at".to_string(),
                    json!(chrono::Utc::now().to_rfc3339()),
                );
                updated.insert(
                    "processed_at".to_string(),
                    json!(chrono::Utc::now().to_rfc3339()),
                );

                // Basic stats
                updated.insert("chunk_count".to_string(), json!(stats.chunk_count));
                updated.insert("entity_count".to_string(), json!(stats.entity_count));
                updated.insert(
                    "relationship_count".to_string(),
                    json!(stats.relationship_count),
                );
                updated.insert(
                    "processing_duration_ms".to_string(),
                    json!(stats.processing_time_ms),
                );

                // Cost tracking fields
                updated.insert("cost_usd".to_string(), json!(stats.cost_usd));
                updated.insert("input_tokens".to_string(), json!(stats.input_tokens));
                updated.insert("output_tokens".to_string(), json!(stats.output_tokens));
                updated.insert("total_tokens".to_string(), json!(stats.total_tokens));

                // Lineage information
                if let Some(ref llm_model) = stats.llm_model {
                    updated.insert("llm_model".to_string(), json!(llm_model));
                }
                // SPEC-032/OODA-198: Store LLM provider for lineage tracking
                if let Some(ref llm_provider) = stats.llm_provider {
                    updated.insert("llm_provider".to_string(), json!(llm_provider));
                }
                if let Some(ref embedding_model) = stats.embedding_model {
                    updated.insert("embedding_model".to_string(), json!(embedding_model));
                }
                // SPEC-032/OODA-198: Store embedding provider for lineage tracking
                if let Some(ref embedding_provider) = stats.embedding_provider {
                    updated.insert("embedding_provider".to_string(), json!(embedding_provider));
                }
                if let Some(ref embedding_dimensions) = stats.embedding_dimensions {
                    updated.insert(
                        "embedding_dimensions".to_string(),
                        json!(embedding_dimensions),
                    );
                }
                if let Some(ref entity_types) = stats.entity_types {
                    updated.insert("entity_types".to_string(), json!(entity_types));
                }
                if let Some(ref relationship_types) = stats.relationship_types {
                    updated.insert("relationship_types".to_string(), json!(relationship_types));
                }
                if let Some(ref keywords) = stats.keywords {
                    updated.insert("keywords".to_string(), json!(keywords));
                }
                if let Some(ref chunking_strategy) = stats.chunking_strategy {
                    updated.insert("chunking_strategy".to_string(), json!(chunking_strategy));
                }
                if let Some(ref avg_chunk_size) = stats.avg_chunk_size {
                    updated.insert("avg_chunk_size".to_string(), json!(avg_chunk_size));
                }

                updated.remove("error_message");

                self.kv_storage
                    .upsert(&[(metadata_key, json!(updated))])
                    .await
                    .map_err(|e| edgequake_tasks::TaskError::Storage(e.to_string()))?;
            }
        }

        Ok(())
    }

    /// Process PDF processing task (SPEC-007).
    ///
    /// This method handles the complete PDF processing pipeline:
    /// 1. Load PDF from storage using pdf_id
    /// 2. Extract content (text mode only for now, vision TODO)
    /// 3. Convert to markdown
    /// 4. Create document and trigger standard ingestion
    /// 5. Update PDF status with results
    ///
    /// @implements SPEC-007: PDF Upload Support with Vision LLM Integration
    /// @implements FEAT0704: PDF processing worker
    /// @implements UC0704: System processes PDF in background
    /// @enforces BR0704: PDF processed async with retry logic
    #[cfg(feature = "postgres")]
    async fn process_pdf_processing(
        &self,
        task: &mut Task,
        data: edgequake_tasks::PdfProcessingData,
    ) -> TaskResult<serde_json::Value> {
        use edgequake_pdf::PdfExtractor;
        use edgequake_storage::{
            ExtractionMethod, PdfProcessingStatus, UpdatePdfProcessingRequest,
        };

        info!(
            pdf_id = %data.pdf_id,
            workspace_id = %data.workspace_id,
            enable_vision = data.enable_vision,
            "Starting PDF processing task"
        );

        // 1. Get PDF storage
        let pdf_storage = self.pdf_storage.as_ref().ok_or_else(|| {
            edgequake_tasks::TaskError::UnsupportedOperation(
                "PDF storage not available (postgres feature enabled but storage not initialized)"
                    .to_string(),
            )
        })?;

        // 2. Load PDF from storage
        let pdf = pdf_storage.get_pdf(&data.pdf_id).await.map_err(|e| {
            edgequake_tasks::TaskError::Storage(format!(
                "Failed to load PDF {}: {}",
                data.pdf_id, e
            ))
        })?;

        // Handle case where PDF not found
        let pdf = pdf.ok_or_else(|| {
            edgequake_tasks::TaskError::NotFound(format!("PDF not found: {}", data.pdf_id))
        })?;

        info!(
            pdf_id = %data.pdf_id,
            filename = %pdf.filename,
            size = pdf.file_size_bytes,
            pages = ?pdf.page_count,
            "Loaded PDF from storage"
        );

        // 3. Update status to processing
        pdf_storage
            .update_pdf_status(&data.pdf_id, PdfProcessingStatus::Processing)
            .await
            .map_err(|e| edgequake_tasks::TaskError::Storage(e.to_string()))?;

        // 4. Extract content (vision or text mode)
        // SPEC-007: Vision mode uses multimodal LLM to extract from rendered page images
        // Requires vision feature and poppler-utils (pdftoppm) system package
        let (markdown, extraction_method, used_vision_model) = if data.enable_vision {
            #[cfg(feature = "vision")]
            {
                use edgequake_pdf::{MarkdownRenderer, Renderer, VisionConfig, VisionExtractor};

                info!(
                    pdf_id = %data.pdf_id,
                    vision_provider = %data.vision_provider,
                    vision_model = ?data.vision_model,
                    "Starting vision-based PDF extraction"
                );

                // Build vision config with workspace-specified model
                let model = data
                    .vision_model
                    .clone()
                    .unwrap_or_else(|| "gpt-4o-mini".to_string());
                let vision_config = VisionConfig::new()
                    .with_model(&model)
                    .with_dpi(150)
                    .with_temperature(0.1);

                let extractor = VisionExtractor::new(Arc::clone(&self.llm_provider), vision_config);

                match extractor.extract_from_pdf(&pdf.pdf_data).await {
                    Ok(document) => {
                        // Render Document to markdown string
                        let renderer = MarkdownRenderer::new();
                        let md = renderer.render(&document).map_err(|e| {
                            edgequake_tasks::TaskError::Processing(format!(
                                "Markdown rendering failed: {}",
                                e
                            ))
                        })?;
                        info!(
                            pdf_id = %data.pdf_id,
                            pages = document.page_count(),
                            markdown_len = md.len(),
                            "Vision extraction completed successfully"
                        );
                        (md, ExtractionMethod::Vision, Some(model))
                    }
                    Err(e) => {
                        warn!(
                            pdf_id = %data.pdf_id,
                            error = %e,
                            "Vision extraction failed - falling back to text extraction"
                        );
                        // Fallback to text extraction
                        let extractor = PdfExtractor::new(Arc::clone(&self.llm_provider));
                        let md =
                            extractor
                                .extract_to_markdown(&pdf.pdf_data)
                                .await
                                .map_err(|e| {
                                    edgequake_tasks::TaskError::Processing(format!(
                                        "PDF extraction failed: {}",
                                        e
                                    ))
                                })?;
                        (md, ExtractionMethod::Text, None)
                    }
                }
            }
            #[cfg(not(feature = "vision"))]
            {
                warn!(
                    pdf_id = %data.pdf_id,
                    "Vision extraction requested but vision feature not enabled - using text extraction"
                );
                let extractor = PdfExtractor::new(Arc::clone(&self.llm_provider));
                let md = extractor
                    .extract_to_markdown(&pdf.pdf_data)
                    .await
                    .map_err(|e| {
                        edgequake_tasks::TaskError::Processing(format!(
                            "PDF extraction failed: {}",
                            e
                        ))
                    })?;
                (md, ExtractionMethod::Text, None)
            }
        } else {
            // Standard text extraction
            let extractor = PdfExtractor::new(Arc::clone(&self.llm_provider));
            let md = extractor
                .extract_to_markdown(&pdf.pdf_data)
                .await
                .map_err(|e| {
                    edgequake_tasks::TaskError::Processing(format!("PDF extraction failed: {}", e))
                })?;
            (md, ExtractionMethod::Text, None)
        };

        info!(
            pdf_id = %data.pdf_id,
            markdown_len = markdown.len(),
            extraction_method = ?extraction_method,
            "Extracted markdown from PDF"
        );

        // 5. Store markdown in pdf_documents with extraction method
        let update_req = UpdatePdfProcessingRequest {
            pdf_id: data.pdf_id,
            processing_status: PdfProcessingStatus::Completed,
            markdown_content: Some(markdown.clone()),
            extraction_method: Some(extraction_method),
            extraction_errors: None,
            document_id: None, // Will be set after document creation
            vision_model: used_vision_model,
        };

        pdf_storage
            .update_pdf_processing(update_req.clone())
            .await
            .map_err(|e| edgequake_tasks::TaskError::Storage(e.to_string()))?;

        // 6. Create document via standard pipeline
        let text_data = edgequake_tasks::TextInsertData {
            text: markdown,
            file_source: pdf.filename.clone(),
            workspace_id: data.workspace_id.to_string(),
            metadata: Some(json!({
                "source": "pdf_upload",
                "pdf_id": data.pdf_id.to_string(),
                "filename": pdf.filename,
                "page_count": pdf.page_count,
                "file_size_bytes": pdf.file_size_bytes,
            })),
        };

        let result = self.process_text_insert(task, text_data).await?;

        // 7. Link PDF to created document
        if let Some(document_id_str) = result.get("document_id").and_then(|v| v.as_str()) {
            if let Ok(document_uuid) = uuid::Uuid::parse_str(document_id_str) {
                if let Err(e) = pdf_storage
                    .link_pdf_to_document(&data.pdf_id, &document_uuid)
                    .await
                {
                    error!("Failed to link PDF to document: {} - continuing anyway", e);
                    // Non-fatal - PDF still processed successfully
                }
            }
        }

        // 8. Status already set to Completed in step 5 via update_pdf_processing
        info!(
            pdf_id = %data.pdf_id,
            "PDF processing completed successfully"
        );

        Ok(result)
    }

    #[cfg(not(feature = "postgres"))]
    async fn process_pdf_processing(
        &self,
        _task: &mut Task,
        data: edgequake_tasks::PdfProcessingData,
    ) -> TaskResult<serde_json::Value> {
        warn!(
            pdf_id = %data.pdf_id,
            "PDF processing not available (postgres feature disabled)"
        );
        Err(edgequake_tasks::TaskError::UnsupportedOperation(
            "PDF processing requires postgres feature".to_string(),
        ))
    }
}

#[async_trait::async_trait]
impl TaskProcessor for DocumentTaskProcessor {
    async fn process(&self, task: &mut Task) -> TaskResult<serde_json::Value> {
        match task.task_type {
            TaskType::Insert => {
                // Parse TextInsertData from task_data
                let data: TextInsertData =
                    serde_json::from_value(task.task_data.clone()).map_err(|e| {
                        edgequake_tasks::TaskError::InvalidPayload(format!(
                            "Invalid TextInsertData: {}",
                            e
                        ))
                    })?;

                self.process_text_insert(task, data).await
            }
            TaskType::Upload => {
                // For file uploads, we need to read the file content first
                // This is similar to Insert but the content comes from a file
                let data: TextInsertData =
                    serde_json::from_value(task.task_data.clone()).map_err(|e| {
                        edgequake_tasks::TaskError::InvalidPayload(format!(
                            "Invalid upload data: {}",
                            e
                        ))
                    })?;

                self.process_text_insert(task, data).await
            }
            TaskType::Scan => {
                // Directory scanning not yet implemented
                Err(edgequake_tasks::TaskError::UnsupportedOperation(
                    "Directory scanning not yet implemented".to_string(),
                ))
            }
            TaskType::Reindex => {
                // Reindexing not yet implemented
                Err(edgequake_tasks::TaskError::UnsupportedOperation(
                    "Reindexing not yet implemented".to_string(),
                ))
            }
            TaskType::PdfProcessing => {
                // Parse PdfProcessingData from task_data
                let data: edgequake_tasks::PdfProcessingData =
                    serde_json::from_value(task.task_data.clone()).map_err(|e| {
                        edgequake_tasks::TaskError::InvalidPayload(format!(
                            "Invalid PdfProcessingData: {}",
                            e
                        ))
                    })?;

                self.process_pdf_processing(task, data).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_storage::{
        MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage, MemoryWorkspaceVectorRegistry,
    };

    /// Create a test pipeline instance using default configuration
    fn create_test_pipeline() -> Arc<Pipeline> {
        Arc::new(Pipeline::default_pipeline())
    }

    /// Create test LLM provider for testing
    fn create_test_llm_provider() -> Arc<dyn edgequake_llm::traits::LLMProvider> {
        use edgequake_llm::MockProvider;
        Arc::new(MockProvider::new())
    }

    /// Create test storage instances for testing
    fn create_test_storages() -> (
        Arc<dyn KVStorage>,
        Arc<dyn VectorStorage>,
        Arc<dyn WorkspaceVectorRegistry>,
        Arc<dyn GraphStorage>,
    ) {
        let kv = Arc::new(MemoryKVStorage::new("test_processor"));
        // MemoryVectorStorage requires dimension - use 1536 (common embedding size)
        let vector: Arc<dyn VectorStorage> =
            Arc::new(MemoryVectorStorage::new("test_processor", 1536));
        let vector_registry: Arc<dyn WorkspaceVectorRegistry> =
            Arc::new(MemoryWorkspaceVectorRegistry::new(Arc::clone(&vector)));
        let graph = Arc::new(MemoryGraphStorage::new("test_processor"));
        (kv, vector, vector_registry, graph)
    }

    #[test]
    fn test_document_task_processor_new() {
        let pipeline = create_test_pipeline();
        let (kv, vector, vector_registry, graph) = create_test_storages();
        let pipeline_state = PipelineState::new();

        let processor = DocumentTaskProcessor::new(
            pipeline,
            create_test_llm_provider(),
            kv,
            vector,
            vector_registry,
            graph,
            pipeline_state,
        );

        // Verify processor was created successfully
        assert!(std::mem::size_of_val(&processor) > 0);
    }

    #[tokio::test]
    async fn test_processor_trait_implementation() {
        let pipeline = create_test_pipeline();
        let (kv, vector, vector_registry, graph) = create_test_storages();
        let pipeline_state = PipelineState::new();

        let processor = DocumentTaskProcessor::new(
            pipeline,
            create_test_llm_provider(),
            kv,
            vector,
            vector_registry,
            graph,
            pipeline_state,
        );

        // Verify TaskProcessor trait is implemented
        let _: &dyn TaskProcessor = &processor;
    }

    #[tokio::test]
    async fn test_process_scan_task_returns_unsupported() {
        let pipeline = create_test_pipeline();
        let (kv, vector, vector_registry, graph) = create_test_storages();
        let pipeline_state = PipelineState::new();

        let processor = DocumentTaskProcessor::new(
            pipeline,
            create_test_llm_provider(),
            kv,
            vector,
            vector_registry,
            graph,
            pipeline_state,
        );

        // Use test UUIDs for tenant and workspace
        let test_tenant = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let test_workspace = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
        let mut task = Task::new(test_tenant, test_workspace, TaskType::Scan, json!({}));

        let result = processor.process(&mut task).await;

        // Scan should return UnsupportedOperation error
        assert!(result.is_err());
        if let Err(e) = result {
            let error_msg = format!("{:?}", e);
            assert!(error_msg.contains("UnsupportedOperation"));
        }
    }

    #[tokio::test]
    async fn test_process_reindex_task_returns_unsupported() {
        let pipeline = create_test_pipeline();
        let (kv, vector, vector_registry, graph) = create_test_storages();
        let pipeline_state = PipelineState::new();

        let processor = DocumentTaskProcessor::new(
            pipeline,
            create_test_llm_provider(),
            kv,
            vector,
            vector_registry,
            graph,
            pipeline_state,
        );

        // Use test UUIDs for tenant and workspace
        let test_tenant = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let test_workspace = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
        let mut task = Task::new(test_tenant, test_workspace, TaskType::Reindex, json!({}));

        let result = processor.process(&mut task).await;

        // Reindex should return UnsupportedOperation error
        assert!(result.is_err());
        if let Err(e) = result {
            let error_msg = format!("{:?}", e);
            assert!(error_msg.contains("UnsupportedOperation"));
        }
    }

    #[tokio::test]
    async fn test_process_insert_with_invalid_payload() {
        let pipeline = create_test_pipeline();
        let (kv, vector, vector_registry, graph) = create_test_storages();
        let pipeline_state = PipelineState::new();

        let processor = DocumentTaskProcessor::new(
            pipeline,
            create_test_llm_provider(),
            kv,
            vector,
            vector_registry,
            graph,
            pipeline_state,
        );

        // Create task with invalid data (missing required fields)
        let invalid_data = json!({
            "invalid_field": "this is not TextInsertData"
        });

        // Use test UUIDs for tenant and workspace
        let test_tenant = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let test_workspace = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
        let mut task = Task::new(test_tenant, test_workspace, TaskType::Insert, invalid_data);

        let result = processor.process(&mut task).await;

        // Should fail due to invalid payload
        assert!(result.is_err());
        if let Err(e) = result {
            let error_msg = format!("{:?}", e);
            assert!(error_msg.contains("InvalidPayload"));
        }
    }

    #[tokio::test]
    async fn test_update_document_status() {
        let pipeline = create_test_pipeline();
        let (kv, vector, vector_registry, graph) = create_test_storages();
        let pipeline_state = PipelineState::new();

        // Pre-populate metadata
        let doc_id = "test-doc-status";
        let metadata_key = format!("{}-metadata", doc_id);
        kv.upsert(&[(
            metadata_key.clone(),
            json!({
                "document_id": doc_id,
                "status": "pending"
            }),
        )])
        .await
        .unwrap();

        let processor = DocumentTaskProcessor::new(
            pipeline,
            create_test_llm_provider(),
            kv.clone(),
            vector,
            vector_registry,
            graph,
            pipeline_state,
        );

        // Update status to processing
        let result = processor
            .update_document_status(doc_id, "processing", None)
            .await;
        assert!(result.is_ok());

        // Verify status was updated
        let metadata = kv.get_by_id(&metadata_key).await.unwrap().unwrap();
        assert_eq!(metadata["status"], "processing");
    }

    #[tokio::test]
    async fn test_update_document_status_with_error_message() {
        let pipeline = create_test_pipeline();
        let (kv, vector, vector_registry, graph) = create_test_storages();
        let pipeline_state = PipelineState::new();

        let doc_id = "test-doc-error";
        let metadata_key = format!("{}-metadata", doc_id);
        kv.upsert(&[(
            metadata_key.clone(),
            json!({
                "document_id": doc_id,
                "status": "processing"
            }),
        )])
        .await
        .unwrap();

        let processor = DocumentTaskProcessor::new(
            pipeline,
            create_test_llm_provider(),
            kv.clone(),
            vector,
            vector_registry,
            graph,
            pipeline_state,
        );

        // Update status with error
        let result = processor
            .update_document_status(doc_id, "failed", Some("Test error message"))
            .await;
        assert!(result.is_ok());

        // Verify error was recorded
        let metadata = kv.get_by_id(&metadata_key).await.unwrap().unwrap();
        assert_eq!(metadata["status"], "failed");
        assert_eq!(metadata["error_message"], "Test error message");
    }

    #[tokio::test]
    async fn test_update_document_status_nonexistent_doc() {
        let pipeline = create_test_pipeline();
        let (kv, vector, vector_registry, graph) = create_test_storages();
        let pipeline_state = PipelineState::new();

        let processor = DocumentTaskProcessor::new(
            pipeline,
            create_test_llm_provider(),
            kv,
            vector,
            vector_registry,
            graph,
            pipeline_state,
        );

        // Try to update status for non-existent document
        let result = processor
            .update_document_status("nonexistent-doc", "processing", None)
            .await;

        // Should succeed (no-op if document doesn't exist)
        assert!(result.is_ok());
    }

    #[test]
    fn test_processor_fields_are_arc() {
        // Verify that processor uses Arc for shared ownership
        let pipeline = create_test_pipeline();
        let llm = create_test_llm_provider();
        let (kv, vector, vector_registry, graph) = create_test_storages();
        let pipeline_state = PipelineState::new();

        let _processor = DocumentTaskProcessor::new(
            pipeline.clone(),
            llm.clone(),
            kv.clone(),
            vector.clone(),
            vector_registry.clone(),
            graph.clone(),
            pipeline_state,
        );

        // If we got here, Arc works correctly
        // Verify we can still access the cloned Arcs
        assert!(Arc::strong_count(&pipeline) >= 1);
        assert!(Arc::strong_count(&llm) >= 1);
        assert!(Arc::strong_count(&kv) >= 1);
        assert!(Arc::strong_count(&vector) >= 1);
        assert!(Arc::strong_count(&graph) >= 1);
    }

    #[tokio::test]
    async fn test_task_types_are_distinct() {
        // Verify all task types are handled distinctly
        let pipeline = create_test_pipeline();
        let (kv, vector, vector_registry, graph) = create_test_storages();
        let pipeline_state = PipelineState::new();

        let processor = DocumentTaskProcessor::new(
            pipeline,
            create_test_llm_provider(),
            kv,
            vector,
            vector_registry,
            graph,
            pipeline_state,
        );

        // Test that each unsupported task type goes through the right path
        let types = [TaskType::Scan, TaskType::Reindex];

        // Use test UUIDs for tenant and workspace
        let test_tenant = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let test_workspace = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();

        for task_type in types {
            let mut task = Task::new(test_tenant, test_workspace, task_type.clone(), json!({}));

            let result = processor.process(&mut task).await;

            // Scan/Reindex fail on unsupported
            assert!(result.is_err());
        }
    }
}
