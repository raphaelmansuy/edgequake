//! SPEC-024 Phase 1.2 — knowledge injection worker path.

use super::*;
use crate::services::injection_process::{run_injection_pipeline, write_injection_status};
use edgequake_tasks::KnowledgeInjectionData;
use tokio_util::sync::CancellationToken;

impl DocumentTaskProcessor {
    /// Process a queued knowledge injection task.
    pub(super) async fn process_knowledge_injection(
        &self,
        task: &mut Task,
        data: KnowledgeInjectionData,
        cancel_token: CancellationToken,
    ) -> TaskResult<serde_json::Value> {
        self.check_cancelled(&cancel_token, "pre-injection", &data.injection_id)
            .await?;

        task.update_progress("processing".to_string(), 1, 10);

        let workspace_id = if data.workspace_id.is_empty() || data.workspace_id == "default" {
            None
        } else {
            Some(data.workspace_id.as_str())
        };

        let pipeline = self
            .get_workspace_pipeline_strict(workspace_id)
            .await
            .map_err(|e| {
                let msg = format!("Workspace pipeline error: {e}");
                TaskError::Process(msg.clone())
            })?;

        let vector_storage = self
            .get_workspace_vector_storage_strict(&data.workspace_id)
            .await
            .map_err(|e| {
                TaskError::Process(format!(
                    "Cannot obtain workspace vector storage for '{}': {e}",
                    data.workspace_id
                ))
            })?;

        self.check_cancelled(&cancel_token, "pre-pipeline", &data.injection_id)
            .await?;

        task.update_progress("extracting".to_string(), 3, 40);

        match run_injection_pipeline(
            self.llm_provider.clone(),
            self.query_cache_invalidator
                .as_ref()
                .map(|e| e.as_ref() as &dyn edgequake_query::QueryResultCacheInvalidator),
            &pipeline,
            self.graph_storage.clone(),
            vector_storage,
            self.kv_storage.clone(),
            self.relational_sink.clone(),
            &data.doc_id,
            &data.content,
            &data.workspace_id,
            data.data_tenant_id.clone(),
        )
        .await
        {
            Ok((entity_count, chunk_ids)) => {
                write_injection_status(
                    &self.kv_storage,
                    &data.meta_key,
                    &data.injection_id,
                    &data.name,
                    &data.content,
                    &data.workspace_id,
                    &data.source_type,
                    data.source_filename.as_deref(),
                    "completed",
                    data.version,
                    entity_count,
                    Some(&chunk_ids),
                    &data.doc_id,
                    &data.created_at,
                    None,
                )
                .await;
                info!(
                    injection_id = %data.injection_id,
                    entity_count,
                    "Injection processing completed"
                );
                task.update_progress("completed".to_string(), 1, 100);
                Ok(json!({
                    "injection_id": data.injection_id,
                    "entity_count": entity_count,
                    "chunk_count": chunk_ids.len(),
                }))
            }
            Err(e) => {
                let err_msg = e.to_string();
                warn!(
                    injection_id = %data.injection_id,
                    error = %err_msg,
                    "Injection processing failed"
                );
                write_injection_status(
                    &self.kv_storage,
                    &data.meta_key,
                    &data.injection_id,
                    &data.name,
                    &data.content,
                    &data.workspace_id,
                    &data.source_type,
                    data.source_filename.as_deref(),
                    "failed",
                    data.version,
                    0,
                    None,
                    &data.doc_id,
                    &data.created_at,
                    Some(&err_msg),
                )
                .await;
                Err(TaskError::Process(err_msg))
            }
        }
    }
}
