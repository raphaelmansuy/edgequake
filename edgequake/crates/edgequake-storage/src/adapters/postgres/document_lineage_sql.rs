//! Document-scope predicates for the typed entity and relationship searches.
//!
//! Lineage question: "was this row derived from one of these documents?"
//! The authoritative answer is the row's own source chunk keys, written in the
//! same statement as the row (`entities.source_chunk_ids`,
//! `relationships.source_chunk_keys`), mapped to documents through
//! `chunks.legacy_chunk_key`. That is also the engine's rule: derive documents
//! from chunk ids, keep on any overlap, drop rows without lineage.
//!
//! Keys, not chunk uuids, because the merger writes entities and relationships
//! in the merge phase and the chunk rows only exist after the persist phase.
//! A key is known when the row is written; a chunk uuid is not.
//!
//! The overlap operator (`&&`) is not leakproof, so under forced row security
//! it filters after the policy instead of becoming a GIN index condition.
//! Callers keep it behind the ANN limit or an `OFFSET 0` id lookup.
//!
//! `$4` is always the `uuid[]` of document ids.

/// Legacy chunk keys of the documents bound at `$4`, resolved once as an
/// uncorrelated InitPlan. A lookup from each row back into `chunks` misses the
/// ANN deadline on a large workspace.
const DOCUMENT_CHUNK_KEYS: &str =
    "ARRAY(SELECT c.legacy_chunk_key FROM chunks c WHERE c.document_id = ANY($4))";

fn required(column: &str) -> String {
    format!("{column} && {DOCUMENT_CHUNK_KEYS}")
}

/// Entity lineage the caller asked for: `AND <overlap>`.
pub(super) fn entity_required() -> String {
    format!("AND {}", required("e.source_chunk_ids"))
}

/// Entity lineage when the caller may pass no documents. The `$4` NULL test
/// hides the documents from a generic plan, so a caller that passes documents
/// swaps this for [`entity_required`].
pub(super) fn entity_optional() -> String {
    format!(
        "AND ($4::uuid[] IS NULL OR {})",
        required("e.source_chunk_ids")
    )
}

/// Relationship lineage the caller asked for: `AND <overlap>`.
pub(super) fn relationship_required() -> String {
    format!("AND {}", required("r.source_chunk_keys"))
}

/// Relationship lineage when the caller may pass no documents.
pub(super) fn relationship_optional() -> String {
    format!(
        "AND ($4::uuid[] IS NULL OR {})",
        required("r.source_chunk_keys")
    )
}

/// The optional entity guard without the leading `AND`, for a `WHERE`.
pub(super) fn entity_guard() -> String {
    entity_optional().trim_start_matches("AND ").to_string()
}

/// The optional relationship guard without the leading `AND`, for a `WHERE`.
pub(super) fn relationship_guard() -> String {
    relationship_optional()
        .trim_start_matches("AND ")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lineage_is_the_rows_own_chunk_keys_mapped_through_chunks() {
        for sql in [entity_required(), relationship_required()] {
            assert!(sql.contains("c.legacy_chunk_key"), "{sql}");
            assert!(sql.contains("c.document_id = ANY($4)"), "{sql}");
            assert!(
                !sql.contains("source_ids"),
                "document ids are not stored on rows: {sql}"
            );
            assert!(
                !sql.contains("c.id"),
                "chunk uuids do not exist at merge time: {sql}"
            );
        }
        assert!(entity_required().contains("e.source_chunk_ids &&"));
        assert!(relationship_required().contains("r.source_chunk_keys &&"));
    }

    #[test]
    fn the_optional_form_only_adds_the_null_test() {
        assert_eq!(
            entity_optional(),
            format!(
                "AND ($4::uuid[] IS NULL OR {})",
                entity_required().trim_start_matches("AND ")
            )
        );
        assert_eq!(
            relationship_optional(),
            format!(
                "AND ($4::uuid[] IS NULL OR {})",
                relationship_required().trim_start_matches("AND ")
            )
        );
        assert!(!entity_guard().starts_with("AND "));
        assert!(!relationship_guard().starts_with("AND "));
    }
}
