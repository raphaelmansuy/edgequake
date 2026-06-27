//! Chunk content hydration from KV (SPEC-024 2.5 SSOT).

use std::collections::HashMap;

use serde_json::Value;

use crate::error::Result;
use crate::traits::KVStorage;

/// Extract chunk text from a KV JSON value.
pub fn content_from_kv_value(value: &Value) -> Option<String> {
    value
        .get("content")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .filter(|s| !s.is_empty())
}

/// Resolve content for a chunk id: inline metadata (legacy) or KV lookup.
pub fn content_from_metadata_or_kv(metadata: &Value, kv_content: Option<&str>) -> String {
    metadata
        .get("content")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or_else(|| kv_content.map(str::to_string))
        .unwrap_or_default()
}

/// Batch-fetch chunk contents from KV by id.
pub async fn batch_fetch_chunk_contents(
    kv: &dyn KVStorage,
    chunk_ids: &[String],
) -> Result<HashMap<String, String>> {
    let mut out = HashMap::new();
    for id in chunk_ids {
        if let Some(value) = kv.get_by_id(id).await? {
            if let Some(content) = content_from_kv_value(&value) {
                out.insert(id.clone(), content);
            }
        }
    }
    Ok(out)
}
