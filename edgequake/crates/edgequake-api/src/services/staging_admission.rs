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

    // IMP-075-01: one RT for staging keys (not 3× get_by_id) — O(K log N) with K=3.
    let staging_keys = vec![
        staging_meta.clone(),
        staging_content.clone(),
        staging_hash.clone(),
    ];
    let staging_vals = kv.get_by_ids_ordered(&staging_keys).await.map_err(kv_err)?;
    let mut batch: Vec<(String, Value)> = Vec::new();
    if let Some(v) = staging_vals.first().and_then(|o| o.clone()) {
        batch.push((final_meta.clone(), v));
    }
    if let Some(v) = staging_vals.get(1).and_then(|o| o.clone()) {
        batch.push((final_content, v));
    }
    if let Some(v) = staging_vals.get(2).and_then(|o| o.clone()) {
        batch.push((final_hash, v));
    }

    if !batch.is_empty() {
        kv.upsert(&batch).await.map_err(kv_err)?;
        // Prefer value already in batch for final_meta (avoid extra RT when we just wrote it).
        let meta_opt = batch
            .iter()
            .find(|(k, _)| k == &final_meta)
            .map(|(_, v)| v.clone());
        if let Some(meta) = meta_opt {
            let _ =
                crate::services::sync_after_metadata_upsert(kv.as_ref(), &final_meta, &meta).await;
        }
    }

    rollback_staging(kv, document_id, workspace_id, content_hash).await
}

/// Delete staging keys on dismiss / after promote (full wipe).
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

/// On pipeline failure: free duplicate-hash reservation + content, but keep
/// staging metadata so list/ActiveRuns show a failed shell (SPEC-086 UX).
/// Full wipe (`rollback_staging`) is reserved for dismiss / post-promote cleanup.
pub async fn release_staging_reservation(
    kv: &Arc<dyn KVStorage>,
    document_id: &str,
    workspace_id: &str,
    content_hash: &str,
) -> Result<(), String> {
    let keys = [
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

    #[tokio::test]
    async fn release_keeps_failed_metadata_clears_hash() {
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("test-release"));
        let doc_id = "doc-3";
        let ws = "ws";
        let hash = "h3";
        kv.upsert(&[
            (
                kv_keys::staging_doc_metadata(doc_id),
                json!({"status": "failed", "admission_staging": true}),
            ),
            (
                kv_keys::staging_doc_content(doc_id),
                json!({"content": "x"}),
            ),
            (kv_keys::staging_workspace_hash(ws, hash), json!(doc_id)),
        ])
        .await
        .unwrap();

        release_staging_reservation(&kv, doc_id, ws, hash)
            .await
            .unwrap();

        assert!(kv
            .get_by_id(&kv_keys::staging_doc_metadata(doc_id))
            .await
            .unwrap()
            .is_some());
        assert!(kv
            .get_by_id(&kv_keys::staging_doc_content(doc_id))
            .await
            .unwrap()
            .is_none());
        assert!(kv
            .get_by_id(&kv_keys::staging_workspace_hash(ws, hash))
            .await
            .unwrap()
            .is_none());
    }
}
