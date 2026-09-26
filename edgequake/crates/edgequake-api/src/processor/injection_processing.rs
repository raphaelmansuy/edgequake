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

        self.bump_task_progress(task, "processing".to_string(), 1, 10)
            .await;

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

        self.bump_task_progress(task, "extracting".to_string(), 3, 40)
            .await;

        let lineage = self
            .get_workspace_provider_lineage(Some(data.workspace_id.as_str()))
            .await;
        let text_embedder = crate::safety_limits::create_safe_embedding_provider(
            &lineage.embedding_provider,
            &lineage.embedding_model,
            lineage.embedding_dimension,
        )
        .ok()
        .map(crate::services::LlmTextEmbedder::arc);

        // SPEC-046: Summary role for merge summarizer when configured.
        let summary_ws = async {
            let uuid = crate::middleware::resolve_workspace_uuid(Some(data.workspace_id.as_str()))?;
            let ws_svc = self.workspace_service.as_ref()?;
            ws_svc.get_workspace(uuid).await.ok().flatten()
        }
        .await;
        let persist_llm = crate::services::resolve_summary_llm_or_fallback(
            summary_ws.as_ref(),
            self.llm_provider.clone(),
            |provider, model| {
                crate::safety_limits::create_safe_llm_provider(provider, model)
                    .map_err(|e| e.to_string())
            },
        );

        match run_injection_pipeline(
            persist_llm,
            self.query_cache_invalidator
                .as_ref()
                .map(|e| e.as_ref() as &dyn edgequake_query::QueryResultCacheInvalidator),
            &pipeline,
            self.graph_storage.clone(),
            vector_storage,
            self.kv_storage.clone(),
            self.relational_sink.clone(),
            self.resolve_lineage_sink().await,
            text_embedder,
            #[cfg(feature = "postgres")]
            crate::services::resolve_relational_chunk_repo(self.optional_pg_pool()),
            #[cfg(not(feature = "postgres"))]
            crate::services::resolve_relational_chunk_repo(None),
            #[cfg(feature = "postgres")]
            self.app_state
                .as_ref()
                .and_then(|state| state.ingestion_committer.clone()),
            #[cfg(feature = "postgres")]
            self.app_state
                .as_ref()
                .and_then(|state| state.document_reader.clone()),
            #[cfg(feature = "postgres")]
            self.pg_pool.clone(),
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
                self.bump_task_progress(task, "completed".to_string(), 1, 100)
                    .await;
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
