//! Workspace bulk operations: rebuild embeddings, rebuild knowledge graph,
//! and reprocess all documents.
//!
//! Implements SPEC-032 (rebuild endpoints) and SPEC-041 (PDF reprocessing).
//!
//! ## DRY Shared Helpers
//!
//! The three bulk operations share significant document discovery and task
//! routing logic. Common patterns are extracted into:
//!
//! - [`DocumentInfo`]: Parsed document metadata
//! - [`collect_workspace_documents`]: Workspace-scoped document discovery
//! - [`build_pdf_task`]: PDF reprocessing task construction
//! - [`read_stored_content`]: Text content retrieval from KV storage
//! - [`enqueue_then_bind_pending`]: Task-first enqueue then bind KV (issue #384)
//! - [`build_reprocess_task`]: SPEC-041 source-type routing (PDF vs text)

mod rebuild_embeddings;
mod rebuild_knowledge_graph;
mod reprocess_documents;

pub use rebuild_embeddings::*;
pub use rebuild_knowledge_graph::*;
pub use reprocess_documents::*;

use uuid::Uuid;

use crate::error::ApiError;
use crate::services::document_metadata_scan::{load_workspace_documents, WorkspaceDocumentRecord};
use crate::state::AppState;

// ============================================================================
// Shared Types
// ============================================================================

/// Parsed document metadata from KV storage (SSOT re-export).
pub(super) type DocumentInfo = WorkspaceDocumentRecord;

// ============================================================================
// Shared Helpers (DRY extraction from rebuild/reprocess handlers)
// ============================================================================

/// Collect all documents belonging to a workspace from KV storage (SSOT delegate).
pub(super) async fn collect_workspace_documents(
    state: &AppState,
    workspace_id: &Uuid,
    workspace_slug: &str,
) -> Result<Vec<DocumentInfo>, ApiError> {
    load_workspace_documents(
        state.storage.kv_storage.as_ref(),
        state.optional_pg_pool(),
        workspace_id,
        workspace_slug,
    )
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to read document metadata: {e}")))
}

/// Build a [`PdfProcessingData`] task for re-extracting a document from its
/// original PDF bytes using the workspace's current PDF parser configuration.
///
/// SPEC-041: PDF documents are re-queued as PdfProcessing tasks to re-extract
/// from the original PDF using the workspace's current PDF parser backend, then
/// rechunk and re-embed with the new embedding model.
pub(super) fn build_pdf_task(
    workspace: &edgequake_core::Workspace,
    tenant: Option<&edgequake_core::Tenant>,
    workspace_id: Uuid,
    pdf_id: Uuid,
    doc_id: &str,
) -> edgequake_tasks::PdfProcessingData {
    // SPEC-123: same vision SSOT as upload (LAW-123-5).
    let vision = edgequake_core::resolve_vision_llm_choice(None, None, Some(workspace), tenant);
    let vision_provider = vision.provider;
    let vision_model = Some(vision.model.clone()).filter(|m| !m.is_empty());
    let vision_model_for_resolve = vision.model.clone();
    let vision_reasoning_effort = crate::services::resolve_vlm_reasoning_effort(
        Some(workspace),
        &vision_provider,
        &vision_model_for_resolve,
        None,
        None,
    );

    edgequake_tasks::PdfProcessingData {
        pdf_id,
        tenant_id: workspace.tenant_id,
        workspace_id,
        enable_vision: true,
        vision_provider,
        vision_model,
        // FIX-REBUILD: Pass existing document ID so the processor updates
        // the existing document in-place instead of creating a duplicate.
        existing_document_id: Some(doc_id.to_string()),
        pdf_parser_backend: workspace.resolved_pdf_parser_backend(),
        pdf_parser_backend_explicit: workspace.pdf_parser_backend.is_some(),
        // WHY: Workspace bulk rebuild re-extracts the KG from the existing
        // markdown; it does not re-convert PDFs by default (avoid spending
        // vision tokens on every rebuild). Restart stays false so the resume
        // shortcut reuses cached markdown.
        restart_from_scratch: false,
        reprocess_mode: Some(edgequake_tasks::ReprocessMode::EntitiesOnly),
        multimodal_process_options: None,
        vision_reasoning_effort,
        vision_extract: Default::default(),
    }
}

/// Read stored text content for a document from KV storage.
///
/// Returns `None` if the content key doesn't exist or the content field
/// is missing from the stored JSON.
pub(super) async fn read_stored_content(state: &AppState, doc_id: &str) -> Option<String> {
    let content_key = format!("{}-content", doc_id);
    match state.storage.kv_storage.get_by_id(&content_key).await {
        Ok(Some(cv)) => cv
            .get("content")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        _ => None,
    }
}

/// Outcome of task-first bulk reprocess (issue #384).
///
/// Status is a projection of a live Task. `no_content` and enqueue failure
/// must not write `pending`.
pub(super) enum BulkReprocessCommit {
    Queued,
    SkippedNoContent,
    EnqueueFailed,
}

/// Persist a reprocess task, then project document KV to pending.
///
/// Never writes in-flight status before `create_task` succeeds. Batch
/// correlation stays on `batch_track_id`; `track_id` is the worker task id.
pub(super) async fn enqueue_then_bind_pending(
    state: &AppState,
    workspace: &edgequake_core::Workspace,
    workspace_id: Uuid,
    doc: &DocumentInfo,
    batch_track_id: &str,
    extra_metadata: serde_json::Map<String, serde_json::Value>,
) -> BulkReprocessCommit {
    let Some((task_type, task_value)) = build_reprocess_task(
        state,
        workspace,
        workspace_id,
        doc,
        batch_track_id,
        extra_metadata,
    )
    .await
    else {
        return BulkReprocessCommit::SkippedNoContent;
    };

    let task = edgequake_tasks::Task::new(workspace.tenant_id, workspace_id, task_type, task_value);
    let task_id = task.track_id.clone();

    if let Err(e) = state.enqueue_task(task).await {
        tracing::info!(
            error = %e,
            doc_id = %doc.doc_id,
            "Failed to enqueue task, skipping"
        );
        return BulkReprocessCommit::EnqueueFailed;
    }

    bind_document_pending_to_task(state, &doc.doc_id, &task_id, batch_track_id).await;
    BulkReprocessCommit::Queued
}

/// Bind KV `pending` + worker `track_id` after the task row exists (issue #384).
async fn bind_document_pending_to_task(
    state: &AppState,
    doc_id: &str,
    task_id: &str,
    batch_track_id: &str,
) {
    use chrono::Utc;

    let metadata_key = crate::services::document_metadata_scan::metadata_key_for_document(doc_id);
    if let Some(mut metadata) = state
        .storage
        .kv_storage
        .get_by_id(&metadata_key)
        .await
        .ok()
        .flatten()
    {
        if let Some(obj) = metadata.as_object_mut() {
            obj.insert("status".to_string(), serde_json::json!("pending"));
            obj.insert("track_id".to_string(), serde_json::json!(task_id));
            obj.insert(
                "batch_track_id".to_string(),
                serde_json::json!(batch_track_id),
            );
            obj.insert(
                "reprocess_at".to_string(),
                serde_json::json!(Utc::now().to_rfc3339()),
            );
            if let Err(e) = crate::services::upsert_metadata_kv_with_index(
                state.storage.kv_storage.as_ref(),
                &metadata_key,
                metadata,
            )
            .await
            {
                tracing::error!(
                    error = %e,
                    document_id = %doc_id,
                    task_id = %task_id,
                    "task persisted but document KV bind failed (SPEC-057 heals orphan tasks)"
                );
            }
        }
    }
}

/// Build a reprocess task for a document, routing by source type (SPEC-041).
///
/// - PDF documents with a valid `pdf_id` → [`PdfProcessingData`] task to
///   re-extract from the original PDF bytes.
/// - Text/Markdown documents or PDFs without a valid `pdf_id` → [`TextInsertData`]
///   task using stored content.
///
/// Returns `None` if the document has no usable content (text documents
/// without stored content are skipped).
///
/// `extra_metadata` allows callers to inject additional fields into the
/// TextInsertData metadata (e.g., `is_embedding_rebuild: true`).
pub(super) async fn build_reprocess_task(
    state: &AppState,
    workspace: &edgequake_core::Workspace,
    workspace_id: Uuid,
    doc: &DocumentInfo,
    track_id: &str,
    extra_metadata: serde_json::Map<String, serde_json::Value>,
) -> Option<(edgequake_tasks::TaskType, serde_json::Value)> {
    use edgequake_tasks::{TaskType, TextInsertData};

    // SPEC-041: Route by source type.
    // PDF with valid pdf_id → re-extract from original PDF.
    if doc.source_type.as_deref() == Some("pdf") {
        if let Some(pdf_id_str) = doc.pdf_id_str.as_deref() {
            if let Ok(pdf_id_uuid) = Uuid::parse_str(pdf_id_str) {
                let tenant = state
                    .workspace_service
                    .get_tenant(workspace.tenant_id)
                    .await
                    .ok()
                    .flatten();
                let pdf_task = build_pdf_task(
                    workspace,
                    tenant.as_ref(),
                    workspace_id,
                    pdf_id_uuid,
                    &doc.doc_id,
                );
                return Some((
                    TaskType::PdfProcessing,
                    serde_json::to_value(&pdf_task).unwrap(),
                ));
            }
            // Malformed pdf_id — log warning and fall through to text path
            tracing::warn!(
                doc_id = %doc.doc_id,
                pdf_id = %pdf_id_str,
                "Malformed pdf_id, falling back to text reprocess"
            );
        }
        // No pdf_id stored — fall through to text path
    }

    // Text/Markdown or PDF without valid pdf_id — read stored content.
    let content = read_stored_content(state, &doc.doc_id).await?;

    let mut metadata_map = serde_json::Map::new();
    metadata_map.insert("document_id".to_string(), serde_json::json!(doc.doc_id));
    metadata_map.insert("title".to_string(), serde_json::json!(doc.title));
    metadata_map.insert("track_id".to_string(), serde_json::json!(track_id));
    metadata_map.insert("is_reprocess".to_string(), serde_json::json!(true));
    metadata_map.insert(
        "workspace_id".to_string(),
        serde_json::json!(workspace_id.to_string()),
    );
    metadata_map.insert(
        "tenant_id".to_string(),
        serde_json::json!(workspace.tenant_id.to_string()),
    );
    metadata_map.extend(extra_metadata);

    let text_task = TextInsertData {
        text: content,
        file_source: doc.title.clone(),
        workspace_id: workspace_id.to_string(),
        metadata: Some(serde_json::Value::Object(metadata_map)),
    };

    Some((TaskType::Insert, serde_json::to_value(&text_task).unwrap()))
}
