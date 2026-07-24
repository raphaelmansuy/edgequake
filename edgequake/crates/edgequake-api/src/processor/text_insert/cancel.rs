use super::super::*;
use tokio_util::sync::CancellationToken;

impl DocumentTaskProcessor {
    /// Stages after graph/vector persist (used for logging only).
    /// SPEC-058: cancel always retracts indexes — cancelled content is not searchable.
    fn is_post_graph_stage(stage: &str) -> bool {
        matches!(stage, "pre-lineage" | "post-lineage")
    }

    /// SPEC-058: best-effort unindex on cancel-before-completed.
    async fn retract_indexes_on_cancel(&self, document_id: &str) {
        let metadata_key =
            crate::services::resolve_document_metadata_key(document_id, &self.kv_storage).await;
        let workspace_id = match self.kv_storage.get_by_id(&metadata_key).await {
            Ok(Some(meta)) => meta
                .get("workspace_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            _ => None,
        };

        let vector = match workspace_id.as_deref() {
            Some(ws) => self
                .get_workspace_vector_storage_strict(ws)
                .await
                .unwrap_or_else(|_| self.vector_storage.clone()),
            None => self.vector_storage.clone(),
        };

        let stats = crate::services::retract_document_indexes(
            &self.graph_storage,
            &vector,
            None,
            document_id,
        )
        .await;
        tracing::info!(
            document_id = %document_id,
            embeddings_deleted = stats.embeddings_deleted,
            entities_removed = stats.entities_removed,
            entities_updated = stats.entities_updated,
            "SPEC-058: retracted indexes after cancel"
        );
    }

    /// Check if the task has been cancelled and return early if so.
    pub(crate) async fn check_cancelled(
        &self,
        cancel_token: &CancellationToken,
        stage: &str,
        document_id: &str,
    ) -> TaskResult<()> {
        if cancel_token.is_cancelled() {
            let post_graph = Self::is_post_graph_stage(stage);
            let msg = format!(
                "Task cancelled during '{}' stage for document {}",
                stage, document_id
            );
            tracing::info!(
                error.source = "task_processor",
                error.action = "cancelled",
                document_id = %document_id,
                stage = %stage,
                post_graph,
                error.message = %msg,
                "Task cancelled — SPEC-058 retracting indexes"
            );
            // SPEC-058: cancel wins — unindex so cancelled content is not searchable.
            self.retract_indexes_on_cancel(document_id).await;
            self.update_document_status(document_id, "cancelled", Some(&msg))
                .await
                .ok();
            // Free staging hash so cancelled shells do not block same-bytes re-upload.
            let meta_key =
                crate::services::resolve_document_metadata_key(document_id, &self.kv_storage).await;
            if let Ok(Some(meta)) = self.kv_storage.get_by_id(&meta_key).await {
                if let (Some(hash), Some(ws)) = (
                    meta.get("content_hash").and_then(|v| v.as_str()),
                    meta.get("workspace_id").and_then(|v| v.as_str()),
                ) {
                    let _ = crate::services::release_staging_reservation(
                        &self.kv_storage,
                        document_id,
                        ws,
                        hash,
                    )
                    .await;
                }
            }
            return Err(TaskError::Cancelled(msg));
        }
        Ok(())
    }
}
