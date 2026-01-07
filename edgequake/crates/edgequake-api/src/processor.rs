//! Document task processor for async document processing.
//!
//! This module implements the `TaskProcessor` trait to process document
//! upload tasks through the pipeline and update storage accordingly.

use std::sync::Arc;

use edgequake_pipeline::Pipeline;
use edgequake_storage::traits::{GraphStorage, KVStorage, VectorStorage};
use edgequake_tasks::{PipelineState, Task, TaskProcessor, TaskResult, TaskType, TextInsertData};
use serde_json::json;
use tracing::{error, info, warn};

/// Document task processor that processes documents through the pipeline.
pub struct DocumentTaskProcessor {
    /// Processing pipeline.
    pipeline: Arc<Pipeline>,
    /// KV storage for document metadata and chunks.
    kv_storage: Arc<dyn KVStorage>,
    /// Vector storage for chunk embeddings.
    vector_storage: Arc<dyn VectorStorage>,
    /// Graph storage for entities and relationships.
    graph_storage: Arc<dyn GraphStorage>,
    /// Pipeline state for progress tracking.
    pipeline_state: PipelineState,
}

impl DocumentTaskProcessor {
    /// Create a new document task processor.
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

        info!(
            "Processing document: {} ({})",
            document_id, data.file_source
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

        // Process through pipeline
        let result = match self.pipeline.process(&document_id, &data.text).await {
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

    #[test]
    fn test_document_task_processor_new() {
        // Test that we can at least import and reference the struct
        // Full testing requires mock implementations
    }
}
