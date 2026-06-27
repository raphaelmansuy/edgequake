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
    persist_document_content, resolve_document_metadata_key, resolve_text_insert_content,
};
use crate::state::AppState;

use super::item_record::MultimodalSummary;
use super::metadata::{apply_process_options_to_metadata, resolve_process_options_from_metadata};
use super::stage::run_multimodal_analyze_stage_outcome;

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
    let metadata_key = resolve_document_metadata_key(&params.document_id, &kv).await;
    let Some(metadata) = kv.get_by_id(&metadata_key).await? else {
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
        .or(stored_opts);

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

    if let Some(err) = outcome.hard_error.as_ref() {
        return Err(ApiError::ValidationError(format!(
            "Multimodal analyze failed: {err}"
        )));
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

    let mut track_id = None;
    let mut requeued = false;

    if params.reindex {
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
