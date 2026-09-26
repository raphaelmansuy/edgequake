//! Graph node source lineage properties (SPEC-045 / BR0007 / SPEC-047 021).
//!
//! SSOT: pipeline writes both `source_chunk_ids` (citation tracking) and
//! `source_ids` (analytics / reconcile / cascade-delete predicates), plus
//! `source_document_ids[]` union (document-scope query — 021 L-A1).

use std::collections::{HashMap, HashSet};

use serde_json::Value;

/// `EDGEQUAKE_CITATION_REQUIRE` — default **on** (SPEC-091 RM2 / LAW-RM7).
pub const CITATION_REQUIRE_ENV: &str = "EDGEQUAKE_CITATION_REQUIRE";

/// Whether newly extracted edges/nodes must carry non-empty `source_chunk_ids`.
pub fn citation_require_enabled() -> bool {
    !matches!(
        std::env::var(CITATION_REQUIRE_ENV)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "off" | "false" | "0" | "no"
    )
}

/// Fail-closed citation check (RM-AC-06).
pub fn require_citation(chunk_ids: &[String]) -> Result<(), String> {
    if !citation_require_enabled() {
        return Ok(());
    }
    if chunk_ids.iter().any(|id| !id.trim().is_empty()) {
        return Ok(());
    }
    Err("SPEC-091 RM2: source_chunk_ids required (EDGEQUAKE_CITATION_REQUIRE)".into())
}

/// Mirror chunk lineage into graph node properties for read-path compatibility.
///
/// Delegates to [`edgequake_storage::insert_chunk_lineage_properties`] so the
/// merger and SPEC-149 projection share one mirror rule (`source_ids` ==
/// `source_chunk_ids`, plus derived `source_document_ids`).
pub fn insert_chunk_lineage_properties(
    properties: &mut HashMap<String, Value>,
    chunk_ids: &[String],
) {
    edgequake_storage::insert_chunk_lineage_properties(properties, chunk_ids);
}

/// Derive document id from EdgeQuake chunk id convention (`{doc}-chunk-N`).
pub fn document_id_from_chunk_id(chunk_id: &str) -> Option<String> {
    if let Some(suffix_idx) = chunk_id.rfind("-chunk-") {
        if suffix_idx > 0 {
            return Some(chunk_id[..suffix_idx].to_string());
        }
    }
    None
}

/// Unique document IDs implied by chunk ids (order-preserving).
pub fn document_ids_from_chunk_ids(chunk_ids: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for cid in chunk_ids {
        if let Some(doc) = document_id_from_chunk_id(cid) {
            if seen.insert(doc.clone()) {
                out.push(doc);
            }
        }
    }
    out
}

/// Resolve incoming document parents: singular + docs derived from chunk ids.
pub fn resolve_incoming_document_ids(singular: Option<&str>, chunk_ids: &[String]) -> Vec<String> {
    let mut docs = Vec::new();
    let mut seen = HashSet::new();
    if let Some(id) = singular {
        if !id.is_empty() && seen.insert(id.to_string()) {
            docs.push(id.to_string());
        }
    }
    for doc in document_ids_from_chunk_ids(chunk_ids) {
        if seen.insert(doc.clone()) {
            docs.push(doc);
        }
    }
    docs
}

/// Union of document ids (order: existing then new).
pub fn merge_document_ids(existing: &[String], incoming: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for id in existing.iter().chain(incoming.iter()) {
        if !id.is_empty() && seen.insert(id.clone()) {
            out.push(id.clone());
        }
    }
    out
}

/// Read `source_document_ids` from graph properties, with singular fallback.
pub fn source_document_ids_from_properties(props: &HashMap<String, Value>) -> Vec<String> {
    let plural: Vec<String> = props
        .get("source_document_ids")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    if !plural.is_empty() {
        return plural;
    }
    props
        .get("source_document_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| vec![s.to_string()])
        .unwrap_or_default()
}

/// Write document lineage: always set plural union; set singular if unset.
pub fn insert_document_lineage_properties(
    properties: &mut HashMap<String, Value>,
    doc_ids: &[String],
) {
    if doc_ids.is_empty() {
        return;
    }
    properties.insert(
        "source_document_ids".to_string(),
        Value::Array(doc_ids.iter().cloned().map(Value::String).collect()),
    );
    if !properties.contains_key("source_document_id") {
        if let Some(first) = doc_ids.first() {
            properties.insert(
                "source_document_id".to_string(),
                Value::String(first.clone()),
            );
        }
    }
}

/// Merge + write document lineage from existing props + incoming singular/chunks.
pub fn merge_and_insert_document_lineage(
    properties: &mut HashMap<String, Value>,
    singular: Option<&str>,
    chunk_ids: &[String],
) {
    let existing = source_document_ids_from_properties(properties);
    let incoming = resolve_incoming_document_ids(singular, chunk_ids);
    let merged = merge_document_ids(&existing, &incoming);
    insert_document_lineage_properties(properties, &merged);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirrors_chunk_ids_to_source_ids() {
        let mut props = HashMap::new();
        insert_chunk_lineage_properties(&mut props, &["doc-a-chunk-0".into()]);
        assert_eq!(
            props.get("source_chunk_ids"),
            props.get("source_ids"),
            "source_ids must mirror source_chunk_ids"
        );
    }

    #[test]
    fn cross_doc_union_on_merge() {
        let mut props = HashMap::new();
        insert_document_lineage_properties(&mut props, &["doc-a".into()]);
        merge_and_insert_document_lineage(&mut props, Some("doc-b"), &["doc-b-chunk-0".into()]);
        let docs = source_document_ids_from_properties(&props);
        assert_eq!(docs.len(), 2);
        assert!(docs.contains(&"doc-a".to_string()));
        assert!(docs.contains(&"doc-b".to_string()));
    }

    #[test]
    fn derive_docs_from_chunk_ids_alone() {
        let incoming =
            resolve_incoming_document_ids(None, &["doc-x-chunk-1".into(), "doc-y-chunk-0".into()]);
        assert_eq!(incoming, vec!["doc-x".to_string(), "doc-y".to_string()]);
    }

    #[test]
    fn contract_spec091_citation_required_rejects_empty() {
        std::env::remove_var(CITATION_REQUIRE_ENV);
        assert!(require_citation(&[]).is_err());
        assert!(require_citation(&["".into()]).is_err());
        assert!(require_citation(&["doc-a-chunk-0".into()]).is_ok());
        std::env::set_var(CITATION_REQUIRE_ENV, "off");
        assert!(require_citation(&[]).is_ok());
        std::env::remove_var(CITATION_REQUIRE_ENV);
    }
}
