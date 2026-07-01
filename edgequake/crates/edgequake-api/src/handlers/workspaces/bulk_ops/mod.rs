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
//! - [`mark_document_pending`]: Document status update to "pending"
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
    workspace_id: Uuid,
    pdf_id: Uuid,
    doc_id: &str,
) -> edgequake_tasks::PdfProcessingData {
    // WHY: When workspace has no explicit vision_llm_provider, fall back to the
    // workspace's main llm_provider instead of hardcoding "ollama". This ensures
    // OpenAI workspaces also rebuild with the correct vision provider.
    let vision_provider = workspace
        .vision_llm_provider
        .as_deref()
        .filter(|p| !p.is_empty())
        .unwrap_or(workspace.llm_provider.as_str())
        .to_string();
    let vision_model = workspace.vision_llm_model.clone().filter(|m| !m.is_empty());

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

/// Mark a document as "pending" for reprocessing in KV storage.
///
/// Updates the document's metadata to set:
/// - `status` → "pending"
/// - `track_id` → the batch tracking ID
/// - `reprocess_at` → current timestamp
pub(super) async fn mark_document_pending(state: &AppState, doc_id: &str, track_id: &str) {
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
            obj.insert("track_id".to_string(), serde_json::json!(track_id));
            obj.insert(
                "reprocess_at".to_string(),
                serde_json::json!(Utc::now().to_rfc3339()),
            );
            let _ = crate::services::upsert_metadata_kv_with_index(
                state.storage.kv_storage.as_ref(),
                &metadata_key,
                metadata,
            )
            .await;
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
                let pdf_task = build_pdf_task(workspace, workspace_id, pdf_id_uuid, &doc.doc_id);
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
