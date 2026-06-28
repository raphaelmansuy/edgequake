//! Resolve document text for worker tasks — staging-first (SPEC-026 P-11).

use std::sync::Arc;

use edgequake_storage::kv_keys;
use edgequake_storage::traits::KVStorage;
use serde_json::json;

/// Resolve document text for a TextInsert worker task.
///
/// - Non-empty inline `inline_text` wins (legacy PDF/reprocess paths).
/// - Otherwise loads staging content, then final `{document_id}-content` from KV.
pub async fn resolve_text_insert_content(
    kv_storage: &Arc<dyn KVStorage>,
    document_id: &str,
    inline_text: &str,
) -> Result<String, String> {
    if !inline_text.is_empty() {
        return Ok(inline_text.to_string());
    }

    for key in [
        kv_keys::staging_doc_content(document_id),
        kv_keys::doc_content(document_id),
    ] {
        if let Some(raw) = kv_storage
            .get_by_id(&key)
            .await
            .map_err(|e| format!("KV read failed for {key}: {e}"))?
        {
            if let Some(text) = raw.get("content").and_then(|v| v.as_str()) {
                return Ok(text.to_string());
            }
        }
    }

    Err(format!("Missing KV content for document {document_id}"))
}

/// Persist markdown body to final (and staging, if present) content keys.
pub async fn persist_document_content(
    kv_storage: &Arc<dyn KVStorage>,
    document_id: &str,
    markdown: &str,
) -> Result<(), String> {
    let payload = json!({ "content": markdown });
    let content_key = kv_keys::doc_content(document_id);
    kv_storage
        .upsert(&[(content_key.clone(), payload.clone())])
        .await
        .map_err(|e| format!("KV write failed for {content_key}: {e}"))?;

    let staging_key = kv_keys::staging_doc_content(document_id);
    if kv_storage
        .get_by_id(&staging_key)
        .await
        .map_err(|e| format!("KV read failed for {staging_key}: {e}"))?
        .is_some()
    {
        kv_storage
            .upsert(&[(staging_key, payload)])
            .await
            .map_err(|e| format!("KV write failed for staging content: {e}"))?;
    }
    Ok(())
}

/// Resolve document metadata — staging-first for in-flight documents.
pub async fn resolve_document_metadata_key(document_id: &str, kv: &Arc<dyn KVStorage>) -> String {
    let staging = kv_keys::staging_doc_metadata(document_id);
    if kv.get_by_id(&staging).await.ok().flatten().is_some() {
        staging
    } else {
        kv_keys::doc_metadata(document_id)
    }
}

/// Patch document metadata at the staging-first key (no-op when absent).
pub async fn patch_document_metadata(
    kv: &Arc<dyn KVStorage>,
    document_id: &str,
    mutator: impl FnOnce(&mut serde_json::Map<String, serde_json::Value>),
) -> Result<(), String> {
    let key = resolve_document_metadata_key(document_id, kv).await;
    let Some(existing) = kv
        .get_by_id(&key)
        .await
        .map_err(|e| format!("KV read failed for {key}: {e}"))?
    else {
        return Ok(());
    };
    let Some(mut obj) = existing.as_object().cloned() else {
        return Ok(());
    };
    mutator(&mut obj);
    let updated = serde_json::Value::Object(obj);
    let write_key = key.clone();
    crate::services::upsert_metadata_kv_with_index(kv.as_ref(), &write_key, updated)
        .await
        .map_err(|e| format!("KV write failed for {key}: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use edgequake_storage::adapters::memory::MemoryKVStorage;
    use edgequake_storage::traits::KVStorage;
    use serde_json::json;

    use super::*;

    #[tokio::test]
    async fn inline_text_bypasses_kv() {
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("test"));
        let text = resolve_text_insert_content(&kv, "doc-1", "inline body")
            .await
            .unwrap();
        assert_eq!(text, "inline body");
    }

    #[tokio::test]
    async fn reads_staging_before_final() {
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("test"));
        let doc_id = "doc-staging";
        kv.upsert(&[(
            kv_keys::staging_doc_content(doc_id),
            json!({ "content": "staging body" }),
        )])
        .await
        .unwrap();
        kv.upsert(&[(
            kv_keys::doc_content(doc_id),
            json!({ "content": "final body" }),
        )])
        .await
        .unwrap();

        let text = resolve_text_insert_content(&kv, doc_id, "").await.unwrap();
        assert_eq!(text, "staging body");
    }

    #[tokio::test]
    async fn patch_updates_staging_metadata() {
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("test"));
        let doc_id = "doc-staging-patch";
        kv.upsert(&[(
            kv_keys::staging_doc_metadata(doc_id),
            json!({ "id": doc_id, "status": "pending", "current_stage": "uploading" }),
        )])
        .await
        .unwrap();

        patch_document_metadata(&kv, doc_id, |updated| {
            updated.insert("status".to_string(), json!("chunking"));
            updated.insert("current_stage".to_string(), json!("chunking"));
        })
        .await
        .unwrap();

        let staging = kv
            .get_by_id(&kv_keys::staging_doc_metadata(doc_id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(staging["status"], "chunking");
        assert!(kv
            .get_by_id(&kv_keys::doc_metadata(doc_id))
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn missing_kv_returns_error() {
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("test"));
        let err = resolve_text_insert_content(&kv, "missing", "")
            .await
            .unwrap_err();
        assert!(err.contains("Missing KV content"));
    }
}
