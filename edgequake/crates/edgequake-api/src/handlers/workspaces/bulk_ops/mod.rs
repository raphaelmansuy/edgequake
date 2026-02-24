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

pub use rebuild_embeddings::rebuild_embeddings;
pub use rebuild_knowledge_graph::rebuild_knowledge_graph;
pub use reprocess_documents::reprocess_all_documents;

use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::isolation::doc_belongs_to_workspace;
use crate::state::AppState;

// ============================================================================
// Shared Types
// ============================================================================

/// Parsed document metadata from KV storage.
///
/// Extracted during workspace document discovery to avoid re-parsing JSON
/// in each handler.
pub(super) struct DocumentInfo {
    pub doc_id: String,
    pub title: String,
    pub chunk_count: usize,
    pub source_type: Option<String>,
    pub pdf_id_str: Option<String>,
    pub status: Option<String>,
}

// ============================================================================
// Shared Helpers (DRY extraction from rebuild/reprocess handlers)
// ============================================================================

/// Collect all documents belonging to a workspace from KV storage.
///
/// Iterates all `-metadata` keys, checks workspace ownership via
/// [`doc_belongs_to_workspace`], and parses relevant document fields.
///
/// WHY: All three bulk operations (rebuild embeddings, rebuild knowledge graph,
/// reprocess documents) need the same document discovery logic. Extracting it
/// here eliminates ~40 lines of duplicated code per handler.
pub(super) async fn collect_workspace_documents(
    state: &AppState,
    workspace_id: &Uuid,
    workspace_slug: &str,
) -> Result<Vec<DocumentInfo>, ApiError> {
    let all_keys: Vec<String> = state
        .kv_storage
        .keys()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to list document keys: {}", e)))?;

    let mut docs = Vec::new();

    for key in all_keys.iter().filter(|k| k.ends_with("-metadata")) {
        let value = match state.kv_storage.get_by_id(key).await {
            Ok(Some(v)) => v,
            Ok(None) => continue,
            Err(e) => {
                tracing::warn!(key = %key, error = %e, "Failed to read document metadata, skipping");
                continue;
            }
        };

        let obj = match value.as_object() {
            Some(o) => o,
            None => continue,
        };

        // WHY: Rebuild must be strictly workspace-scoped so that triggering
        // a rebuild on workspace X never touches workspace Y's documents.
        // Legacy documents may store workspace_id = "default" (string literal)
        // instead of a real UUID; treat those as belonging to the workspace
        // whose slug is also "default".
        let doc_workspace = obj
            .get("workspace_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        if !doc_belongs_to_workspace(doc_workspace, &workspace_id.to_string(), workspace_slug) {
            continue;
        }

        let doc_id = match obj.get("id").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => continue,
        };

        let chunk_count = obj.get("chunk_count").and_then(|v| v.as_u64()).unwrap_or(1) as usize;

        let title = obj
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or(&doc_id)
            .to_string();

        let source_type = obj
            .get("source_type")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let pdf_id_str = obj
            .get("pdf_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let status = obj
            .get("status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        docs.push(DocumentInfo {
            doc_id,
            title,
            chunk_count,
            source_type,
            pdf_id_str,
            status,
        });
    }

    Ok(docs)
}

/// Build a [`PdfProcessingData`] task for re-extracting a document from its
/// original PDF bytes using the workspace's current vision LLM.
///
/// SPEC-041: PDF documents are re-queued as PdfProcessing tasks to re-extract
/// from the original PDF using the workspace's current vision LLM, then rechunk
/// and re-embed with the new embedding model.
pub(super) fn build_pdf_task(
    workspace: &edgequake_core::Workspace,
    workspace_id: Uuid,
    pdf_id: Uuid,
    doc_id: &str,
) -> edgequake_tasks::PdfProcessingData {
    let vision_provider = workspace
        .vision_llm_provider
        .as_deref()
        .filter(|p| !p.is_empty())
        .unwrap_or("ollama")
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
    }
}

/// Read stored text content for a document from KV storage.
///
/// Returns `None` if the content key doesn't exist or the content field
/// is missing from the stored JSON.
pub(super) async fn read_stored_content(state: &AppState, doc_id: &str) -> Option<String> {
    let content_key = format!("{}-content", doc_id);
    match state.kv_storage.get_by_id(&content_key).await {
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

    let metadata_key = format!("{}-metadata", doc_id);
    if let Some(mut metadata) = state
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
            let _ = state.kv_storage.upsert(&[(metadata_key, metadata)]).await;
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
