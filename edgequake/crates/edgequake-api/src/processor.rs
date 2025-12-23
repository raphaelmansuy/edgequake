//! Document task processor for async document processing.
//!
//! This module implements the `TaskProcessor` trait to process document
//! upload tasks through the pipeline and update storage accordingly.

use std::sync::Arc;

use edgequake_pipeline::Pipeline;
use edgequake_storage::traits::{GraphStorage, KVStorage};
use edgequake_tasks::{PipelineState, Task, TaskProcessor, TaskResult, TaskType, TextInsertData};
use serde_json::json;
use tracing::{error, info, warn};

/// Document task processor that processes documents through the pipeline.
pub struct DocumentTaskProcessor {
    /// Processing pipeline.
    pipeline: Arc<Pipeline>,
    /// KV storage for document metadata and chunks.
    kv_storage: Arc<dyn KVStorage>,
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
        graph_storage: Arc<dyn GraphStorage>,
        pipeline_state: PipelineState,
    ) -> Self {
        Self {
            pipeline,
            kv_storage,
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

        // Update task progress - extraction
        task.update_progress("extraction".to_string(), 4, 60);
        self.pipeline_state
            .info(format!("Extracting entities from {}...", document_id))
            .await;

        // Store entities and relationships in graph storage
        for extraction in &result.extractions {
            for entity in &extraction.entities {
                let mut properties = std::collections::HashMap::new();
                properties.insert("entity_type".to_string(), json!(entity.entity_type));
                properties.insert("description".to_string(), json!(entity.description));
                properties.insert("importance".to_string(), json!(entity.importance));
                properties.insert("source_ids".to_string(), json!(vec![&document_id]));

                if let Err(e) = self
                    .graph_storage
                    .upsert_node(&entity.name, properties)
                    .await
                {
                    warn!("Failed to store entity {}: {}", entity.name, e);
                }
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

                if let Err(e) = self
                    .graph_storage
                    .upsert_edge(&relationship.source, &relationship.target, properties)
                    .await
                {
                    warn!(
                        "Failed to store relationship {}->{}: {}",
                        relationship.source, relationship.target, e
                    );
                }
            }
        }

        // Update task progress - indexing complete
        task.update_progress("indexing".to_string(), 4, 100);

        // Update document status to completed with stats
        self.update_document_status_with_stats(
            &document_id,
            "completed",
            result.stats.chunk_count,
            result.stats.entity_count,
            result.stats.relationship_count,
        )
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

    /// Update document metadata with processing stats.
    async fn update_document_status_with_stats(
        &self,
        document_id: &str,
        status: &str,
        chunk_count: usize,
        entity_count: usize,
        relationship_count: usize,
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
                updated.insert("chunk_count".to_string(), json!(chunk_count));
                updated.insert("entity_count".to_string(), json!(entity_count));
                updated.insert("relationship_count".to_string(), json!(relationship_count));
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
