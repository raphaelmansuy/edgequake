//! Staging KV promote/rollback for admission saga (SPEC-026 Phase 2 P-11).

use std::sync::Arc;

use edgequake_storage::kv_keys;
use edgequake_storage::traits::KVStorage;
use serde_json::Value;

/// Promote staging keys to final document keys after successful processing.
pub async fn promote_staging_to_final(
    kv: &Arc<dyn KVStorage>,
    document_id: &str,
    workspace_id: &str,
    content_hash: &str,
) -> Result<(), String> {
    let staging_meta = kv_keys::staging_doc_metadata(document_id);
    let staging_content = kv_keys::staging_doc_content(document_id);
    let staging_hash = kv_keys::staging_workspace_hash(workspace_id, content_hash);

    let final_meta = kv_keys::doc_metadata(document_id);
    let final_content = kv_keys::doc_content(document_id);
    let final_hash = super::ContentHasher::workspace_hash_key(workspace_id, content_hash);

    let mut batch: Vec<(String, Value)> = Vec::new();

    if let Some(v) = kv.get_by_id(&staging_meta).await.map_err(kv_err)? {
        batch.push((final_meta.clone(), v));
    }
    if let Some(v) = kv.get_by_id(&staging_content).await.map_err(kv_err)? {
        batch.push((final_content, v));
    }
    if let Some(v) = kv.get_by_id(&staging_hash).await.map_err(kv_err)? {
        batch.push((final_hash, v));
    }

    if !batch.is_empty() {
        kv.upsert(&batch).await.map_err(kv_err)?;
        if let Some(meta) = kv.get_by_id(&final_meta).await.map_err(kv_err)? {
            let _ =
                crate::services::sync_after_metadata_upsert(kv.as_ref(), &final_meta, &meta).await;
        }
    }

    rollback_staging(kv, document_id, workspace_id, content_hash).await
}

/// Delete staging keys on failure or after promote.
pub async fn rollback_staging(
    kv: &Arc<dyn KVStorage>,
    document_id: &str,
    workspace_id: &str,
    content_hash: &str,
) -> Result<(), String> {
    let keys = [
        kv_keys::staging_doc_metadata(document_id),
        kv_keys::staging_doc_content(document_id),
        kv_keys::staging_workspace_hash(workspace_id, content_hash),
    ];
    for key in keys {
        let _ = kv.delete(&[key]).await;
    }
    Ok(())
}

fn kv_err(e: impl std::fmt::Display) -> String {
    format!("KV error: {e}")
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use edgequake_storage::adapters::memory::MemoryKVStorage;
    use edgequake_storage::traits::KVStorage;
    use serde_json::json;

    use super::*;

    #[tokio::test]
    async fn promote_copies_staging_to_final() {
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("test"));
        let doc_id = "doc-1";
        let ws = "ws-1";
        let hash = "abc";

        kv.upsert(&[
            (
                kv_keys::staging_doc_metadata(doc_id),
                json!({"status": "pending"}),
            ),
            (
                kv_keys::staging_doc_content(doc_id),
                json!({"content": "hello"}),
            ),
            (kv_keys::staging_workspace_hash(ws, hash), json!(doc_id)),
        ])
        .await
        .unwrap();

        promote_staging_to_final(&kv, doc_id, ws, hash)
            .await
            .unwrap();

        assert!(kv
            .get_by_id(&kv_keys::doc_metadata(doc_id))
            .await
            .unwrap()
            .is_some());
        assert!(kv
            .get_by_id(&kv_keys::doc_content(doc_id))
            .await
            .unwrap()
            .is_some());
        assert!(kv
            .get_by_id(&kv_keys::staging_doc_metadata(doc_id))
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn rollback_deletes_staging_on_failure() {
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("test"));
        let doc_id = "doc-2";
        kv.upsert(&[(
            kv_keys::staging_doc_content(doc_id),
            json!({"content": "x"}),
        )])
        .await
        .unwrap();

        rollback_staging(&kv, doc_id, "ws", "h").await.unwrap();
        assert!(kv
            .get_by_id(&kv_keys::staging_doc_content(doc_id))
            .await
            .unwrap()
            .is_none());
        assert!(kv
            .get_by_id(&kv_keys::doc_content(doc_id))
            .await
            .unwrap()
            .is_none());
    }
}
