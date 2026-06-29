//! Post-retrieval context filtering by document IDs.
//!
//! Filters a `QueryContext` to only include items from allowed documents.
//! Applied after vector search / mode-specific retrieval but BEFORE
//! truncation and LLM answer generation.
//!
//! @implements SPEC-005: Document date and pattern filters (Tier 1)
//! @implements SPEC-031: Strict entity/relationship lineage filtering
//!
//! ## Filter strictness
//!
//! | Item type    | Has lineage data | Behavior                         |
//! |--------------|-----------------|----------------------------------|
//! | Chunk        | always (strict) | exclude if doc_id not in allowed |
//! | Entity/Rel   | ids[] non-empty | keep if ANY id ∈ allowed         |
//! | Entity/Rel   | single id only  | keep if id ∈ allowed             |
//! | Entity/Rel   | NO lineage data | keep (truly unknown provenance)  |
//!
//! Once ANY lineage data is present, it MUST match for the item to be kept.
//! The lenient fallback (keep if no data) only applies to truly orphan items.

use std::collections::HashSet;

use crate::context::QueryContext;

/// Filter a `QueryContext` to only keep items from the allowed document set.
///
/// - **Chunks**: strict — excluded if `document_id` is absent or not in set.
/// - **Entities**: checked in priority order:
///   1. `source_document_ids[]` (plural union) — keep if ANY id ∈ allowed
///   2. `source_document_id` (singular) — keep if id ∈ allowed
///   3. No lineage data at all — kept (unknown provenance)
/// - **Relationships**: same rule as entities.
///
/// @implements SPEC-031
pub fn filter_context_by_document_ids(context: &mut QueryContext, allowed_ids: Option<&[String]>) {
    let allowed = match allowed_ids {
        Some(ids) => ids,
        None => return, // No filter active — keep everything
    };

    let id_set: HashSet<&str> = allowed.iter().map(|s| s.as_str()).collect();

    // Chunks: strict — must have a matching document_id
    context.chunks.retain(|chunk| {
        chunk
            .document_id
            .as_deref()
            .map(|id| id_set.contains(id))
            .unwrap_or(false)
    });

    // Entities: check source_document_ids (plural) first, then singular
    context.entities.retain(|entity| {
        entity_or_rel_passes_filter(
            &entity.source_document_ids,
            entity.source_document_id.as_deref(),
            &id_set,
        )
    });

    // Relationships: same rule as entities
    context.relationships.retain(|rel| {
        entity_or_rel_passes_filter(
            &rel.source_document_ids,
            rel.source_document_id.as_deref(),
            &id_set,
        )
    });
}

/// Returns true if an entity/relationship should be kept given the allowed set.
///
/// Priority:
/// 1. `source_document_ids[]` non-empty → ANY must match
/// 2. `source_document_id` Some → must match
/// 3. Both empty/None → keep (no provenance tracked; could be globally-derived)
///
/// @implements SPEC-031
fn entity_or_rel_passes_filter(
    source_document_ids: &[String],
    source_document_id: Option<&str>,
    id_set: &HashSet<&str>,
) -> bool {
    // Priority 1: plural union array (SPEC-031)
    if !source_document_ids.is_empty() {
        return source_document_ids
            .iter()
            .any(|id| id_set.contains(id.as_str()));
    }
    // Priority 2: singular legacy field
    if let Some(id) = source_document_id {
        return id_set.contains(id);
    }
    // Priority 3: no lineage data — keep (truly unknown provenance)
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{QueryContext, RetrievedChunk, RetrievedEntity, RetrievedRelationship};

    fn make_chunk(id: &str, doc_id: Option<&str>) -> RetrievedChunk {
        let mut chunk = RetrievedChunk::new(id, format!("content of {}", id), 0.9);
        if let Some(d) = doc_id {
            chunk = chunk.with_document_id(d);
        }
        chunk
    }

    fn make_entity(name: &str, doc_id: Option<&str>) -> RetrievedEntity {
        let mut entity =
            RetrievedEntity::new(name, "PERSON", format!("desc of {}", name)).with_score(0.8);
        if let Some(d) = doc_id {
            entity = entity.with_source_document_id(d);
        }
        entity
    }

    fn make_entity_multi(name: &str, doc_ids: &[&str]) -> RetrievedEntity {
        RetrievedEntity::new(name, "PERSON", format!("desc of {}", name))
            .with_score(0.8)
            .with_source_document_ids(doc_ids.iter().map(|s| s.to_string()).collect())
    }

    fn make_relationship(src: &str, tgt: &str, doc_id: Option<&str>) -> RetrievedRelationship {
        let mut rel = RetrievedRelationship::new(src, tgt, "KNOWS").with_score(0.7);
        if let Some(d) = doc_id {
            rel = rel.with_source_document_id(d);
        }
        rel
    }

    fn make_relationship_multi(src: &str, tgt: &str, doc_ids: &[&str]) -> RetrievedRelationship {
        RetrievedRelationship::new(src, tgt, "KNOWS")
            .with_score(0.7)
            .with_source_document_ids(doc_ids.iter().map(|s| s.to_string()).collect())
    }

    fn sample_context() -> QueryContext {
        let mut ctx = QueryContext::new();
        ctx.chunks = vec![
            make_chunk("c1", Some("doc-a")),
            make_chunk("c2", Some("doc-b")),
            make_chunk("c3", Some("doc-c")),
            make_chunk("c4", None), // orphan chunk
        ];
        ctx.entities = vec![
            make_entity("Alice", Some("doc-a")),
            make_entity("Bob", Some("doc-b")),
            make_entity("Charlie", None), // no provenance
        ];
        ctx.relationships = vec![
            make_relationship("Alice", "Bob", Some("doc-a")),
            make_relationship("Bob", "Charlie", Some("doc-c")),
            make_relationship("X", "Y", None), // no provenance
        ];
        ctx
    }

    // ── Existing tests (updated for strict filter) ───────────────────────────

    #[test]
    fn test_none_filter_is_noop() {
        let mut ctx = sample_context();
        let original_chunks = ctx.chunks.len();
        let original_entities = ctx.entities.len();
        let original_rels = ctx.relationships.len();

        filter_context_by_document_ids(&mut ctx, None);

        assert_eq!(ctx.chunks.len(), original_chunks);
        assert_eq!(ctx.entities.len(), original_entities);
        assert_eq!(ctx.relationships.len(), original_rels);
    }

    #[test]
    fn test_filter_keeps_matching_documents() {
        let mut ctx = sample_context();
        let allowed = vec!["doc-a".to_string(), "doc-b".to_string()];

        filter_context_by_document_ids(&mut ctx, Some(&allowed));

        // Chunks: doc-a, doc-b kept; doc-c and orphan excluded
        assert_eq!(ctx.chunks.len(), 2);
        assert!(ctx
            .chunks
            .iter()
            .all(|c| c.document_id.as_deref() == Some("doc-a")
                || c.document_id.as_deref() == Some("doc-b")));

        // Entities: Alice (doc-a) ✓, Bob (doc-b) ✓, Charlie (no provenance) ✓
        assert_eq!(ctx.entities.len(), 3);

        // Relationships: Alice→Bob (doc-a) ✓, Bob→Charlie (doc-c) ✗, X→Y (no prov) ✓
        assert_eq!(ctx.relationships.len(), 2);
    }

    #[test]
    fn test_empty_filter_removes_all_chunks() {
        let mut ctx = sample_context();
        let allowed: Vec<String> = vec![];

        filter_context_by_document_ids(&mut ctx, Some(&allowed));

        // All chunks removed (none match empty set)
        assert_eq!(ctx.chunks.len(), 0);

        // Entities without provenance still kept (lenient fallback)
        assert_eq!(ctx.entities.len(), 1);
        assert_eq!(ctx.entities[0].name, "Charlie");

        // Relationships without provenance still kept
        assert_eq!(ctx.relationships.len(), 1);
    }

    #[test]
    fn test_filter_single_document() {
        let mut ctx = sample_context();
        let allowed = vec!["doc-c".to_string()];

        filter_context_by_document_ids(&mut ctx, Some(&allowed));

        assert_eq!(ctx.chunks.len(), 1);
        assert_eq!(ctx.chunks[0].document_id.as_deref(), Some("doc-c"));

        // Entities: Charlie (no prov) ✓; Alice (doc-a) ✗; Bob (doc-b) ✗
        assert_eq!(ctx.entities.len(), 1);
        assert_eq!(ctx.entities[0].name, "Charlie");

        // Relationships: Bob→Charlie (doc-c) ✓; Alice→Bob (doc-a) ✗; X→Y (no prov) ✓
        assert_eq!(ctx.relationships.len(), 2);
    }

    // ── SPEC-031 new tests: source_document_ids (plural) ─────────────────────

    #[test]
    fn test_spec031_multi_doc_entity_kept_if_any_id_matches() {
        // Entity appeared in both doc-a and doc-b — scope is doc-a
        let mut ctx = QueryContext::new();
        ctx.entities = vec![
            make_entity_multi("MultiDoc", &["doc-a", "doc-b"]), // ANY in allowed
            make_entity_multi("WrongDoc", &["doc-x", "doc-y"]), // NONE in allowed
        ];

        let allowed = vec!["doc-a".to_string()];
        filter_context_by_document_ids(&mut ctx, Some(&allowed));

        assert_eq!(ctx.entities.len(), 1, "only MultiDoc should be kept");
        assert_eq!(ctx.entities[0].name, "MultiDoc");
    }

    #[test]
    fn test_spec031_multi_doc_entity_excluded_if_no_id_matches() {
        let mut ctx = QueryContext::new();
        ctx.entities = vec![make_entity_multi("CrossDoc", &["doc-x", "doc-z"])];

        let allowed = vec!["doc-a".to_string()];
        filter_context_by_document_ids(&mut ctx, Some(&allowed));

        assert_eq!(ctx.entities.len(), 0, "CrossDoc has no allowed source doc");
    }

    #[test]
    fn test_spec031_source_document_ids_takes_priority_over_singular() {
        // source_document_ids = ["doc-a"] but source_document_id = "doc-x"
        // Plural should win — doc-a is in allowed set
        let entity = RetrievedEntity::new("Conflict", "PERSON", "desc")
            .with_source_document_id("doc-x") // singular (wrong)
            .with_source_document_ids(vec!["doc-a".to_string()]); // plural (correct)

        let mut ctx = QueryContext::new();
        ctx.entities = vec![entity];

        let allowed = vec!["doc-a".to_string()];
        filter_context_by_document_ids(&mut ctx, Some(&allowed));

        assert_eq!(
            ctx.entities.len(),
            1,
            "plural source_document_ids takes priority"
        );
    }

    #[test]
    fn test_spec031_singular_fallback_when_plural_empty() {
        // source_document_ids = [] but source_document_id = "doc-a" — use singular
        let entity =
            RetrievedEntity::new("Fallback", "PERSON", "desc").with_source_document_id("doc-a"); // singular used when plural absent

        let mut ctx = QueryContext::new();
        ctx.entities = vec![entity];

        let allowed = vec!["doc-a".to_string()];
        filter_context_by_document_ids(&mut ctx, Some(&allowed));

        assert_eq!(ctx.entities.len(), 1, "singular fallback works");
    }

    #[test]
    fn test_spec031_no_lineage_data_kept_always() {
        // Truly no provenance (globally derived, no source tracked)
        let entity = RetrievedEntity::new("Global", "CONCEPT", "global concept");
        // source_document_id = None, source_document_ids = []

        let mut ctx = QueryContext::new();
        ctx.entities = vec![entity];

        // Even with a strict scope, no-provenance entities are kept
        let allowed = vec!["doc-x".to_string()];
        filter_context_by_document_ids(&mut ctx, Some(&allowed));

        assert_eq!(ctx.entities.len(), 1, "no-provenance entities always kept");
    }

    #[test]
    fn test_spec031_strict_when_any_lineage_present() {
        // Has source_document_id but it doesn't match — SHOULD be excluded
        // Old lenient behavior: would keep this. New strict: exclude.
        let entity = make_entity("TypicalEntity", Some("doc-z")); // doc-z NOT in allowed

        let mut ctx = QueryContext::new();
        ctx.entities = vec![entity];

        let allowed = vec!["doc-a".to_string()];
        filter_context_by_document_ids(&mut ctx, Some(&allowed));

        assert_eq!(
            ctx.entities.len(),
            0,
            "entity with wrong source_document_id must be excluded (strict filter)"
        );
    }

    #[test]
    fn test_spec031_multi_doc_relationship_kept_if_any_matches() {
        let mut ctx = QueryContext::new();
        ctx.relationships = vec![
            make_relationship_multi("A", "B", &["doc-a", "doc-b"]),
            make_relationship_multi("C", "D", &["doc-z"]),
        ];

        let allowed = vec!["doc-b".to_string()];
        filter_context_by_document_ids(&mut ctx, Some(&allowed));

        assert_eq!(ctx.relationships.len(), 1);
        assert_eq!(ctx.relationships[0].source, "A");
    }
}
