//! Virtual sidecar manifest persistence in KV.

use edgequake_storage::contracts::CheckpointArtifactStore;

use edgequake_storage::traits::KVStorage;
use serde_json::Value;

use super::item_record::MultimodalSummary;
use super::manifest::MultimodalManifest;
use super::metadata::METADATA_FIELD;

/// KV key for document multimodal manifest JSON.
pub fn manifest_key(document_id: &str) -> String {
    format!("{document_id}-multimodal-manifest")
}

/// Persist manifest blob to KV (+ typed artifact dual-write, SPEC-091 Wave B5).
pub async fn persist_manifest(
    kv: &dyn KVStorage,
    store: Option<&dyn CheckpointArtifactStore>,
    document_id: &str,
    manifest: &MultimodalManifest,
) -> Result<(), String> {
    let key = manifest_key(document_id);
    let value = serde_json::to_value(manifest).map_err(|e| e.to_string())?;
    kv.upsert(&[(key, value.clone())])
        .await
        .map_err(|e| e.to_string())?;
    crate::services::relational_sidecar_store::typed_artifact_put(
        store,
        document_id,
        crate::services::relational_sidecar_store::ARTIFACT_KIND_MM_MANIFEST,
        &value,
    )
    .await;
    Ok(())
}

/// Load manifest (typed-first when the artifact family is cut over; KV fallback).
pub async fn load_manifest(
    kv: &dyn KVStorage,
    store: Option<&dyn CheckpointArtifactStore>,
    document_id: &str,
) -> Option<MultimodalManifest> {
    let value = if crate::services::relational_sidecar_store::artifacts_prefer_relational() {
        match crate::services::relational_sidecar_store::typed_artifact_get(
            store,
            document_id,
            crate::services::relational_sidecar_store::ARTIFACT_KIND_MM_MANIFEST,
        )
        .await
        {
            Some(v) => Some(v),
            None => kv.get_by_id(&manifest_key(document_id)).await.ok()?,
        }
    } else {
        kv.get_by_id(&manifest_key(document_id)).await.ok()?
    };
    serde_json::from_value(value?).ok()
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
