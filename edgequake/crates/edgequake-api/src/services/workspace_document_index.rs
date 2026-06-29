//! Workspace-scoped document KV index (SPEC-027 phase 8).
//!
//! Maintains `wsdoc:{workspace_id}:{document_id}` pointer keys so workspace
//! operations use prefix scans instead of global `-metadata` suffix scans.

use edgequake_storage::error::StorageError;
use edgequake_storage::kv_keys;
use edgequake_storage::traits::KVStorage;

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

/// List metadata keys for documents in a workspace via index prefix scan.
pub async fn list_workspace_metadata_keys(
    kv: &dyn KVStorage,
    workspace_id: &str,
) -> Result<Vec<String>, StorageError> {
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
    Ok(metadata_keys)
}

/// Document ids indexed under a workspace (prefix scan).
pub async fn list_workspace_document_ids(
    kv: &dyn KVStorage,
    workspace_id: &str,
) -> Result<Vec<String>, StorageError> {
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

        let keys = list_workspace_metadata_keys(kv.as_ref(), &ws)
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

        let keys = list_workspace_metadata_keys(kv.as_ref(), &ws)
            .await
            .unwrap();
        assert_eq!(keys, vec![meta_key.to_string()]);
    }
}
