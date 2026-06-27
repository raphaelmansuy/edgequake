//! Resolve document text for worker tasks — staging-first (SPEC-026 P-11).

use std::sync::Arc;

use edgequake_storage::kv_keys;
use edgequake_storage::traits::KVStorage;

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

/// Resolve document metadata — staging-first for in-flight documents.
pub async fn resolve_document_metadata_key(document_id: &str, kv: &Arc<dyn KVStorage>) -> String {
    let staging = kv_keys::staging_doc_metadata(document_id);
    if kv.get_by_id(&staging).await.ok().flatten().is_some() {
        staging
    } else {
        kv_keys::doc_metadata(document_id)
    }
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
    async fn missing_kv_returns_error() {
        let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("test"));
        let err = resolve_text_insert_content(&kv, "missing", "")
            .await
            .unwrap_err();
        assert!(err.contains("Missing KV content"));
    }
}
