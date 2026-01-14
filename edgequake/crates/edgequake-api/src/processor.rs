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
use edgequake_pipeline::{LLMExtractor, Pipeline};
use edgequake_storage::traits::{GraphStorage, KVStorage, VectorStorage};
use edgequake_tasks::{PipelineState, Task, TaskProcessor, TaskResult, TaskType, TextInsertData};
use serde_json::json;
use tracing::{error, info, warn};

/// Document task processor that processes documents through the pipeline.
///
/// SPEC-032: This processor supports workspace-specific LLM and embedding providers.
/// When a task includes workspace_id in its metadata, the processor will:
/// 1. Look up the workspace configuration
/// 2. Create a workspace-specific pipeline with the configured providers
/// 3. Process the document using those providers
///
/// This ensures that rebuild/reprocess operations use the workspace's configured
/// models, not the server's default models.
pub struct DocumentTaskProcessor {
    /// Default processing pipeline (fallback when workspace not specified).
    pipeline: Arc<Pipeline>,
    /// KV storage for document metadata and chunks.
    kv_storage: Arc<dyn KVStorage>,
    /// Vector storage for chunk embeddings.
    vector_storage: Arc<dyn VectorStorage>,
    /// Graph storage for entities and relationships.
    graph_storage: Arc<dyn GraphStorage>,
    /// Pipeline state for progress tracking.
    pipeline_state: PipelineState,
    /// Workspace service for looking up workspace configuration (SPEC-032).
    workspace_service: Option<SharedWorkspaceService>,
    /// Models configuration for creating providers (SPEC-032).
    models_config: Option<Arc<ModelsConfig>>,
}

impl DocumentTaskProcessor {
    /// Create a new document task processor (legacy, without workspace support).
    pub fn new(
        pipeline: Arc<Pipeline>,
        kv_storage: Arc<dyn KVStorage>,
        vector_storage: Arc<dyn VectorStorage>,
        graph_storage: Arc<dyn GraphStorage>,
        pipeline_state: PipelineState,
    ) -> Self {
        Self {
            pipeline,
            kv_storage,
            vector_storage,
            graph_storage,
            pipeline_state,
            workspace_service: None,
            models_config: None,
        }
    }

    /// Create a new document task processor with workspace-specific pipeline support.
    ///
    /// SPEC-032: This constructor enables workspace-specific LLM and embedding providers.
    /// When processing tasks with workspace_id in metadata, the processor will use
    /// the workspace's configured providers instead of the server defaults.
    pub fn with_workspace_support(
        pipeline: Arc<Pipeline>,
        kv_storage: Arc<dyn KVStorage>,
        vector_storage: Arc<dyn VectorStorage>,
        graph_storage: Arc<dyn GraphStorage>,
        pipeline_state: PipelineState,
        workspace_service: SharedWorkspaceService,
        models_config: Arc<ModelsConfig>,
    ) -> Self {
        Self {
            pipeline,
            kv_storage,
            vector_storage,
            graph_storage,
            pipeline_state,
            workspace_service: Some(workspace_service),
            models_config: Some(models_config),
        }
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

        // If no workspace support configured, use default pipeline
        let (workspace_service, _models_config): (&SharedWorkspaceService, &Arc<ModelsConfig>) =
            match (&self.workspace_service, &self.models_config) {
                (Some(ws), Some(mc)) => (ws, mc),
                _ => return Arc::clone(&self.pipeline),
            };

        // If no workspace_id provided, use default pipeline
        let workspace_id = match workspace_id {
            Some(id) if !id.is_empty() && id != "default" => id,
            _ => return Arc::clone(&self.pipeline),
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
                // Try to create workspace-specific LLM provider
                let llm_provider =
                    ProviderFactory::create_llm_provider(&ws.llm_provider, &ws.llm_model);

                // Try to create workspace-specific embedding provider
                let embedding_provider = ProviderFactory::create_embedding_provider(
                    &ws.embedding_provider,
                    &ws.embedding_model,
                    ws.embedding_dimension,
                );

                // If both providers were created successfully, build workspace pipeline
                if let (Ok(llm), Ok(embedding)) = (llm_provider, embedding_provider) {
                    info!(
                        workspace_id = workspace_id,
                        llm_model = %ws.llm_full_id(),
                        embedding_model = %ws.embedding_full_id(),
                        "Using workspace-specific LLM configuration for document processing"
                    );

                    let extractor = Arc::new(LLMExtractor::new(llm));
                    return Arc::new(
                        Pipeline::default_pipeline()
                            .with_extractor(extractor)
                            .with_embedding_provider(embedding),
                    );
                }

                warn!(
                    workspace_id = workspace_id,
                    llm_config = %ws.llm_full_id(),
                    embedding_config = %ws.embedding_full_id(),
                    "Failed to create workspace-specific providers, using default pipeline"
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
        let workspace_id = data
            .metadata
            .as_ref()
            .and_then(|m| m.get("workspace_id"))
            .and_then(|v| v.as_str());

        // Get workspace-specific pipeline (or default if not available)
        let pipeline = self.get_workspace_pipeline(workspace_id).await;

        info!(
            document_id = %document_id,
            workspace_id = ?workspace_id,
            file_source = %data.file_source,
            "Processing document with workspace-specific pipeline"
        );

        // Update task progress - chunking
        task.update_progress("chunking".to_string(), 4, 10);

        // Log to pipeline state
        self.pipeline_state
            .info(format!("Chunking document {}...", document_id))
            .await;

        // Update document status to processing
        self.update_document_status(&document_id, "processing", None)
            .await?;

        // Process through pipeline (using workspace-specific or default)
        let result = match pipeline.process(&document_id, &data.text).await {
            Ok(result) => result,
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

                if self
                    .vector_storage
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

        // Store entities and relationships in graph storage using batch operations
        // Collect all nodes for batch upsert
        let mut nodes_batch: Vec<(String, std::collections::HashMap<String, serde_json::Value>)> =
            Vec::new();
        let mut edges_batch: Vec<(
            String,
            String,
            std::collections::HashMap<String, serde_json::Value>,
        )> = Vec::new();

        for extraction in &result.extractions {
            for entity in &extraction.entities {
                let mut properties = std::collections::HashMap::new();
                properties.insert("entity_type".to_string(), json!(entity.entity_type));
                properties.insert("description".to_string(), json!(entity.description));
                properties.insert("importance".to_string(), json!(entity.importance));
                properties.insert("source_ids".to_string(), json!(vec![&document_id]));
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
                properties.insert("source_ids".to_string(), json!(vec![&document_id]));
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

        // Update document status to completed with stats and lineage
        self.update_document_status_with_stats(&document_id, "completed", &result.stats)
            .await?;

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
                if let Some(ref embedding_model) = stats.embedding_model {
                    updated.insert("embedding_model".to_string(), json!(embedding_model));
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_storage::{MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage};

    /// Create a test pipeline instance using default configuration
    fn create_test_pipeline() -> Arc<Pipeline> {
        Arc::new(Pipeline::default_pipeline())
    }

    /// Create test storage instances for testing
    fn create_test_storages() -> (
        Arc<dyn KVStorage>,
        Arc<dyn VectorStorage>,
        Arc<dyn GraphStorage>,
    ) {
        let kv = Arc::new(MemoryKVStorage::new("test_processor"));
        // MemoryVectorStorage requires dimension - use 1536 (common embedding size)
        let vector = Arc::new(MemoryVectorStorage::new("test_processor", 1536));
        let graph = Arc::new(MemoryGraphStorage::new("test_processor"));
        (kv, vector, graph)
    }

    #[test]
    fn test_document_task_processor_new() {
        let pipeline = create_test_pipeline();
        let (kv, vector, graph) = create_test_storages();
        let pipeline_state = PipelineState::new();

        let processor = DocumentTaskProcessor::new(pipeline, kv, vector, graph, pipeline_state);

        // Verify processor was created successfully
        assert!(std::mem::size_of_val(&processor) > 0);
    }

    #[tokio::test]
    async fn test_processor_trait_implementation() {
        let pipeline = create_test_pipeline();
        let (kv, vector, graph) = create_test_storages();
        let pipeline_state = PipelineState::new();

        let processor = DocumentTaskProcessor::new(pipeline, kv, vector, graph, pipeline_state);

        // Verify TaskProcessor trait is implemented
        let _: &dyn TaskProcessor = &processor;
    }

    #[tokio::test]
    async fn test_process_scan_task_returns_unsupported() {
        let pipeline = create_test_pipeline();
        let (kv, vector, graph) = create_test_storages();
        let pipeline_state = PipelineState::new();

        let processor = DocumentTaskProcessor::new(pipeline, kv, vector, graph, pipeline_state);

        let mut task = Task::new(TaskType::Scan, json!({}));

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
        let (kv, vector, graph) = create_test_storages();
        let pipeline_state = PipelineState::new();

        let processor = DocumentTaskProcessor::new(pipeline, kv, vector, graph, pipeline_state);

        let mut task = Task::new(TaskType::Reindex, json!({}));

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
        let (kv, vector, graph) = create_test_storages();
        let pipeline_state = PipelineState::new();

        let processor = DocumentTaskProcessor::new(pipeline, kv, vector, graph, pipeline_state);

        // Create task with invalid data (missing required fields)
        let invalid_data = json!({
            "invalid_field": "this is not TextInsertData"
        });

        let mut task = Task::new(TaskType::Insert, invalid_data);

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
        let (kv, vector, graph) = create_test_storages();
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

        let processor =
            DocumentTaskProcessor::new(pipeline, kv.clone(), vector, graph, pipeline_state);

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
        let (kv, vector, graph) = create_test_storages();
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

        let processor =
            DocumentTaskProcessor::new(pipeline, kv.clone(), vector, graph, pipeline_state);

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
        let (kv, vector, graph) = create_test_storages();
        let pipeline_state = PipelineState::new();

        let processor = DocumentTaskProcessor::new(pipeline, kv, vector, graph, pipeline_state);

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
        let (kv, vector, graph) = create_test_storages();
        let pipeline_state = PipelineState::new();

        let _processor = DocumentTaskProcessor::new(
            pipeline.clone(),
            kv.clone(),
            vector.clone(),
            graph.clone(),
            pipeline_state,
        );

        // If we got here, Arc works correctly
        // Verify we can still access the cloned Arcs
        assert!(Arc::strong_count(&pipeline) >= 1);
        assert!(Arc::strong_count(&kv) >= 1);
        assert!(Arc::strong_count(&vector) >= 1);
        assert!(Arc::strong_count(&graph) >= 1);
    }

    #[tokio::test]
    async fn test_task_types_are_distinct() {
        // Verify all task types are handled distinctly
        let pipeline = create_test_pipeline();
        let (kv, vector, graph) = create_test_storages();
        let pipeline_state = PipelineState::new();

        let processor = DocumentTaskProcessor::new(pipeline, kv, vector, graph, pipeline_state);

        // Test that each unsupported task type goes through the right path
        let types = [TaskType::Scan, TaskType::Reindex];

        for task_type in types {
            let mut task = Task::new(task_type.clone(), json!({}));

            let result = processor.process(&mut task).await;

            // Scan/Reindex fail on unsupported
            assert!(result.is_err());
        }
    }
}
