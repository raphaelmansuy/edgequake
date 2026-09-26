//! Workspace `max_documents` admission (SPEC-066/067) — fail-closed when quota set.
//!
//! Declared on workspace metadata (`Workspace::max_documents`). Enforced at
//! upload / new PDF document mint. Counts committed wsdoc index keys **plus**
//! in-flight staging metadata for the same workspace.

use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::services::workspace_document_index::list_workspace_metadata_keys;
use crate::state::AppState;
use edgequake_core::WorkspaceService;
use edgequake_storage::traits::KVStorage;

/// Reject new uploads when workspace document count ≥ `max_documents`.
///
/// No-op when the workspace has no `max_documents` metadata (unlimited).
/// When checking a workspace that declares quota, bad UUID / missing workspace fail-closed.
pub async fn enforce_max_documents_admission(
    state: &AppState,
    workspace_id: &str,
) -> ApiResult<()> {
    enforce_max_documents_admission_parts(
        state.workspace_service.as_ref(),
        state.storage.kv_storage.as_ref(),
        workspace_id,
    )
    .await
}

/// Core admission used by HTTP (`AppState`) and worker mint paths.
pub async fn enforce_max_documents_admission_parts(
    workspace_service: &dyn WorkspaceService,
    kv: &dyn KVStorage,
    workspace_id: &str,
) -> ApiResult<()> {
    let ws_uuid = Uuid::parse_str(workspace_id).map_err(|_| {
        ApiError::BadRequest(format!(
            "invalid workspace_id for max_documents admission: {workspace_id}"
        ))
    })?;

    let workspace = workspace_service
        .get_workspace(ws_uuid)
        .await
        .map_err(|e| ApiError::Internal(format!("workspace lookup failed: {e}")))?
        .ok_or_else(|| {
            ApiError::BadRequest(format!(
                "workspace not found for max_documents admission: {workspace_id}"
            ))
        })?;

    let Some(max_docs) = workspace.max_documents() else {
        return Ok(());
    };
    if max_docs == 0 {
        return Err(ApiError::Conflict(
            "workspace max_documents is 0 — uploads disabled".into(),
        ));
    }

    let count = count_workspace_documents_for_quota(kv, workspace_id).await?;
    if count >= max_docs {
        return Err(ApiError::Conflict(format!(
            "workspace document quota exceeded: {count}/{max_docs} (max_documents)"
        )));
    }
    Ok(())
}

/// Committed wsdoc keys + staging metadata rows for this workspace.
pub async fn count_workspace_documents_for_quota(
    kv: &dyn KVStorage,
    workspace_id: &str,
) -> ApiResult<usize> {
    let committed = list_workspace_metadata_keys(kv, None, workspace_id)
        .await
        .map_err(|e| ApiError::Internal(format!("quota wsdoc scan failed: {e}")))?;
    let staging = count_staging_docs_for_workspace(kv, workspace_id).await?;
    Ok(committed.len() + staging)
}

async fn count_staging_docs_for_workspace(
    kv: &dyn KVStorage,
    workspace_id: &str,
) -> ApiResult<usize> {
    let keys = kv
        .keys_with_prefix("staging:")
        .await
        .map_err(|e| ApiError::Internal(format!("quota staging scan failed: {e}")))?;
    let mut n = 0usize;
    for key in keys {
        if !key.ends_with("-metadata") {
            continue;
        }
        let Some(meta) = kv
            .get_by_id(&key)
            .await
            .map_err(|e| ApiError::Internal(format!("quota staging get failed: {e}")))?
        else {
            continue;
        };
        let ws = meta
            .get("workspace_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if ws == workspace_id {
            n += 1;
        }
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_storage::adapters::memory::MemoryKVStorage;
    use edgequake_storage::kv_keys;
    use edgequake_storage::traits::KVStorage;
    use std::sync::Arc;

    #[test]
    fn quota_message_format() {
        let err = ApiError::Conflict(format!(
            "workspace document quota exceeded: {}/{} (max_documents)",
            10, 10
        ));
        assert!(err.to_string().contains("quota exceeded"));
    }

    #[tokio::test]
    async fn counts_committed_and_staging() {
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("quota-test"));
        let ws = "ws-quota";
        let doc = "doc-1";
        kv.upsert(&[(
            kv_keys::workspace_doc_index(ws, doc),
            serde_json::json!({"document_id": doc, "workspace_id": ws}),
        )])
        .await
        .unwrap();
        kv.upsert(&[(
            kv_keys::staging_doc_metadata("doc-staging"),
            serde_json::json!({"workspace_id": ws, "title": "pending"}),
        )])
        .await
        .unwrap();
        kv.upsert(&[(
            kv_keys::staging_doc_metadata("doc-other"),
            serde_json::json!({"workspace_id": "other", "title": "x"}),
        )])
        .await
        .unwrap();

        let n = count_workspace_documents_for_quota(kv.as_ref(), ws)
            .await
            .unwrap();
        assert_eq!(n, 2);
    }
}
