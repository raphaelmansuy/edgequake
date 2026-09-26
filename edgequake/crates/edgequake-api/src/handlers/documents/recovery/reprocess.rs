//! GAP-039: Reprocess failed documents handler.
//!
//! Finds documents eligible for reprocess (failed / cancelled / orphan waiting,
//! or force-widened completed/in-flight) and requeues them. Lifecycle-exclusive
//! states (`deleting`, `delete_failed`, cancel-in-flight) are never reprocessed —
//! see [`crate::services::reprocess_admission`].

use axum::response::Response;
use axum::{extract::State, response::IntoResponse, Json};
use chrono::Utc;
#[cfg(feature = "postgres")]
use edgequake_pdf::PdfParserBackend;
use tracing::debug;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::documents_types::*;
use crate::middleware::TenantContext;
use crate::state::AppState;

use crate::services::document_metadata_scan::load_scoped_document_metadata;
#[cfg(feature = "postgres")]
use crate::services::resolve_process_options_from_metadata;

use super::reprocess_one::{
    admit_document_for_reprocess, reprocess_one_document, ReprocessOneOutcome, ReprocessOneParams,
};

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
    if let Some(ws_uuid) =
        crate::middleware::resolve_workspace_uuid(tenant_ctx.workspace_id.as_deref())
    {
        if crate::services::workspace_wipe_in_flight(&state, ws_uuid).await {
            return Err(ApiError::Conflict(
                "Workspace wipe in progress — retry reprocess after wipe completes".into(),
            ));
        }
    }

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
    let scoped_metadata = load_scoped_document_metadata(
        state.storage.kv_storage.as_ref(),
        state.optional_pg_pool(),
        &tenant_ctx,
    )
    .await?;

    let mut docs_to_reprocess = Vec::new();
    let mut requeued_ids = Vec::new();
    let mut document_task_ids: Vec<ReprocessDocumentTaskId> = Vec::new();
    let mut skip_reasons: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut targeted_doc_seen = false;

    let workspace_uuid = tenant_ctx
        .workspace_id
        .as_deref()
        .and_then(|s| uuid::Uuid::parse_str(s).ok());

    for value in scoped_metadata {
        if docs_to_reprocess.len() >= request.max_documents {
            break;
        }

        let Some(obj) = value.as_object() else {
            continue;
        };
        let status = obj.get("status").and_then(|v| v.as_str());
        let doc_track_id = obj.get("track_id").and_then(|v| v.as_str());
        let Some(doc_id) = obj.get("id").and_then(|v| v.as_str()) else {
            continue;
        };

        // document_id filter: only that exact document.
        if let Some(ref filter_doc_id) = request.document_id {
            if doc_id != filter_doc_id.as_str() {
                continue;
            }
            targeted_doc_seen = true;
        }

        // track_id filter (batch correlation).
        if let Some(ref filter_track) = request.track_id {
            if doc_track_id != Some(filter_track.as_str()) {
                continue;
            }
        }

        // When neither filter is set, only scan recoverable / orphan candidates
        // (admission SSOT still applies — e.g. deleting never enters).
        if request.document_id.is_none() && request.track_id.is_none() {
            let prefilter_ok = status.is_some_and(|s| {
                crate::services::is_reprocess_terminal_recoverable(s)
                    || crate::services::is_reprocess_orphan_waiting_status(s)
            });
            if !prefilter_ok {
                continue;
            }
        }

        let decision = admit_document_for_reprocess(
            &state,
            doc_id,
            doc_track_id,
            status,
            request.force,
            restart_from_scratch,
            workspace_uuid,
        )
        .await;

        match decision {
            crate::services::ReprocessAdmitDecision::Admit => {
                docs_to_reprocess.push((doc_id.to_string(), doc_id.to_string()));
            }
            crate::services::ReprocessAdmitDecision::Skip(reason) => {
                *skip_reasons.entry(reason.as_str().to_string()).or_insert(0) += 1;
                tracing::info!(
                    document_id = %doc_id,
                    status = ?status,
                    force = request.force,
                    restart_from_scratch,
                    skip_reason = %reason,
                    "Reprocess admission skipped document"
                );
            }
        }

        // Targeted document_id: stop after the match (admit or skip).
        if request.document_id.is_some() {
            break;
        }
    }

    // Targeted document_id not present in scoped metadata.
    if let Some(ref filter_doc_id) = request.document_id {
        if !targeted_doc_seen {
            *skip_reasons
                .entry(
                    crate::services::ReprocessSkipReason::NotFound
                        .as_str()
                        .to_string(),
                )
                .or_insert(0) += 1;
            tracing::info!(
                document_id = %filter_doc_id,
                "Reprocess target document not found in workspace metadata"
            );
        }
    }

    // Requeue documents for processing
    for (doc_id, _doc_key) in &docs_to_reprocess {
        match reprocess_one_document(ReprocessOneParams {
            state: &state,
            tenant_ctx: &tenant_ctx,
            doc_id,
            force: request.force,
            reprocess_mode,
            restart_from_scratch,
            workspace_uuid,
            new_track_id: &new_track_id,
        })
        .await
        {
            ReprocessOneOutcome::Requeued {
                document_id,
                task_id,
            } => {
                document_task_ids.push(ReprocessDocumentTaskId {
                    document_id: document_id.clone(),
                    task_id,
                });
                requeued_ids.push(document_id);
            }
            ReprocessOneOutcome::Skipped { reason } => {
                *skip_reasons.entry(reason.to_string()).or_insert(0) += 1;
            }
        }
    }

    // SPEC-040: Retry failed PDF documents from the documents DB table.
    // WHY: PDF docs are stored in the `documents` DB table, not in KV store.
    // The KV-based reprocess loop above cannot find them.
    #[cfg(feature = "postgres")]
    if let Some(ref pdf_storage) = state.storage.pdf_storage {
        use edgequake_storage::{ListPdfFilter, PdfProcessingStatus};

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
                match requeue_one_failed_pdf(
                    &state,
                    pdf_storage.as_ref(),
                    &tenant_ctx,
                    &new_track_id,
                    pdf,
                    restart_from_scratch,
                    reprocess_mode,
                )
                .await
                {
                    Ok((doc_id, task_id)) => {
                        document_task_ids.push(ReprocessDocumentTaskId {
                            document_id: doc_id.clone(),
                            task_id,
                        });
                        requeued_ids.push(doc_id);
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            "Failed PDF leftover requeue skipped (issue #384 batch isolation)"
                        );
                        *skip_reasons
                            .entry("enqueue_failed".to_string())
                            .or_insert(0) += 1;
                    }
                }
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
        // Honesty: admission skips (deleting/cancelling/…) + mid-requeue skips.
        skipped: skip_reasons
            .values()
            .copied()
            .sum::<usize>()
            .max(docs_to_reprocess.len().saturating_sub(requeued_ids.len())),
        skip_reasons,
        document_ids: requeued_ids,
        task_id: single_task_id,
        document_task_ids,
    };
    Ok(response)
}

/// Persist a failed-PDF reprocess task, then project Pending / KV (issue #384).
///
/// One PDF failure must not abort the rest of the batch (`?` used to).
#[cfg(feature = "postgres")]
async fn requeue_one_failed_pdf(
    state: &AppState,
    pdf_storage: &dyn edgequake_storage::PdfDocumentStorage,
    tenant_ctx: &TenantContext,
    batch_track_id: &str,
    pdf: edgequake_storage::PdfDocument,
    restart_from_scratch: bool,
    reprocess_mode: edgequake_tasks::ReprocessMode,
) -> ApiResult<(String, String)> {
    use edgequake_storage::PdfProcessingStatus;
    use edgequake_tasks::{PdfProcessingData, Task, TaskStatus, TaskType};

    let tenant_uuid = tenant_ctx
        .tenant_id
        .as_deref()
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or(pdf.workspace_id);

    let (vision_provider, vision_model, pdf_parser_backend, vision_ws) = match state
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
            (vp, vm, backend, Some(ws))
        }
        Ok(None) | Err(_) => {
            let opts = crate::handlers::pdf_upload::types::PdfUploadOptions::default();
            let vp = opts.resolved_vision_provider(None, None);
            let vm = Some(opts.vision_model(None, None));
            let backend = PdfParserBackend::from_env().unwrap_or_default();
            (vp, vm, backend, None)
        }
    };

    let vision_model_for_resolve = vision_model
        .clone()
        .unwrap_or_else(|| crate::vision_env::default_vision_model_for_provider(&vision_provider));
    let vision_reasoning_effort = crate::services::resolve_vlm_reasoning_effort(
        vision_ws.as_ref(),
        &vision_provider,
        &vision_model_for_resolve,
        None,
        None,
    );

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
        let metadata_key = edgequake_storage::kv_keys::doc_metadata(&document_uuid.to_string());
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
        vision_reasoning_effort,
        vision_extract: Default::default(),
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
        task_data: serde_json::to_value(&task_data)
            .map_err(|e| ApiError::Internal(format!("Failed to serialize PDF task data: {e}")))?,
        metadata: None,
        progress: None,
        result: None,
        lease_owner: None,
        lease_token: None,
        lease_expires_at: None,
        fairness_hold_until: None,
    };

    state.enqueue_task(task).await?;

    if let Err(e) = pdf_storage
        .update_pdf_status(&pdf.pdf_id, PdfProcessingStatus::Pending)
        .await
    {
        tracing::error!(
            error = %e,
            pdf_id = %pdf.pdf_id,
            task_id = %track_id,
            "PDF task persisted but status reset to Pending failed"
        );
    }

    let correlating_id = if let Some(document_uuid) = pdf.document_id {
        let doc_id = document_uuid.to_string();
        let metadata_key = edgequake_storage::kv_keys::doc_metadata(&doc_id);
        if let Ok(Some(mut metadata)) = state.storage.kv_storage.get_by_id(&metadata_key).await {
            if let Some(obj) = metadata.as_object_mut() {
                obj.insert("track_id".to_string(), serde_json::json!(track_id.clone()));
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
        doc_id
    } else {
        pdf.pdf_id.to_string()
    };

    crate::handlers::pdf_upload::seed_pdf_job_progress(
        state,
        &track_id,
        &pdf.pdf_id.to_string(),
        &pdf.filename,
        Some(batch_track_id),
    )
    .await;

    tracing::info!(
        pdf_id = %pdf.pdf_id,
        task_id = %track_id,
        batch_track_id = %batch_track_id,
        "Re-enqueued failed PDF for reprocessing"
    );

    Ok((correlating_id, track_id))
}
