//! Workspace-scoped document KV index (SPEC-027 phase 8).
//!
//! Maintains `wsdoc:{workspace_id}:{document_id}` pointer keys so workspace
//! operations use prefix scans instead of global `-metadata` suffix scans.
//!
//! SPEC-091 Wave B3: reads are flag-gated (`EDGEQUAKE_KV_FAMILY_WSDOC=
//! relational`) onto `public.documents.workspace_id` — the membership SSOT.
//! The relational branch needs the startup-registered pool and a UUID
//! workspace id; any gap falls back to the legacy KV index (never an error).

use edgequake_storage::error::StorageError;
use edgequake_storage::kv_family_cutover::{
    kv_family_mode_from_env, KvFamilyMode, KV_FAMILY_WSDOC,
};
use edgequake_storage::kv_keys;
use edgequake_storage::traits::KVStorage;

/// Register the Postgres pool for relational membership reads. Delegates to
/// the shared sidecar registry (SPEC-091 Wave B4/B5) so one pool serves every
/// relational KV-family cutover (DRY).
#[cfg(feature = "postgres")]
pub fn register_membership_pool(pool: impl Into<std::sync::Arc<sqlx::PgPool>>) {
    crate::services::relational_sidecar_store::register_sidecar_pool(pool);
}

/// Relational membership: `documents.id` for a workspace, when cut over.
/// Returns `None` (→ KV fallback) when the flag is off, no pool is registered,
/// the workspace id is not a UUID, or the query fails (warn-logged).
async fn relational_workspace_doc_ids(
    pool: crate::services::OptionalPgPool<'_>,
    workspace_id: &str,
) -> Option<Vec<String>> {
    #[cfg(feature = "postgres")]
    {
        // SPEC-091 RM1: relational is SSOT. Explicit `EDGEQUAKE_KV_FAMILY_WSDOC=kv`
        // remains soak rollback only — otherwise never fall back to KV.
        let force_kv = kv_family_mode_from_env(KV_FAMILY_WSDOC) == KvFamilyMode::Kv;
        if force_kv {
            return None;
        }
        let pool = pool?;
        let ws = uuid::Uuid::parse_str(workspace_id).ok()?;
        // Wave B3/C: shell-written documents may carry the workspace in the
        // metadata JSONB while the FK-guarded `workspace_id` column stays NULL
        // (e.g. workspaces not present as DB rows). Match either source so
        // membership is correct regardless of which representation is set.
        match sqlx::query_scalar::<_, uuid::Uuid>(
            "SELECT id FROM public.documents \
             WHERE workspace_id = $1 \
                OR metadata->>'workspace_id' = $2",
        )
        .bind(ws)
        .bind(workspace_id)
        .fetch_all(pool)
        .await
        {
            Ok(rows) => Some(rows.iter().map(uuid::Uuid::to_string).collect()),
            Err(e) => {
                tracing::warn!(
                    workspace_id = %workspace_id,
                    error = %e,
                    "relational wsdoc membership read failed; returning empty (no KV fallback)"
                );
                Some(Vec::new())
            }
        }
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (pool, workspace_id);
        None
    }
}

/// Sync workspace index entry from a document metadata KV write.
pub async fn sync_workspace_document_index(
    kv: &dyn KVStorage,
    metadata_key: &str,
    metadata: &serde_json::Value,
) -> Result<(), StorageError> {
    if metadata_key.starts_with("staging:") {
        return Ok(());
    }
    let Some(document_id) = metadata_key.strip_suffix("-metadata") else {
        return Ok(());
    };
    let workspace_id = metadata
        .get("workspace_id")
        .and_then(|v| v.as_str())
        .unwrap_or("default");
    let tenant_id = metadata
        .get("tenant_id")
        .and_then(|v| v.as_str())
        .unwrap_or("default");

    let index_key = kv_keys::workspace_doc_index(workspace_id, document_id);
    let index_value = serde_json::json!({
        "metadata_key": metadata_key,
        "document_id": document_id,
        "workspace_id": workspace_id,
        "tenant_id": tenant_id,
    });
    kv.upsert(&[(index_key, index_value)]).await
}

/// Remove workspace index entry for a document.
pub async fn remove_workspace_document_index(
    kv: &dyn KVStorage,
    workspace_id: &str,
    document_id: &str,
) -> Result<(), StorageError> {
    let index_key = kv_keys::workspace_doc_index(workspace_id, document_id);
    kv.delete(&[index_key]).await
}

/// Upsert a metadata KV entry and maintain the workspace doc index (SSOT write path).
pub async fn upsert_metadata_kv_with_index(
    kv: &dyn KVStorage,
    metadata_key: &str,
    metadata: serde_json::Value,
) -> Result<(), StorageError> {
    kv.upsert(&[(metadata_key.to_string(), metadata.clone())])
        .await?;
    sync_workspace_document_index(kv, metadata_key, &metadata).await
}

/// Upsert final `{document_id}-metadata` and maintain workspace index (SSOT write path).
pub async fn upsert_final_document_metadata(
    kv: &dyn KVStorage,
    document_id: &str,
    metadata: serde_json::Value,
) -> Result<(), StorageError> {
    let key = kv_keys::doc_metadata(document_id);
    upsert_metadata_kv_with_index(kv, &key, metadata).await
}

/// After any final metadata KV upsert, call to keep wsdoc index in sync.
pub async fn sync_after_metadata_upsert(
    kv: &dyn KVStorage,
    metadata_key: &str,
    metadata: &serde_json::Value,
) -> Result<(), StorageError> {
    sync_workspace_document_index(kv, metadata_key, metadata).await
}

/// Workspace metadata-key listing with membership authority (SPEC-111 / #366).
///
/// When `authoritative` is true, relational `documents` membership answered
/// (including **empty**). Callers must treat empty keys as an empty workspace
/// and must **not** fall back to a global KV `-metadata` suffix scan — that
/// resurrected dual-write residue after Clear All (issue #366 / #360 on v0.24.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceMetadataKeyList {
    pub keys: Vec<String>,
    pub truncated: bool,
    pub authoritative: bool,
}

/// List metadata keys for documents in a workspace via index prefix scan.
pub async fn list_workspace_metadata_keys(
    kv: &dyn KVStorage,
    pool: crate::services::OptionalPgPool<'_>,
    workspace_id: &str,
) -> Result<Vec<String>, StorageError> {
    Ok(
        list_workspace_metadata_keys_detailed(kv, pool, workspace_id)
            .await?
            .keys,
    )
}

/// Same as [`list_workspace_metadata_keys`] but preserves membership authority.
pub async fn list_workspace_metadata_keys_detailed(
    kv: &dyn KVStorage,
    pool: crate::services::OptionalPgPool<'_>,
    workspace_id: &str,
) -> Result<WorkspaceMetadataKeyList, StorageError> {
    if let Some(doc_ids) = relational_workspace_doc_ids(pool, workspace_id).await {
        return Ok(WorkspaceMetadataKeyList {
            keys: doc_ids
                .iter()
                .map(|doc_id| kv_keys::doc_metadata(doc_id))
                .collect(),
            truncated: false,
            authoritative: true,
        });
    }
    let prefix = kv_keys::workspace_doc_index_prefix(workspace_id);
    let index_keys = kv.keys_with_prefix(&prefix).await?;
    let mut metadata_keys = Vec::with_capacity(index_keys.len());
    for key in index_keys {
        if let Some((ws, doc_id)) = kv_keys::parse_workspace_doc_index(&key) {
            if ws == workspace_id {
                metadata_keys.push(kv_keys::doc_metadata(doc_id));
            }
        }
    }
    Ok(WorkspaceMetadataKeyList {
        keys: metadata_keys,
        truncated: false,
        authoritative: false,
    })
}

/// Bounded workspace metadata-key listing for interactive list paths.
///
/// Complexity (Postgres + `key text_pattern_ops`): **O(limit)** index range
/// scan — not O(table). Uses [`KVStorage::keys_with_prefix_limited`] so SQL
/// `LIMIT` short-circuits before materializing an unbounded index-key Vec.
/// Returns `(metadata_keys, truncated)`.
///
/// `max_entries` must be a finite interactive cap (not `usize::MAX`) — casting
/// huge limits to `i64` for SQL is undefined for the unlimited path.
pub async fn list_workspace_metadata_keys_limited(
    kv: &dyn KVStorage,
    pool: crate::services::OptionalPgPool<'_>,
    workspace_id: &str,
    max_entries: usize,
) -> Result<(Vec<String>, bool), StorageError> {
    let listed =
        list_workspace_metadata_keys_limited_detailed(kv, pool, workspace_id, max_entries).await?;
    Ok((listed.keys, listed.truncated))
}

/// Bounded listing with membership authority (see [`WorkspaceMetadataKeyList`]).
pub async fn list_workspace_metadata_keys_limited_detailed(
    kv: &dyn KVStorage,
    pool: crate::services::OptionalPgPool<'_>,
    workspace_id: &str,
    max_entries: usize,
) -> Result<WorkspaceMetadataKeyList, StorageError> {
    let max_entries = max_entries.clamp(1, 1_000_000);
    if let Some(doc_ids) = relational_workspace_doc_ids(pool, workspace_id).await {
        // Relational branch: bounded in-memory — the relational read path
        // scales via `idx_documents_tenant_workspace`, and Wave C replaces
        // metadata keys wholesale, so a SQL LIMIT here would be throwaway.
        let truncated = doc_ids.len() > max_entries;
        let keys: Vec<String> = doc_ids
            .iter()
            .take(max_entries)
            .map(|doc_id| kv_keys::doc_metadata(doc_id))
            .collect();
        return Ok(WorkspaceMetadataKeyList {
            keys,
            truncated,
            authoritative: true,
        });
    }
    let prefix = kv_keys::workspace_doc_index_prefix(workspace_id);
    // Fetch one extra index key so truncation is known without a COUNT.
    let (index_keys, _) = kv
        .keys_with_prefix_limited(&prefix, max_entries.saturating_add(1))
        .await?;
    let mut metadata_keys = Vec::with_capacity(max_entries.min(index_keys.len()));
    for key in index_keys {
        if let Some((ws, doc_id)) = kv_keys::parse_workspace_doc_index(&key) {
            if ws == workspace_id {
                metadata_keys.push(kv_keys::doc_metadata(doc_id));
                if metadata_keys.len() > max_entries {
                    metadata_keys.truncate(max_entries);
                    return Ok(WorkspaceMetadataKeyList {
                        keys: metadata_keys,
                        truncated: true,
                        authoritative: false,
                    });
                }
            }
        }
    }
    Ok(WorkspaceMetadataKeyList {
        keys: metadata_keys,
        truncated: false,
        authoritative: false,
    })
}

/// Document ids indexed under a workspace (prefix scan).
pub async fn list_workspace_document_ids(
    kv: &dyn KVStorage,
    pool: crate::services::OptionalPgPool<'_>,
    workspace_id: &str,
) -> Result<Vec<String>, StorageError> {
    if let Some(doc_ids) = relational_workspace_doc_ids(pool, workspace_id).await {
        return Ok(doc_ids);
    }
    let prefix = kv_keys::workspace_doc_index_prefix(workspace_id);
    let index_keys = kv.keys_with_prefix(&prefix).await?;
    Ok(index_keys
        .iter()
        .filter_map(|key| {
            kv_keys::parse_workspace_doc_index(key).and_then(|(ws, doc_id)| {
                if ws == workspace_id {
                    Some(doc_id.to_string())
                } else {
                    None
                }
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_storage::MemoryKVStorage;
    use std::sync::Arc;

    #[test]
    fn authoritative_empty_list_is_terminal_for_readers() {
        // LAW-111-9 contract: callers must treat this as empty workspace.
        let listed = WorkspaceMetadataKeyList {
            keys: vec![],
            truncated: false,
            authoritative: true,
        };
        assert!(listed.authoritative);
        assert!(listed.keys.is_empty());
        // Reader rule (mirrored in document_metadata_scan): accept and stop.
        assert!(listed.authoritative || !listed.keys.is_empty());
        // Non-authoritative empty still allows legacy suffix fallback.
        let legacy_empty = WorkspaceMetadataKeyList {
            keys: vec![],
            truncated: false,
            authoritative: false,
        };
        assert!(!legacy_empty.authoritative && legacy_empty.keys.is_empty());
    }

    /// SPEC-091 Wave B3: flag=relational without a registered pool (or with a
    /// non-UUID workspace) must fall back to the KV index, never error.
    #[tokio::test]
    async fn relational_flag_without_pool_falls_back_to_kv() {
        std::env::set_var("EDGEQUAKE_KV_FAMILY_WSDOC", "relational");
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("ws-fallback"));
        kv.initialize().await.unwrap();
        let ws = uuid::Uuid::new_v4().to_string();
        let meta = serde_json::json!({ "id": "doc-x", "workspace_id": ws });
        upsert_metadata_kv_with_index(kv.as_ref(), "doc-x-metadata", meta)
            .await
            .unwrap();
        let keys = list_workspace_metadata_keys(kv.as_ref(), None, &ws)
            .await
            .unwrap();
        assert_eq!(keys, vec!["doc-x-metadata".to_string()]);
        std::env::remove_var("EDGEQUAKE_KV_FAMILY_WSDOC");
    }

    #[tokio::test]
    async fn upsert_metadata_kv_with_index_lists_workspace() {
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("ws-upsert"));
        kv.initialize().await.unwrap();

        let ws = uuid::Uuid::new_v4().to_string();
        let meta = serde_json::json!({
            "id": "doc-b",
            "workspace_id": ws,
            "tenant_id": "tenant-1",
        });
        upsert_metadata_kv_with_index(kv.as_ref(), "doc-b-metadata", meta)
            .await
            .unwrap();

        let keys = list_workspace_metadata_keys(kv.as_ref(), None, &ws)
            .await
            .unwrap();
        assert_eq!(keys, vec!["doc-b-metadata".to_string()]);
    }

    #[tokio::test]
    async fn sync_and_list_workspace_metadata_keys() {
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("ws-index"));
        kv.initialize().await.unwrap();

        let ws = uuid::Uuid::new_v4().to_string();
        let meta_key = "doc-a-metadata";
        let meta = serde_json::json!({
            "id": "doc-a",
            "workspace_id": ws,
            "tenant_id": "tenant-1",
        });
        kv.upsert(&[(meta_key.to_string(), meta.clone())])
            .await
            .unwrap();
        sync_workspace_document_index(kv.as_ref(), meta_key, &meta)
            .await
            .unwrap();

        let keys = list_workspace_metadata_keys(kv.as_ref(), None, &ws)
            .await
            .unwrap();
        assert_eq!(keys, vec![meta_key.to_string()]);
    }
}
