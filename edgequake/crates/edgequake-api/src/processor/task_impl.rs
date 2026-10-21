use super::*;
use tokio_util::sync::CancellationToken;

#[async_trait::async_trait]
impl TaskProcessor for DocumentTaskProcessor {
    async fn process(
        &self,
        task: &mut Task,
        cancel_token: CancellationToken,
    ) -> TaskResult<serde_json::Value> {
        self.process_with_fairness(task, cancel_token, None).await
    }

    async fn process_with_fairness(
        &self,
        task: &mut Task,
        cancel_token: CancellationToken,
        fairness: Option<edgequake_tasks::FairnessPermit>,
    ) -> TaskResult<serde_json::Value> {
        #[cfg(feature = "postgres")]
        {
            let scope = edgequake_storage::adapters::postgres::rls::RlsScope {
                tenant_id: task.tenant_id,
                workspace_id: Some(task.workspace_id),
                user_id: None,
            };
            return edgequake_storage::adapters::postgres::rls::RLS_SCOPE
                .scope(
                    scope,
                    self.process_with_fairness_body(task, cancel_token, fairness),
                )
                .await;
        }
        #[cfg(not(feature = "postgres"))]
        {
            self.process_with_fairness_body(task, cancel_token, fairness)
                .await
        }
    }

    async fn on_permanent_failure(&self, task: &Task, error_msg: &str) {
        // The worker calls this after `process_with_fairness` has left its
        // tenant scope. Status resets (including stuck `deleting` rows) have
        // to re-enter that task's own tenant or FORCE RLS updates zero rows.
        #[cfg(feature = "postgres")]
        {
            let scope = edgequake_storage::adapters::postgres::rls::RlsScope {
                tenant_id: task.tenant_id,
                workspace_id: Some(task.workspace_id),
                user_id: None,
            };
            return edgequake_storage::adapters::postgres::rls::RLS_SCOPE
                .scope(scope, self.on_permanent_failure_body(task, error_msg))
                .await;
        }
        #[cfg(not(feature = "postgres"))]
        {
            self.on_permanent_failure_body(task, error_msg).await
        }
    }
}

impl DocumentTaskProcessor {
    async fn process_with_fairness_body(
        &self,
        task: &mut Task,
        cancel_token: CancellationToken,
        fairness: Option<edgequake_tasks::FairnessPermit>,
    ) -> TaskResult<serde_json::Value> {
        // Fail at t=0 (not after 50 min of LLM spend) if the document was tombstoned.
        if let Some(error) = self.reject_if_document_tombstoned(task).await {
            return Err(error);
        }
        let ports = self.installed_operational_ports();
        crate::services::relational_sidecar_store::with_ports(ports, async {
            match task.task_type {
                TaskType::Insert | TaskType::Upload => {
                    let data: TextInsertData = serde_json::from_value(task.task_data.clone())
                        .map_err(|e| {
                            edgequake_tasks::TaskError::InvalidPayload(format!(
                                "Invalid TextInsertData: {}",
                                e
                            ))
                        })?;
                    self.process_text_insert(task, data, cancel_token, fairness)
                        .await
                }
                TaskType::Scan => Err(edgequake_tasks::TaskError::UnsupportedOperation(
                    "Directory scanning not yet implemented".to_string(),
                )),
                TaskType::Reindex => Err(edgequake_tasks::TaskError::UnsupportedOperation(
                    "Reindexing not yet implemented".to_string(),
                )),
                TaskType::PdfProcessing => {
                    let _hold = fairness;
                    let data: edgequake_tasks::PdfProcessingData =
                        serde_json::from_value(task.task_data.clone()).map_err(|e| {
                            edgequake_tasks::TaskError::InvalidPayload(format!(
                                "Invalid PdfProcessingData: {}",
                                e
                            ))
                        })?;
                    self.process_pdf_processing(task, data, cancel_token).await
                }
                TaskType::KnowledgeInjection => {
                    let _hold = fairness;
                    let data: KnowledgeInjectionData =
                        serde_json::from_value(task.task_data.clone()).map_err(|e| {
                            edgequake_tasks::TaskError::InvalidPayload(format!(
                                "Invalid KnowledgeInjectionData: {}",
                                e
                            ))
                        })?;
                    self.process_knowledge_injection(task, data, cancel_token)
                        .await
                }
                TaskType::Deletion => {
                    let _hold = fairness;
                    let data: edgequake_tasks::DeletionTaskData =
                        serde_json::from_value(task.task_data.clone()).map_err(|e| {
                            edgequake_tasks::TaskError::InvalidPayload(format!(
                                "Invalid DeletionTaskData: {}",
                                e
                            ))
                        })?;
                    self.process_document_deletion(task, data, cancel_token)
                        .await
                }
                TaskType::BatchDeletion => {
                    let _hold = fairness;
                    let data: edgequake_tasks::BatchDeletionTaskData =
                        serde_json::from_value(task.task_data.clone()).map_err(|e| {
                            edgequake_tasks::TaskError::InvalidPayload(format!(
                                "Invalid BatchDeletionTaskData: {}",
                                e
                            ))
                        })?;
                    self.process_batch_deletion(task, data, cancel_token).await
                }
                TaskType::WorkspaceWipe => {
                    let _hold = fairness;
                    let data: edgequake_tasks::WorkspaceWipeTaskData =
                        serde_json::from_value(task.task_data.clone()).map_err(|e| {
                            edgequake_tasks::TaskError::InvalidPayload(format!(
                                "Invalid WorkspaceWipeTaskData: {}",
                                e
                            ))
                        })?;
                    self.process_workspace_wipe(task, data, cancel_token).await
                }
            }
        })
        .await
    }
}

impl DocumentTaskProcessor {
    /// Called when a task has permanently failed (retries exhausted or circuit breaker tripped).
    ///
    /// WHY: Updates document metadata to "failed" status so the frontend shows the actual
    /// error instead of leaving the document stuck in "processing" forever. Also updates
    /// PDF processing status for PDF tasks and cleans up progress tracking.
    async fn on_permanent_failure_body(&self, task: &Task, error_msg: &str) {
        // Extract document_id from task_data to update document status.
        // For PdfProcessing tasks, it's in existing_document_id.
        // For Insert/Upload tasks, it's in metadata.document_id.
        let document_id = super::deleted_document_guard::task_document_id(task);

        error!(
            task_id = %task.track_id,
            tenant_id = %task.tenant_id,
            document_id = ?document_id,
            retry_count = task.retry_count,
            circuit_breaker_tripped = task.circuit_breaker_tripped,
            error.message = %error_msg,
            "Permanent task failure — updating document status to 'failed'"
        );

        let task_type_label = format!("{:?}", task.task_type);
        edgequake_observability::record_document_processing(
            &task_type_label,
            "permanent_failure",
            "failure",
            0.0,
        );

        // Tombstoned document: keep it `delete_failed` (not a retryable-looking `failed`).
        let handled_as_deleted = match document_id {
            Some(ref doc_id) => {
                self.handle_deleted_document_failure(doc_id, error_msg)
                    .await
            }
            None => false,
        };

        // Update document metadata to "failed" with the actual error message
        if let (false, Some(ref doc_id)) = (handled_as_deleted, &document_id) {
            let failure_msg = format!(
                "Processing failed permanently after {} attempts. {}",
                task.retry_count, error_msg
            );
            if let Err(e) = self
                .update_document_status(doc_id, "failed", Some(&failure_msg))
                .await
            {
                error!(
                    document_id = %doc_id,
                    error = %e,
                    "Failed to update document status on permanent failure"
                );
            }
        }

        // For PDF tasks, also update the PDF processing status.
        // SPEC-057 P0: user/system cancel → Cancelled, never Failed.
        #[cfg(feature = "postgres")]
        if task.task_type == TaskType::PdfProcessing {
            if let Some(ref pdf_storage) = self.pdf_storage {
                if let Some(pdf_id_str) = task.task_data.get("pdf_id").and_then(|v| v.as_str()) {
                    if let Ok(pdf_id) = uuid::Uuid::parse_str(pdf_id_str) {
                        use edgequake_storage::PdfProcessingStatus;
                        let pdf_status = if crate::services::is_cancel_error_message(error_msg) {
                            PdfProcessingStatus::Cancelled
                        } else {
                            PdfProcessingStatus::Failed
                        };
                        if let Err(e) = pdf_storage.update_pdf_status(&pdf_id, pdf_status).await {
                            error!(
                                pdf_id = %pdf_id,
                                error = %e,
                                "Failed to update PDF processing status on permanent failure"
                            );
                        }
                    }
                }
            }
        }

        // Clean up progress tracking (fire-and-forget)
        let state = self.pipeline_state.clone();
        let track_id = task.track_id.clone();
        tokio::spawn(async move {
            state.remove_pdf_progress(&track_id).await;
        });

        // SPEC-024: Mark injection KV metadata failed on permanent task failure.
        if task.task_type == TaskType::KnowledgeInjection {
            if let Ok(data) =
                serde_json::from_value::<KnowledgeInjectionData>(task.task_data.clone())
            {
                crate::services::injection_process::write_injection_status(
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
                    Some(error_msg),
                )
                .await;
            }
        }

        // Self-heal: never leave documents stuck in `deleting`.
        if task.task_type == TaskType::Deletion {
            if let Ok(data) =
                serde_json::from_value::<edgequake_tasks::DeletionTaskData>(task.task_data.clone())
            {
                if let Some(state) = self.app_state.as_ref() {
                    crate::services::reset_deleting_status(
                        state,
                        &data.document_id,
                        &data.key_prefix,
                        &format!("Deletion failed permanently: {error_msg}"),
                        Some(&data.deletion_track_id),
                    )
                    .await;
                }
            }
        }

        if task.task_type == TaskType::WorkspaceWipe {
            if let Ok(data) = serde_json::from_value::<edgequake_tasks::WorkspaceWipeTaskData>(
                task.task_data.clone(),
            ) {
                if let Some(state) = self.app_state.as_ref() {
                    crate::services::broadcast_wipe_failed(
                        state,
                        &data,
                        &format!("Workspace wipe failed permanently: {error_msg}"),
                    );
                }
            }
        }
    }
}
