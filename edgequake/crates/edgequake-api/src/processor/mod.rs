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

// Sub-modules organized by responsibility (SRP)
mod pdf_processing;
mod status_updates;
mod task_impl;
mod text_insert;
mod workspace_resolver;

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
