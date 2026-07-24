//! Document metadata KV integrity helpers (SPEC-045 SSOT).
//!
//! The metadata **key** (`{document_id}-metadata`) is authoritative over JSON
//! fields. Misaligned batch KV reads must never swap blobs across keys.

use serde_json::Value;

/// Suffix for document metadata keys (`{document_id}-metadata`).
pub const DOCUMENT_METADATA_SUFFIX: &str = "-metadata";

/// Extract document id from a metadata KV key.
#[inline]
pub fn document_id_from_metadata_key(key: &str) -> Option<String> {
    crate::kv_key_schema::kv_keys::parse_doc_metadata(key).map(str::to_string)
}

/// Canonical document id: KV key wins over JSON `id`.
#[inline]
pub fn canonical_document_id(metadata_key: &str, metadata: &Value) -> String {
    document_id_from_metadata_key(metadata_key).unwrap_or_else(|| {
        metadata
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or(metadata_key)
            .to_string()
    })
}

/// True when JSON `id` disagrees with the metadata KV key (corruption signal).
pub fn metadata_id_drift(metadata_key: &str, metadata: &Value) -> bool {
    let Some(key_id) = document_id_from_metadata_key(metadata_key) else {
        return false;
    };
    metadata
        .get("id")
        .and_then(|v| v.as_str())
        .is_some_and(|json_id| json_id != key_id)
}

/// Repair JSON fields to match the metadata KV key. Returns true if mutated.
pub fn repair_document_metadata_in_place(metadata_key: &str, metadata: &mut Value) -> bool {
    let Some(key_id) = document_id_from_metadata_key(metadata_key) else {
        return false;
    };
    let Some(obj) = metadata.as_object_mut() else {
        return false;
    };

    let mut changed = false;
    if obj
        .get("id")
        .and_then(|v| v.as_str())
        .is_some_and(|json_id| json_id != key_id)
    {
        obj.insert("id".to_string(), Value::String(key_id.clone()));
        changed = true;
    }

    if let Some(title) = obj.get("title").and_then(|v| v.as_str()) {
        if title.ends_with("-metadata") || title == metadata_key {
            obj.remove("title");
            changed = true;
        }
    }

    changed
}

/// Apply relational title overlay when KV title is missing or drift was detected.
pub fn overlay_relational_title(
    metadata_key: &str,
    metadata: &mut Value,
    relational_title: &str,
    had_id_drift: bool,
) -> bool {
    if relational_title.is_empty() {
        return false;
    }
    let Some(obj) = metadata.as_object_mut() else {
        return false;
    };
    let kv_title = obj.get("title").and_then(|v| v.as_str()).unwrap_or("");
    let should_overlay = had_id_drift
        || kv_title.is_empty()
        || kv_title.ends_with("-metadata")
        || kv_title == metadata_key;
    if should_overlay && kv_title != relational_title {
        obj.insert(
            "title".to_string(),
            Value::String(relational_title.to_string()),
        );
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn canonical_id_prefers_metadata_key() {
        let key = "real-doc-metadata";
        let meta = json!({ "id": "wrong-id", "title": "x.pdf" });
        assert_eq!(canonical_document_id(key, &meta), "real-doc");
    }

    #[test]
    fn canonical_id_strips_staging_prefix() {
        let key = "staging:real-doc-metadata";
        let meta = json!({ "id": "real-doc", "title": "x.md" });
        assert_eq!(canonical_document_id(key, &meta), "real-doc");
        assert!(!metadata_id_drift(key, &meta));
    }

    #[test]
    fn detects_id_drift() {
        let key = "doc-a-metadata";
        assert!(metadata_id_drift(
            key,
            &json!({ "id": "doc-b", "title": "t.pdf" })
        ));
        assert!(!metadata_id_drift(key, &json!({ "id": "doc-a" })));
    }

    #[test]
    fn repairs_id_from_key() {
        let key = "doc-a-metadata";
        let mut meta = json!({ "id": "doc-b", "title": "deep.pdf" });
        assert!(repair_document_metadata_in_place(key, &mut meta));
        assert_eq!(meta["id"], "doc-a");
        assert_eq!(meta["title"], "deep.pdf");
    }

    #[test]
    fn overlay_relational_title_after_drift() {
        let key = "doc-a-metadata";
        let mut meta = json!({ "id": "doc-a", "title": "deep_2604.pdf" });
        assert!(overlay_relational_title(
            key,
            &mut meta,
            "Chanel-meeting.pdf",
            true
        ));
        assert_eq!(meta["title"], "Chanel-meeting.pdf");
    }
}
