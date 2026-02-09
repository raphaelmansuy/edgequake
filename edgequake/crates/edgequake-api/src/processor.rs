//! Document task processor for async document processing.
//!
//! This module implements the `TaskProcessor` trait to process document
//! upload tasks through the pipeline and update storage accordingly.
//!
//! # WHY: Pipeline Provider vs Query Provider
//!
//! This is the #1 source of confusion in EdgeQuake. There are TWO independent
//! LLM provider selection paths, and they produce interleaved log lines:
//!
//! ```text
//!  ┌─────────────────────────────────────────────────────────────────────┐
//!  │  CONCURRENT LOG INTERLEAVING (why users think query uses Ollama)    │
//!  │                                                                     │
//!  │  Time   Source      Log                                            │
//!  │  ─────  ──────────  ──────────────────────────────────────────     │
//!  │  03:38  QUERY       Resolved LLM provider=openai model=gpt-5-nano │
//!  │  03:38  QUERY       Using full config for streaming ...            │
//!  │  03:38  PIPELINE    Chunk extraction timed out, will retry ...     │
//!  │  03:38  PIPELINE    Ollama chat request: gemma3:latest   ◄── HERE │
//!  │  03:39  QUERY       Sent context event with 150 sources            │
//!  │                                                                     │
//!  │  The Ollama log is from a BACKGROUND pipeline task, not the query! │
//!  └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Pipeline Provider Selection Flow
//!
//! ```text
//!  Worker picks task from queue
//!       │
//!       ▼
//!  process_text_insert(task)
//!       │
//!       ├── Extract workspace_id from task metadata
//!       │
//!       ▼
//!  strict_workspace_mode?
//!       │
//!       ├── YES (production) ──► get_workspace_pipeline_strict()
//!       │                             │
//!       │                             ├── Lookup workspace in DB
//!       │                             ├── create_safe_llm_provider(ws.llm_provider, ws.llm_model)
//!       │                             ├── create_safe_embedding_provider(ws.embedding_*)
//!       │                             │
//!       │                             ├── Both OK? ──► Workspace Pipeline (correct provider)
//!       │                             └── Either fails? ──► TaskError (task fails clearly)
//!       │
//!       └── NO (legacy/test) ──► get_workspace_pipeline()
//!                                     │
//!                                     ├── Same workspace lookup...
//!                                     │
//!                                     ├── Both OK? ──► Workspace Pipeline (correct)
//!                                     └── Any failure? ──► DEFAULT pipeline (Ollama!)
//!                                                          ^ THIS is the silent bug
//! ```
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

use crate::handlers::websocket_types::ProgressBroadcaster;
#[cfg(feature = "postgres")]
use crate::pipeline_progress_callback::PipelineProgressCallback;
use crate::state::SharedWorkspaceService;
use edgequake_llm::ModelsConfig;
use edgequake_pipeline::{ChunkProgressCallback, ChunkProgressUpdate, LLMExtractor, Pipeline};
use edgequake_storage::traits::{GraphStorage, KVStorage, VectorStorage, WorkspaceVectorRegistry};
use edgequake_tasks::{
    PipelinePhase, PipelineState, Task, TaskError, TaskProcessor, TaskResult, TaskType,
    TextInsertData,
};
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
    /// Only used when postgres+vision features are enabled, but stored for future extensibility.
    #[allow(dead_code)]
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
    /// OODA-10: Progress broadcaster for WebSocket clients.
    /// WHY: PDF page progress needs to reach frontend via WebSocket.
    progress_broadcaster: Option<ProgressBroadcaster>,
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
            progress_broadcaster: None, // OODA-10: Added for WebSocket clients
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
    #[allow(clippy::too_many_arguments)]
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
            progress_broadcaster: None, // OODA-10: Added for WebSocket clients
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
    #[allow(clippy::too_many_arguments)]
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
            progress_broadcaster: None, // OODA-10: Added for WebSocket clients
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

    /// OODA-10: Set progress broadcaster for WebSocket event delivery.
    ///
    /// This enables PDF page progress events to be broadcast to connected
    /// WebSocket clients in real-time.
    pub fn with_progress_broadcaster(mut self, broadcaster: ProgressBroadcaster) -> Self {
        self.progress_broadcaster = Some(broadcaster);
        self
    }

    /// Get a workspace-specific pipeline if workspace_id is provided and valid.
    ///
    /// SPEC-032: Creates a new Pipeline instance configured with the workspace's
    /// LLM and embedding providers. Falls back to the default pipeline if:
    /// - No workspace_id provided
    /// - Workspace not found
    /// - Failed to create workspace-specific providers
    ///
    /// # WHY: Silent Fallback is Dangerous
    ///
    /// When this method falls back to `self.pipeline` (the server default, typically
    /// Ollama from auto-detection), documents get extracted with the WRONG provider.
    /// This produces confusing logs where Ollama appears even though the workspace
    /// is configured for OpenAI. Production code uses `get_workspace_pipeline_strict`
    /// instead, which fails the task explicitly.
    ///
    /// # WHY: This Method Still Exists
    ///
    /// Kept for backward compatibility in test/memory mode where strict workspace
    /// isolation isn't required. Production (PostgreSQL mode) always uses strict.
    async fn get_workspace_pipeline(&self, workspace_id: Option<&str>) -> Arc<Pipeline> {
        use edgequake_llm::ProviderFactory;

        info!(
            workspace_id = ?workspace_id,
            has_workspace_service = self.workspace_service.is_some(),
            has_models_config = self.models_config.is_some(),
            "[PIPELINE] SPEC-032: Getting pipeline for workspace"
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
                // @implements FEAT0780: Safety limits for LLM calls (DocumentTaskProcessor)
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
                            "[PIPELINE] SPEC-032: Using workspace-specific providers for document processing"
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
                    "Falling back to default pipeline due to provider creation failure. \
                     WHY: This means document extraction will use the SERVER DEFAULT provider (likely Ollama) \
                     instead of the workspace-configured provider. Check API keys and provider config."
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

    /// OODA-16: Strict variant that returns error instead of falling back.
    ///
    /// WHY: In production, silent fallback to default pipeline causes data to be
    /// processed with wrong providers (e.g., Ollama 768-dim instead of OpenAI 1536-dim).
    /// This strict method ensures tasks fail clearly when workspace providers can't be created.
    async fn get_workspace_pipeline_strict(
        &self,
        workspace_id: Option<&str>,
    ) -> Result<Arc<Pipeline>, String> {
        use edgequake_llm::ProviderFactory;

        info!(
            workspace_id = ?workspace_id,
            has_workspace_service = self.workspace_service.is_some(),
            has_models_config = self.models_config.is_some(),
            "[PIPELINE] OODA-16: Getting pipeline for workspace (STRICT mode)"
        );

        // If no workspace support configured, fail explicitly
        let (workspace_service, _models_config): (&SharedWorkspaceService, &Arc<ModelsConfig>) =
            match (&self.workspace_service, &self.models_config) {
                (Some(ws), Some(mc)) => (ws, mc),
                _ => {
                    return Err("OODA-16: No workspace support configured on processor".to_string());
                }
            };

        // If no workspace_id provided, fail explicitly
        let workspace_id = match workspace_id {
            Some(id) if !id.is_empty() && id != "default" => id,
            _ => {
                return Err(format!(
                    "OODA-16: Invalid workspace_id '{:?}' - must provide valid workspace ID in strict mode",
                    workspace_id
                ));
            }
        };

        // Parse workspace_id to UUID
        let workspace_uuid = uuid::Uuid::parse_str(workspace_id).map_err(|e| {
            format!(
                "OODA-16: Invalid workspace ID format '{}': {}",
                workspace_id, e
            )
        })?;

        // Look up workspace configuration
        let ws = workspace_service
            .get_workspace(workspace_uuid)
            .await
            .map_err(|e| {
                format!(
                    "OODA-16: Failed to lookup workspace '{}': {}",
                    workspace_id, e
                )
            })?
            .ok_or_else(|| {
                format!(
                    "OODA-16: Workspace '{}' not found in database",
                    workspace_id
                )
            })?;

        // Create workspace-specific LLM provider - FAIL on error
        let llm_provider =
            ProviderFactory::create_safe_llm_provider(&ws.llm_provider, &ws.llm_model).map_err(
                |e| {
                    format!(
                        "OODA-16: Failed to create LLM provider '{}' with model '{}': {}. \
                     Check if OPENAI_API_KEY is set for OpenAI providers.",
                        ws.llm_provider, ws.llm_model, e
                    )
                },
            )?;

        // Create workspace-specific embedding provider - FAIL on error
        let embedding_provider = ProviderFactory::create_safe_embedding_provider(
            &ws.embedding_provider,
            &ws.embedding_model,
            ws.embedding_dimension,
        )
        .map_err(|e| {
            format!(
                "OODA-16: Failed to create embedding provider '{}' with model '{}': {}. \
                 Check if OPENAI_API_KEY is set for OpenAI providers.",
                ws.embedding_provider, ws.embedding_model, e
            )
        })?;

        // SUCCESS: Both providers created
        info!(
            workspace_id = workspace_id,
            llm_provider = %ws.llm_provider,
            llm_model = %ws.llm_model,
            embedding_provider = %ws.embedding_provider,
            embedding_model = %ws.embedding_model,
            "[PIPELINE] OODA-16: Successfully created workspace-specific providers (STRICT mode)"
        );

        let extractor = Arc::new(LLMExtractor::new(Arc::clone(&llm_provider)));
        Ok(Arc::new(
            Pipeline::default_pipeline()
                .with_extractor(extractor)
                .with_embedding_provider(Arc::clone(&embedding_provider)),
        ))
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

        // SPEC-002: Extract source_type from task metadata for unified pipeline tracking
        let source_type = data
            .metadata
            .as_ref()
            .and_then(|m| m.get("source_type"))
            .and_then(|v| v.as_str())
            .unwrap_or("markdown") // Default to markdown for text uploads
            .to_string();

        // OODA-05: Extract tenant_id from metadata for multi-tenant visibility
        let tenant_id = data
            .metadata
            .as_ref()
            .and_then(|m| m.get("tenant_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // OODA-49: Extract pdf_id from metadata for PDF document viewing
        // WHY: PDF documents need pdf_id stored in metadata for the frontend to build download URLs
        let pdf_id = data
            .metadata
            .as_ref()
            .and_then(|m| m.get("pdf_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // SPEC-002: Ensure document metadata includes source_type
        // This is needed for PDFs that bypass the upload handler
        // OODA-05: Pass tenant_id/workspace_id for multi-tenant context
        // OODA-49: Pass pdf_id for PDF document viewing
        // OODA-ITERATION-03: Pass track_id for cancel button support
        self.ensure_document_source_type(
            &document_id,
            &source_type,
            tenant_id.as_deref(),
            Some(&data.workspace_id),
            pdf_id.as_deref(),
            Some(&task.track_id),
        )
        .await?;

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

        // OODA-16: Get workspace-specific pipeline with strict mode support
        // WHY: In strict mode, fail the task if workspace providers can't be created
        // instead of silently falling back to default (wrong dimensions, wrong provider)
        let pipeline = if self.strict_workspace_mode {
            match self.get_workspace_pipeline_strict(workspace_id).await {
                Ok(p) => p,
                Err(e) => {
                    error!(
                        document_id = %document_id,
                        workspace_id = ?workspace_id,
                        error = %e,
                        "OODA-16: Failed to create workspace pipeline in strict mode"
                    );
                    // Update document status to Failed with clear error message
                    let _ = self
                        .update_document_status(
                            &document_id,
                            "failed",
                            Some(&format!("Workspace provider error: {}", e)),
                        )
                        .await;
                    return Err(TaskError::Process(format!(
                        "Workspace pipeline error: {}",
                        e
                    )));
                }
            }
        } else {
            // Non-strict mode: fallback to default pipeline (legacy behavior)
            self.get_workspace_pipeline(workspace_id).await
        };

        // SPEC-032/OODA-198: Capture provider lineage for tracking
        let provider_lineage = self.get_workspace_provider_lineage(workspace_id).await;

        info!(
            document_id = %document_id,
            workspace_id = ?workspace_id,
            file_source = %data.file_source,
            extraction_provider = %provider_lineage.extraction_provider,
            extraction_model = %provider_lineage.extraction_model,
            embedding_provider = %provider_lineage.embedding_provider,
            "[PIPELINE] Processing document with workspace-specific pipeline"
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

        // OODA-17: Update PDF phase progress for PDF uploads
        // WHY: PDFs need all 6 phases tracked (Upload, PdfConversion, Chunking, Embedding, Extraction, GraphStorage)
        // The PdfConversion phase is tracked by PipelineProgressCallback, but remaining phases need explicit tracking
        let is_pdf_source = source_type == "pdf";
        let track_id = task.track_id.clone();
        if is_pdf_source {
            // Estimate: text length / 2000 chars per chunk (rough heuristic)
            let estimated_chunks = std::cmp::max(1, data.text.len() / 2000);
            self.pipeline_state
                .start_pdf_phase(&track_id, PipelinePhase::Chunking, estimated_chunks)
                .await;
        }

        // SPEC-001/Objective-A: Create chunk progress callback for real-time updates
        // WHY: Users need to see granular progress like "Chunk 12/35 (34%) - ETA: 53s"
        // OODA-PERF-01: Enhanced to update document metadata for UI polling fallback
        // WHY: If WebSocket fails, users still see extraction progress via metadata polling
        let task_id = task.track_id.clone();
        let doc_id_for_callback = document_id.clone();
        let doc_id_for_metadata = document_id.clone();
        let pipeline_state_for_callback = self.pipeline_state.clone();
        let kv_storage_for_callback = Arc::clone(&self.kv_storage);
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

                // OODA-PERF-01: Update document metadata every 3 chunks for UI polling
                // WHY: Reduce KV writes while maintaining visibility (update ~every 3-5 seconds)
                let should_update_metadata = update.chunk_index.is_multiple_of(3)
                    || update.chunk_index == update.total_chunks - 1;
                if should_update_metadata {
                    let doc_id_clone = doc_id_for_metadata.clone();
                    let kv_clone = Arc::clone(&kv_storage_for_callback);
                    let chunk_idx = update.chunk_index;
                    let total = update.total_chunks;

                    // Fire-and-forget metadata update to avoid blocking extraction
                    tokio::spawn(async move {
                        let metadata_key = format!("{}-metadata", doc_id_clone);
                        if let Ok(Some(existing)) = kv_clone.get_by_id(&metadata_key).await {
                            if let Some(obj) = existing.as_object() {
                                let mut updated = obj.clone();
                                let progress_pct =
                                    ((chunk_idx as f64 / total as f64) * 100.0).round() as u32;
                                updated.insert("current_stage".to_string(), json!("extracting"));
                                updated.insert(
                                    "stage_message".to_string(),
                                    json!(format!(
                                        "Extracting entities: chunk {}/{} ({}%)",
                                        chunk_idx + 1,
                                        total,
                                        progress_pct
                                    )),
                                );
                                updated.insert(
                                    "stage_progress".to_string(),
                                    json!(progress_pct as f64 / 100.0),
                                );
                                updated.insert(
                                    "updated_at".to_string(),
                                    json!(chrono::Utc::now().to_rfc3339()),
                                );

                                let _ = kv_clone.upsert(&[(metadata_key, json!(updated))]).await;
                            }
                        }
                    });
                }
            });

        // SPEC-003: Process through pipeline with RESILIENT chunk-level extraction
        // WHY: Uses map-reduce pattern to continue processing even if some chunks fail
        // This enables partial results instead of complete document failure
        // @implements FEAT0022: Chunk-level resilience and error isolation (processor)
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
                // FIX-3: Comprehensive error logging with context
                let error_msg = format!("Pipeline processing failed: {}", e);
                error!(
                    document_id = %document_id,
                    workspace_id = ?workspace_id,
                    tenant_id = ?tenant_id,
                    content_length = data.text.len(),
                    error = %e,
                    "CRITICAL: Pipeline processing failed - document marked as failed"
                );

                // Update document status to failed with detailed error
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

        // OODA-17: Update PDF phase progress - chunking complete, start extraction
        if is_pdf_source {
            self.pipeline_state
                .complete_pdf_phase(&track_id, PipelinePhase::Chunking)
                .await;
            // Extraction phase: estimate entity count from chunk count
            let estimated_entities = result.chunks.len() * 3; // ~3 entities per chunk heuristic
            self.pipeline_state
                .start_pdf_phase(&track_id, PipelinePhase::Extraction, estimated_entities)
                .await;
        }

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

        // OODA-17: Update PDF phase progress - extraction complete, start embedding
        if is_pdf_source {
            self.pipeline_state
                .complete_pdf_phase(&track_id, PipelinePhase::Extraction)
                .await;
            // Embedding phase: total = chunks to embed
            self.pipeline_state
                .start_pdf_phase(&track_id, PipelinePhase::Embedding, result.chunks.len())
                .await;
        }

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

        // OODA-17: Update PDF phase progress - embedding complete, start graph storage
        if is_pdf_source {
            self.pipeline_state
                .complete_pdf_phase(&track_id, PipelinePhase::Embedding)
                .await;
            // GraphStorage phase: estimate operations = entities + relationships
            let total_entities: usize = result.extractions.iter().map(|e| e.entities.len()).sum();
            let total_rels: usize = result
                .extractions
                .iter()
                .map(|e| e.relationships.len())
                .sum();
            self.pipeline_state
                .start_pdf_phase(
                    &track_id,
                    PipelinePhase::GraphStorage,
                    total_entities + total_rels,
                )
                .await;
        }

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
        // FIX: Use workspace_vector_storage instead of self.vector_storage to avoid
        // dimension mismatch (768 vs 1536) when workspace uses different embedding model
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
                    if let Err(e) = workspace_vector_storage
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

        // FIX-1: Validate processing results before marking completed
        // WHY: Prevent silent failures where status="completed" but entity_count=0
        // CRITICAL: This detects documents that went through pipeline but extracted nothing
        let final_status = if result.stats.entity_count == 0 && result.stats.chunk_count > 0 {
            // Pipeline created chunks but extracted 0 entities - likely LLM failure
            warn!(
                document_id = %document_id,
                chunk_count = result.stats.chunk_count,
                failed_chunks = result.stats.failed_chunks,
                "ANOMALY: Document processed but extracted 0 entities from {} chunks - marking as partial_failure",
                result.stats.chunk_count
            );
            "partial_failure"
        } else if result.stats.failed_chunks == result.stats.chunk_count
            && result.stats.chunk_count > 0
        {
            // ALL chunks failed extraction - complete failure
            error!(
                document_id = %document_id,
                chunk_count = result.stats.chunk_count,
                "CRITICAL: ALL {} chunks failed entity extraction - marking as failed",
                result.stats.chunk_count
            );
            "failed"
        } else if result.stats.chunk_count == 0 {
            // No chunks created at all - chunking failed
            error!(
                document_id = %document_id,
                content_length = data.text.len(),
                "CRITICAL: Document chunking produced 0 chunks - marking as failed"
            );
            "failed"
        } else {
            "completed"
        };

        // Update document status with validation
        self.update_document_status_with_stats(&document_id, final_status, &stats_with_lineage)
            .await?;

        // OODA-17: Update PDF phase progress - graph storage complete, all phases done
        if is_pdf_source {
            self.pipeline_state
                .complete_pdf_phase(&track_id, PipelinePhase::GraphStorage)
                .await;
            info!(
                track_id = %track_id,
                document_id = %document_id,
                "PDF pipeline phases complete: all 6 phases finished"
            );
        }

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
    ///
    /// @implements SPEC-002: Unified Ingestion Pipeline
    /// Updates both legacy `status` field and new `current_stage` field for backward compatibility.
    /// Creates metadata if it doesn't exist (for PDF documents that bypass upload handler).
    async fn update_document_status(
        &self,
        document_id: &str,
        status: &str,
        error_message: Option<&str>,
    ) -> TaskResult<()> {
        let metadata_key = format!("{}-metadata", document_id);

        // SPEC-002: Map legacy status names to unified stage names
        let unified_stage = match status {
            "pending" => "uploading",
            "processing" => "preprocessing",
            "chunking" => "chunking",
            "extracting" => "extracting",
            "embedding" => "embedding",
            "indexing" => "storing",
            "completed" | "indexed" => "completed",
            "failed" => "failed",
            other => other, // Pass through unknown statuses
        };

        // SPEC-002: Build stage message based on status
        let stage_message = match status {
            "pending" => "Document queued for processing",
            "processing" | "preprocessing" => "Preprocessing document...",
            "chunking" => "Splitting document into chunks...",
            "extracting" => "Extracting entities and relationships...",
            "embedding" => "Generating vector embeddings...",
            "indexing" | "storing" => "Storing in knowledge graph...",
            "completed" | "indexed" => "Processing complete",
            "failed" => "Processing failed",
            _ => "Processing...",
        };

        // Get existing metadata or create new
        let existing = self
            .kv_storage
            .get_by_id(&metadata_key)
            .await
            .ok()
            .flatten();

        let updated_json = if let Some(existing_val) = existing {
            if let Some(obj) = existing_val.as_object() {
                let mut updated = obj.clone();
                updated.insert("status".to_string(), json!(status));
                updated.insert(
                    "updated_at".to_string(),
                    json!(chrono::Utc::now().to_rfc3339()),
                );
                updated.insert("current_stage".to_string(), json!(unified_stage));
                updated.insert("stage_message".to_string(), json!(stage_message));

                if let Some(msg) = error_message {
                    updated.insert("error_message".to_string(), json!(msg));
                    updated.insert("stage_message".to_string(), json!(msg));
                }

                json!(updated)
            } else {
                return Ok(()); // Malformed metadata, skip update
            }
        } else {
            // SPEC-002: Create new metadata for documents that don't have it
            // This happens for PDFs that bypass the upload handler
            let mut new_metadata = serde_json::Map::new();
            new_metadata.insert("id".to_string(), json!(document_id));
            new_metadata.insert("status".to_string(), json!(status));
            new_metadata.insert("current_stage".to_string(), json!(unified_stage));
            new_metadata.insert("stage_message".to_string(), json!(stage_message));
            new_metadata.insert(
                "created_at".to_string(),
                json!(chrono::Utc::now().to_rfc3339()),
            );
            new_metadata.insert(
                "updated_at".to_string(),
                json!(chrono::Utc::now().to_rfc3339()),
            );
            // Note: source_type will be set later if available from task metadata

            if let Some(msg) = error_message {
                new_metadata.insert("error_message".to_string(), json!(msg));
                new_metadata.insert("stage_message".to_string(), json!(msg));
            }

            json!(new_metadata)
        };

        self.kv_storage
            .upsert(&[(metadata_key, updated_json)])
            .await
            .map_err(|e| edgequake_tasks::TaskError::Storage(e.to_string()))?;

        Ok(())
    }

    /// Ensure document metadata includes source_type.
    ///
    /// @implements SPEC-002: Unified Ingestion Pipeline
    /// Sets source_type (pdf, markdown, text) for unified pipeline display.
    /// Creates metadata if it doesn't exist (for PDFs that bypass upload handler).
    ///
    /// OODA-05: Added tenant_id/workspace_id parameters to ensure multi-tenant context
    /// is propagated when creating new document metadata. Without these fields,
    /// documents become invisible in workspace-filtered queries.
    ///
    /// OODA-49: Added pdf_id parameter for PDF documents to enable frontend PDF viewing.
    /// The pdf_id is a UUID that references the PDF binary stored in pdf_storage.
    ///
    /// OODA-ITERATION-03: Added track_id parameter for cancel button support.
    /// WHY: Frontend cancel button requires doc.track_id to call POST /tasks/{track_id}/cancel
    async fn ensure_document_source_type(
        &self,
        document_id: &str,
        source_type: &str,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
        pdf_id: Option<&str>,
        track_id: Option<&str>,
    ) -> TaskResult<()> {
        let metadata_key = format!("{}-metadata", document_id);

        // Get existing metadata or create new
        let existing = self
            .kv_storage
            .get_by_id(&metadata_key)
            .await
            .ok()
            .flatten();

        let updated_json = if let Some(existing_val) = existing {
            if let Some(obj) = existing_val.as_object() {
                // Only update if source_type is not already set
                if obj.get("source_type").is_none() {
                    let mut updated = obj.clone();
                    updated.insert("source_type".to_string(), json!(source_type));
                    updated.insert(
                        "updated_at".to_string(),
                        json!(chrono::Utc::now().to_rfc3339()),
                    );
                    // OODA-05: Also update tenant/workspace if missing
                    if obj.get("tenant_id").is_none() {
                        if let Some(tid) = tenant_id {
                            updated.insert("tenant_id".to_string(), json!(tid));
                        }
                    }
                    if obj.get("workspace_id").is_none() {
                        if let Some(wid) = workspace_id {
                            updated.insert("workspace_id".to_string(), json!(wid));
                        }
                    }
                    // OODA-49: Also update pdf_id if missing
                    // WHY: PDF documents need pdf_id for frontend to build download URLs
                    if obj.get("pdf_id").is_none() {
                        if let Some(pid) = pdf_id {
                            updated.insert("pdf_id".to_string(), json!(pid));
                        }
                    }
                    // OODA-ITERATION-03: Also update track_id if missing
                    // WHY: Cancel button requires track_id to call cancel API
                    if obj.get("track_id").is_none() {
                        if let Some(tid) = track_id {
                            updated.insert("track_id".to_string(), json!(tid));
                        }
                    }
                    Some(json!(updated))
                } else {
                    // OODA-49: Even if source_type is set, check if pdf_id needs to be added
                    // WHY: Fix existing documents that have source_type but missing pdf_id
                    let needs_pdf_id = pdf_id.is_some() && obj.get("pdf_id").is_none();
                    // OODA-ITERATION-03: Also check if track_id needs to be added
                    let needs_track_id = track_id.is_some() && obj.get("track_id").is_none();

                    if needs_pdf_id || needs_track_id {
                        let mut updated = obj.clone();
                        if let Some(pid) = pdf_id {
                            if obj.get("pdf_id").is_none() {
                                updated.insert("pdf_id".to_string(), json!(pid));
                            }
                        }
                        if let Some(tid) = track_id {
                            if obj.get("track_id").is_none() {
                                updated.insert("track_id".to_string(), json!(tid));
                            }
                        }
                        updated.insert(
                            "updated_at".to_string(),
                            json!(chrono::Utc::now().to_rfc3339()),
                        );
                        Some(json!(updated))
                    } else {
                        None // Already has source_type, pdf_id, and track_id (or not needed), skip update
                    }
                }
            } else {
                None // Malformed metadata, skip update
            }
        } else {
            // Create new metadata for documents that don't have it (e.g., PDFs)
            // OODA-05: Include tenant_id/workspace_id for multi-tenant visibility
            let mut new_metadata = serde_json::Map::new();
            new_metadata.insert("id".to_string(), json!(document_id));
            new_metadata.insert("source_type".to_string(), json!(source_type));
            new_metadata.insert("current_stage".to_string(), json!("preprocessing"));
            new_metadata.insert("stage_message".to_string(), json!("Processing document..."));
            new_metadata.insert("status".to_string(), json!("processing"));
            new_metadata.insert(
                "created_at".to_string(),
                json!(chrono::Utc::now().to_rfc3339()),
            );
            new_metadata.insert(
                "updated_at".to_string(),
                json!(chrono::Utc::now().to_rfc3339()),
            );
            // OODA-05: Critical - include tenant/workspace context
            if let Some(tid) = tenant_id {
                new_metadata.insert("tenant_id".to_string(), json!(tid));
            }
            if let Some(wid) = workspace_id {
                new_metadata.insert("workspace_id".to_string(), json!(wid));
            }
            // OODA-49: Include pdf_id for PDF documents
            // WHY: Frontend needs pdf_id to build download URLs for PDF viewing
            if let Some(pid) = pdf_id {
                new_metadata.insert("pdf_id".to_string(), json!(pid));
            }
            // OODA-ITERATION-03: Include track_id for cancel button support
            // WHY: Frontend cancel button requires doc.track_id to call POST /tasks/{track_id}/cancel
            if let Some(tid) = track_id {
                new_metadata.insert("track_id".to_string(), json!(tid));
            }
            Some(json!(new_metadata))
        };

        if let Some(json) = updated_json {
            self.kv_storage
                .upsert(&[(metadata_key, json)])
                .await
                .map_err(|e| edgequake_tasks::TaskError::Storage(e.to_string()))?;
        }

        Ok(())
    }

    /// Update document metadata with processing stats and lineage information.
    ///
    /// @implements SPEC-002: Unified Ingestion Pipeline
    /// Sets both legacy `status` and new `current_stage` fields.
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

                // SPEC-002: Set unified current_stage and stage_message for completion
                let unified_stage = if status == "completed" || status == "indexed" {
                    "completed"
                } else {
                    status
                };
                updated.insert("current_stage".to_string(), json!(unified_stage));
                updated.insert("stage_progress".to_string(), json!(1.0)); // 100% complete

                // SPEC-002: Informative completion message with stats
                let stage_message = format!(
                    "Processed {} chunks, extracted {} entities and {} relationships",
                    stats.chunk_count, stats.entity_count, stats.relationship_count
                );
                updated.insert("stage_message".to_string(), json!(stage_message));

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

        // 3.1 Create document metadata early with "converting" stage
        // WHY: Users need to see the document appear in the UI immediately with visual feedback
        // showing that PDF → Markdown conversion is happening.
        // OODA-ITERATION-03: Include track_id for cancel button support
        // WHY: Frontend cancel button requires doc.track_id to call POST /tasks/{track_id}/cancel
        let early_doc_id = uuid::Uuid::new_v4().to_string();
        let metadata_key = format!("{}-metadata", early_doc_id);
        let metadata_json = json!({
            "id": early_doc_id,
            "title": pdf.filename.clone(),
            "file_name": pdf.filename.clone(),
            "source_type": "pdf",
            "status": "processing",
            "current_stage": "converting",
            "stage_message": format!("Converting PDF to Markdown (0/{} pages)", pdf.page_count.unwrap_or(0)),
            "stage_progress": 0.0,
            "pdf_id": data.pdf_id.to_string(),
            "tenant_id": data.tenant_id.to_string(),
            "workspace_id": data.workspace_id.to_string(),
            "track_id": task.track_id.clone(),
            "created_at": chrono::Utc::now().to_rfc3339(),
            "updated_at": chrono::Utc::now().to_rfc3339(),
        });

        self.kv_storage
            .upsert(&[(metadata_key.clone(), metadata_json.clone())])
            .await
            .map_err(|e| edgequake_tasks::TaskError::Storage(e.to_string()))?;

        info!(
            document_id = %early_doc_id,
            pdf_id = %data.pdf_id,
            "Created early document metadata with 'converting' stage"
        );

        // OODA-09: Create progress callback for real-time page-by-page feedback
        // WHY: Users need to see extraction progress like "Extracting page 5/10..."
        // OODA-10: Also attach progress broadcaster if available for WebSocket delivery
        // OODA-16: Add filename for progress display
        let mut callback = PipelineProgressCallback::new(
            self.pipeline_state.clone(),
            data.pdf_id.to_string(),
            task.track_id.clone(),
        )
        .with_filename(pdf.filename.clone())
        .with_document_metadata(early_doc_id.clone(), Arc::clone(&self.kv_storage));

        if let Some(ref broadcaster) = self.progress_broadcaster {
            callback = callback.with_broadcaster(broadcaster.clone());
        }
        let progress_callback = Arc::new(callback);
        // OODA-09: Coerce to trait object for use with extract_to_markdown_with_progress
        let progress_callback: Arc<dyn edgequake_pdf::ProgressCallback> = progress_callback;

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

                // OODA-11: Use progress callback for vision extraction
                match extractor
                    .extract_from_pdf_with_progress(&pdf.pdf_data, Arc::clone(&progress_callback))
                    .await
                {
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
                        // Fallback to text extraction with progress callback (OODA-09)
                        let extractor = PdfExtractor::new(Arc::clone(&self.llm_provider));
                        let md = extractor
                            .extract_to_markdown_with_progress(
                                &pdf.pdf_data,
                                Arc::clone(&progress_callback),
                            )
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
                // OODA-09: Use progress callback for text extraction
                let extractor = PdfExtractor::new(Arc::clone(&self.llm_provider));
                let md = extractor
                    .extract_to_markdown_with_progress(
                        &pdf.pdf_data,
                        Arc::clone(&progress_callback),
                    )
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
            // Standard text extraction with progress callback (OODA-09)
            let extractor = PdfExtractor::new(Arc::clone(&self.llm_provider));
            let md = extractor
                .extract_to_markdown_with_progress(&pdf.pdf_data, Arc::clone(&progress_callback))
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
        // SPEC-002: Include source_type: "pdf" for unified pipeline tracking
        // OODA-05: Include tenant_id/workspace_id for multi-tenant document visibility
        // Pass the early_doc_id so we reuse the same document that's already showing in UI
        let text_data = edgequake_tasks::TextInsertData {
            text: markdown,
            file_source: pdf.filename.clone(),
            workspace_id: data.workspace_id.to_string(),
            metadata: Some(json!({
                "document_id": early_doc_id.clone(),  // Reuse early document ID
                "source": "pdf_upload",
                "source_type": "pdf",
                "pdf_id": data.pdf_id.to_string(),
                "filename": pdf.filename,
                "page_count": pdf.page_count,
                "file_size_bytes": pdf.file_size_bytes,
                "tenant_id": data.tenant_id.to_string(),
                "workspace_id": data.workspace_id.to_string(),
            })),
        };

        let result = self.process_text_insert(task, text_data).await?;

        // 7. Link PDF to created document (use early_doc_id)
        if let Ok(document_uuid) = uuid::Uuid::parse_str(&early_doc_id) {
            if let Err(e) = pdf_storage
                .link_pdf_to_document(&data.pdf_id, &document_uuid)
                .await
            {
                error!("Failed to link PDF to document: {} - continuing anyway", e);
                // Non-fatal - PDF still processed successfully
            }
        }

        // 8. Status already set to Completed in step 5 via update_pdf_processing
        info!(
            pdf_id = %data.pdf_id,
            "PDF processing completed successfully"
        );

        // OODA-16: Clean up progress tracking (fire-and-forget)
        // WHY: Free memory for completed uploads. GET endpoint will return 404.
        let state = self.pipeline_state.clone();
        let track_id = task.track_id.clone();
        tokio::spawn(async move {
            state.remove_pdf_progress(&track_id).await;
        });

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
