//! Document upload admission SSOT (SPEC-025 5.4).
//!
//! Single path for hash check, KV pre-write, and async task enqueue shared by
//! text, file, and batch upload handlers.

use axum::http::StatusCode;
use chrono::Utc;
use serde_json::{json, Value};
use uuid::Uuid;

use edgequake_pipeline::{ChunkOptions, ChunkStrategy};
use edgequake_storage::kv_keys;
use edgequake_tasks::{Task, TaskType, TextInsertData};

use crate::error::{ApiError, ApiResult};
use crate::handlers::documents::storage_helpers::{
    resolve_workspace_duplicate_for_reingestion, DuplicateReingestAction,
};
use crate::handlers::documents_types::{default_enable_gleaning, default_max_gleaning};
use crate::middleware::TenantContext;
use crate::services::ContentHasher;
use crate::services::{
    apply_process_options_to_metadata, metadata_multimodal_patch, persist_manifest,
    MultimodalSummary,
};
use crate::state::AppState;

/// Gleaning options stored in task metadata for worker pipeline builder.
#[derive(Debug, Clone, Copy)]
pub struct GleaningAdmissionOptions {
    pub enable_gleaning: bool,
    pub max_gleaning: usize,
}

impl Default for GleaningAdmissionOptions {
    fn default() -> Self {
        Self {
            enable_gleaning: default_enable_gleaning(),
            max_gleaning: default_max_gleaning(),
        }
    }
}

/// Input for admitting a document into the async worker queue.
#[derive(Debug, Clone)]
pub struct DocumentAdmissionInput {
    pub text_content: String,
    pub title: String,
    pub source_type: &'static str,
    pub mime_type: Option<String>,
    pub raw_byte_size: usize,
    pub content_hash: String,
    pub custom_metadata: Option<Value>,
    pub track_id: Option<String>,
    pub gleaning: GleaningAdmissionOptions,
    pub document_type: Option<&'static str>,
    /// Explicit chunk strategy; auto-selects markdown for `.md` when None.
    pub chunk_strategy: Option<ChunkStrategy>,
    pub chunk_options: Option<ChunkOptions>,
    /// True when content originated from VLM image analysis (SPEC-026 P-07).
    pub multimodal: bool,
    /// Ingest path label, e.g. `"vlm_describe"` for image uploads.
    pub ingest_mode: Option<&'static str>,
    /// Virtual sidecar manifest for multimodal uploads (Phase 4e).
    pub multimodal_manifest: Option<crate::services::MultimodalManifest>,
}

/// Result of successful admission (202 path).
#[derive(Debug, Clone)]
pub struct DocumentAdmissionAccepted {
    pub document_id: String,
    pub track_id: String,
    pub task_id: String,
    pub content_hash: String,
}

/// Result when duplicate is still processing (no new task).
#[derive(Debug, Clone)]
pub struct DocumentAdmissionDuplicateProcessing {
    pub document_id: String,
}

/// Admission outcome for upload handlers.
#[derive(Debug, Clone)]
pub enum DocumentAdmissionOutcome {
    Accepted(DocumentAdmissionAccepted),
    DuplicateProcessing(DocumentAdmissionDuplicateProcessing),
}

impl DocumentAdmissionInput {
    pub fn build_track_id(&self, prefix: &str) -> String {
        self.track_id.clone().unwrap_or_else(|| {
            format!(
                "{prefix}_{}_{}",
                Utc::now().format("%Y%m%d%H%M%S"),
                &Uuid::new_v4().to_string()[..8]
            )
        })
    }
}

/// Admit document: dedup, KV writes, enqueue worker task.
pub async fn admit_document_for_processing(
    state: &AppState,
    tenant_ctx: &TenantContext,
    input: DocumentAdmissionInput,
    track_prefix: &str,
) -> ApiResult<DocumentAdmissionOutcome> {
    let workspace_id = tenant_ctx.workspace_id_or_default();
    let tenant_id = tenant_ctx.tenant_id_or_default();

    let hash_key = ContentHasher::workspace_hash_key(&workspace_id, &input.content_hash);
    let staging_hash_key = kv_keys::staging_workspace_hash(&workspace_id, &input.content_hash);

    match resolve_workspace_duplicate_for_reingestion(state, tenant_ctx, &hash_key, &workspace_id)
        .await?
    {
        DuplicateReingestAction::NoDuplicate => {}
        DuplicateReingestAction::ClearedForReingestion { old_document_id } => {
            tracing::info!(
                old_doc_id = %old_document_id,
                workspace_id = %workspace_id,
                title = %input.title,
                "Duplicate cleared for re-ingestion"
            );
        }
        DuplicateReingestAction::StillProcessing {
            existing_document_id,
        } => {
            return Ok(DocumentAdmissionOutcome::DuplicateProcessing(
                DocumentAdmissionDuplicateProcessing {
                    document_id: existing_document_id,
                },
            ));
        }
    }

    // In-flight staging duplicate (P-11)
    if let Some(staging_doc) = state
        .storage
        .kv_storage
        .get_by_id(&staging_hash_key)
        .await?
        .and_then(|v| v.as_str().map(|s| s.to_string()))
    {
        return Ok(DocumentAdmissionOutcome::DuplicateProcessing(
            DocumentAdmissionDuplicateProcessing {
                document_id: staging_doc,
            },
        ));
    }

    if let Some(ref opts) = input.chunk_options {
        opts.validate().map_err(ApiError::ValidationError)?;
    }

    let chunk_strategy = ChunkStrategy::resolve_for_upload(
        input.chunk_strategy,
        input.mime_type.as_deref(),
        &input.title,
    );

    let document_id = crate::services::ingest_admission::allocate_new_document_id(state).await;
    let track_id = input.build_track_id(track_prefix);
    let content_summary = crate::validation::generate_content_summary(&input.text_content);
    let content_length = input.text_content.len();

    // P-11: staging KV only until worker promotes on success
    state
        .storage
        .kv_storage
        .upsert(&[(staging_hash_key, json!(document_id))])
        .await?;

    let staging_metadata_key = kv_keys::staging_doc_metadata(&document_id);
    let mut doc_metadata = json!({
        "id": document_id,
        "title": input.title,
        "content_summary": content_summary,
        "content_length": content_length,
        "file_size_bytes": input.raw_byte_size,
        "content_hash": input.content_hash,
        "sha256_checksum": input.content_hash,
        "track_id": track_id,
        "created_at": Utc::now().to_rfc3339(),
        "status": "pending",
        "tenant_id": tenant_id,
        "workspace_id": workspace_id,
        "source_type": input.source_type,
        "chunking_strategy": chunk_strategy.as_str(),
        "current_stage": "uploading",
        "stage_progress": 0.0,
        "stage_message": "Document received, starting processing",
        "admission_staging": true,
    });

    if let Some(mime) = &input.mime_type {
        doc_metadata["mime_type"] = json!(mime);
        doc_metadata["file_name"] = json!(input.title);
        doc_metadata["file_size"] = json!(input.raw_byte_size);
    }
    if let Some(doc_type) = input.document_type {
        doc_metadata["document_type"] = json!(doc_type);
    }
    if let Some(custom) = input.custom_metadata {
        doc_metadata["custom_metadata"] = custom;
    }
    if input.multimodal {
        doc_metadata["multimodal"] = json!(true);
    }
    if let Some(mode) = input.ingest_mode {
        doc_metadata["ingest_mode"] = json!(mode);
    }

    if let Some(ref manifest) = input.multimodal_manifest {
        let _ = persist_manifest(&*state.storage.kv_storage, &document_id, manifest).await;
        let summary = MultimodalSummary::from_records(
            &manifest
                .items
                .iter()
                .filter_map(|i| i.analyze_result.as_ref())
                .cloned()
                .collect::<Vec<_>>(),
        );
        if let Some(patch) = metadata_multimodal_patch(&summary, None).as_object() {
            if let Some(obj) = doc_metadata.as_object_mut() {
                for (k, v) in patch {
                    obj.insert(k.clone(), v.clone());
                }
                // Standalone image uploads imply `i` for mm-chunk indexing (Phase 4g).
                if input.source_type == "image" {
                    apply_process_options_to_metadata(obj, Some("i"));
                }
            }
        }
    }

    state
        .storage
        .kv_storage
        .upsert(&[(staging_metadata_key, doc_metadata)])
        .await?;

    state
        .storage
        .kv_storage
        .upsert(&[(
            kv_keys::staging_doc_content(&document_id),
            json!({ "content": input.text_content }),
        )])
        .await?;

    let chunk_options_json = input
        .chunk_options
        .as_ref()
        .and_then(|o| serde_json::to_value(o).ok());

    // SPEC-025 6.1: task payload references KV only — no duplicate text in JSONB.
    let task_data = TextInsertData {
        text: String::new(),
        file_source: input.title.clone(),
        workspace_id: workspace_id.clone(),
        metadata: Some(json!({
            "document_id": document_id,
            "title": input.title,
            "tenant_id": tenant_id,
            "workspace_id": workspace_id,
            "source_type": input.source_type,
            "mime_type": input.mime_type,
            "content_hash": input.content_hash,
            "file_size_bytes": input.raw_byte_size,
            "enable_gleaning": input.gleaning.enable_gleaning,
            "max_gleaning": input.gleaning.max_gleaning,
            "chunk_strategy": chunk_strategy.as_str(),
            "chunk_options": chunk_options_json,
        })),
    };

    let task = Task::new(
        uuid::Uuid::parse_str(&tenant_id)
            .map_err(|_| ApiError::ValidationError("Invalid tenant ID".to_string()))?,
        uuid::Uuid::parse_str(&workspace_id)
            .map_err(|_| ApiError::ValidationError("Invalid workspace ID".to_string()))?,
        TaskType::Insert,
        serde_json::to_value(task_data).unwrap(),
    );
    let task_id = task.track_id.clone();
    state.enqueue_task(task).await?;

    Ok(DocumentAdmissionOutcome::Accepted(
        DocumentAdmissionAccepted {
            document_id,
            track_id,
            task_id,
            content_hash: input.content_hash,
        },
    ))
}

/// HTTP status for accepted async upload.
pub const ADMISSION_ACCEPTED_STATUS: StatusCode = StatusCode::ACCEPTED;

/// Parse chunk strategy + options from JSON upload fields (SSOT for all upload paths).
pub fn parse_upload_chunk_fields(
    chunk_strategy: Option<&str>,
    chunk_options: Option<serde_json::Value>,
) -> (Option<ChunkStrategy>, Option<ChunkOptions>) {
    let strategy = chunk_strategy.and_then(ChunkStrategy::parse);
    let options = chunk_options.and_then(|v| serde_json::from_value(v).ok());
    (strategy, options)
}

/// Multipart form fields shared by file and batch upload handlers (DRY).
#[derive(Debug, Default, Clone)]
pub struct MultipartUploadFields {
    pub metadata: Option<Value>,
    chunk_strategy_raw: Option<String>,
    chunk_options_raw: Option<Value>,
}

impl MultipartUploadFields {
    pub fn ingest_text_field(&mut self, name: &str, text: &str) {
        match name {
            "metadata" if !text.is_empty() => {
                self.metadata = serde_json::from_str(text).ok();
            }
            "chunk_strategy" if !text.is_empty() => {
                self.chunk_strategy_raw = Some(text.to_string());
            }
            "chunk_options" if !text.is_empty() => {
                self.chunk_options_raw = serde_json::from_str(text).ok();
            }
            _ => {}
        }
    }

    /// Resolved chunk fields with metadata envelope fallback.
    pub fn effective_chunk_fields(
        &self,
    ) -> (Option<ChunkStrategy>, Option<ChunkOptions>, Option<Value>) {
        let (mut strategy, mut options) = parse_upload_chunk_fields(
            self.chunk_strategy_raw.as_deref(),
            self.chunk_options_raw.clone(),
        );
        if let Some(ref meta) = self.metadata {
            let (meta_strategy, meta_options) = chunk_fields_from_metadata(meta);
            if strategy.is_none() {
                strategy = meta_strategy;
            }
            if options.is_none() {
                options = meta_options;
            }
        }
        (strategy, options, self.metadata.clone())
    }
}

/// Merge chunk fields from custom metadata JSON (batch/file metadata envelope).
pub fn chunk_fields_from_metadata(
    metadata: &serde_json::Value,
) -> (Option<ChunkStrategy>, Option<ChunkOptions>) {
    let strategy = metadata
        .get("chunk_strategy")
        .and_then(|v| v.as_str())
        .and_then(ChunkStrategy::parse);
    let options = metadata
        .get("chunk_options")
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    (strategy, options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn default_gleaning_matches_upload_defaults() {
        let opts = GleaningAdmissionOptions::default();
        assert!(opts.enable_gleaning);
        assert_eq!(opts.max_gleaning, 1);
    }

    #[test]
    fn admission_task_payload_is_kv_ref_only() {
        let task_data = TextInsertData {
            text: String::new(),
            file_source: "sample.md".to_string(),
            workspace_id: "default".to_string(),
            metadata: Some(json!({
                "document_id": "doc-123",
                "title": "sample.md",
            })),
        };
        let serialized = serde_json::to_value(&task_data).unwrap();
        assert_eq!(serialized.get("text").and_then(|v| v.as_str()), Some(""));
    }

    #[test]
    fn parse_upload_chunk_fields_ssot() {
        let (strategy, opts) =
            parse_upload_chunk_fields(Some("recursive"), Some(json!({ "chunk_token_size": 1200 })));
        assert_eq!(strategy, Some(ChunkStrategy::Recursive));
        assert_eq!(opts.and_then(|o| o.chunk_token_size), Some(1200));
    }

    #[test]
    fn resolve_upload_defaults_to_recursive_for_plain_text() {
        assert_eq!(
            ChunkStrategy::resolve_for_upload(None, Some("text/plain"), "notes.txt"),
            ChunkStrategy::Recursive
        );
    }
}
