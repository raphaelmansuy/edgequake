use super::*;

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

    /// Called when a task has permanently failed (retries exhausted or circuit breaker tripped).
    ///
    /// WHY: Updates document metadata to "failed" status so the frontend shows the actual
    /// error instead of leaving the document stuck in "processing" forever. Also updates
    /// PDF processing status for PDF tasks and cleans up progress tracking.
    async fn on_permanent_failure(&self, task: &Task, error_msg: &str) {
        // Extract document_id from task_data to update document status.
        // For PdfProcessing tasks, it's in existing_document_id.
        // For Insert/Upload tasks, it's in metadata.document_id.
        let document_id = task
            .task_data
            .get("existing_document_id")
            .and_then(|v| v.as_str())
            .or_else(|| {
                task.task_data
                    .get("metadata")
                    .and_then(|m| m.get("document_id"))
                    .and_then(|v| v.as_str())
            })
            .map(|s| s.to_string());

        error!(
            task_id = %task.track_id,
            tenant_id = %task.tenant_id,
            document_id = ?document_id,
            retry_count = task.retry_count,
            circuit_breaker_tripped = task.circuit_breaker_tripped,
            "Permanent task failure — updating document status to 'failed'"
        );

        // Update document metadata to "failed" with the actual error message
        if let Some(ref doc_id) = document_id {
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

        // For PDF tasks, also update the PDF processing status
        #[cfg(feature = "postgres")]
        if task.task_type == TaskType::PdfProcessing {
            if let Some(ref pdf_storage) = self.pdf_storage {
                if let Some(pdf_id_str) = task.task_data.get("pdf_id").and_then(|v| v.as_str()) {
                    if let Ok(pdf_id) = uuid::Uuid::parse_str(pdf_id_str) {
                        use edgequake_storage::PdfProcessingStatus;
                        if let Err(e) = pdf_storage
                            .update_pdf_status(&pdf_id, PdfProcessingStatus::Failed)
                            .await
                        {
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
    }
}
