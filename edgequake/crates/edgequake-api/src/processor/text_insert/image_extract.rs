//! SPEC-180 07/08: the image-chunk path of a normal workspace that embeds images.
//!
//! A standalone image is one chunk, `source_modality = image`, embedded from its pixels by
//! the multimodal embedder. There is no chat model, no caption and no graph. The original
//! bytes come from original storage; the upload handler writes them just after admission,
//! so the read waits a short while for them.

use std::time::Duration;

use edgequake_core::multimodal_embed::{embed_codes, EmbedError, EmbedInput};
use edgequake_pipeline::{ProcessingResult, ProcessingStats, TextChunk, MODALITY_IMAGE};
use tokio_util::sync::CancellationToken;

use super::super::*;
use super::types::{TextInsertExtracted, TextInsertPrepared};

const ORIGINAL_POLL: Duration = Duration::from_millis(200);
const ORIGINAL_POLL_ATTEMPTS: u32 = 50;

/// The one chunk of an image document: empty content, a pixel vector, the image modality.
pub(super) fn image_chunk(document_id: &str, vector: Vec<f32>) -> TextChunk {
    let id = edgequake_storage::kv_keys::doc_chunk(document_id, 0);
    let mut chunk = TextChunk::new(id, "", 0, 0, 0);
    chunk.embedding = Some(vector);
    chunk.modality = Some(MODALITY_IMAGE.to_string());
    chunk
}

/// The result a single image chunk stands for. Nothing is extracted.
pub(super) fn image_result(
    document_id: &str,
    chunk: TextChunk,
    model: &str,
    provider: &str,
    started: std::time::Instant,
) -> ProcessingResult {
    let dimensions = chunk.embedding.as_ref().map(Vec::len);
    let stats = ProcessingStats {
        chunk_count: 1,
        successful_chunks: 1,
        embedding_model: Some(model.to_string()),
        embedding_provider: Some(provider.to_string()),
        embedding_dimensions: dimensions,
        chunking_strategy: Some("image".to_string()),
        processing_time_ms: started.elapsed().as_millis() as u64,
        ..ProcessingStats::default()
    };
    ProcessingResult {
        document_id: document_id.to_string(),
        chunks: vec![chunk],
        extractions: Vec::new(),
        stats,
        lineage: None,
    }
}

impl DocumentTaskProcessor {
    /// True when the document is an image and its workspace embeds images.
    pub(super) async fn takes_image_chunk_path(&self, prepared: &TextInsertPrepared) -> bool {
        prepared.source_type == "image"
            && self
                .get_workspace_capabilities(prepared.workspace_id.as_deref())
                .await
                .embeds_images
    }

    pub(super) async fn text_insert_extract_image(
        &self,
        task: &mut Task,
        prepared: TextInsertPrepared,
        cancel_token: CancellationToken,
    ) -> TaskResult<TextInsertExtracted> {
        let document_id = prepared.document_id.clone();
        self.check_cancelled(&cancel_token, "pre-embed", &document_id)
            .await?;
        self.update_document_status(&document_id, "extracting", None)
            .await?;
        let started = std::time::Instant::now();
        let outcome = self.embed_original_image(&prepared).await;
        let (vector, model, provider) = match outcome {
            Ok(done) => done,
            Err(error) => return self.fail_image_document(&document_id, error).await,
        };
        let chunk = image_chunk(&document_id, vector);
        let result = image_result(&document_id, chunk, &model, &provider, started);
        self.bump_task_progress(task, "extraction_complete".to_string(), 4, 30)
            .await;
        Ok(TextInsertExtracted { prepared, result })
    }

    async fn embed_original_image(
        &self,
        prepared: &TextInsertPrepared,
    ) -> Result<(Vec<f32>, String, String), EmbedError> {
        let ws = self
            .workspace_for_image(prepared.workspace_id.as_deref())
            .await?;
        let embedder =
            crate::embeddings::multimodal_embedder_for_workspace(self.optional_pg_pool(), &ws)
                .await
                .ok_or_else(|| {
                    EmbedError::new(
                        edgequake_core::rag_profile::codes::EMBEDDING_MODEL_NOT_MULTIMODAL,
                        format!("{} cannot embed images", ws.embedding_model),
                    )
                })??;
        let (mime, bytes) = self.read_original(prepared).await?;
        let vectors = embedder.embed(&[EmbedInput::image(mime, bytes)]).await?;
        let vector = vectors.into_iter().next().ok_or_else(|| {
            EmbedError::new(embed_codes::REQUEST, "the embedder returned no vector")
        })?;
        Ok((
            vector,
            ws.embedding_model.clone(),
            ws.embedding_provider.clone(),
        ))
    }

    async fn workspace_for_image(
        &self,
        workspace_id: Option<&str>,
    ) -> Result<edgequake_core::Workspace, EmbedError> {
        let missing = || EmbedError::new(embed_codes::REQUEST, "workspace not found");
        let uuid = workspace_id
            .and_then(|id| crate::middleware::resolve_workspace_uuid(Some(id)))
            .ok_or_else(missing)?;
        let service = self.workspace_service.as_ref().ok_or_else(missing)?;
        service
            .get_workspace(uuid)
            .await
            .ok()
            .flatten()
            .ok_or_else(missing)
    }

    #[cfg(feature = "postgres")]
    fn original_storage(&self) -> Option<Arc<dyn edgequake_storage::DocumentOriginalStorage>> {
        self.app_state
            .as_ref()
            .and_then(|state| state.storage.original_storage.clone())
    }

    #[cfg(not(feature = "postgres"))]
    fn original_storage(&self) -> Option<Arc<dyn edgequake_storage::DocumentOriginalStorage>> {
        None
    }

    /// Original bytes and MIME type, waiting for the upload handler to store them.
    async fn read_original(
        &self,
        prepared: &TextInsertPrepared,
    ) -> Result<(String, Vec<u8>), EmbedError> {
        let not_found = || {
            EmbedError::new(
                embed_codes::REQUEST,
                "the uploaded image was not found in original storage",
            )
        };
        let storage = self.original_storage().ok_or_else(not_found)?;
        let document = uuid::Uuid::parse_str(&prepared.document_id).map_err(|_| not_found())?;
        let workspace = prepared
            .workspace_id
            .as_deref()
            .and_then(|id| crate::middleware::resolve_workspace_uuid(Some(id)))
            .ok_or_else(not_found)?;
        for _ in 0..ORIGINAL_POLL_ATTEMPTS {
            if let Ok(Some(original)) = storage.get_original(&workspace, &document).await {
                return Ok((original.content_type, original.original_data));
            }
            tokio::time::sleep(ORIGINAL_POLL).await;
        }
        Err(not_found())
    }

    async fn fail_image_document(
        &self,
        document_id: &str,
        error: EmbedError,
    ) -> TaskResult<TextInsertExtracted> {
        let message = format!("{}: {}", error.code, error.message);
        error!(document_id = %document_id, error.code = error.code, "image embedding failed");
        self.update_document_status(document_id, "failed", Some(&message))
            .await?;
        self.pipeline_state
            .document_failed(document_id, &message)
            .await;
        Err(edgequake_tasks::TaskError::Process(message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_image_document_is_one_empty_chunk_with_a_vector() {
        let chunk = image_chunk("doc-1", vec![0.5; 4]);
        assert_eq!(chunk.index, 0);
        assert!(chunk.content.is_empty());
        assert_eq!(chunk.modality.as_deref(), Some(MODALITY_IMAGE));
        assert_eq!(chunk.embedding.as_ref().map(Vec::len), Some(4));
    }

    #[test]
    fn the_result_extracts_nothing_and_counts_one_chunk() {
        let chunk = image_chunk("doc-1", vec![0.5; 4]);
        let result = image_result("doc-1", chunk, "m", "p", std::time::Instant::now());
        assert_eq!(result.stats.chunk_count, 1);
        assert_eq!(result.stats.entity_count, 0);
        assert!(result.extractions.is_empty());
        assert_eq!(result.stats.embedding_dimensions, Some(4));
    }
}
