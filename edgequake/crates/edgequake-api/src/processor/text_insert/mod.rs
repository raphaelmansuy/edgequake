//! Text insert worker pipeline (SPEC-025 6.6 SRP split).

mod cancel;
mod decision_outcome;
mod extraction;
mod finalize;
mod image_extract;
mod persist;
mod prepare;
mod types;

use super::*;
use edgequake_tasks::FairnessPermit;
use tokio_util::sync::CancellationToken;

impl DocumentTaskProcessor {
    pub(super) async fn process_text_insert(
        &self,
        task: &mut Task,
        data: TextInsertData,
        cancel_token: CancellationToken,
        fairness: Option<FairnessPermit>,
    ) -> TaskResult<serde_json::Value> {
        let document_id = data
            .metadata
            .as_ref()
            .and_then(|m| m.get("document_id"))
            .and_then(|v| v.as_str())
            .unwrap_or(&data.file_source)
            .to_string();
        let tenant_id = data
            .metadata
            .as_ref()
            .and_then(|m| m.get("tenant_id"))
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let workspace_id = data.workspace_id.clone();
        edgequake_observability::with_ingest_task_span(async {
            let _langfuse = self
                .stamp_ingest_langfuse_for_document(
                    &document_id,
                    tenant_id.as_deref(),
                    Some(workspace_id.as_str()),
                )
                .await;
            self.process_text_insert_inner(task, data, cancel_token, fairness)
                .await
        })
        .await
    }

    async fn process_text_insert_inner(
        &self,
        task: &mut Task,
        data: TextInsertData,
        cancel_token: CancellationToken,
        mut fairness: Option<FairnessPermit>,
    ) -> TaskResult<serde_json::Value> {
        let processing_start = std::time::Instant::now();
        let stage_t0 = std::time::Instant::now();
        let prepared = self
            .text_insert_prepare(task, data, cancel_token.clone())
            .await?;
        edgequake_observability::metrics::record_ingest_stage_duration(
            "prepare",
            stage_t0.elapsed().as_secs_f64(),
        );

        let stage_t0 = std::time::Instant::now();
        let doc_lang = prepared
            .data
            .metadata
            .as_ref()
            .and_then(|m| m.get("document_language"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        // SPEC-180: an image of an image-embedding workspace is one pixel-embedded chunk.
        let extracted = if self.takes_image_chunk_path(&prepared).await {
            self.text_insert_extract_image(task, prepared, cancel_token.clone())
                .await?
        } else {
            edgequake_pipeline::with_optional_document_language(
                doc_lang,
                self.text_insert_extract(task, prepared, cancel_token.clone()),
            )
            .await?
        };
        edgequake_observability::metrics::record_ingest_stage_duration(
            "extract",
            stage_t0.elapsed().as_secs_f64(),
        );

        let stage_t0 = std::time::Instant::now();
        let persisted = self
            .text_insert_persist(task, extracted, cancel_token.clone(), &mut fairness)
            .await?;
        // Permit should already be dropped inside persist; drop any remainder.
        drop(fairness);
        edgequake_observability::metrics::record_ingest_stage_duration(
            "persist",
            stage_t0.elapsed().as_secs_f64(),
        );

        let stage_t0 = std::time::Instant::now();
        let out = self
            .text_insert_finalize(persisted, cancel_token, processing_start)
            .await;
        edgequake_observability::metrics::record_ingest_stage_duration(
            "finalize",
            stage_t0.elapsed().as_secs_f64(),
        );
        out
    }
}
