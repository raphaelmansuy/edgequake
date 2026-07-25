//! Resolve document text for worker tasks — staging-first (SPEC-026 P-11).

use std::sync::Arc;

use edgequake_storage::kv_keys;
use edgequake_storage::traits::KVStorage;
use serde_json::json;

/// Resolve document text for a TextInsert worker task.
///
/// - Non-empty inline `inline_text` wins (legacy PDF/reprocess paths).
/// - Otherwise loads staging content, then final `{document_id}-content` from KV.
///
/// IMP-075-08: staging + final content in one `get_by_ids_ordered` (O(1) RT).
pub async fn resolve_text_insert_content(
    kv_storage: &Arc<dyn KVStorage>,
    document_id: &str,
    inline_text: &str,
) -> Result<String, String> {
    if !inline_text.is_empty() {
        return Ok(inline_text.to_string());
    }

    let keys = [
        kv_keys::staging_doc_content(document_id),
        kv_keys::doc_content(document_id),
    ];
    let vals = kv_storage
        .get_by_ids_ordered(&keys)
        .await
        .map_err(|e| format!("KV batch content read failed for {document_id}: {e}"))?;

    // Staging-first order is preserved by the keys array above.
    for raw in vals.into_iter().flatten() {
        if let Some(text) = raw.get("content").and_then(|v| v.as_str()) {
            return Ok(text.to_string());
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

/// Staging + final metadata loaded in **one** RT (IMP-075-10 SSOT).
///
/// Callers that only need the preferred row use [`StagingFinalMeta::preferred`];
/// merge-progress uses both rows for the terminal-final race guard.
#[derive(Debug, Clone)]
pub struct StagingFinalMeta {
    pub staging_key: String,
    pub final_key: String,
    pub staging: Option<serde_json::Value>,
    pub final_meta: Option<serde_json::Value>,
}

impl StagingFinalMeta {
    /// Staging-first preferred key + value, if either exists.
    pub fn preferred(&self) -> Option<(String, serde_json::Value)> {
        if let Some(m) = &self.staging {
            return Some((self.staging_key.clone(), m.clone()));
        }
        self.final_meta
            .as_ref()
            .map(|m| (self.final_key.clone(), m.clone()))
    }

    /// Preferred key, or final key when neither value exists (create path).
    pub fn preferred_key_or_final(&self) -> String {
        self.preferred()
            .map(|(k, _)| k)
            .unwrap_or_else(|| self.final_key.clone())
    }
}

/// Load staging and final document metadata in **one** RT.
pub async fn load_staging_and_final_metadata(
    kv: &dyn KVStorage,
    document_id: &str,
) -> Result<StagingFinalMeta, String> {
    let staging_key = kv_keys::staging_doc_metadata(document_id);
    let final_key = kv_keys::doc_metadata(document_id);
    let keys = [staging_key.clone(), final_key.clone()];
    let vals = kv
        .get_by_ids_ordered(&keys)
        .await
        .map_err(|e| format!("KV batch metadata read failed for {document_id}: {e}"))?;
    Ok(StagingFinalMeta {
        staging_key,
        final_key,
        staging: vals.first().and_then(|v| v.clone()),
        final_meta: vals.get(1).and_then(|v| v.clone()),
    })
}

/// Resolve document metadata **key only** — staging-first for in-flight documents.
///
/// # Prefer
/// [`load_staging_first_metadata`] / [`load_staging_and_final_metadata`] when the
/// caller also needs the value (avoids a second RT). As of IMP-075-11 this helper
/// has **no production dual-read call sites**; keep it key-only or migrate to SSOT.
pub async fn resolve_document_metadata_key(document_id: &str, kv: &Arc<dyn KVStorage>) -> String {
    match load_staging_and_final_metadata(kv.as_ref(), document_id).await {
        Ok(pair) => pair.preferred_key_or_final(),
        _ => kv_keys::doc_metadata(document_id),
    }
}

/// Load staging-first document metadata in **one** RT (IMP-075-09/10).
///
/// Returns `(key, value)` preferring `staging:…-metadata` when present, else
/// final `{id}-metadata`. `None` when neither key exists.
pub async fn load_staging_first_metadata(
    kv: &dyn KVStorage,
    document_id: &str,
) -> Result<Option<(String, serde_json::Value)>, String> {
    Ok(load_staging_and_final_metadata(kv, document_id)
        .await?
        .preferred())
}

/// Patch document metadata at the staging-first key (no-op when absent).
///
/// Reliability: refuses to mutate documents already in a terminal status so
/// fire-and-forget progress callbacks cannot clobber `completed`/`failed`.
pub async fn patch_document_metadata(
    kv: &Arc<dyn KVStorage>,
    document_id: &str,
    mutator: impl FnOnce(&mut serde_json::Map<String, serde_json::Value>),
) -> Result<(), String> {
    let Some((key, existing)) = load_staging_first_metadata(kv.as_ref(), document_id).await? else {
        return Ok(());
    };
    let Some(mut obj) = existing.as_object().cloned() else {
        return Ok(());
    };
    if obj
        .get("status")
        .and_then(|v| v.as_str())
        .is_some_and(crate::document_metadata::is_terminal_document_status)
    {
        tracing::debug!(
            document_id = %document_id,
            status = obj.get("status").and_then(|v| v.as_str()).unwrap_or(""),
            "Skipping metadata progress patch — document already terminal"
        );
        return Ok(());
    }
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
    async fn patch_skips_terminal_document() {
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("test"));
        let doc_id = "doc-terminal-patch";
        kv.upsert(&[(
            kv_keys::doc_metadata(doc_id),
            json!({
                "id": doc_id,
                "status": "completed",
                "current_stage": "completed",
                "stage_message": "done"
            }),
        )])
        .await
        .unwrap();

        patch_document_metadata(&kv, doc_id, |updated| {
            updated.insert("status".to_string(), json!("indexing"));
            updated.insert("current_stage".to_string(), json!("storing"));
        })
        .await
        .unwrap();

        let meta = kv
            .get_by_id(&kv_keys::doc_metadata(doc_id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(meta["status"], "completed");
        assert_eq!(meta["current_stage"], "completed");
    }

    #[tokio::test]
    async fn load_staging_and_final_prefers_staging() {
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("test-pair"));
        let doc_id = "doc-pair";
        kv.upsert(&[
            (
                kv_keys::staging_doc_metadata(doc_id),
                json!({ "id": doc_id, "status": "processing" }),
            ),
            (
                kv_keys::doc_metadata(doc_id),
                json!({ "id": doc_id, "status": "completed" }),
            ),
        ])
        .await
        .unwrap();

        let pair = load_staging_and_final_metadata(kv.as_ref(), doc_id)
            .await
            .unwrap();
        assert!(pair.staging.is_some());
        assert!(pair.final_meta.is_some());
        let (key, val) = pair.preferred().unwrap();
        assert_eq!(key, kv_keys::staging_doc_metadata(doc_id));
        assert_eq!(val["status"], "processing");
    }

    #[tokio::test]
    async fn load_staging_and_final_falls_back_to_final() {
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("test-final-only"));
        let doc_id = "doc-final-only";
        kv.upsert(&[(
            kv_keys::doc_metadata(doc_id),
            json!({ "id": doc_id, "status": "completed" }),
        )])
        .await
        .unwrap();

        let pair = load_staging_and_final_metadata(kv.as_ref(), doc_id)
            .await
            .unwrap();
        assert!(pair.staging.is_none());
        let (key, val) = pair.preferred().unwrap();
        assert_eq!(key, kv_keys::doc_metadata(doc_id));
        assert_eq!(val["status"], "completed");
        assert_eq!(pair.preferred_key_or_final(), kv_keys::doc_metadata(doc_id));
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
