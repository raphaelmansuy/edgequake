//! Text insert worker pipeline (SPEC-025 6.6 SRP split).

mod cancel;
mod extraction;
mod finalize;
mod persist;
mod prepare;
mod types;

use super::*;
use tokio_util::sync::CancellationToken;

impl DocumentTaskProcessor {
    pub(super) async fn process_text_insert(
        &self,
        task: &mut Task,
        data: TextInsertData,
        cancel_token: CancellationToken,
    ) -> TaskResult<serde_json::Value> {
        let processing_start = std::time::Instant::now();
        let prepared = self.text_insert_prepare(task, data).await?;
        let extracted = self
            .text_insert_extract(task, prepared, cancel_token.clone())
            .await?;
        let persisted = self
            .text_insert_persist(task, extracted, cancel_token.clone())
            .await?;
        self.text_insert_finalize(persisted, cancel_token, processing_start)
            .await
    }
}
