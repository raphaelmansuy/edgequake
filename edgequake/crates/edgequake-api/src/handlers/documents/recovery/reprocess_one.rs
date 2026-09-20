//! Per-document reprocess unit of work (issue #385).
//!
//! After early admit writes `processing`/`cleaning`, the only legal exits are
//! enqueue (commit) or restore of the pre-admit snapshot (compensate). This
//! module never `?`-aborts the batch handler.

use chrono::Utc;
use edgequake_pdf::PdfParserBackend;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::services::pending_doc_task_reconcile::{
    ensure_task_for_pending_document, is_orphan_waiting_status, EnsureTaskOutcome,
};
use crate::services::reprocess_admission::ReprocessSkipReason;
use crate::services::reprocess_stage_reset::{
    graph_cleanup_overlay_error, post_admit_skip_reason, restore_pre_admit_metadata_best_effort,
};
use crate::services::resolve_process_options_from_metadata;
use crate::state::AppState;

/// Outcome of one document in a reprocess batch.
pub(crate) enum ReprocessOneOutcome {
    Requeued {
        document_id: String,
        task_id: String,
    },
    Skipped {
        reason: &'static str,
    },
}

pub(crate) struct ReprocessOneParams<'a> {
    pub state: &'a AppState,
    pub tenant_ctx: &'a TenantContext,
    pub doc_id: &'a str,
    pub force: bool,
    pub reprocess_mode: edgequake_tasks::ReprocessMode,
    pub restart_from_scratch: bool,
    pub workspace_uuid: Option<Uuid>,
    pub new_track_id: &'a str,
}

pub(crate) async fn reprocess_one_document(params: ReprocessOneParams<'_>) -> ReprocessOneOutcome {
    let ReprocessOneParams {
        state,
        tenant_ctx,
        doc_id,
        force,
        reprocess_mode,
        restart_from_scratch,
        workspace_uuid,
        new_track_id,
    } = params;

    let workspace_id_for_tasks = tenant_ctx
        .workspace_id
        .as_deref()
        .unwrap_or("default")
        .to_string();

    let metadata_key = crate::services::document_metadata_scan::metadata_key_for_document(doc_id);
    let mut metadata_opt = match state.storage.kv_storage.get_by_id(&metadata_key).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(
                document_id = %doc_id,
                error = %e,
                "Reprocess skipped — failed to load document metadata"
            );
            return ReprocessOneOutcome::Skipped {
                reason: ReprocessSkipReason::EnqueueFailed.as_str(),
            };
        }
    };
    let doc_status = metadata_opt
        .as_ref()
        .and_then(|m| m.get("status"))
        .and_then(|v| v.as_str());
    let doc_track_id = metadata_opt
        .as_ref()
        .and_then(|m| m.get("track_id"))
        .and_then(|v| v.as_str());

    let decision = admit_document_for_reprocess(
        state,
        doc_id,
        doc_track_id,
        doc_status,
        force,
        restart_from_scratch,
        workspace_uuid,
    )
    .await;
    if let crate::services::ReprocessAdmitDecision::Skip(reason) = decision {
        tracing::info!(
            document_id = %doc_id,
            skip_reason = %reason,
            "Reprocess skipped at requeue (status raced since selection)"
        );
        return ReprocessOneOutcome::Skipped {
            reason: reason.as_str(),
        };
    }

    let use_recovery_ssot = metadata_opt.as_ref().is_some_and(|meta| {
        doc_status.is_some_and(is_orphan_waiting_status)
            || crate::services::is_interrupted_restart_metadata(meta)
    });
    if use_recovery_ssot {
        let meta = metadata_opt.as_ref().expect("checked is_some");
        let content_key = format!("{doc_id}-content");
        let content = match state.storage.kv_storage.get_by_id(&content_key).await {
            Ok(v) => v.and_then(|v| {
                v.get("content")
                    .and_then(|c| c.as_str())
                    .map(str::to_string)
            }),
            Err(e) => {
                tracing::error!(
                    document_id = %doc_id,
                    error = %e,
                    "Reprocess recovery skipped — failed to load content"
                );
                return ReprocessOneOutcome::Skipped {
                    reason: ReprocessSkipReason::EnqueueFailed.as_str(),
                };
            }
        };
        match ensure_task_for_pending_document(
            state,
            doc_id,
            meta,
            content.as_deref(),
            new_track_id,
            "reprocess_recovery_enqueue",
        )
        .await
        {
            Ok(EnsureTaskOutcome::Enqueued { task_id }) => {
                return ReprocessOneOutcome::Requeued {
                    document_id: doc_id.to_string(),
                    task_id,
                };
            }
            Ok(EnsureTaskOutcome::AlreadyScheduled) => {
                return ReprocessOneOutcome::Skipped {
                    reason: "already_scheduled",
                };
            }
            Ok(EnsureTaskOutcome::SkippedNoContent) => {
                return ReprocessOneOutcome::Skipped {
                    reason: "no_content",
                };
            }
            Ok(EnsureTaskOutcome::SkippedNotEligible) => {
                // Fall through to ordinary reprocess cleanup/rebuild path.
            }
            Ok(EnsureTaskOutcome::RequiresReupload { reason }) => {
                tracing::warn!(
                    document_id = %doc_id,
                    %reason,
                    "Interrupted recovery requires re-upload"
                );
                return ReprocessOneOutcome::Skipped {
                    reason: "reupload_required",
                };
            }
            Err(e) => {
                tracing::error!(
                    document_id = %doc_id,
                    error = %e,
                    "Reprocess recovery enqueue failed"
                );
                return ReprocessOneOutcome::Skipped {
                    reason: ReprocessSkipReason::EnqueueFailed.as_str(),
                };
            }
        }
    }

    let pdf_id_for_flight = metadata_opt
        .as_ref()
        .and_then(|m| m.as_object())
        .and_then(|obj| obj.get("pdf_id"))
        .and_then(|v| v.as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok());

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
                return ReprocessOneOutcome::Skipped {
                    reason: "already_processing",
                };
            }
        }
    }

    let previous_metadata_for_rollback = metadata_opt.clone();
    if let Some(mut metadata) = metadata_opt.clone() {
        if let Some(obj) = metadata.as_object_mut() {
            crate::services::reprocess_stage_reset::apply_early_reprocess_admit(
                obj,
                new_track_id,
                reprocess_mode,
            );
            if let Err(e) = crate::services::upsert_metadata_kv_with_index(
                state.storage.kv_storage.as_ref(),
                &metadata_key,
                metadata.clone(),
            )
            .await
            {
                tracing::error!(
                    document_id = %doc_id,
                    error = %e,
                    "Early reprocess admit write failed"
                );
                return ReprocessOneOutcome::Skipped {
                    reason: ReprocessSkipReason::EnqueueFailed.as_str(),
                };
            }
            metadata_opt = Some(metadata);
            tracing::debug!(
                document_id = %doc_id,
                batch_track_id = %new_track_id,
                "Early reprocess cleaning stage written before graph cleanup"
            );
        }
    }

    match enqueue_after_early_admit(
        state,
        tenant_ctx,
        doc_id,
        &metadata_key,
        &mut metadata_opt,
        &workspace_id_for_tasks,
        reprocess_mode,
        restart_from_scratch,
        new_track_id,
    )
    .await
    {
        Ok(Some(task_id)) => ReprocessOneOutcome::Requeued {
            document_id: doc_id.to_string(),
            task_id,
        },
        Ok(None) => {
            if let Some(prev) = previous_metadata_for_rollback {
                restore_pre_admit_metadata_best_effort(
                    state.storage.kv_storage.as_ref(),
                    &metadata_key,
                    doc_id,
                    prev,
                    None,
                )
                .await;
            }
            tracing::warn!(
                document_id = %doc_id,
                "Rolled back early reprocess status — no task created"
            );
            ReprocessOneOutcome::Skipped {
                reason: "no_content",
            }
        }
        Err(e) => {
            let reason = post_admit_skip_reason(&e);
            let overlay = graph_cleanup_overlay_error(&e);
            if let Some(prev) = previous_metadata_for_rollback {
                restore_pre_admit_metadata_best_effort(
                    state.storage.kv_storage.as_ref(),
                    &metadata_key,
                    doc_id,
                    prev,
                    overlay.as_deref(),
                )
                .await;
            }
            tracing::warn!(
                document_id = %doc_id,
                skip_reason = %reason,
                error = %e,
                "Rolled back early reprocess admit after post-admit failure"
            );
            ReprocessOneOutcome::Skipped {
                reason: reason.as_str(),
            }
        }
    }
}

/// Retract + stage write + task create/enqueue. `?` is local to this function.
#[allow(clippy::too_many_arguments)]
async fn enqueue_after_early_admit(
    state: &AppState,
    tenant_ctx: &TenantContext,
    doc_id: &str,
    metadata_key: &str,
    metadata_opt: &mut Option<serde_json::Value>,
    workspace_id_for_tasks: &str,
    reprocess_mode: edgequake_tasks::ReprocessMode,
    restart_from_scratch: bool,
    new_track_id: &str,
) -> ApiResult<Option<String>> {
    // Finished attempts (Failed / Indexed / Cancelled) are left in `tasks`
    // as the audit record (issue #386 / BR0903). Reprocess enqueues a new
    // track_id; it must not overwrite the previous attempt's status or
    // error. Physical GC is prune_terminal_tasks, not this admit step.
    let purged = super::super::storage_helpers::purge_persisted_tasks_for_document(
        state,
        doc_id,
        None,
        Some(workspace_id_for_tasks),
    )
    .await;
    if purged > 0 {
        tracing::info!(
            document_id = %doc_id,
            inflight_tasks_cancelled = purged,
            "Cancelled in-flight tasks before reprocessing"
        );
    }

    let vector =
        crate::services::get_workspace_vector_storage_for_delete(state, workspace_id_for_tasks)
            .await?;
    // SPEC-119: checked retract fails closed on discovery timeouts.
    let retract_stats = crate::services::retract_document_indexes_checked(
        &state.storage.graph_storage,
        &vector,
        None,
        doc_id,
    )
    .await?;
    tracing::info!(
        document_id = %doc_id,
        entities_removed = retract_stats.entities_removed,
        entities_updated = retract_stats.entities_updated,
        relationships_removed = retract_stats.relationships_removed,
        embeddings_deleted = retract_stats.embeddings_deleted,
        "Retracted indexes before reprocessing (single cascade)"
    );
    let cleanup_admit_stats = Some(crate::services::reprocess_stage_reset::CleanupAdmitStats {
        entities_removed: retract_stats.entities_removed,
        relationships_removed: retract_stats.relationships_removed,
    });

    if let Some(mut metadata) = metadata_opt.clone() {
        if let Some(obj) = metadata.as_object_mut() {
            crate::services::reprocess_stage_reset::apply_post_cleanup_admission(
                obj,
                reprocess_mode,
                cleanup_admit_stats,
            );
            obj.insert("track_id".to_string(), serde_json::json!(new_track_id));
            crate::services::upsert_metadata_kv_with_index(
                state.storage.kv_storage.as_ref(),
                metadata_key,
                metadata.clone(),
            )
            .await?;
            *metadata_opt = Some(metadata);
        }
    }

    let content_key = format!("{}-content", doc_id);

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

    let workspace_id = tenant_ctx
        .workspace_id
        .clone()
        .unwrap_or_else(|| "default".to_string());
    let tenant_id = tenant_ctx
        .tenant_id
        .clone()
        .unwrap_or_else(|| "default".to_string());

    if source_type.as_deref() == Some("pdf") {
        if let Some(ref pid_str) = pdf_id_str {
            if let Ok(pdf_id_uuid) = uuid::Uuid::parse_str(pid_str) {
                let task_id = enqueue_pdf_reprocess(
                    state,
                    doc_id,
                    metadata_key,
                    metadata_opt,
                    pid_str,
                    pdf_id_uuid,
                    &tenant_id,
                    &workspace_id,
                    reprocess_mode,
                    restart_from_scratch,
                    new_track_id,
                )
                .await?;
                return Ok(Some(task_id));
            }
        }
    }

    if let Some(content_value) = state.storage.kv_storage.get_by_id(&content_key).await? {
        if let Some(content) = content_value.get("content").and_then(|v| v.as_str()) {
            let task_id = enqueue_text_reprocess(
                state,
                doc_id,
                metadata_key,
                content,
                &tenant_id,
                &workspace_id,
                reprocess_mode,
                restart_from_scratch,
                new_track_id,
            )
            .await?;
            return Ok(Some(task_id));
        }
    }

    Ok(None)
}

#[allow(clippy::too_many_arguments, unused_mut)]
async fn enqueue_pdf_reprocess(
    state: &AppState,
    doc_id: &str,
    metadata_key: &str,
    metadata_opt: &Option<serde_json::Value>,
    pid_str: &str,
    pdf_id_uuid: Uuid,
    tenant_id: &str,
    workspace_id: &str,
    mut reprocess_mode: edgequake_tasks::ReprocessMode,
    mut restart_from_scratch: bool,
    new_track_id: &str,
) -> ApiResult<String> {
    if !restart_from_scratch {
        #[cfg(feature = "postgres")]
        if let Some(pdf_storage) = state.storage.pdf_storage.as_ref() {
            let needs_full = match pdf_storage.get_pdf(&pdf_id_uuid).await {
                Ok(Some(pdf)) => super::super::storage_helpers::pdf_needs_full_reconversion(
                    pdf.markdown_content.as_deref(),
                ),
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
    }

    let (vision_provider, vision_model, pdf_parser_backend, vision_ws) =
        if let Ok(ws_uuid) = uuid::Uuid::parse_str(workspace_id) {
            if let Ok(Some(ws)) = state.workspace_service.get_workspace(ws_uuid).await {
                let vp = ws
                    .vision_llm_provider
                    .as_deref()
                    .filter(|p| !p.is_empty())
                    .unwrap_or("ollama")
                    .to_string();
                let vm = ws.vision_llm_model.clone().filter(|m| !m.is_empty());
                let backend = ws.resolved_pdf_parser_backend();
                (vp, vm, backend, Some(ws))
            } else {
                (
                    "ollama".to_string(),
                    None,
                    PdfParserBackend::from_env().unwrap_or_default(),
                    None,
                )
            }
        } else {
            (
                "ollama".to_string(),
                None,
                PdfParserBackend::from_env().unwrap_or_default(),
                None,
            )
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

    use edgequake_tasks::{PdfProcessingData, Task, TaskType};

    if restart_from_scratch {
        if let Err(e) = super::super::storage_helpers::clear_document_markdown_and_content(
            state,
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
        tenant_id: uuid::Uuid::parse_str(tenant_id)
            .map_err(|_| ApiError::ValidationError("Invalid tenant ID".to_string()))?,
        workspace_id: uuid::Uuid::parse_str(workspace_id)
            .map_err(|_| ApiError::ValidationError("Invalid workspace ID".to_string()))?,
        enable_vision: true,
        vision_provider,
        vision_model,
        existing_document_id: Some(doc_id.to_string()),
        pdf_parser_backend,
        pdf_parser_backend_explicit: true,
        restart_from_scratch,
        reprocess_mode: Some(reprocess_mode),
        multimodal_process_options,
        vision_reasoning_effort,
        vision_extract: Default::default(),
    };

    let task = Task::new(
        uuid::Uuid::parse_str(tenant_id)
            .map_err(|_| ApiError::ValidationError("Invalid tenant ID".to_string()))?,
        uuid::Uuid::parse_str(workspace_id)
            .map_err(|_| ApiError::ValidationError("Invalid workspace ID".to_string()))?,
        TaskType::PdfProcessing,
        serde_json::to_value(&pdf_task).unwrap(),
    );
    let task_track_id = task.track_id.clone();

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
                metadata_key,
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
        state,
        &task_track_id,
        pid_str,
        filename,
        Some(new_track_id),
    )
    .await;

    state.enqueue_task(task).await?;

    tracing::info!(
        document_id = %doc_id,
        pdf_id = %pid_str,
        task_id = %task_track_id,
        batch_track_id = %new_track_id,
        "Queued PDF reprocessing task (PdfProcessing) with existing document ID"
    );
    Ok(task_track_id)
}

#[allow(clippy::too_many_arguments)]
async fn enqueue_text_reprocess(
    state: &AppState,
    doc_id: &str,
    metadata_key: &str,
    content: &str,
    tenant_id: &str,
    workspace_id: &str,
    reprocess_mode: edgequake_tasks::ReprocessMode,
    restart_from_scratch: bool,
    new_track_id: &str,
) -> ApiResult<String> {
    use edgequake_tasks::{Task, TaskType, TextInsertData};

    let title = doc_id.to_string();
    let task_data = TextInsertData {
        text: content.to_string(),
        file_source: title.clone(),
        workspace_id: workspace_id.to_string(),
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
        uuid::Uuid::parse_str(tenant_id)
            .map_err(|_| ApiError::ValidationError("Invalid tenant ID".to_string()))?,
        uuid::Uuid::parse_str(workspace_id)
            .map_err(|_| ApiError::ValidationError("Invalid workspace ID".to_string()))?,
        TaskType::Insert,
        serde_json::to_value(task_data).unwrap(),
    );
    let task_track_id = task.track_id.clone();

    if let Some(mut metadata) = state.storage.kv_storage.get_by_id(metadata_key).await? {
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
                metadata_key,
                metadata,
            )
            .await?;
        }
    }

    state.enqueue_task(task).await?;
    Ok(task_track_id)
}

/// Gather live task/deletion/cancel facts and evaluate the admission SSOT.
pub(super) async fn admit_document_for_reprocess(
    state: &AppState,
    document_id: &str,
    doc_track_id: Option<&str>,
    status: Option<&str>,
    force: bool,
    restart_from_scratch: bool,
    workspace_uuid: Option<Uuid>,
) -> crate::services::ReprocessAdmitDecision {
    let has_active_ingest_task =
        crate::services::pending_doc_task_reconcile::has_active_task_for_document(
            state.tasks.storage.as_ref(),
            document_id,
            workspace_uuid,
        )
        .await
        .unwrap_or(true); // fail closed: assume active if lookup fails

    let has_active_deletion_task =
        crate::services::find_active_deletion_track_id(state, document_id, workspace_uuid)
            .await
            .is_some();

    let cancel_intent = match doc_track_id {
        Some(tid) if !tid.is_empty() => {
            state
                .tasks
                .cancellation_registry
                .has_cancel_intent(tid)
                .await
        }
        _ => false,
    };

    crate::services::evaluate_reprocess_admission(crate::services::ReprocessAdmitContext {
        status,
        force,
        restart_from_scratch,
        has_active_ingest_task,
        has_active_deletion_task,
        cancel_intent,
    })
}
