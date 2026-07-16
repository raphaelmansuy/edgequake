//! Document-scoped source lineage SQL predicates (SPEC-021 P-A3 / SPEC-045).
//!
//! SSOT for matching AGE node/edge properties against a document chunk prefix.
//! Covers legacy `source_id`, modern `source_ids`, and pipeline `source_chunk_ids`.

/// Max chunk indices probed for GIN `@>` entity-count reconcile (list hot path).
///
/// WHY: `LIKE '%…%'` / `jsonb_array_elements_text` forces a Seq Scan over
/// `_ag_label_vertex` (~140k+ nodes → multi-second Documents list). Exact
/// chunk-id containment uses `idx_*_source_ids_gin` (~1 ms). Documents with
/// more than this many chunks still get a correct lower bound; stats write
/// path (P-A1) remains the primary count source.
pub(in crate::adapters::postgres::graph) const SOURCE_CHUNK_PROBE_LIMIT: usize = 256;

use super::escape::escape_sql_literal;

/// Normalize a document / chunk prefix to the `{doc_id}-chunk-` form.
///
/// Accepts either a bare document id or an already-suffixed
/// [`crate::kv_keys::doc_chunk_prefix`] value.
pub(in crate::adapters::postgres::graph) fn normalize_doc_chunk_prefix(prefix: &str) -> String {
    if prefix.ends_with("-chunk-") {
        prefix.to_string()
    } else {
        format!("{prefix}-chunk-")
    }
}

/// Build concrete chunk-id candidates for GIN `@>` probes (`{prefix}0`..`N-1`).
///
/// Batched list counts use SQL `generate_series` instead; this helper remains
/// for single-prefix probes, scan predicates, and tests.
#[allow(dead_code)]
pub(in crate::adapters::postgres::graph) fn source_chunk_id_candidates(
    prefix: &str,
    limit: usize,
) -> Vec<String> {
    let chunk_prefix = normalize_doc_chunk_prefix(prefix);
    let n = limit.max(1).min(SOURCE_CHUNK_PROBE_LIMIT);
    (0..n)
        .map(|i| format!("{chunk_prefix}{i}"))
        .collect()
}

/// Build a SQL predicate matching jsonb `properties` against one document prefix.
///
/// `props` must already be a jsonb expression (e.g. `(agtype_to_json(v.properties))::jsonb`).
/// `doc_prefix` is the document id (used to derive `{doc_id}-chunk-` patterns).
///
/// Prefer [`source_chunk_id_candidates`] + GIN `@>` for list hot paths; this
/// LIKE/unnest form remains for scan/filter ops where prefix match is required.
pub(in crate::adapters::postgres::graph) fn jsonb_matches_doc_source_prefix(
    props: &str,
    doc_prefix: &str,
) -> String {
    let esc = escape_sql_literal(doc_prefix);
    let chunk = escape_sql_literal(&normalize_doc_chunk_prefix(doc_prefix));

    format!(
        "({props}->>'source_id' LIKE '{esc}%' \
         OR {props}->>'source_id' LIKE '%|{esc}%' \
         OR {props}->>'source_id' LIKE '%|{chunk}%' \
         OR {props}->>'source_id' LIKE '{chunk}%' \
         OR EXISTS ( \
             SELECT 1 FROM jsonb_array_elements_text( \
                 CASE \
                     WHEN jsonb_typeof({props}->'source_ids') = 'array' \
                     THEN {props}->'source_ids' \
                     ELSE '[]'::jsonb \
                 END \
             ) src \
             WHERE src LIKE '{esc}%' OR src LIKE '{chunk}%' OR src = '{esc}' \
         ) \
         OR EXISTS ( \
             SELECT 1 FROM jsonb_array_elements_text( \
                 CASE \
                     WHEN jsonb_typeof({props}->'source_chunk_ids') = 'array' \
                     THEN {props}->'source_chunk_ids' \
                     ELSE '[]'::jsonb \
                 END \
             ) src \
             WHERE src LIKE '{esc}%' OR src LIKE '{chunk}%' OR src = '{esc}' \
         ))",
        props = props,
        esc = esc,
        chunk = chunk,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_source_chunk_ids_array_path() {
        let sql = jsonb_matches_doc_source_prefix("props", "doc-abc");
        assert!(sql.contains("source_chunk_ids"));
        assert!(sql.contains("source_ids"));
        assert!(sql.contains("doc-abc-chunk-"));
    }

    #[test]
    fn normalize_accepts_bare_doc_id_and_chunk_prefix() {
        assert_eq!(normalize_doc_chunk_prefix("doc-abc"), "doc-abc-chunk-");
        assert_eq!(
            normalize_doc_chunk_prefix("doc-abc-chunk-"),
            "doc-abc-chunk-"
        );
    }

    #[test]
    fn chunk_candidates_are_gin_friendly_exact_ids() {
        let ids = source_chunk_id_candidates("doc-abc-chunk-", 3);
        assert_eq!(
            ids,
            vec![
                "doc-abc-chunk-0".to_string(),
                "doc-abc-chunk-1".to_string(),
                "doc-abc-chunk-2".to_string()
            ]
        );
    }
}
