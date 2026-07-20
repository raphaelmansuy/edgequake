//! GAP-039: Reprocess failed documents handler.
//!
//! Finds documents in "failed" or "cancelled" status and requeues them
//! for processing. Supports both KV-based text documents and PostgreSQL
//! PDF documents (via `postgres` feature).

use axum::response::Response;
use axum::{extract::State, response::IntoResponse, Json};
use chrono::Utc;
use edgequake_pdf::PdfParserBackend;
use tracing::debug;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::documents_types::*;
use crate::middleware::TenantContext;
use crate::state::AppState;

use crate::services::document_metadata_scan::load_scoped_document_metadata;
use crate::services::pending_doc_task_reconcile::{
    ensure_task_for_pending_document, is_orphan_waiting_status, EnsureTaskOutcome,
};
use crate::services::resolve_process_options_from_metadata;

use super::super::storage_helpers::cleanup_document_graph_data;

/// Reprocess failed documents.
#[utoipa::path(
    post,
    path = "/api/v1/documents/reprocess",
    tag = "Documents",
    request_body = ReprocessFailedRequest,
    responses(
        (status = 200, description = "Documents requeued (legacy default)", body = ReprocessFailedResponse),
        (status = 202, description = "Reprocess accepted when REST-025 opt-in or strict startup", body = ReprocessFailedResponse),
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
) -> ApiResult<Response> {
    let request = body.map(|b| b.0).unwrap_or_default();
    let workspace_id = tenant_ctx.workspace_id.clone();
    let return_202 = state.security.v1_rpc_return_202;
    let response = run_reprocess_failed(state, tenant_ctx, request).await?;
    if let Some(ws) = workspace_id.as_deref() {
        let track_id = response.track_id.clone();
        return crate::services::v1_rpc_migration::respond_v1_async_rpc(
            ws,
            Some(track_id.as_str()),
            return_202,
            response,
        );
    }
    Ok(Json(response).into_response())
}

pub(crate) async fn run_reprocess_failed(
    state: AppState,
    tenant_ctx: TenantContext,
    request: ReprocessFailedRequest,
) -> ApiResult<ReprocessFailedResponse> {
    // Resolve reprocess intent (DRY single knob). Default is EntitiesOnly so
    // existing callers (failed-retry, bulk reprocess) keep current behavior.
    let reprocess_mode = request
        .mode
        .as_deref()
        .map(|m| {
            m.parse::<edgequake_tasks::ReprocessMode>()
                .unwrap_or(edgequake_tasks::ReprocessMode::EntitiesOnly)
        })
        .unwrap_or_default();
    let restart_from_scratch = reprocess_mode.restart_from_scratch();
    debug!(
        "reprocess_failed called with tenant context: tenant_id={:?}, workspace_id={:?}, document_id={:?}, force={}, mode={}",
        tenant_ctx.tenant_id, tenant_ctx.workspace_id, request.document_id, request.force, reprocess_mode
    );

    // Generate new track ID for reprocess batch
    let new_track_id = format!(
        "reprocess_{}_{}",
        Utc::now().format("%Y%m%d_%H%M%S"),
        &Uuid::new_v4().to_string()[..8]
    );

    // P-G7 + SPEC-027: batch scoped metadata (suffix index + tenant filter).
    let scoped_metadata =
        load_scoped_document_metadata(state.storage.kv_storage.as_ref(), &tenant_ctx).await?;

    let mut docs_to_reprocess = Vec::new();
    let mut requeued_ids = Vec::new();
    let mut document_task_ids: Vec<ReprocessDocumentTaskId> = Vec::new();
    let mut skip_reasons: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for value in scoped_metadata {
        if docs_to_reprocess.len() >= request.max_documents {
            break;
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
                // When document_id is specified with force=true, allow any status.
                // SPEC-054/#298: also allow orphan pending/queued without force
                // (waiting docs with no active worker task).
                if !request.force && status != Some("failed") && status != Some("cancelled") {
                    let orphan_waiting = matches!(status, Some("pending") | Some("queued"));
                    if !orphan_waiting {
                        continue;
                    }
                    if let Some(id) = doc_id {
                        let ws = tenant_ctx
                            .workspace_id
                            .as_deref()
                            .and_then(|s| uuid::Uuid::parse_str(s).ok());
                        let has_task = crate::services::pending_doc_task_reconcile::has_active_task_for_document(
                            state.tasks.storage.as_ref(),
                            id,
                            ws,
                        )
                        .await
                        .unwrap_or(true);
                        if has_task {
                            continue;
                        }
                    }
                }
                if let Some(id) = doc_id {
                    docs_to_reprocess.push((id.to_string(), id.to_string()));
                }
                break; // Found the specific document
            }

            // If track_id filter is specified, match by track_id
            if let Some(ref filter_track) = request.track_id {
                if doc_track_id != Some(filter_track.as_str()) {
                    continue;
                }
            }

            // Default behavior: failed/cancelled, plus orphan pending/queued (#298).
            let mut include = status == Some("failed") || status == Some("cancelled");
            if !include && matches!(status, Some("pending") | Some("queued")) {
                if let Some(id) = doc_id {
                    let ws = tenant_ctx
                        .workspace_id
                        .as_deref()
                        .and_then(|s| uuid::Uuid::parse_str(s).ok());
                    let has_task =
                        crate::services::pending_doc_task_reconcile::has_active_task_for_document(
                            state.tasks.storage.as_ref(),
                            id,
                            ws,
                        )
                        .await
                        .unwrap_or(true);
                    include = !has_task;
                }
            }
            if include {
                if let Some(id) = doc_id {
                    docs_to_reprocess.push((id.to_string(), id.to_string()));
                }
            }
        }
    }

    // Requeue documents for processing
    for (doc_id, _doc_key) in &docs_to_reprocess {
        // WHY (#304): never pass the literal `"default"` alias into Uuid::parse_str —
        // that returns ValidationError after early-admit already wrote status=processing,
        // leaving the document stuck with "Reprocess has no effect".
        let workspace_id_for_tasks = tenant_ctx.workspace_id_or_default();
        let tenant_id_for_tasks = tenant_ctx.tenant_id_or_default();

        // Read metadata early so soft single-flight can see pdf_id before any purge.
        let metadata_key =
            crate::services::document_metadata_scan::metadata_key_for_document(doc_id);
        let mut metadata_opt = state.storage.kv_storage.get_by_id(&metadata_key).await?;
        let doc_status = metadata_opt
            .as_ref()
            .and_then(|m| m.get("status"))
            .and_then(|v| v.as_str());

        // SPEC-054/#298 (DRY): orphan pending/queued without force → SSOT recovery.
        // Avoids purge/cleanup/rebuild paths meant for failed re-runs.
        let use_orphan_ssot = !request.force
            && doc_status.is_some_and(is_orphan_waiting_status)
            && metadata_opt.is_some();
        if use_orphan_ssot {
            let meta = metadata_opt.as_ref().expect("checked is_some");
            let content_key = format!("{doc_id}-content");
            let content = state
                .storage
                .kv_storage
                .get_by_id(&content_key)
                .await?
                .and_then(|v| {
                    v.get("content")
                        .and_then(|c| c.as_str())
                        .map(str::to_string)
                });
            match ensure_task_for_pending_document(
                &state,
                doc_id,
                meta,
                content.as_deref(),
                &new_track_id,
                "reprocess_orphan_pending",
            )
            .await?
            {
                EnsureTaskOutcome::Enqueued { task_id } => {
                    document_task_ids.push(ReprocessDocumentTaskId {
                        document_id: doc_id.clone(),
                        task_id,
                    });
                    requeued_ids.push(doc_id.clone());
                    continue;
                }
                EnsureTaskOutcome::AlreadyScheduled => {
                    *skip_reasons
                        .entry("already_scheduled".to_string())
                        .or_insert(0) += 1;
                    continue;
                }
                EnsureTaskOutcome::SkippedNoContent => {
                    *skip_reasons.entry("no_content".to_string()).or_insert(0) += 1;
                    continue;
                }
                EnsureTaskOutcome::SkippedNotEligible => {
                    *skip_reasons.entry("not_eligible".to_string()).or_insert(0) += 1;
                    continue;
                }
            }
        }

        let pdf_id_for_flight = metadata_opt
            .as_ref()
            .and_then(|m| m.as_object())
            .and_then(|obj| obj.get("pdf_id"))
            .and_then(|v| v.as_str())
            .and_then(|s| uuid::Uuid::parse_str(s).ok());

        // SPEC-047 P6: soft reprocess must not kill an in-flight pipeline
        // (double extract + double embed). Only Full restart_from_scratch may
        // purge and replace an active PdfProcessing task.
        if !restart_from_scratch {
            if let (Some(pdf_uuid), Ok(ws_uuid)) = (
                pdf_id_for_flight,
                uuid::Uuid::parse_str(&workspace_id_for_tasks),
            ) {
                if let Ok(Some(active)) = state
                    .tasks
                    .storage
                    .find_active_pdf_processing_task(pdf_uuid, ws_uuid)
                    .await
                {
                    tracing::info!(
                        document_id = %doc_id,
                        track_id = %active.track_id,
                        "Single-flight: skipping soft reprocess; PDF task already in flight"
                    );
                    *skip_reasons
                        .entry("already_processing".to_string())
                        .or_insert(0) += 1;
                    continue;
                }
            }
        }

        // Early admit: write processing/cleaning + provisional track_id BEFORE graph
        // cleanup so list polls during the 5–10s cleanup window stay non-terminal
        // and show honest "Cleaning" UX (not false "Queued — waiting for worker").
        // Do not move cleanup after enqueue (race with worker); status-first is enough.
        let previous_metadata_for_rollback = metadata_opt.clone();
        if let Some(mut metadata) = metadata_opt.clone() {
            if let Some(obj) = metadata.as_object_mut() {
                crate::services::reprocess_stage_reset::apply_early_reprocess_admit(
                    obj,
                    &new_track_id,
                    reprocess_mode,
                );
                crate::services::upsert_metadata_kv_with_index(
                    state.storage.kv_storage.as_ref(),
                    &metadata_key,
                    metadata.clone(),
                )
                .await?;
                metadata_opt = Some(metadata);
                tracing::debug!(
                    document_id = %doc_id,
                    batch_track_id = %new_track_id,
                    "Early reprocess cleaning stage written before graph cleanup"
                );
            }
        }

        // Edge case: cancel any in-flight task for this document before requeueing.
        // WHY: A force=true Full reprocess on a doc that is still processing (or has a
        // lingering queued task) would race the worker. For Full re-conversion this
        // is especially important — we clear markdown and must not let a concurrent
        // task reuse half-cleared state. purge_persisted_tasks_for_document cancels
        // and removes persisted tasks referencing this document id.
        let purged = super::super::storage_helpers::purge_persisted_tasks_for_document(
            &state,
            doc_id,
            None,
            Some(&workspace_id_for_tasks),
        )
        .await;
        if purged > 0 {
            tracing::info!(
                document_id = %doc_id,
                tasks_purged = purged,
                "Cancelled in-flight tasks before reprocessing"
            );
        }
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
        let cleanup_admit_stats =
            match cleanup_document_graph_data(doc_id, &state.storage.graph_storage, None).await {
                Ok(stats) => {
                    tracing::info!(
                        document_id = %doc_id,
                        entities_removed = stats.entities_removed,
                        entities_updated = stats.entities_updated,
                        relationships_removed = stats.relationships_removed,
                        "Cleaned up partial data before reprocessing"
                    );
                    Some(crate::services::reprocess_stage_reset::CleanupAdmitStats {
                        entities_removed: stats.entities_removed,
                        relationships_removed: stats.relationships_removed,
                    })
                }
                Err(e) => {
                    tracing::warn!(
                        document_id = %doc_id,
                        error = %e,
                        "Failed to cleanup partial data before reprocessing, continuing anyway"
                    );
                    None
                }
            };

        // Transition cleaning → queued (or merging) once graph cleanup finishes.
        // True admission: waiting for a free worker / merge start.
        if let Some(mut metadata) = metadata_opt.clone() {
            if let Some(obj) = metadata.as_object_mut() {
                crate::services::reprocess_stage_reset::apply_post_cleanup_admission(
                    obj,
                    reprocess_mode,
                    cleanup_admit_stats,
                );
                // Keep provisional track_id until Task is created below.
                obj.insert("track_id".to_string(), serde_json::json!(new_track_id));
                crate::services::upsert_metadata_kv_with_index(
                    state.storage.kv_storage.as_ref(),
                    &metadata_key,
                    metadata.clone(),
                )
                .await?;
                metadata_opt = Some(metadata);
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
        // metadata_opt already loaded above for single-flight check.

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

        // Prefer document metadata workspace/tenant when present (canonical UUIDs),
        // otherwise fall back to the request context helpers that resolve `"default"`.
        let workspace_id = metadata_opt
            .as_ref()
            .and_then(|m| m.get("workspace_id"))
            .and_then(|v| v.as_str())
            .and_then(|s| crate::middleware::resolve_workspace_uuid(Some(s)))
            .map(|u| u.to_string())
            .unwrap_or_else(|| workspace_id_for_tasks.clone());
        let tenant_id = metadata_opt
            .as_ref()
            .and_then(|m| m.get("tenant_id"))
            .and_then(|v| v.as_str())
            .and_then(|s| crate::middleware::resolve_tenant_uuid(Some(s)))
            .map(|u| u.to_string())
            .unwrap_or_else(|| tenant_id_for_tasks.clone());

        // FIX-REBUILD: Route PDF documents through PdfProcessing for full re-extraction.
        // WHY (#304): interrupted uploads may have `pdf_id` but missing/empty
        // `source_type` or KV content — still treat as PDF so Reprocess enqueues work.
        let is_pdf_document = source_type.as_deref() == Some("pdf") || pdf_id_str.is_some();
        let task_created = if is_pdf_document {
            if let Some(ref pid_str) = pdf_id_str {
                if let Ok(pdf_id_uuid) = uuid::Uuid::parse_str(pid_str) {
                    // Edge case: empty-markdown fallback.
                    // WHY: If the user picked EntitiesOnly (reuse markdown) but the
                    // cached markdown is missing/empty, there is nothing to reuse —
                    // entity extraction would run over an empty document. Auto-upgrade
                    // to Full so the PDF is re-converted from scratch. This is a
                    // safe, idempotent promotion: Full is a strict superset of
                    // EntitiesOnly's work.
                    //
                    // Also upgrade when the doc was interrupted by server restart
                    // (#304): checkpoints/markdown are often incomplete after a kill.
                    #[allow(unused_mut)]
                    let mut reprocess_mode = reprocess_mode;
                    #[allow(unused_mut)]
                    let mut restart_from_scratch = restart_from_scratch;
                    let interrupted_by_restart = metadata_opt
                        .as_ref()
                        .and_then(|m| m.get("error_message"))
                        .and_then(|v| v.as_str())
                        .is_some_and(|msg| {
                            msg.contains("Interrupted by server restart")
                                || msg.contains("Server restarted")
                        });
                    if !restart_from_scratch && interrupted_by_restart {
                        tracing::info!(
                            document_id = %doc_id,
                            pdf_id = %pid_str,
                            "Reprocess after server-restart interrupt — upgrading to full re-conversion (#304)"
                        );
                        reprocess_mode = edgequake_tasks::ReprocessMode::Full;
                        restart_from_scratch = true;
                    }
                    if !restart_from_scratch {
                        // `pdf_storage` is only present under the `postgres` feature
                        // (StorageRuntime::pdf_storage is `#[cfg(feature = "postgres")]`).
                        // Without postgres there is no cached markdown to inspect, so the
                        // empty-markdown fallback is skipped and the caller's mode is honored.
                        #[cfg(feature = "postgres")]
                        if let Some(pdf_storage) = state.storage.pdf_storage.as_ref() {
                            let needs_full = match pdf_storage.get_pdf(&pdf_id_uuid).await {
                                Ok(Some(pdf)) => {
                                    super::super::storage_helpers::pdf_needs_full_reconversion(
                                        pdf.markdown_content.as_deref(),
                                    )
                                }
                                // Unknown/missing row: cannot guarantee markdown, so
                                // promote to Full to force a fresh conversion.
                                Ok(None) => true,
                                Err(e) => {
                                    tracing::warn!(
                                        pdf_id = %pid_str,
                                        error = %e,
                                        "Failed to read PDF for empty-markdown fallback; defaulting to Full"
                                    );
                                    true
                                }
                            };
                            if needs_full {
                                tracing::info!(
                                    document_id = %doc_id,
                                    pdf_id = %pid_str,
                                    "Reprocess entities requested but cached markdown is empty — upgrading to full re-conversion"
                                );
                                reprocess_mode = edgequake_tasks::ReprocessMode::Full;
                                restart_from_scratch = true;
                            }
                        }
                        // Without postgres / when KV content is also missing, promote
                        // so soft reprocess does not no-op after interrupt (#304).
                        #[cfg(not(feature = "postgres"))]
                        {
                            let content_missing = state
                                .storage
                                .kv_storage
                                .get_by_id(&content_key)
                                .await?
                                .and_then(|v| {
                                    v.get("content")
                                        .and_then(|c| c.as_str())
                                        .map(|s| s.trim().is_empty())
                                })
                                .unwrap_or(true);
                            if content_missing {
                                reprocess_mode = edgequake_tasks::ReprocessMode::Full;
                                restart_from_scratch = true;
                            }
                        }
                    }

                    // Look up workspace to get vision provider/model settings
                    let (vision_provider, vision_model, pdf_parser_backend) = if let Ok(ws_uuid) =
                        uuid::Uuid::parse_str(&workspace_id)
                    {
                        if let Ok(Some(ws)) = state.workspace_service.get_workspace(ws_uuid).await {
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

                    // PDF re-conversion (Full mode): clear cached markdown so the
                    // resume shortcut cannot reuse a stale conversion. The worker
                    // also clears KV content/chunks when restart_from_scratch=true.
                    if restart_from_scratch {
                        if let Err(e) =
                            super::super::storage_helpers::clear_document_markdown_and_content(
                                &state,
                                doc_id,
                                &pdf_id_uuid,
                            )
                            .await
                        {
                            tracing::warn!(
                                document_id = %doc_id,
                                pdf_id = %pid_str,
                                error = %e,
                                "Failed to pre-clear markdown for full re-conversion, continuing"
                            );
                        }
                    }

                    let multimodal_process_options = metadata_opt
                        .as_ref()
                        .and_then(resolve_process_options_from_metadata);

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
                        pdf_parser_backend_explicit: true,
                        restart_from_scratch,
                        reprocess_mode: Some(reprocess_mode),
                        multimodal_process_options,
                    };

                    // SPEC-054: create task first so document.track_id == progress key.
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
                    let task_track_id = task.track_id.clone();

                    // Update status for reprocess (SPEC-048: reset stage fields)
                    // Progress SSOT: bind document.track_id to server task id (not batch id).
                    if let Some(mut metadata) = metadata_opt.clone() {
                        if let Some(obj) = metadata.as_object_mut() {
                            obj.insert("track_id".to_string(), serde_json::json!(task_track_id));
                            obj.insert(
                                "retry_at".to_string(),
                                serde_json::json!(Utc::now().to_rfc3339()),
                            );
                            crate::services::reprocess_stage_reset::apply_reprocess_stage_reset(
                                obj,
                                reprocess_mode,
                            );
                            crate::services::upsert_metadata_kv_with_index(
                                state.storage.kv_storage.as_ref(),
                                &metadata_key,
                                metadata,
                            )
                            .await?;
                        }
                    }

                    let filename = metadata_opt
                        .as_ref()
                        .and_then(|m| m.as_object())
                        .and_then(|obj| {
                            obj.get("file_path")
                                .or_else(|| obj.get("title"))
                                .and_then(|v| v.as_str())
                        })
                        .unwrap_or(doc_id);
                    crate::handlers::pdf_upload::seed_pdf_job_progress(
                        &state,
                        &task_track_id,
                        pid_str,
                        filename,
                        Some(new_track_id.as_str()),
                    )
                    .await;

                    state.enqueue_task(task).await?;

                    document_task_ids.push(ReprocessDocumentTaskId {
                        document_id: doc_id.clone(),
                        task_id: task_track_id.clone(),
                    });

                    tracing::info!(
                        document_id = %doc_id,
                        pdf_id = %pid_str,
                        task_id = %task_track_id,
                        batch_track_id = %new_track_id,
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
        let mut requeued_this_doc = task_created;
        if !task_created {
            if let Some(content_value) = state.storage.kv_storage.get_by_id(&content_key).await? {
                if let Some(content) = content_value.get("content").and_then(|v| v.as_str()) {
                    use edgequake_tasks::{Task, TaskType, TextInsertData};

                    let title = doc_id.clone();
                    // Create task first so metadata.track_id matches the progress/WS key.
                    let task_data = TextInsertData {
                        text: content.to_string(),
                        file_source: title.clone(),
                        workspace_id: workspace_id.clone(),
                        metadata: Some(serde_json::json!({
                            "document_id": doc_id,
                            "title": title,
                            "is_retry": true,
                            "tenant_id": tenant_id,
                            "workspace_id": workspace_id,
                            "force_fresh_extraction": restart_from_scratch,
                            "merge_only": reprocess_mode.merge_only(),
                            "batch_track_id": new_track_id,
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
                    let task_track_id = task.track_id.clone();

                    // Update status for reprocess (SPEC-048: reset stage fields)
                    if let Some(mut metadata) =
                        state.storage.kv_storage.get_by_id(&metadata_key).await?
                    {
                        if let Some(obj) = metadata.as_object_mut() {
                            obj.insert("track_id".to_string(), serde_json::json!(task_track_id));
                            obj.insert(
                                "retry_at".to_string(),
                                serde_json::json!(Utc::now().to_rfc3339()),
                            );
                            crate::services::reprocess_stage_reset::apply_reprocess_stage_reset(
                                obj,
                                reprocess_mode,
                            );
                            crate::services::upsert_metadata_kv_with_index(
                                state.storage.kv_storage.as_ref(),
                                &metadata_key,
                                metadata,
                            )
                            .await?;
                        }
                    }

                    state.enqueue_task(task).await?;

                    document_task_ids.push(ReprocessDocumentTaskId {
                        document_id: doc_id.clone(),
                        task_id: task_track_id,
                    });
                    requeued_this_doc = true;
                }
            }
        }

        if requeued_this_doc {
            requeued_ids.push(doc_id.clone());
        } else if let Some(prev) = previous_metadata_for_rollback {
            // Early admit wrote processing; restore prior metadata when we could not enqueue.
            let _ = crate::services::upsert_metadata_kv_with_index(
                state.storage.kv_storage.as_ref(),
                &metadata_key,
                prev,
            )
            .await;
            *skip_reasons.entry("no_content".to_string()).or_insert(0) += 1;
            tracing::warn!(
                document_id = %doc_id,
                "Rolled back early reprocess status — no task created"
            );
        }
    }

    // SPEC-040: Retry failed PDF documents from the documents DB table.
    // WHY: PDF docs are stored in the `documents` DB table, not in KV store.
    // The KV-based reprocess loop above cannot find them.
    #[cfg(feature = "postgres")]
    if let Some(ref pdf_storage) = state.storage.pdf_storage {
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

                // SPEC-051 GAP-051-04: Resolve ALL vision settings from workspace,
                // not from PdfUploadOptions::default().
                // WHY: Previously vision_provider and vision_model used default
                // env-var resolution, ignoring workspace-level overrides. Only
                // pdf_parser_backend was read from the workspace. Now all three
                // come from the same workspace.get_workspace() call (DRY).
                let (vision_provider, vision_model, pdf_parser_backend) = match state
                    .workspace_service
                    .get_workspace(pdf.workspace_id)
                    .await
                {
                    Ok(Some(ws)) => {
                        let vp = ws
                            .vision_llm_provider
                            .as_deref()
                            .filter(|p| !p.is_empty())
                            .unwrap_or("ollama")
                            .to_string();
                        let vm = ws.vision_llm_model.clone().filter(|m| !m.is_empty());
                        let backend = ws.resolved_pdf_parser_backend();
                        (vp, vm, backend)
                    }
                    Ok(None) | Err(_) => {
                        // Fallback: env-var defaults (same as upload path default).
                        let opts = crate::handlers::pdf_upload::types::PdfUploadOptions::default();
                        let vp = opts.resolved_vision_provider();
                        let vm = Some(opts.vision_model());
                        let backend = PdfParserBackend::from_env().unwrap_or_default();
                        (vp, vm, backend)
                    }
                };

                // Edge case: empty-markdown fallback for failed PDFs.
                // WHY: A failed PDF typically has no/partial markdown. EntitiesOnly
                // would re-extract over an empty document, so promote to Full when
                // the cached markdown is missing/empty. Safe superset of work.
                let mut restart_from_scratch = restart_from_scratch;
                let mut reprocess_mode = reprocess_mode;
                if !restart_from_scratch {
                    let needs_full = match pdf_storage.get_pdf(&pdf.pdf_id).await {
                        Ok(Some(p)) => super::super::storage_helpers::pdf_needs_full_reconversion(
                            p.markdown_content.as_deref(),
                        ),
                        Ok(None) => true,
                        Err(e) => {
                            tracing::warn!(
                                pdf_id = %pdf.pdf_id,
                                error = %e,
                                "Failed to read failed PDF for empty-markdown fallback; defaulting to Full"
                            );
                            true
                        }
                    };
                    if needs_full {
                        tracing::info!(
                            pdf_id = %pdf.pdf_id,
                            "Failed PDF has empty cached markdown — upgrading reprocess to full re-conversion"
                        );
                        reprocess_mode = edgequake_tasks::ReprocessMode::Full;
                        restart_from_scratch = true;
                    }
                }

                // Full re-conversion: clear any partial cached markdown so the
                // resume shortcut cannot reuse a failed/partial conversion.
                if restart_from_scratch {
                    if let Err(e) = pdf_storage.clear_markdown(&pdf.pdf_id).await {
                        tracing::warn!(
                            pdf_id = %pdf.pdf_id,
                            error = %e,
                            "Failed to clear markdown for failed-PDF full re-conversion"
                        );
                    }
                }

                let multimodal_process_options = if let Some(document_uuid) = pdf.document_id {
                    let metadata_key =
                        edgequake_storage::kv_keys::doc_metadata(&document_uuid.to_string());
                    state
                        .storage
                        .kv_storage
                        .get_by_id(&metadata_key)
                        .await
                        .ok()
                        .flatten()
                        .as_ref()
                        .and_then(resolve_process_options_from_metadata)
                } else {
                    None
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
                    pdf_parser_backend_explicit: true,
                    restart_from_scratch,
                    reprocess_mode: Some(reprocess_mode),
                    multimodal_process_options,
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
                    lease_owner: None,
                    lease_token: None,
                    lease_expires_at: None,
                };

                // Bind KV document.track_id to task id when a document row exists.
                if let Some(document_uuid) = pdf.document_id {
                    let doc_id = document_uuid.to_string();
                    let metadata_key = edgequake_storage::kv_keys::doc_metadata(&doc_id);
                    if let Ok(Some(mut metadata)) =
                        state.storage.kv_storage.get_by_id(&metadata_key).await
                    {
                        if let Some(obj) = metadata.as_object_mut() {
                            obj.insert("track_id".to_string(), serde_json::json!(track_id));
                            obj.insert(
                                "retry_at".to_string(),
                                serde_json::json!(Utc::now().to_rfc3339()),
                            );
                            crate::services::reprocess_stage_reset::apply_reprocess_stage_reset(
                                obj,
                                reprocess_mode,
                            );
                            let _ = crate::services::upsert_metadata_kv_with_index(
                                state.storage.kv_storage.as_ref(),
                                &metadata_key,
                                metadata,
                            )
                            .await;
                        }
                    }
                    document_task_ids.push(ReprocessDocumentTaskId {
                        document_id: doc_id.clone(),
                        task_id: track_id.clone(),
                    });
                    requeued_ids.push(doc_id);
                } else {
                    requeued_ids.push(pdf.pdf_id.to_string());
                    document_task_ids.push(ReprocessDocumentTaskId {
                        document_id: pdf.pdf_id.to_string(),
                        task_id: track_id.clone(),
                    });
                }

                crate::handlers::pdf_upload::seed_pdf_job_progress(
                    &state,
                    &track_id,
                    &pdf.pdf_id.to_string(),
                    &pdf.filename,
                    Some(new_track_id.as_str()),
                )
                .await;

                state.enqueue_task(task).await?;

                tracing::info!(
                    pdf_id = %pdf.pdf_id,
                    task_id = %track_id,
                    batch_track_id = %new_track_id,
                    "Re-enqueued failed PDF for reprocessing"
                );
            }
        }
    }

    let single_task_id = if document_task_ids.len() == 1 {
        Some(document_task_ids[0].task_id.clone())
    } else {
        None
    };

    let response = ReprocessFailedResponse {
        track_id: new_track_id,
        v2_migration: tenant_ctx
            .workspace_id
            .as_ref()
            .map(|ws| crate::services::job_registry::v2_migration_hint("reprocess_failed", ws)),
        failed_found: docs_to_reprocess.len(),
        requeued: requeued_ids.len(),
        skipped: docs_to_reprocess.len().saturating_sub(requeued_ids.len()),
        skip_reasons,
        document_ids: requeued_ids,
        task_id: single_task_id,
        document_task_ids,
    };
    Ok(response)
}
