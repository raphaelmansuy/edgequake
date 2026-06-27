//! Virtual sidecar manifest persistence in KV.

use edgequake_storage::traits::KVStorage;
use serde_json::Value;

use super::item_record::MultimodalSummary;
use super::manifest::MultimodalManifest;
use super::metadata::METADATA_FIELD;

/// KV key for document multimodal manifest JSON.
pub fn manifest_key(document_id: &str) -> String {
    format!("{document_id}-multimodal-manifest")
}

/// Persist manifest blob to KV.
pub async fn persist_manifest(
    kv: &dyn KVStorage,
    document_id: &str,
    manifest: &MultimodalManifest,
) -> Result<(), String> {
    let key = manifest_key(document_id);
    let value = serde_json::to_value(manifest).map_err(|e| e.to_string())?;
    kv.upsert(&[(key, value)]).await.map_err(|e| e.to_string())
}

/// Load manifest from KV (virtual sidecar).
pub async fn load_manifest(kv: &dyn KVStorage, document_id: &str) -> Option<MultimodalManifest> {
    let key = manifest_key(document_id);
    let value = kv.get_by_id(&key).await.ok()??;
    serde_json::from_value(value).ok()
}
pub fn metadata_multimodal_patch(
    summary: &MultimodalSummary,
    process_options: Option<&str>,
) -> Value {
    let mut obj = serde_json::Map::new();
    obj.insert(
        "multimodal_summary".into(),
        serde_json::to_value(summary).unwrap_or(Value::Null),
    );
    obj.insert(
        "multimodal_manifest_version".into(),
        Value::Number(MultimodalManifest::CURRENT_VERSION.into()),
    );
    obj.insert(
        "multimodal_analyzed_at".into(),
        Value::String(chrono::Utc::now().to_rfc3339()),
    );
    if let Some(opts) = process_options.filter(|s| !s.is_empty()) {
        obj.insert(METADATA_FIELD.into(), Value::String(opts.to_string()));
    }
    Value::Object(obj)
}
