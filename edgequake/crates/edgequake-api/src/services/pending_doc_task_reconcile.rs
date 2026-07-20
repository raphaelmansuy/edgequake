//! SPEC-054 / GitHub #298 — reconcile pending documents that lack worker tasks.
//!
//! # First principles
//!
//! Document KV `pending`/`queued` is not work. A Task row reachable by workers is.
//! Startup auto-recovery used to rewrite metadata to `pending` without enqueueing,
//! leaving the pipeline idle forever (especially after in-memory task loss).
//!
//! This module is the SSOT (DRY + SRP) for:
//! - detecting "orphan pending" (non-terminal waiting doc, no active task)
//! - building the correct recovery task (PDF vs text)
//! - idempotent enqueue via [`AppState::enqueue_task`]

use serde_json::{json, Value};
use tracing::{info, warn};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::services::task_document_sync::extract_document_id_from_task;
use crate::state::AppState;
use edgequake_tasks::storage::{Pagination, TaskFilter, TaskStorage};
use edgequake_tasks::{PdfProcessingData, Task, TaskStatus, TaskType, TextInsertData};

/// Outcome of ensuring a recovery task for one document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnsureTaskOutcome {
    /// New task persisted + queued. Carries server task track_id (progress SSOT).
    Enqueued { task_id: String },
    /// Pending/processing task already references this document.
    AlreadyScheduled,
    /// Not enough metadata/content to build a task.
    SkippedNoContent,
    /// Document is not in a waiting/orphan-eligible status.
    SkippedNotEligible,
}

/// Report from a bulk reconcile pass.
#[derive(Debug, Default, Clone)]
pub struct ReconcilePendingReport {
    pub scanned: usize,
    pub enqueued: usize,
    pub already_scheduled: usize,
    pub skipped_no_content: usize,
    pub skipped_not_eligible: usize,
    pub errors: usize,
    pub document_ids: Vec<String>,
}

/// Waiting statuses that can be stranded without a task (#298).
pub fn is_orphan_waiting_status(status: &str) -> bool {
    matches!(status.to_lowercase().as_str(), "pending" | "queued")
}

/// True when TaskStorage already has a pending/processing task for this document.
pub async fn has_active_task_for_document(
    storage: &dyn TaskStorage,
    document_id: &str,
    workspace_id: Option<Uuid>,
) -> ApiResult<bool> {
    for status in [TaskStatus::Pending, TaskStatus::Processing] {
        let mut page = 1u32;
        loop {
            let list = storage
                .list_tasks(
                    TaskFilter {
                        workspace_id,
                        status: Some(status),
                        ..Default::default()
                    },
                    Pagination {
                        page,
                        page_size: 100,
                        ..Default::default()
                    },
                )
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;

            for task in &list.tasks {
                if extract_document_id_from_task(task).as_deref() == Some(document_id) {
                    return Ok(true);
                }
            }

            if page >= list.total_pages.max(1) {
                break;
            }
            page += 1;
        }
    }
    Ok(false)
}

fn parse_uuid_field(meta: &Value, key: &str) -> Option<Uuid> {
    let raw = meta.get(key).and_then(|v| v.as_str())?;
    match key {
        // WHY (#304): document metadata often stores the legacy `"default"` alias.
        // Uuid::parse_str("default") fails → recovery enqueue is skipped forever.
        "workspace_id" => crate::middleware::resolve_workspace_uuid(Some(raw)),
        "tenant_id" => crate::middleware::resolve_tenant_uuid(Some(raw)),
        _ => Uuid::parse_str(raw).ok(),
    }
}

fn meta_str<'a>(meta: &'a Value, key: &str) -> Option<&'a str> {
    meta.get(key).and_then(|v| v.as_str())
}

/// Resolve vision provider for PDF recovery (metadata → env default).
fn resolve_pdf_recovery_vision_provider(metadata: &Value) -> String {
    meta_str(metadata, "vision_provider")
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .unwrap_or_else(crate::vision_env::resolved_vision_provider_from_env)
}

/// Resolve vision model for PDF recovery (metadata only; env default when absent).
fn resolve_pdf_recovery_vision_model(metadata: &Value, vision_provider: &str) -> Option<String> {
    meta_str(metadata, "vision_model")
        .filter(|m| !m.is_empty())
        .map(str::to_string)
        .or_else(|| {
            Some(crate::vision_env::default_vision_model_for_provider(
                vision_provider,
            ))
        })
}

/// SSOT builder for PDF recovery tasks (DRY across reconcile / recover-stuck / reprocess).
pub fn build_pdf_recovery_task_data(
    metadata: &Value,
    pdf_id: Uuid,
    tenant_id: Uuid,
    workspace_id: Uuid,
    document_id: &str,
) -> PdfProcessingData {
    let vision_provider = resolve_pdf_recovery_vision_provider(metadata);
    PdfProcessingData {
        pdf_id,
        tenant_id,
        workspace_id,
        enable_vision: true,
        vision_provider: vision_provider.clone(),
        vision_model: resolve_pdf_recovery_vision_model(metadata, &vision_provider),
        existing_document_id: Some(document_id.to_string()),
        pdf_parser_backend: edgequake_pdf::PdfParserBackend::Vision,
        pdf_parser_backend_explicit: false,
        restart_from_scratch: false,
        reprocess_mode: None,
        multimodal_process_options: None,
    }
}

/// Build a recovery task for document metadata (PDF → PdfProcessing, else Insert).
///
/// Returns `None` when the document cannot be re-enqueued (no pdf_id and no content).
pub fn build_recovery_task_from_metadata(
    document_id: &str,
    metadata: &Value,
    batch_track_id: &str,
    reason: &str,
) -> Option<(TaskType, Value, Uuid, Uuid)> {
    let tenant_id = parse_uuid_field(metadata, "tenant_id")?;
    let workspace_id = parse_uuid_field(metadata, "workspace_id")?;
    let title = meta_str(metadata, "title")
        .or_else(|| meta_str(metadata, "file_name"))
        .unwrap_or(document_id);

    let source_type = meta_str(metadata, "source_type");
    let pdf_id = meta_str(metadata, "pdf_id").and_then(|s| Uuid::parse_str(s).ok());

    if source_type == Some("pdf") {
        if let Some(pdf_id) = pdf_id {
            let task_data = build_pdf_recovery_task_data(
                metadata,
                pdf_id,
                tenant_id,
                workspace_id,
                document_id,
            );
            let value = serde_json::to_value(&task_data).ok()?;
            return Some((TaskType::PdfProcessing, value, tenant_id, workspace_id));
        }
    }

    // Text path requires content to be loaded by the caller via ensure_*.
    let _ = (title, batch_track_id, reason);
    None
}

/// Ensure a pending/queued document has a worker task (idempotent).
///
/// For text documents, `content` must be provided when no `pdf_id` route applies.
pub async fn ensure_task_for_pending_document(
    state: &AppState,
    document_id: &str,
    metadata: &Value,
    content: Option<&str>,
    batch_track_id: &str,
    reason: &str,
) -> ApiResult<EnsureTaskOutcome> {
    let status = meta_str(metadata, "status").unwrap_or("pending");
    if !is_orphan_waiting_status(status)
        && !crate::document_metadata::is_active_processing_status(status)
    {
        return Ok(EnsureTaskOutcome::SkippedNotEligible);
    }

    let workspace_id = parse_uuid_field(metadata, "workspace_id");
    if has_active_task_for_document(state.tasks.storage.as_ref(), document_id, workspace_id).await?
    {
        return Ok(EnsureTaskOutcome::AlreadyScheduled);
    }

    // PDF single-flight via pdf_id when present.
    if let (Some(pdf_id), Some(ws)) = (
        meta_str(metadata, "pdf_id").and_then(|s| Uuid::parse_str(s).ok()),
        workspace_id,
    ) {
        if let Ok(Some(_)) = state
            .tasks
            .storage
            .find_active_pdf_processing_task(pdf_id, ws)
            .await
        {
            return Ok(EnsureTaskOutcome::AlreadyScheduled);
        }
    }

    let tenant_id = parse_uuid_field(metadata, "tenant_id")
        .unwrap_or_else(crate::middleware::default_tenant_uuid);
    let workspace_id =
        workspace_id.unwrap_or_else(crate::middleware::default_workspace_uuid);

    let title = meta_str(metadata, "title")
        .or_else(|| meta_str(metadata, "file_name"))
        .unwrap_or(document_id)
        .to_string();

    let (task_type, task_value) = if meta_str(metadata, "source_type") == Some("pdf") {
        if let Some(pdf_id) = meta_str(metadata, "pdf_id").and_then(|s| Uuid::parse_str(s).ok()) {
            let task_data = build_pdf_recovery_task_data(
                metadata,
                pdf_id,
                tenant_id,
                workspace_id,
                document_id,
            );
            (
                TaskType::PdfProcessing,
                serde_json::to_value(&task_data)
                    .map_err(|e| ApiError::Internal(format!("serialize PDF recovery task: {e}")))?,
            )
        } else if let Some(text) = content.filter(|c| !c.trim().is_empty()) {
            text_insert_task_value(
                document_id,
                &title,
                text,
                batch_track_id,
                reason,
                tenant_id,
                workspace_id,
            )?
        } else {
            return Ok(EnsureTaskOutcome::SkippedNoContent);
        }
    } else if let Some(text) = content.filter(|c| !c.trim().is_empty()) {
        text_insert_task_value(
            document_id,
            &title,
            text,
            batch_track_id,
            reason,
            tenant_id,
            workspace_id,
        )?
    } else {
        return Ok(EnsureTaskOutcome::SkippedNoContent);
    };

    // Create task first so document.track_id == progress/WS key (SPEC-054 SSOT).
    let task = Task::new(tenant_id, workspace_id, task_type, task_value);
    let task_id = task.track_id.clone();

    let metadata_key =
        crate::services::document_metadata_scan::metadata_key_for_document(document_id);
    if let Ok(Some(mut meta_val)) = state.storage.kv_storage.get_by_id(&metadata_key).await {
        if let Some(obj) = meta_val.as_object_mut() {
            obj.insert("status".to_string(), json!("pending"));
            obj.insert("track_id".to_string(), json!(task_id));
            obj.insert(
                "stage_message".to_string(),
                json!(format!("SPEC-054/#298: {reason}")),
            );
            obj.insert(
                "updated_at".to_string(),
                json!(chrono::Utc::now().to_rfc3339()),
            );
            // Keep batch id for multi-doc correlation only (not a progress key).
            obj.insert("batch_track_id".to_string(), json!(batch_track_id));
            let _ = crate::services::upsert_metadata_kv_with_index(
                state.storage.kv_storage.as_ref(),
                &metadata_key,
                meta_val,
            )
            .await;
        }
    }

    state.enqueue_task(task).await?;

    // SPEC-054: every PDF enqueue path must seed progress under task_id
    // (upload + full reprocess already do; reconcile/stuck must match).
    if task_type == TaskType::PdfProcessing {
        if let Some(pdf_id) = meta_str(metadata, "pdf_id") {
            crate::handlers::pdf_upload::seed_pdf_job_progress(
                state,
                &task_id,
                pdf_id,
                &title,
                Some(batch_track_id),
            )
            .await;
        }
    }

    info!(
        document_id = %document_id,
        task_id = %task_id,
        batch_track_id = %batch_track_id,
        reason = %reason,
        "SPEC-054/#298: enqueued recovery task for orphan pending document"
    );
    Ok(EnsureTaskOutcome::Enqueued { task_id })
}

fn text_insert_task_value(
    document_id: &str,
    title: &str,
    text: &str,
    batch_track_id: &str,
    reason: &str,
    tenant_id: Uuid,
    workspace_id: Uuid,
) -> ApiResult<(TaskType, Value)> {
    let task_data = TextInsertData {
        text: text.to_string(),
        file_source: title.to_string(),
        workspace_id: workspace_id.to_string(),
        metadata: Some(json!({
            "document_id": document_id,
            "title": title,
            "track_id": batch_track_id,
            "is_recovery": true,
            "recovery_reason": reason,
            "tenant_id": tenant_id.to_string(),
            "workspace_id": workspace_id.to_string(),
        })),
    };
    Ok((
        TaskType::Insert,
        serde_json::to_value(&task_data)
            .map_err(|e| ApiError::Internal(format!("serialize text recovery task: {e}")))?,
    ))
}

/// Default startup/periodic enqueue budget (stampede guard).
/// Override with `EDGEQUAKE_STARTUP_RECONCILE_MAX`.
pub const DEFAULT_STARTUP_RECONCILE_MAX: usize = 32;

/// Resolve startup reconcile enqueue budget from env.
pub fn startup_reconcile_max_from_env() -> usize {
    std::env::var("EDGEQUAKE_STARTUP_RECONCILE_MAX")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_STARTUP_RECONCILE_MAX)
}

async fn ensure_one_orphan(
    state: &AppState,
    document_id: &str,
    metadata: &Value,
    batch_track_id: &str,
    reason: &str,
    report: &mut ReconcilePendingReport,
) {
    report.scanned += 1;
    let content_key = format!("{document_id}-content");
    let content = state
        .storage
        .kv_storage
        .get_by_id(&content_key)
        .await
        .ok()
        .flatten()
        .and_then(|v| {
            v.get("content")
                .and_then(|c| c.as_str())
                .map(str::to_string)
        });

    match ensure_task_for_pending_document(
        state,
        document_id,
        metadata,
        content.as_deref(),
        batch_track_id,
        reason,
    )
    .await
    {
        Ok(EnsureTaskOutcome::Enqueued { .. }) => {
            report.enqueued += 1;
            report.document_ids.push(document_id.to_string());
        }
        Ok(EnsureTaskOutcome::AlreadyScheduled) => report.already_scheduled += 1,
        Ok(EnsureTaskOutcome::SkippedNoContent) => report.skipped_no_content += 1,
        Ok(EnsureTaskOutcome::SkippedNotEligible) => report.skipped_not_eligible += 1,
        Err(e) => {
            report.errors += 1;
            warn!(
                document_id = %document_id,
                error = %e,
                "SPEC-054/#298: failed to enqueue recovery task"
            );
        }
    }
}

/// Ensure tasks for an explicit list of document IDs (startup recover targets).
pub async fn reconcile_pending_documents_by_ids(
    state: &AppState,
    document_ids: &[String],
    max_documents: usize,
    reason: &str,
) -> ApiResult<ReconcilePendingReport> {
    use crate::services::document_metadata_scan::metadata_key_for_document;

    let batch_track_id = format!(
        "reconcile298_{}_{}",
        chrono::Utc::now().format("%Y%m%d_%H%M%S"),
        &Uuid::new_v4().to_string()[..8]
    );
    let mut report = ReconcilePendingReport::default();

    for document_id in document_ids.iter().take(max_documents) {
        if report.enqueued >= max_documents {
            break;
        }
        let metadata_key = metadata_key_for_document(document_id);
        let Some(metadata) = state
            .storage
            .kv_storage
            .get_by_id(&metadata_key)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
        else {
            report.skipped_no_content += 1;
            continue;
        };
        ensure_one_orphan(
            state,
            document_id,
            &metadata,
            &batch_track_id,
            reason,
            &mut report,
        )
        .await;
    }

    info!(
        scanned = report.scanned,
        enqueued = report.enqueued,
        already_scheduled = report.already_scheduled,
        "SPEC-054/#298: priority pending-doc reconcile complete"
    );
    Ok(report)
}

/// Scan metadata and enqueue tasks for waiting docs with no active task.
///
/// Stampede guard: stops after `max_documents` enqueues (not full DB scan
/// processing). Prefer [`reconcile_pending_documents_by_ids`] for recovered
/// docs from the current boot.
pub async fn reconcile_pending_documents_missing_tasks(
    state: &AppState,
    max_documents: usize,
    reason: &str,
) -> ApiResult<ReconcilePendingReport> {
    use crate::services::document_metadata_scan::load_all_document_metadata_entries;

    let batch_track_id = format!(
        "reconcile298_{}_{}",
        chrono::Utc::now().format("%Y%m%d_%H%M%S"),
        &Uuid::new_v4().to_string()[..8]
    );

    let mut report = ReconcilePendingReport::default();
    // Suffix scan returns keys; we filter status before loading content.
    let entries = load_all_document_metadata_entries(state.storage.kv_storage.as_ref())
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    for (_key, value) in entries {
        if report.enqueued >= max_documents {
            break;
        }

        let Some(obj) = value.as_object() else {
            continue;
        };
        let status = obj.get("status").and_then(|v| v.as_str()).unwrap_or("");
        if !is_orphan_waiting_status(status) {
            continue;
        }

        let Some(document_id) = obj.get("id").and_then(|v| v.as_str()) else {
            continue;
        };

        ensure_one_orphan(
            state,
            document_id,
            &value,
            &batch_track_id,
            reason,
            &mut report,
        )
        .await;
    }

    info!(
        scanned = report.scanned,
        enqueued = report.enqueued,
        already_scheduled = report.already_scheduled,
        skipped_no_content = report.skipped_no_content,
        errors = report.errors,
        max_documents,
        "SPEC-054/#298: pending-document task reconcile complete"
    );
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orphan_waiting_statuses() {
        assert!(is_orphan_waiting_status("pending"));
        assert!(is_orphan_waiting_status("queued"));
        assert!(is_orphan_waiting_status("PENDING"));
        assert!(!is_orphan_waiting_status("processing"));
        assert!(!is_orphan_waiting_status("failed"));
        assert!(!is_orphan_waiting_status("completed"));
    }

    #[test]
    fn parse_uuid_field_resolves_default_alias() {
        let meta = json!({
            "tenant_id": "default",
            "workspace_id": "default",
        });
        assert_eq!(
            parse_uuid_field(&meta, "workspace_id"),
            Some(crate::middleware::default_workspace_uuid())
        );
        assert_eq!(
            parse_uuid_field(&meta, "tenant_id"),
            Some(crate::middleware::default_tenant_uuid())
        );
    }

    #[test]
    fn build_pdf_recovery_accepts_default_workspace_alias() {
        let meta = json!({
            "id": "doc-1",
            "tenant_id": "default",
            "workspace_id": "default",
            "source_type": "pdf",
            "pdf_id": "33333333-3333-3333-3333-333333333333",
            "title": "paper.pdf",
            "status": "pending"
        });
        let built = build_recovery_task_from_metadata("doc-1", &meta, "batch", "test");
        assert!(
            built.is_some(),
            "recovery must accept legacy default alias (#304)"
        );
        let (ty, _, tenant, workspace) = built.unwrap();
        assert_eq!(ty, TaskType::PdfProcessing);
        assert_eq!(tenant, crate::middleware::default_tenant_uuid());
        assert_eq!(workspace, crate::middleware::default_workspace_uuid());
    }

    #[test]
    fn build_pdf_recovery_task_from_metadata() {
        let meta = json!({
            "id": "doc-1",
            "tenant_id": "11111111-1111-1111-1111-111111111111",
            "workspace_id": "22222222-2222-2222-2222-222222222222",
            "source_type": "pdf",
            "pdf_id": "33333333-3333-3333-3333-333333333333",
            "title": "paper.pdf",
            "status": "pending"
        });
        let built = build_recovery_task_from_metadata("doc-1", &meta, "batch", "test");
        assert!(built.is_some());
        let (ty, _, _, _) = built.unwrap();
        assert_eq!(ty, TaskType::PdfProcessing);
    }

    #[test]
    fn pdf_without_pdf_id_returns_none_from_builder() {
        let meta = json!({
            "id": "doc-1",
            "tenant_id": "11111111-1111-1111-1111-111111111111",
            "workspace_id": "22222222-2222-2222-2222-222222222222",
            "source_type": "pdf",
            "status": "pending"
        });
        assert!(build_recovery_task_from_metadata("doc-1", &meta, "batch", "test").is_none());
    }

    #[test]
    fn startup_reconcile_max_defaults_when_env_unset() {
        // Isolate from developer shells that may set the var.
        let prev = std::env::var("EDGEQUAKE_STARTUP_RECONCILE_MAX").ok();
        std::env::remove_var("EDGEQUAKE_STARTUP_RECONCILE_MAX");
        assert_eq!(
            startup_reconcile_max_from_env(),
            DEFAULT_STARTUP_RECONCILE_MAX
        );
        if let Some(v) = prev {
            std::env::set_var("EDGEQUAKE_STARTUP_RECONCILE_MAX", v);
        }
    }

    #[test]
    fn pdf_recovery_uses_metadata_vision_provider_not_hardcoded_ollama() {
        let meta = json!({
            "id": "doc-mistral",
            "tenant_id": "11111111-1111-1111-1111-111111111111",
            "workspace_id": "22222222-2222-2222-2222-222222222222",
            "source_type": "pdf",
            "pdf_id": "33333333-3333-3333-3333-333333333333",
            "vision_provider": "mistral",
            "vision_model": "mistral-small-latest",
            "status": "pending"
        });
        let task = build_pdf_recovery_task_data(
            &meta,
            Uuid::parse_str("33333333-3333-3333-3333-333333333333").unwrap(),
            Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
            Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
            "doc-mistral",
        );
        assert_eq!(task.vision_provider, "mistral");
        assert_eq!(task.vision_model.as_deref(), Some("mistral-small-latest"));
    }
}
