//! GAP-039: Reprocess failed documents handler.
//!
//! Finds documents in "failed" or "cancelled" status and requeues them
//! for processing. Supports both KV-based text documents and PostgreSQL
//! PDF documents (via `postgres` feature).

use axum::{extract::State, Json};
use chrono::Utc;
use edgequake_pdf::PdfParserBackend;
use tracing::debug;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::documents_types::*;
use crate::middleware::TenantContext;
use crate::state::AppState;

use super::super::storage_helpers::{cleanup_document_graph_data, metadata_matches_tenant_context};

/// Reprocess failed documents.
#[utoipa::path(
    post,
    path = "/api/v1/documents/reprocess",
    tag = "Documents",
    request_body = ReprocessFailedRequest,
    responses(
        (status = 200, description = "Documents requeued for processing", body = ReprocessFailedResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn reprocess_failed(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    // WHY: Body is optional - frontend may omit body entirely, which would cause
    // "EOF while parsing a value" 400 error. Using Option<Json<>> with .unwrap_or_default()
    // makes this endpoint resilient to missing or empty request body.
    body: Option<Json<ReprocessFailedRequest>>,
) -> ApiResult<Json<ReprocessFailedResponse>> {
    let request = body.map(|b| b.0).unwrap_or_default();
    debug!(
        "reprocess_failed called with tenant context: tenant_id={:?}, workspace_id={:?}, document_id={:?}, force={}",
        tenant_ctx.tenant_id, tenant_ctx.workspace_id, request.document_id, request.force
    );

    // Generate new track ID for reprocess batch
    let new_track_id = format!(
        "reprocess_{}_{}",
        Utc::now().format("%Y%m%d_%H%M%S"),
        &Uuid::new_v4().to_string()[..8]
    );

    // Get all metadata keys
    let all_keys: Vec<String> = state.kv_storage.keys().await?;

    let mut docs_to_reprocess = Vec::new();
    let mut requeued_ids = Vec::new();

    // Find documents to reprocess
    for key in all_keys.iter().filter(|k| k.ends_with("-metadata")) {
        if docs_to_reprocess.len() >= request.max_documents {
            break;
        }

        if let Some(value) = state.kv_storage.get_by_id(key).await? {
            if !metadata_matches_tenant_context(&value, &tenant_ctx) {
                continue;
            }

            if let Some(obj) = value.as_object() {
                let status = obj.get("status").and_then(|v| v.as_str());
                let doc_track_id = obj.get("track_id").and_then(|v| v.as_str());
                let doc_id = obj.get("id").and_then(|v| v.as_str());

                // If document_id filter is specified, only match that exact document
                if let Some(ref filter_doc_id) = request.document_id {
                    if doc_id != Some(filter_doc_id.as_str()) {
                        continue;
                    }
                    // When document_id is specified with force=true, allow any status
                    // Otherwise, only reprocess if failed
                    if !request.force && status != Some("failed") {
                        continue;
                    }
                    if let Some(id) = doc_id {
                        docs_to_reprocess.push((id.to_string(), key.replace("-metadata", "")));
                    }
                    break; // Found the specific document
                }

                // If track_id filter is specified, match by track_id
                if let Some(ref filter_track) = request.track_id {
                    if doc_track_id != Some(filter_track.as_str()) {
                        continue;
                    }
                }

                // Default behavior: reprocess failed and cancelled documents
                // WHY: Cancelled documents should be retryable just like failed ones.
                // Users may cancel a document during processing and want to retry later.
                if status == Some("failed") || status == Some("cancelled") {
                    if let Some(id) = doc_id {
                        docs_to_reprocess.push((id.to_string(), key.replace("-metadata", "")));
                    }
                }
            }
        }
    }

    // Requeue documents for processing
    for (doc_id, _doc_key) in &docs_to_reprocess {
        // OODA-08: Clean up partial graph data from previous attempt BEFORE requeueing
        // WHY: Without cleanup, reprocessing creates duplicate entities and corrupts source_ids
        //
        // Scenario without cleanup:
        //   T1: Document processed 60% → entities A, B created with source_ids = [doc]
        //   T2: Processing fails
        //   T3: reprocess_failed called
        //   T4: Document reprocessed → entities A, B upserted with source_ids = [doc]
        //   T5: Now entities have inflated source_ids (double reference)
        //   T6: Delete document → entities still exist (incorrect)
        //
        // With cleanup:
        //   T1-T2: Same as above
        //   T3: reprocess_failed cleans up A, B (deletes them since source_ids = [doc])
        //   T4: Document reprocessed → entities A, B created fresh
        //   T5: source_ids correctly = [doc]
        //   T6: Delete document → entities properly deleted
        match cleanup_document_graph_data(doc_id, &state.graph_storage, None).await {
            Ok(stats) => {
                tracing::info!(
                    document_id = %doc_id,
                    entities_removed = stats.entities_removed,
                    entities_updated = stats.entities_updated,
                    relationships_removed = stats.relationships_removed,
                    "Cleaned up partial data before reprocessing"
                );
            }
            Err(e) => {
                tracing::warn!(
                    document_id = %doc_id,
                    error = %e,
                    "Failed to cleanup partial data before reprocessing, continuing anyway"
                );
            }
        }
        // Get document content
        let content_key = format!("{}-content", doc_id);

        // FIX-REBUILD: Read metadata to check if this is a PDF document
        // WHY: PDF documents must be routed through PdfProcessing tasks so the full
        // pipeline runs from original PDF bytes (vision extraction → chunking →
        // embedding → entity extraction). Using TaskType::Insert for PDFs would
        // only re-ingest the previously extracted markdown, missing re-extraction
        // with any new vision LLM model.
        let metadata_key = format!("{}-metadata", doc_id);
        let metadata_opt = state.kv_storage.get_by_id(&metadata_key).await?;

        let source_type = metadata_opt
            .as_ref()
            .and_then(|m| m.as_object())
            .and_then(|obj| obj.get("source_type"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let pdf_id_str = metadata_opt
            .as_ref()
            .and_then(|m| m.as_object())
            .and_then(|obj| obj.get("pdf_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Use tenant context for workspace_id, fallback to "default"
        let workspace_id = tenant_ctx
            .workspace_id
            .clone()
            .unwrap_or_else(|| "default".to_string());
        let tenant_id = tenant_ctx
            .tenant_id
            .clone()
            .unwrap_or_else(|| "default".to_string());

        // FIX-REBUILD: Route PDF documents through PdfProcessing for full re-extraction
        let task_created =
            if source_type.as_deref() == Some("pdf") {
                if let Some(ref pid_str) = pdf_id_str {
                    if let Ok(pdf_id_uuid) = uuid::Uuid::parse_str(pid_str) {
                        // Update status to pending
                        if let Some(mut metadata) = metadata_opt.clone() {
                            if let Some(obj) = metadata.as_object_mut() {
                                obj.insert("status".to_string(), serde_json::json!("pending"));
                                obj.insert("track_id".to_string(), serde_json::json!(new_track_id));
                                obj.insert(
                                    "retry_at".to_string(),
                                    serde_json::json!(Utc::now().to_rfc3339()),
                                );
                                state
                                    .kv_storage
                                    .upsert(&[(metadata_key.clone(), metadata)])
                                    .await?;
                            }
                        }

                        // Look up workspace to get vision provider/model settings
                        let (vision_provider, vision_model, pdf_parser_backend) =
                            if let Ok(ws_uuid) = uuid::Uuid::parse_str(&workspace_id) {
                                if let Ok(Some(ws)) =
                                    state.workspace_service.get_workspace(ws_uuid).await
                                {
                                    let vp = ws
                                        .vision_llm_provider
                                        .as_deref()
                                        .filter(|p| !p.is_empty())
                                        .unwrap_or("ollama")
                                        .to_string();
                                    let vm = ws.vision_llm_model.clone().filter(|m| !m.is_empty());
                                    (vp, vm, ws.resolved_pdf_parser_backend())
                                } else {
                                    (
                                        "ollama".to_string(),
                                        None,
                                        PdfParserBackend::from_env().unwrap_or_default(),
                                    )
                                }
                            } else {
                                (
                                    "ollama".to_string(),
                                    None,
                                    PdfParserBackend::from_env().unwrap_or_default(),
                                )
                            };

                        use edgequake_tasks::{PdfProcessingData, Task, TaskType};

                        let pdf_task = PdfProcessingData {
                            pdf_id: pdf_id_uuid,
                            tenant_id: uuid::Uuid::parse_str(&tenant_id).map_err(|_| {
                                ApiError::ValidationError("Invalid tenant ID".to_string())
                            })?,
                            workspace_id: uuid::Uuid::parse_str(&workspace_id).map_err(|_| {
                                ApiError::ValidationError("Invalid workspace ID".to_string())
                            })?,
                            enable_vision: true,
                            vision_provider,
                            vision_model,
                            // FIX-REBUILD: Reuse existing document ID
                            existing_document_id: Some(doc_id.clone()),
                            pdf_parser_backend,
                            restart_from_scratch: false,
                        };

                        let task = Task::new(
                            uuid::Uuid::parse_str(&tenant_id).map_err(|_| {
                                ApiError::ValidationError("Invalid tenant ID".to_string())
                            })?,
                            uuid::Uuid::parse_str(&workspace_id).map_err(|_| {
                                ApiError::ValidationError("Invalid workspace ID".to_string())
                            })?,
                            TaskType::PdfProcessing,
                            serde_json::to_value(&pdf_task).unwrap(),
                        );

                        state.task_storage.create_task(&task).await.map_err(|e| {
                            ApiError::Internal(format!("Failed to create task: {}", e))
                        })?;

                        state.task_queue.send(task).await.map_err(|e| {
                            ApiError::Internal(format!("Failed to queue task: {}", e))
                        })?;

                        tracing::info!(
                            document_id = %doc_id,
                            pdf_id = %pid_str,
                            "Queued PDF reprocessing task (PdfProcessing) with existing document ID"
                        );
                        true
                    } else {
                        false // Invalid pdf_id, fall through to text reprocess
                    }
                } else {
                    false // No pdf_id, fall through to text reprocess
                }
            } else {
                false // Not a PDF document
            };

        // Fallback: text/markdown documents or PDF without valid pdf_id
        if !task_created {
            if let Some(content_value) = state.kv_storage.get_by_id(&content_key).await? {
                if let Some(content) = content_value.get("content").and_then(|v| v.as_str()) {
                    // Update status to pending
                    if let Some(mut metadata) = state.kv_storage.get_by_id(&metadata_key).await? {
                        if let Some(obj) = metadata.as_object_mut() {
                            obj.insert("status".to_string(), serde_json::json!("pending"));
                            obj.insert("track_id".to_string(), serde_json::json!(new_track_id));
                            obj.insert(
                                "retry_at".to_string(),
                                serde_json::json!(Utc::now().to_rfc3339()),
                            );

                            state.kv_storage.upsert(&[(metadata_key, metadata)]).await?;
                        }
                    }

                    // Create new task
                    use edgequake_tasks::{Task, TaskType, TextInsertData};

                    let title = doc_id.clone();
                    let task_data = TextInsertData {
                        text: content.to_string(),
                        file_source: title.clone(),
                        workspace_id: workspace_id.clone(),
                        metadata: Some(serde_json::json!({
                            "document_id": doc_id,
                            "title": title,
                            "track_id": new_track_id,
                            "is_retry": true,
                            "tenant_id": tenant_id,
                            "workspace_id": workspace_id,
                        })),
                    };

                    let task = Task::new(
                        uuid::Uuid::parse_str(&tenant_id).map_err(|_| {
                            ApiError::ValidationError("Invalid tenant ID".to_string())
                        })?,
                        uuid::Uuid::parse_str(&workspace_id).map_err(|_| {
                            ApiError::ValidationError("Invalid workspace ID".to_string())
                        })?,
                        TaskType::Insert,
                        serde_json::to_value(task_data).unwrap(),
                    );

                    state
                        .task_storage
                        .create_task(&task)
                        .await
                        .map_err(|e| ApiError::Internal(format!("Failed to create task: {}", e)))?;

                    state
                        .task_queue
                        .send(task)
                        .await
                        .map_err(|e| ApiError::Internal(format!("Failed to queue task: {}", e)))?;

                    requeued_ids.push(doc_id.clone());
                }
            }
        } else {
            requeued_ids.push(doc_id.clone());
        }
    }

    // SPEC-040: Retry failed PDF documents from the documents DB table.
    // WHY: PDF docs are stored in the `documents` DB table, not in KV store.
    // The KV-based reprocess loop above cannot find them.
    #[cfg(feature = "postgres")]
    if let Some(ref pdf_storage) = state.pdf_storage {
        use edgequake_storage::{ListPdfFilter, PdfProcessingStatus};
        use edgequake_tasks::{PdfProcessingData, Task, TaskStatus, TaskType};

        let filter_workspace = tenant_ctx
            .workspace_id
            .as_deref()
            .and_then(|s| Uuid::parse_str(s).ok());

        let remaining = request
            .max_documents
            .saturating_sub(docs_to_reprocess.len());
        if remaining > 0 {
            let failed_pdfs = pdf_storage
                .list_pdfs(ListPdfFilter {
                    workspace_id: filter_workspace,
                    processing_status: Some(PdfProcessingStatus::Failed),
                    page: Some(1),
                    page_size: Some(remaining),
                })
                .await
                .map_err(|e| ApiError::Internal(format!("Failed to list failed PDFs: {}", e)))?;

            // DRY: Use the same resolution chain as PdfUploadOptions (types.rs).
            // WHY: Previously this was a duplicated inline chain that could diverge
            // from the upload path, causing reprocessed PDFs to use different
            // provider/model than new uploads.
            let vision_opts = crate::handlers::pdf_upload::types::PdfUploadOptions::default();
            let vision_provider = vision_opts.resolved_vision_provider();
            let vision_model: Option<String> = Some(vision_opts.vision_model());

            for pdf in failed_pdfs.items {
                // Determine tenant_id: prefer from context, fall back to a
                // workspace-scoped default (workspace_id itself as tenant proxy).
                let tenant_uuid = tenant_ctx
                    .tenant_id
                    .as_deref()
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .unwrap_or(pdf.workspace_id);

                // Reset PDF status so the worker will process it.
                pdf_storage
                    .update_pdf_status(&pdf.pdf_id, PdfProcessingStatus::Pending)
                    .await
                    .map_err(|e| {
                        ApiError::Internal(format!("Failed to reset PDF status: {}", e))
                    })?;

                let pdf_parser_backend = match state
                    .workspace_service
                    .get_workspace(pdf.workspace_id)
                    .await
                {
                    Ok(Some(workspace)) => workspace.resolved_pdf_parser_backend(),
                    Ok(None) | Err(_) => PdfParserBackend::from_env().unwrap_or_default(),
                };

                let task_data = PdfProcessingData {
                    pdf_id: pdf.pdf_id,
                    tenant_id: tenant_uuid,
                    workspace_id: pdf.workspace_id,
                    enable_vision: true,
                    vision_provider: vision_provider.clone(),
                    vision_model: vision_model.clone(),
                    existing_document_id: pdf.document_id.map(|id| id.to_string()),
                    pdf_parser_backend,
                    restart_from_scratch: false,
                };

                let track_id = format!("pdf-{}", Uuid::new_v4());

                let task = Task {
                    track_id: track_id.clone(),
                    tenant_id: tenant_uuid,
                    workspace_id: pdf.workspace_id,
                    task_type: TaskType::PdfProcessing,
                    status: TaskStatus::Pending,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    started_at: None,
                    completed_at: None,
                    error_message: None,
                    error: None,
                    retry_count: 0,
                    max_retries: 3,
                    consecutive_timeout_failures: 0,
                    circuit_breaker_tripped: false,
                    task_data: serde_json::to_value(&task_data).map_err(|e| {
                        ApiError::Internal(format!("Failed to serialize PDF task data: {}", e))
                    })?,
                    metadata: None,
                    progress: None,
                    result: None,
                };

                state
                    .task_storage
                    .create_task(&task)
                    .await
                    .map_err(|e| ApiError::Internal(format!("Failed to create PDF task: {}", e)))?;

                state
                    .task_queue
                    .send(task)
                    .await
                    .map_err(|e| ApiError::Internal(format!("Failed to queue PDF task: {}", e)))?;

                requeued_ids.push(pdf.pdf_id.to_string());
                tracing::info!(
                    pdf_id = %pdf.pdf_id,
                    track_id = %track_id,
                    "Re-enqueued failed PDF for reprocessing"
                );
            }
        }
    }

    Ok(Json(ReprocessFailedResponse {
        track_id: new_track_id,
        failed_found: docs_to_reprocess.len(),
        requeued: requeued_ids.len(),
        document_ids: requeued_ids,
    }))
}
