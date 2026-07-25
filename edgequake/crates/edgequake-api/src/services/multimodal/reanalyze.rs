//! Re-analyze multimodal items without re-parse (LightRAG analyze worker parity, Phase 4h).

use std::sync::Arc;

use chrono::Utc;
use serde_json::json;
use tracing::{info, warn};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::documents::storage_helpers::{
    cleanup_document_graph_data, metadata_matches_tenant_context,
};
use crate::middleware::TenantContext;
use crate::services::text_insert_content::{
    load_staging_first_metadata, persist_document_content, resolve_text_insert_content,
};
use crate::state::AppState;

use super::item_record::MultimodalSummary;
use super::metadata::{apply_process_options_to_metadata, resolve_process_options_from_metadata};
use super::stage::run_multimodal_analyze_stage_outcome;
use crate::services::process_fingerprint::{
    apply_fingerprint_to_metadata, resolve_fingerprint_from_metadata, should_purge_on_reanalyze,
    ProcessFingerprintInput,
};

/// Parameters for multimodal re-analyze (no PDF re-convert).
#[derive(Debug, Clone)]
pub struct MultimodalReanalyzeParams {
    pub document_id: String,
    pub process_options: Option<String>,
    /// When true, queue entity re-index after analyze (LightRAG reprocess `entities` mode).
    pub reindex: bool,
}

/// Outcome of a multimodal re-analyze request.
#[derive(Debug, Clone)]
pub struct MultimodalReanalyzeOutcome {
    pub document_id: String,
    pub track_id: Option<String>,
    pub requeued: bool,
    pub summary: MultimodalSummary,
    pub hard_error: Option<String>,
}

/// Re-run multimodal analyze on stored markdown and optionally re-index entities.
pub async fn reanalyze_document_multimodal(
    state: &AppState,
    tenant_ctx: &TenantContext,
    params: MultimodalReanalyzeParams,
) -> ApiResult<MultimodalReanalyzeOutcome> {
    let kv = Arc::clone(&state.storage.kv_storage);
    // IMP-075-11: one RT staging+final (not resolve key then re-get).
    let Some((_, metadata)) = load_staging_first_metadata(kv.as_ref(), &params.document_id)
        .await
        .map_err(ApiError::Internal)?
    else {
        return Err(ApiError::NotFound(format!(
            "Document {} not found",
            params.document_id
        )));
    };

    if !metadata_matches_tenant_context(&metadata, tenant_ctx) {
        return Err(ApiError::NotFound(format!(
            "Document {} not found",
            params.document_id
        )));
    }

    let markdown = resolve_text_insert_content(&kv, &params.document_id, "")
        .await
        .map_err(ApiError::ValidationError)?;

    let stored_opts = resolve_process_options_from_metadata(&metadata);
    let process_options = params
        .process_options
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| stored_opts.clone());

    let filename = metadata
        .get("title")
        .or_else(|| metadata.get("filename"))
        .and_then(|v| v.as_str())
        .unwrap_or(&params.document_id)
        .to_string();

    let workspace_id = metadata
        .get("workspace_id")
        .and_then(|v| v.as_str())
        .or(tenant_ctx.workspace_id.as_deref())
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or(Uuid::nil());

    let tenant_id = metadata
        .get("tenant_id")
        .and_then(|v| v.as_str())
        .or(tenant_ctx.tenant_id.as_deref())
        .unwrap_or("default")
        .to_string();

    let outcome = run_multimodal_analyze_stage_outcome(
        markdown,
        process_options.as_deref(),
        &filename,
        Some(&state.workspace_service),
        workspace_id,
        Arc::clone(&state.query.llm_provider),
        None,
        Some(&params.document_id),
        Some(Arc::clone(&kv)),
    )
    .await;

    if crate::services::multimodal::should_abort_multimodal_hard_error(
        outcome.hard_error.as_deref(),
    ) {
        let err = outcome.hard_error.as_deref().unwrap_or("unknown");
        return Err(ApiError::ValidationError(format!(
            "Multimodal analyze failed: {err}"
        )));
    } else if let Some(err) = outcome.hard_error.as_ref() {
        info!(
            document_id = %params.document_id,
            error = %err,
            "Multimodal analyze hard error in degraded mode — continuing"
        );
    }

    persist_document_content(&kv, &params.document_id, &outcome.markdown)
        .await
        .map_err(ApiError::Internal)?;

    if let Some(opts) = process_options.as_deref() {
        let _ = crate::services::text_insert_content::patch_document_metadata(
            &kv,
            &params.document_id,
            |obj| apply_process_options_to_metadata(obj, Some(opts)),
        )
        .await;
    }

    // SPEC-046 EQ-046-14: fingerprint multimodal options; force purge when stale
    // (LightRAG `_purge_stale_extraction_if_resuming` parity).
    let mut fp_input = ProcessFingerprintInput::from_document_metadata(&metadata);
    if let Some(ref opts) = process_options {
        fp_input.multimodal_process_options = opts.clone();
    }
    let new_fp = fp_input.digest();
    let stored_fp = resolve_fingerprint_from_metadata(&metadata);
    let explicit_options_change = params
        .process_options
        .as_deref()
        .filter(|s| !s.is_empty())
        .is_some_and(|new| stored_opts.as_deref() != Some(new));
    let options_stale =
        should_purge_on_reanalyze(stored_fp.as_deref(), &fp_input, explicit_options_change);

    let _ = crate::services::text_insert_content::patch_document_metadata(
        &kv,
        &params.document_id,
        |obj| apply_fingerprint_to_metadata(obj, &new_fp),
    )
    .await;

    let mut track_id = None;
    let mut requeued = false;

    // Reindex when requested OR when process_options fingerprint changed
    let should_reindex = params.reindex || options_stale;
    if options_stale && !params.reindex {
        info!(
            document_id = %params.document_id,
            "SPEC-046: process_options fingerprint stale — forcing graph purge + reindex"
        );
    }

    if should_reindex {
        let new_track_id = format!(
            "reanalyze_{}_{}",
            Utc::now().format("%Y%m%d_%H%M%S"),
            &Uuid::new_v4().to_string()[..8]
        );
        track_id = Some(new_track_id.clone());

        match cleanup_document_graph_data(&params.document_id, &state.storage.graph_storage, None)
            .await
        {
            Ok(stats) => {
                info!(
                    document_id = %params.document_id,
                    entities_removed = stats.entities_removed,
                    relationships_removed = stats.relationships_removed,
                    "Cleaned graph data before multimodal reindex"
                );
            }
            Err(e) => {
                warn!(
                    document_id = %params.document_id,
                    error = %e,
                    "Graph cleanup before reanalyze failed; continuing"
                );
            }
        }

        let _ = crate::services::text_insert_content::patch_document_metadata(
            &kv,
            &params.document_id,
            |obj| {
                obj.insert("status".into(), json!("pending"));
                obj.insert("track_id".into(), json!(new_track_id));
                obj.insert("retry_at".into(), json!(Utc::now().to_rfc3339()));
                obj.insert("reanalyze".into(), json!(true));
            },
        )
        .await;

        use edgequake_tasks::{Task, TaskType, TextInsertData};

        let task_data = TextInsertData {
            text: String::new(),
            file_source: filename,
            workspace_id: workspace_id.to_string(),
            metadata: Some(json!({
                "document_id": params.document_id,
                "track_id": new_track_id,
                "is_retry": true,
                "reanalyze": true,
                "tenant_id": tenant_id,
                "workspace_id": workspace_id.to_string(),
                "multimodal_process_options": process_options,
            })),
        };

        let task = Task::new(
            Uuid::parse_str(&tenant_id).unwrap_or(Uuid::nil()),
            workspace_id,
            TaskType::Insert,
            serde_json::to_value(task_data).map_err(|e| ApiError::Internal(e.to_string()))?,
        );

        state.enqueue_task(task).await?;
        requeued = true;
    }

    Ok(MultimodalReanalyzeOutcome {
        document_id: params.document_id,
        track_id,
        requeued,
        summary: outcome.summary,
        hard_error: None,
    })
}
