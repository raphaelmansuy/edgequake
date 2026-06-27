//! Multimodal process_options persistence in document KV metadata (DRY SSOT).

use serde_json::Value;

/// KV metadata field for LightRAG `process_options` string (e.g. `"i"`, `"ite"`).
pub const METADATA_FIELD: &str = "multimodal_process_options";

/// Read stored process_options from document metadata JSON.
pub fn resolve_process_options_from_metadata(metadata: &Value) -> Option<String> {
    metadata
        .as_object()
        .and_then(|obj| obj.get(METADATA_FIELD))
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .filter(|s| !s.is_empty())
}

/// Patch metadata object with process_options when present on the task.
pub fn apply_process_options_to_metadata(
    obj: &mut serde_json::Map<String, Value>,
    process_options: Option<&str>,
) {
    if let Some(opts) = process_options.filter(|s| !s.is_empty()) {
        obj.insert(METADATA_FIELD.to_string(), Value::String(opts.to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn resolves_from_metadata_field() {
        let meta = json!({ "multimodal_process_options": "ite" });
        assert_eq!(
            resolve_process_options_from_metadata(&meta).as_deref(),
            Some("ite")
        );
    }

    #[test]
    fn missing_field_returns_none() {
        assert!(resolve_process_options_from_metadata(&json!({})).is_none());
    }
}
