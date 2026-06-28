//! Document storage helper facade — re-exports service SSOT modules (SPEC-027 phase 6).
//!
//! Handlers import from here for ascending compatibility; logic lives in `services/`.

use uuid::Uuid;

use crate::error::ApiError;
use crate::middleware::TenantContext;
use crate::state::AppState;

pub use crate::services::document_graph_cascade::{cleanup_document_graph_data, CleanupStats};
pub use crate::services::document_reingest::{
    resolve_workspace_duplicate_for_reingestion, DuplicateReingestAction,
};
pub use crate::services::document_task_cleanup::{
    purge_persisted_tasks_for_document, purge_workspace_tasks,
};
pub use crate::services::document_vector_storage::get_workspace_vector_storage_for_delete;

/// Check whether a metadata payload belongs to the requester's tenant + workspace.
pub(crate) fn metadata_matches_tenant_context(
    metadata: &serde_json::Value,
    tenant_ctx: &TenantContext,
) -> bool {
    crate::workspace_scope::metadata_matches_tenant_context(metadata, tenant_ctx)
}

/// Attach tenant/workspace scope to graph node or edge properties (BR0201).
#[allow(dead_code)]
pub(crate) fn insert_graph_tenant_scope(
    properties: &mut std::collections::HashMap<String, serde_json::Value>,
    tenant_id: &Option<String>,
    workspace_id: &str,
) {
    if let Some(ref tid) = tenant_id {
        properties.insert("tenant_id".to_string(), serde_json::json!(tid));
    }
    properties.insert("workspace_id".to_string(), serde_json::json!(workspace_id));
}

#[cfg(test)]
mod graph_scope_tests {
    use super::insert_graph_tenant_scope;

    #[test]
    fn insert_graph_tenant_scope_sets_workspace_and_tenant() {
        let mut props = std::collections::HashMap::new();
        insert_graph_tenant_scope(&mut props, &Some("tenant-abc".to_string()), "workspace-xyz");
        assert_eq!(
            props.get("workspace_id").and_then(|v| v.as_str()),
            Some("workspace-xyz")
        );
        assert_eq!(
            props.get("tenant_id").and_then(|v| v.as_str()),
            Some("tenant-abc")
        );
    }
}

/// Clear cached PDF markdown + KV content/chunks so a full re-conversion runs.
pub(crate) async fn clear_document_markdown_and_content(
    state: &AppState,
    document_id: &str,
    pdf_id: &Uuid,
) -> Result<(), ApiError> {
    #[cfg(feature = "postgres")]
    if let Some(ref pdf_storage) = state.storage.pdf_storage {
        if let Err(e) = pdf_storage.clear_markdown(pdf_id).await {
            tracing::warn!(
                pdf_id = %pdf_id,
                document_id = %document_id,
                error = %e,
                "Failed to clear cached PDF markdown before re-conversion"
            );
        }
    }
    let _ = pdf_id;

    let doc_prefix = format!("{}-", document_id);
    let keys: Vec<String> = state
        .storage
        .kv_storage
        .keys_with_prefix(&doc_prefix)
        .await?;
    let chunk_prefix = format!("{}-chunk-", document_id);
    let keys_to_delete: Vec<String> = keys
        .iter()
        .filter(|k| k.ends_with("-content") || k.starts_with(&chunk_prefix))
        .cloned()
        .collect();

    if !keys_to_delete.is_empty() {
        if let Err(e) = state.storage.kv_storage.delete(&keys_to_delete).await {
            tracing::warn!(
                document_id = %document_id,
                error = %e,
                "Failed to clear KV content/chunks before re-conversion"
            );
        }
    }

    Ok(())
}

/// Returns true when cached markdown is missing or empty (requires full PDF re-conversion).
#[allow(dead_code)]
pub(crate) fn pdf_needs_full_reconversion(markdown: Option<&str>) -> bool {
    markdown.is_none_or(|md| md.trim().is_empty())
}

#[cfg(test)]
mod reconvert_tests {
    use super::pdf_needs_full_reconversion;

    #[test]
    fn none_markdown_requires_full() {
        assert!(pdf_needs_full_reconversion(None));
    }

    #[test]
    fn empty_markdown_requires_full() {
        assert!(pdf_needs_full_reconversion(Some("")));
        assert!(pdf_needs_full_reconversion(Some("   \n\t ")));
    }

    #[test]
    fn non_empty_markdown_reuses_entities() {
        assert!(!pdf_needs_full_reconversion(Some("# Hello\nworld")));
        assert!(!pdf_needs_full_reconversion(Some("x")));
    }
}
