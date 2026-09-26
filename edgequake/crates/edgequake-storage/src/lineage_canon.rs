//! Shared graph retain policy for projection apply.
//!
//! `graph_contributions` is the authority for whether another document still
//! names a node or edge. Property rewrites only subtract lineage; they never
//! decide deletion on their own.
//!
//! SPEC-149 lineage contract: writers must emit both `source_chunk_ids` (citation)
//! and a mirrored `source_ids` array (GIN discovery / cascade / document-scope).
//! [`canonicalize_source_lineage`] is the SSOT for that mirror.

use std::collections::{BTreeSet, HashMap, HashSet};

use serde_json::Value;
use uuid::Uuid;

use crate::kv_keys;

/// Indexed lineage array keys used by GIN discovery and cascade probes.
pub const INDEXED_LINEAGE_ARRAY_KEYS: [&str; 2] = ["source_ids", "source_chunk_ids"];

/// Ensure graph properties carry the full lineage contract:
/// - `source_chunk_ids` and `source_ids` both hold the sorted union of the two
/// - `source_document_ids` union of plural, singular, and docs parsed from chunks
///
/// Safe to call repeatedly (idempotent). Never drops a chunk id: legacy rows
/// that only carry `source_ids` keep them when merged with durable-path facts.
/// Callers that must *subtract* or *cap* lineage use [`set_chunk_lineage`].
pub fn canonicalize_source_lineage(properties: &mut HashMap<String, Value>) {
    if !properties.contains_key("source_chunk_ids") && !properties.contains_key("source_ids") {
        derive_document_lineage(properties, &BTreeSet::new());
        return;
    }
    let chunks = union_string_sets(
        string_set(properties.get("source_chunk_ids")),
        string_set(properties.get("source_ids")),
    );
    set_chunk_lineage(properties, &chunks);
}

/// Overwrite both lineage arrays with exactly `chunks`, then derive documents.
///
/// Authoritative write: stale `source_ids` / `source_chunk_ids` values are
/// discarded. Used by the merger (capped lists) and retain-on-delete (subtract).
pub fn set_chunk_lineage(properties: &mut HashMap<String, Value>, chunks: &BTreeSet<String>) {
    let chunk_json = json_strings(chunks);
    properties.insert("source_chunk_ids".into(), chunk_json.clone());
    properties.insert("source_ids".into(), chunk_json);
    derive_document_lineage(properties, chunks);
}

fn derive_document_lineage(properties: &mut HashMap<String, Value>, chunks: &BTreeSet<String>) {
    let mut documents = string_set(properties.get("source_document_ids"));
    if let Some(id) = scalar_string(properties.get("source_document_id")) {
        documents.insert(id);
    }
    for chunk in chunks {
        if let Some((doc_id, _)) = kv_keys::parse_doc_chunk(chunk) {
            if !doc_id.is_empty() {
                documents.insert(doc_id.to_string());
            }
        }
    }
    if !documents.is_empty() {
        properties.insert("source_document_ids".into(), json_strings(&documents));
        if !properties.contains_key("source_document_id") {
            if let Some(first) = documents.iter().next() {
                properties.insert("source_document_id".into(), Value::String(first.clone()));
            }
        }
    }
}

/// Insert chunk lineage arrays into properties (pipeline / merger SSOT).
///
/// `chunk_ids` is authoritative (the merger passes an already merged and
/// capped list), so both arrays are overwritten via [`set_chunk_lineage`].
pub fn insert_chunk_lineage_properties(
    properties: &mut HashMap<String, Value>,
    chunk_ids: &[String],
) {
    let sorted: BTreeSet<String> = chunk_ids
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    set_chunk_lineage(properties, &sorted);
}

/// Union lineage arrays from an existing graph record into the incoming map.
///
/// Scalar keys on `incoming` stay last-write. Chunk lineage becomes the sorted
/// union of `source_chunk_ids` and `source_ids` from both maps, and
/// `source_document_ids` the union of plural and singular ids, so a legacy row
/// that only carries `source_ids` keeps its lineage when a new document merges.
pub fn union_source_properties(
    existing: &HashMap<String, Value>,
    incoming: &mut HashMap<String, Value>,
) {
    let has_lineage = INDEXED_LINEAGE_ARRAY_KEYS
        .iter()
        .any(|key| existing.contains_key(*key) || incoming.contains_key(*key));
    if has_lineage {
        let mut chunks = BTreeSet::new();
        for key in INDEXED_LINEAGE_ARRAY_KEYS {
            chunks.extend(string_set(existing.get(key)));
            chunks.extend(string_set(incoming.get(key)));
        }
        incoming.insert("source_chunk_ids".into(), json_strings(&chunks));
        incoming.insert("source_ids".into(), json_strings(&chunks));
    }

    let mut documents = string_set(existing.get("source_document_ids"));
    documents.extend(string_set(incoming.get("source_document_ids")));
    if let Some(id) = scalar_string(existing.get("source_document_id")) {
        documents.insert(id);
    }
    if let Some(id) = scalar_string(incoming.get("source_document_id")) {
        documents.insert(id);
    }
    if !documents.is_empty() {
        incoming.insert("source_document_ids".into(), json_strings(&documents));
    }
    canonicalize_source_lineage(incoming);
}

/// Rebuild lineage from contributions that still belong to other documents.
pub fn sources_from_contributions(rows: &[(Uuid, Value)]) -> (Vec<String>, Vec<String>) {
    let mut chunks = BTreeSet::new();
    let mut documents = BTreeSet::new();
    for (document_id, payload) in rows {
        documents.insert(document_id.to_string());
        let Some(properties) = payload.get("properties") else {
            continue;
        };
        chunks.extend(string_set(properties.get("source_chunk_ids")));
        chunks.extend(string_set(properties.get("source_ids")));
        if let Some(id) = scalar_string(properties.get("source_document_id")) {
            documents.insert(id);
        }
        documents.extend(string_set(properties.get("source_document_ids")));
    }
    (
        chunks.into_iter().collect(),
        documents.into_iter().collect(),
    )
}

/// Replace lineage keys with the surviving contribution set.
///
/// Callers must persist with a full property replace. A source-union upsert
/// would put the removed document's chunk ids back. Both lineage arrays are
/// overwritten so a stale `source_ids` cannot survive under Replace mode.
pub fn apply_retained_sources(
    properties: &mut HashMap<String, Value>,
    chunks: &[String],
    documents: &[String],
) {
    properties.insert("source_document_ids".into(), json_strings_slice(documents));
    match scalar_string(properties.get("source_document_id")) {
        Some(current) if documents.iter().any(|id| id == &current) => {}
        _ => {
            if let Some(kept) = documents.first() {
                properties.insert("source_document_id".into(), Value::String(kept.clone()));
            } else {
                properties.remove("source_document_id");
            }
        }
    }
    let chunks: BTreeSet<String> = chunks.iter().cloned().collect();
    set_chunk_lineage(properties, &chunks);
}

/// Document ids referenced by graph properties (plural + singular + chunk parse).
pub fn document_ids_from_properties(properties: &HashMap<String, Value>) -> BTreeSet<String> {
    let mut documents = string_set(properties.get("source_document_ids"));
    if let Some(id) = scalar_string(properties.get("source_document_id")) {
        documents.insert(id);
    }
    for chunk in string_set(properties.get("source_chunk_ids"))
        .into_iter()
        .chain(string_set(properties.get("source_ids")))
    {
        if let Some((doc_id, _)) = kv_keys::parse_doc_chunk(&chunk) {
            if !doc_id.is_empty() {
                documents.insert(doc_id.to_string());
            }
        }
    }
    documents
}

/// Document owning a lineage token: `{doc}-chunk-{n}` or a bare document id.
///
/// Legacy writers (and Migration 158) record the bare document id when chunk
/// ids are unknown. Discovery probes it exactly, so retain-on-delete must keep
/// a surviving document's bare token; dropping it leaves the document in
/// `source_document_ids` but invisible to its scoped graph.
fn token_owned_by(token: &str, is_owner: impl Fn(&str) -> bool) -> bool {
    is_owner(token) || kv_keys::parse_doc_chunk(token).is_some_and(|(doc_id, _)| is_owner(doc_id))
}

/// Lineage tokens (chunk ids or bare document ids) owned by the given documents.
pub fn chunks_for_documents(
    properties: &HashMap<String, Value>,
    document_ids: &HashSet<String>,
) -> Vec<String> {
    let mut out = BTreeSet::new();
    for chunk in string_set(properties.get("source_chunk_ids"))
        .into_iter()
        .chain(string_set(properties.get("source_ids")))
    {
        if token_owned_by(&chunk, |doc| document_ids.contains(doc)) {
            out.insert(chunk);
        }
    }
    out.into_iter().collect()
}

/// Result of deciding whether a node/edge should be retained after a document
/// delete when both contribution-backed and legacy (no contribution) documents
/// may share the entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedLineage {
    pub chunks: Vec<String>,
    pub documents: Vec<String>,
}

/// Compute retained lineage after excluding `excluding_doc`.
///
/// - Contribution rows for other documents always survive.
/// - `legacy_live_docs` are document ids present on the graph properties,
///   absent from contributions, that still exist in `public.documents`.
/// - Returns `None` when nothing remains (caller should hard-delete).
pub fn retained_lineage(
    properties: &HashMap<String, Value>,
    excluding_doc: Uuid,
    contribution_rows: &[(Uuid, Value)],
    legacy_live_docs: &HashSet<String>,
) -> Option<RetainedLineage> {
    let excluding = excluding_doc.to_string();
    let (mut chunks, mut documents) = sources_from_contributions(contribution_rows);

    let legacy_set: HashSet<String> = legacy_live_docs
        .iter()
        .filter(|id| id.as_str() != excluding)
        .cloned()
        .collect();

    if !legacy_set.is_empty() {
        for chunk in chunks_for_documents(properties, &legacy_set) {
            if !chunks.iter().any(|c| c == &chunk) {
                chunks.push(chunk);
            }
        }
        for doc in &legacy_set {
            if !documents.iter().any(|d| d == doc) {
                documents.push(doc.clone());
            }
        }
    }

    documents.retain(|d| d != &excluding);
    chunks.retain(|c| !token_owned_by(c, |doc| doc == excluding));

    if chunks.is_empty() && documents.is_empty() {
        return None;
    }
    chunks.sort();
    chunks.dedup();
    documents.sort();
    documents.dedup();
    Some(RetainedLineage { chunks, documents })
}

fn union_string_sets(mut left: BTreeSet<String>, right: BTreeSet<String>) -> BTreeSet<String> {
    left.extend(right);
    left
}

fn string_set(value: Option<&Value>) -> BTreeSet<String> {
    let Some(value) = value else {
        return BTreeSet::new();
    };
    match value {
        Value::Array(items) => items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
        Value::String(text) if !text.is_empty() => BTreeSet::from([text.clone()]),
        _ => BTreeSet::new(),
    }
}

fn scalar_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

fn json_strings(values: &BTreeSet<String>) -> Value {
    Value::Array(values.iter().cloned().map(Value::String).collect())
}

fn json_strings_slice(values: &[String]) -> Value {
    Value::Array(values.iter().cloned().map(Value::String).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn map(value: Value) -> HashMap<String, Value> {
        serde_json::from_value(value).expect("object")
    }

    #[test]
    fn canonicalize_mirrors_chunk_ids_to_source_ids() {
        let mut props = map(json!({
            "source_chunk_ids": ["doc-a-chunk-1", "doc-a-chunk-0"],
            "source_document_id": "doc-a"
        }));
        canonicalize_source_lineage(&mut props);
        assert_eq!(
            props.get("source_ids"),
            props.get("source_chunk_ids"),
            "source_ids must mirror source_chunk_ids"
        );
        assert_eq!(
            props["source_chunk_ids"],
            json!(["doc-a-chunk-0", "doc-a-chunk-1"])
        );
        assert_eq!(props["source_document_ids"], json!(["doc-a"]));
    }

    #[test]
    fn canonicalize_derives_documents_from_chunk_ids() {
        let mut props = map(json!({
            "source_chunk_ids": ["aaaa-bbbb-cccc-dddd-eeeeeeeeeeee-chunk-0"]
        }));
        canonicalize_source_lineage(&mut props);
        assert_eq!(
            props["source_document_ids"],
            json!(["aaaa-bbbb-cccc-dddd-eeeeeeeeeeee"])
        );
        assert_eq!(
            props["source_document_id"],
            "aaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
        );
    }

    #[test]
    fn insert_chunk_lineage_delegates_to_canonicalize() {
        let mut props = HashMap::new();
        insert_chunk_lineage_properties(&mut props, &["doc-x-chunk-0".into()]);
        assert_eq!(
            props.get("source_chunk_ids"),
            props.get("source_ids"),
            "pipeline insert must mirror"
        );
        assert_eq!(props["source_document_ids"], json!(["doc-x"]));
    }

    #[test]
    fn union_keeps_both_documents_chunk_ids() {
        let existing = map(json!({
            "description": "from A",
            "source_chunk_ids": ["chunk-a"],
            "source_document_id": "doc-a"
        }));
        let mut incoming = map(json!({
            "description": "from B",
            "source_chunk_ids": ["chunk-b"],
            "source_document_id": "doc-b"
        }));
        union_source_properties(&existing, &mut incoming);
        assert_eq!(incoming["description"], "from B");
        assert_eq!(incoming["source_chunk_ids"], json!(["chunk-a", "chunk-b"]));
        assert_eq!(incoming["source_ids"], json!(["chunk-a", "chunk-b"]));
        assert_eq!(incoming["source_document_ids"], json!(["doc-a", "doc-b"]));
    }

    #[test]
    fn retained_sources_drop_the_removed_document_and_mirror_source_ids() {
        let doc_b = Uuid::new_v4();
        let rows = [(
            doc_b,
            json!({
                "kind": "node",
                "properties": { "source_chunk_ids": ["chunk-b"] }
            }),
        )];
        let (chunks, documents) = sources_from_contributions(&rows);
        let mut properties = map(json!({
            "description": "from A",
            "source_chunk_ids": ["chunk-a", "chunk-b"],
            "source_document_id": "doc-a",
            "source_document_ids": ["doc-a", "doc-b"]
        }));
        apply_retained_sources(&mut properties, &chunks, &documents);
        assert_eq!(properties["source_chunk_ids"], json!(["chunk-b"]));
        assert_eq!(properties["source_ids"], json!(["chunk-b"]));
        assert_eq!(
            properties["source_document_ids"],
            json!([doc_b.to_string()])
        );
        assert_eq!(properties["source_document_id"], doc_b.to_string());
        assert_eq!(properties["description"], "from A");
    }

    #[test]
    fn retained_lineage_contributions_only() {
        let doc_a = Uuid::new_v4();
        let doc_b = Uuid::new_v4();
        let props = map(json!({
            "source_chunk_ids": [
                format!("{doc_a}-chunk-0"),
                format!("{doc_b}-chunk-0")
            ],
            "source_document_ids": [doc_a.to_string(), doc_b.to_string()]
        }));
        let rows = [(
            doc_b,
            json!({
                "properties": { "source_chunk_ids": [format!("{doc_b}-chunk-0")] }
            }),
        )];
        let kept = retained_lineage(&props, doc_a, &rows, &HashSet::new()).expect("keep");
        assert_eq!(kept.documents, vec![doc_b.to_string()]);
        assert_eq!(kept.chunks, vec![format!("{doc_b}-chunk-0")]);
    }

    #[test]
    fn retained_lineage_legacy_only() {
        let doc_a = Uuid::new_v4();
        let doc_legacy = Uuid::new_v4();
        let props = map(json!({
            "source_chunk_ids": [
                format!("{doc_a}-chunk-0"),
                format!("{doc_legacy}-chunk-1")
            ],
            "source_document_ids": [doc_a.to_string(), doc_legacy.to_string()]
        }));
        let legacy = HashSet::from([doc_legacy.to_string()]);
        let kept = retained_lineage(&props, doc_a, &[], &legacy).expect("keep legacy");
        assert_eq!(kept.documents, vec![doc_legacy.to_string()]);
        assert_eq!(kept.chunks, vec![format!("{doc_legacy}-chunk-1")]);
    }

    #[test]
    fn retained_lineage_mixed_contribution_and_legacy() {
        let doc_a = Uuid::new_v4();
        let doc_b = Uuid::new_v4();
        let doc_legacy = Uuid::new_v4();
        let props = map(json!({
            "source_chunk_ids": [
                format!("{doc_a}-chunk-0"),
                format!("{doc_b}-chunk-0"),
                format!("{doc_legacy}-chunk-2")
            ]
        }));
        let rows = [(
            doc_b,
            json!({
                "properties": { "source_chunk_ids": [format!("{doc_b}-chunk-0")] }
            }),
        )];
        let legacy = HashSet::from([doc_legacy.to_string()]);
        let kept = retained_lineage(&props, doc_a, &rows, &legacy).expect("keep both");
        assert!(kept.documents.contains(&doc_b.to_string()));
        assert!(kept.documents.contains(&doc_legacy.to_string()));
        assert!(!kept.documents.contains(&doc_a.to_string()));
    }

    #[test]
    fn retained_lineage_deleted_legacy_document_is_dropped() {
        let doc_a = Uuid::new_v4();
        let doc_gone = Uuid::new_v4();
        let props = map(json!({
            "source_chunk_ids": [
                format!("{doc_a}-chunk-0"),
                format!("{doc_gone}-chunk-0")
            ]
        }));
        // legacy_live_docs empty ⇒ gone doc is not live in public.documents
        assert!(retained_lineage(&props, doc_a, &[], &HashSet::new()).is_none());
    }

    #[test]
    fn retained_lineage_keeps_legacy_bare_document_token() {
        let doc_a = Uuid::new_v4();
        let doc_legacy = Uuid::new_v4();
        let props = map(json!({
            "source_ids": [format!("{doc_a}-chunk-0"), doc_legacy.to_string()],
            "source_chunk_ids": [format!("{doc_a}-chunk-0"), doc_legacy.to_string()],
            "source_document_ids": [doc_a.to_string(), doc_legacy.to_string()]
        }));
        let legacy = HashSet::from([doc_legacy.to_string()]);
        let kept = retained_lineage(&props, doc_a, &[], &legacy).expect("keep legacy");
        assert_eq!(kept.chunks, vec![doc_legacy.to_string()]);
        assert_eq!(kept.documents, vec![doc_legacy.to_string()]);
    }

    #[test]
    fn retained_lineage_drops_the_deleted_documents_bare_token() {
        let doc_a = Uuid::new_v4();
        let doc_b = Uuid::new_v4();
        let props = map(json!({
            "source_ids": [doc_a.to_string(), format!("{doc_b}-chunk-3")],
            "source_document_ids": [doc_a.to_string(), doc_b.to_string()]
        }));
        let rows = [(
            doc_b,
            json!({ "properties": { "source_ids": [doc_a.to_string(), format!("{doc_b}-chunk-3")] } }),
        )];
        let kept = retained_lineage(&props, doc_a, &rows, &HashSet::new()).expect("keep b");
        assert_eq!(kept.chunks, vec![format!("{doc_b}-chunk-3")]);
        assert_eq!(kept.documents, vec![doc_b.to_string()]);
    }

    #[test]
    fn chunks_for_documents_ignores_unrelated_bare_tokens() {
        let props = map(json!({ "source_ids": ["doc-a", "doc-b-chunk-0", "doc-c"] }));
        let owned = chunks_for_documents(&props, &HashSet::from(["doc-a".to_string()]));
        assert_eq!(owned, vec!["doc-a".to_string()]);
    }

    #[test]
    fn union_keeps_legacy_source_ids_only_lineage() {
        let existing = map(json!({
            "source_ids": ["legacy-chunk-0"],
            "source_document_id": "legacy"
        }));
        let mut incoming = map(json!({
            "source_chunk_ids": ["new-chunk-0"],
            "source_document_id": "new"
        }));
        union_source_properties(&existing, &mut incoming);
        let both = json!(["legacy-chunk-0", "new-chunk-0"]);
        assert_eq!(incoming["source_chunk_ids"], both);
        assert_eq!(incoming["source_ids"], both);
        assert_eq!(incoming["source_document_ids"], json!(["legacy", "new"]));
    }

    #[test]
    fn canonicalize_unions_divergent_lineage_arrays() {
        let mut props = map(json!({
            "source_chunk_ids": ["doc-a-chunk-0"],
            "source_ids": ["doc-b-chunk-0"]
        }));
        canonicalize_source_lineage(&mut props);
        let both = json!(["doc-a-chunk-0", "doc-b-chunk-0"]);
        assert_eq!(props["source_chunk_ids"], both);
        assert_eq!(props["source_ids"], both);
    }

    #[test]
    fn insert_chunk_lineage_is_authoritative_for_capped_lists() {
        let mut props = map(json!({
            "source_chunk_ids": ["doc-a-chunk-0", "doc-a-chunk-1"],
            "source_ids": ["doc-a-chunk-0", "doc-a-chunk-1"]
        }));
        insert_chunk_lineage_properties(&mut props, &["doc-a-chunk-1".into()]);
        assert_eq!(props["source_chunk_ids"], json!(["doc-a-chunk-1"]));
        assert_eq!(props["source_ids"], json!(["doc-a-chunk-1"]));
    }

    #[test]
    fn canonicalize_without_arrays_only_derives_documents() {
        let mut props = map(json!({ "source_document_id": "doc-a" }));
        canonicalize_source_lineage(&mut props);
        assert!(!props.contains_key("source_ids"));
        assert!(!props.contains_key("source_chunk_ids"));
        assert_eq!(props["source_document_ids"], json!(["doc-a"]));
    }

    #[test]
    fn retain_canonicalize_does_not_revive_subtracted_source_ids() {
        let mut properties = map(json!({
            "source_chunk_ids": ["doc-a-chunk-0", "doc-b-chunk-0"],
            "source_ids": ["doc-a-chunk-0", "doc-b-chunk-0"],
            "source_document_ids": ["doc-a", "doc-b"]
        }));
        apply_retained_sources(
            &mut properties,
            &["doc-b-chunk-0".into()],
            &["doc-b".into()],
        );
        assert_eq!(properties["source_chunk_ids"], json!(["doc-b-chunk-0"]));
        assert_eq!(properties["source_ids"], json!(["doc-b-chunk-0"]));
        assert_eq!(properties["source_document_ids"], json!(["doc-b"]));
    }
}
