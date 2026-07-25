//! Workspace-scoped content-hash duplicate resolution (SPEC-040 #253).
//!
//! WHY: `doc:hash:{workspace}:{sha256}` can outlive KV metadata after partial
//! deletes or failed ingests. PDF rows already recycle via `pdf_workspace_dedup`;
//! this module provides the same orphan pattern for text/markdown uploads.

use tracing::warn;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::services::document_metadata_scan::metadata_key_for_document;
use crate::state::AppState;
use crate::workspace_scope::metadata_matches_tenant_context;
use edgequake_storage::kv_keys;

/// Returns true when promoted or staging metadata still backs this hash mapping.
///
/// IMP-075-07: final + staging in one `get_by_ids_ordered` (O(1) RT), not two sequential gets.
pub async fn workspace_has_visible_document_for_hash(
    state: &AppState,
    document_id: &str,
    tenant_ctx: &TenantContext,
) -> ApiResult<bool> {
    let metadata_key = metadata_key_for_document(document_id);
    let staging_key = kv_keys::staging_doc_metadata(document_id);
    let keys = [metadata_key, staging_key];
    let vals = state
        .storage
        .kv_storage
        .get_by_ids_ordered(&keys)
        .await
        .map_err(|e| ApiError::Internal(format!("KV batch metadata read failed: {e}")))?;

    for meta in vals.into_iter().flatten() {
        if metadata_matches_tenant_context(&meta, tenant_ctx) {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Remove hash + staging hash keys when no visible document backs them.
pub async fn recycle_orphan_workspace_hash(
    state: &AppState,
    hash_key: &str,
    workspace_id: &str,
    document_id: &str,
) -> ApiResult<()> {
    warn!(
        document_id = %document_id,
        workspace_id = %workspace_id,
        hash_key = %hash_key,
        "Recycling orphan content-hash key (no visible workspace document)"
    );

    let mut keys = vec![hash_key.to_string()];
    if let Some(content_hash) = content_hash_from_workspace_hash_key(hash_key, workspace_id) {
        keys.push(kv_keys::staging_workspace_hash(workspace_id, &content_hash));
    }

    state
        .storage
        .kv_storage
        .delete(&keys)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to recycle orphan hash key: {e}")))?;

    Ok(())
}

/// Parse SHA-256 hex from `doc:hash:{workspace_id}:{content_hash}`.
pub fn content_hash_from_workspace_hash_key(hash_key: &str, workspace_id: &str) -> Option<String> {
    let prefix = format!("doc:hash:{workspace_id}:");
    hash_key.strip_prefix(&prefix).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_content_hash_from_workspace_hash_key() {
        let ws = "ws-123";
        let hash = "abc123";
        let key = format!("doc:hash:{ws}:{hash}");
        assert_eq!(
            content_hash_from_workspace_hash_key(&key, ws).as_deref(),
            Some(hash)
        );
    }
}
