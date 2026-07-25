//! Worker handler for `TaskType::BatchDeletion` (SPEC-084 / GH-317).

use edgequake_tasks::{BatchDeletionTaskData, DeletionTaskData, Task, TaskResult};
use tokio_util::sync::CancellationToken;

use crate::handlers::documents::delete::resolve_kv_key_prefix_for_batch;
use crate::middleware::TenantContext;
use crate::services::{
    perform_document_deletion, purge_document_list_surfaces, ListSurfacePurgeOpts,
};

use super::DocumentTaskProcessor;

impl DocumentTaskProcessor {
    pub(super) async fn process_batch_deletion(
        &self,
        task: &mut Task,
        data: BatchDeletionTaskData,
        cancel_token: CancellationToken,
    ) -> TaskResult<serde_json::Value> {
        let Some(state) = self.app_state.as_ref() else {
            return Err(edgequake_tasks::TaskError::Processing(
                "Batch deletion requires AppState on DocumentTaskProcessor".to_string(),
            ));
        };

        let tenant_ctx = TenantContext {
            tenant_id: Some(data.tenant_id.clone()),
            workspace_id: Some(data.workspace_id.clone()),
            user_id: None,
        };

        let total = data.document_ids.len().max(1);
        let mut deleted = 0usize;
        let mut failed_ids = Vec::new();

        for (idx, document_id) in data.document_ids.iter().enumerate() {
            if cancel_token.is_cancelled() {
                return Err(edgequake_tasks::TaskError::Cancelled(
                    "Batch deletion cancelled".to_string(),
                ));
            }

            let pct = ((idx as f64 / total as f64) * 100.0).clamp(0.0, 100.0) as u8;
            task.update_progress(format!("deleting {document_id}"), idx as u32, pct);

            match build_deletion_task_data(state, document_id, &tenant_ctx, &data.batch_track_id)
                .await
            {
                // Already-absent KV: still purge list surfaces (SQL/wsdoc ghosts
                // from historical incomplete cascades). Never count success without this.
                Ok(None) => {
                    match purge_document_list_surfaces(
                        state,
                        document_id,
                        &data.workspace_id,
                        &tenant_ctx,
                        ListSurfacePurgeOpts {
                            key_prefix: Some(document_id),
                            content_hash: None,
                            pdf_id: None,
                        },
                    )
                    .await
                    {
                        Ok(_) => deleted += 1,
                        Err(e) => {
                            tracing::warn!(
                                document_id = %document_id,
                                error = %e,
                                "batch deletion: orphan list-surface purge failed"
                            );
                            failed_ids.push(document_id.clone());
                        }
                    }
                }
                Ok(Some(del_data)) => {
                    match perform_document_deletion(state, &del_data, &tenant_ctx).await {
                        Ok(_) => deleted += 1,
                        Err(e) => {
                            tracing::warn!(
                                document_id = %document_id,
                                error = %e,
                                "batch deletion: document cascade failed"
                            );
                            failed_ids.push(document_id.clone());
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        document_id = %document_id,
                        error = %e,
                        "batch deletion: resolve failed"
                    );
                    failed_ids.push(document_id.clone());
                }
            }
        }

        task.update_progress("batch_deletion_complete".to_string(), 1, 100);

        Ok(serde_json::json!({
            "batch_track_id": data.batch_track_id,
            "deleted_count": deleted,
            "failed_ids": failed_ids,
            "planned_delete_count": data.document_ids.len(),
        }))
    }
}

async fn build_deletion_task_data(
    state: &crate::state::AppState,
    document_id: &str,
    tenant_ctx: &TenantContext,
    batch_track_id: &str,
) -> Result<Option<DeletionTaskData>, String> {
    let (actual_key_prefix, metadata_key, has_metadata) =
        resolve_kv_key_prefix_for_batch(document_id, state).await;

    let chunk_prefix = format!("{actual_key_prefix}-chunk-");
    let chunk_ids: Vec<String> = state
        .storage
        .kv_storage
        .keys_with_prefix(&chunk_prefix)
        .await
        .unwrap_or_default();

    // IMP-075-12: content + metadata in one RT (not sequential get_by_id).
    let content_key = format!("{actual_key_prefix}-content");
    let keys = [content_key, metadata_key.clone()];
    let vals = state
        .storage
        .kv_storage
        .get_by_ids_ordered(&keys)
        .await
        .unwrap_or_default();
    let has_content = vals.first().and_then(|v| v.as_ref()).is_some();
    let metadata_val = vals.get(1).and_then(|v| v.clone());

    if !has_metadata && chunk_ids.is_empty() && !has_content {
        return Ok(None);
    }

    let default_ws = tenant_ctx.workspace_id_or_default();
    let (workspace_id, document_status, content_hash, pdf_id, ingest_track_id) = if has_metadata {
        if let Some(metadata) = metadata_val {
            (
                metadata
                    .get("workspace_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or(default_ws.as_str())
                    .to_string(),
                metadata
                    .get("status")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                metadata
                    .get("content_hash")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                metadata
                    .get("pdf_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                metadata
                    .get("track_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            )
        } else {
            (default_ws, None, None, None, None)
        }
    } else {
        (default_ws, None, None, None, None)
    };

    Ok(Some(DeletionTaskData {
        document_id: document_id.to_string(),
        key_prefix: actual_key_prefix,
        workspace_id,
        tenant_id: tenant_ctx
            .tenant_id
            .clone()
            .unwrap_or_else(|| "default".to_string()),
        deletion_track_id: format!("{batch_track_id}:{document_id}"),
        metadata_key: if has_metadata {
            Some(metadata_key)
        } else {
            None
        },
        chunk_ids,
        has_content,
        content_hash,
        pdf_id,
        ingest_track_id,
        document_status,
    }))
}
