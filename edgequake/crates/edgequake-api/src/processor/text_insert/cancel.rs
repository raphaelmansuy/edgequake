use super::super::*;
use tokio_util::sync::CancellationToken;

impl DocumentTaskProcessor {
    /// Check if the task has been cancelled and return early if so.
    pub(crate) async fn check_cancelled(
        &self,
        cancel_token: &CancellationToken,
        stage: &str,
        document_id: &str,
    ) -> TaskResult<()> {
        if cancel_token.is_cancelled() {
            let msg = format!(
                "Task cancelled during '{}' stage for document {}",
                stage, document_id
            );
            tracing::info!(
                error.source = "task_processor",
                error.action = "cancelled",
                document_id = %document_id,
                stage = %stage,
                error.message = %msg,
                "Task cancelled"
            );
            self.update_document_status(document_id, "cancelled", Some(&msg))
                .await
                .ok();
            return Err(TaskError::Cancelled(msg));
        }
        Ok(())
    }
}
