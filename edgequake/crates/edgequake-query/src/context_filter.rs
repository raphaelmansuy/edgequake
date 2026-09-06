//! Post-retrieval context filtering by document IDs.
//!
//! Filters a `QueryContext` to only include items from allowed documents.
//! Applied after vector search / mode-specific retrieval but BEFORE
//! truncation and LLM answer generation.
//!
//! @implements SPEC-005: Document date and pattern filters (Tier 1)
//! @implements SPEC-031: Strict entity/relationship lineage filtering
//! @implements SPEC-047 / 021 L2–L4: derive docs from chunk ids; fail-closed
//! @implements SPEC-146: hub description from authorized occurrences only
//!
//! ## Filter strictness
//!
//! | Item type    | Has lineage data | Behavior                         |
//! |--------------|-----------------|----------------------------------|
//! | Chunk        | always (strict) | exclude if doc_id not in allowed |
//! | Entity/Rel   | docs (any path) | keep if ANY id ∈ allowed         |
//! | Entity/Rel   | NO lineage data | **drop** under active scope (L4) |
//!
//! Doc resolution (DRY via [`crate::lineage_scope`]): plural → singular →
//! derive from `source_chunk_ids` / `source_chunk_id`.

use std::collections::HashSet;

use crate::context::QueryContext;
use crate::helpers::extract_document_id;
use crate::lineage_scope::{lineage_intersects_allowed, resolve_lineage_document_ids};

/// LightRAG / merger field separator for multi-occurrence descriptions.
const GRAPH_FIELD_SEP: &str = "<SEP>";

/// Filter a `QueryContext` to only keep items from the allowed document set.
///
/// - **Chunks**: strict — must have a matching `document_id`.
/// - **Entities / relationships**:
///   1. `source_document_ids[]` (union)
///   2. `source_document_id` (singular)
///   3. docs derived from chunk id(s)
///   4. still empty → **drop** (021 L4)
/// - **Entity descriptions**: when fragment count matches `source_chunk_ids`,
///   keep only fragments whose chunk derives an allowed document (SPEC-146).
///
/// @implements SPEC-031 · SPEC-047 / 021 · SPEC-146
pub fn filter_context_by_document_ids(context: &mut QueryContext, allowed_ids: Option<&[String]>) {
    let allowed = match allowed_ids {
        Some(ids) => ids,
        None => return,
    };

    let id_set: HashSet<&str> = allowed.iter().map(|s| s.as_str()).collect();

    context.chunks.retain(|chunk| {
        chunk
            .document_id
            .as_deref()
            .map(|id| id_set.contains(id))
            .unwrap_or(false)
    });

    context.entities.retain(|entity| {
        let docs = resolve_lineage_document_ids(
            &entity.source_document_ids,
            entity.source_document_id.as_deref(),
            &entity.source_chunk_ids,
        );
        lineage_intersects_allowed(&docs, &id_set)
    });

    for entity in &mut context.entities {
        entity.description = filter_hub_description_refs(
            &entity.description,
            &entity.source_chunk_ids,
            &id_set,
        );
    }

    context.relationships.retain(|rel| {
        let chunk_ids: Vec<String> = if !rel.source_chunk_ids.is_empty() {
            rel.source_chunk_ids.clone()
        } else {
            rel.source_chunk_id.iter().cloned().collect()
        };
        let docs = resolve_lineage_document_ids(
            &rel.source_document_ids,
            rel.source_document_id.as_deref(),
            &chunk_ids,
        );
        lineage_intersects_allowed(&docs, &id_set)
    });
}

/// Map a provenance ref (chunk id or bare document UUID) to a document id.
fn ref_to_document_id(source_ref: &str) -> Option<String> {
    if let Some(doc) = extract_document_id(source_ref) {
        return Some(doc);
    }
    // Bare document UUID (graph `source_ids`).
    if source_ref.len() == 36 {
        return Some(source_ref.to_string());
    }
    None
}

/// Keep description fragments aligned with authorized provenance (SPEC-146).
///
/// `source_refs` may be chunk ids (`{doc}-chunk-N`) or bare document UUIDs.
/// When `allowed_document_ids` is `None`, returns the description unchanged.
pub fn filter_hub_description_by_allow(
    description: &str,
    source_refs: &[String],
    allowed_document_ids: Option<&[String]>,
) -> String {
    let Some(allowed) = allowed_document_ids else {
        return description.to_string();
    };
    let id_set: HashSet<&str> = allowed.iter().map(|s| s.as_str()).collect();
    filter_hub_description_refs(description, source_refs, &id_set)
}

fn filter_hub_description_refs(
    description: &str,
    source_refs: &[String],
    allowed: &HashSet<&str>,
) -> String {
    if description.is_empty() || source_refs.is_empty() {
        return description.to_string();
    }
    if !description.contains(GRAPH_FIELD_SEP) {
        // Single blob — keep only if every source is authorized.
        let all_ok = source_refs.iter().all(|r| {
            ref_to_document_id(r)
                .map(|d| allowed.contains(d.as_str()))
                .unwrap_or(false)
        });
        return if all_ok {
            description.to_string()
        } else {
            // Partial authorization without fragment alignment → strip (fail-closed leak).
            String::new()
        };
    }
    let fragments: Vec<&str> = description.split(GRAPH_FIELD_SEP).collect();
    if fragments.len() != source_refs.len() {
        // Misaligned — cannot safely attribute; strip to avoid cross-label leak.
        return String::new();
    }
    fragments
        .iter()
        .zip(source_refs.iter())
        .filter_map(|(frag, r)| {
            let doc = ref_to_document_id(r)?;
            if allowed.contains(doc.as_str()) {
                Some((*frag).trim())
            } else {
                None
            }
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(GRAPH_FIELD_SEP)
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
            make_chunk("c4", None),
        ];
        ctx.entities = vec![
            make_entity("Alice", Some("doc-a")),
            make_entity("Bob", Some("doc-b")),
            make_entity("Charlie", None), // no provenance → drop under scope
        ];
        ctx.relationships = vec![
            make_relationship("Alice", "Bob", Some("doc-a")),
            make_relationship("Bob", "Charlie", Some("doc-c")),
            make_relationship("X", "Y", None),
        ];
        ctx
    }

    #[test]
    fn test_none_filter_is_noop() {
        let mut ctx = sample_context();
        let original_chunks = ctx.chunks.len();
        filter_context_by_document_ids(&mut ctx, None);
        assert_eq!(ctx.chunks.len(), original_chunks);
        assert_eq!(ctx.entities.len(), 3);
    }

    #[test]
    fn test_filter_keeps_matching_documents() {
        let mut ctx = sample_context();
        let allowed = vec!["doc-a".to_string(), "doc-b".to_string()];
        filter_context_by_document_ids(&mut ctx, Some(&allowed));

        assert_eq!(ctx.chunks.len(), 2);
        // Alice + Bob; Charlie dropped (021 L4)
        assert_eq!(ctx.entities.len(), 2);
        // Alice→Bob kept; X→Y dropped
        assert_eq!(ctx.relationships.len(), 1);
        assert_eq!(ctx.relationships[0].source, "Alice");
    }

    #[test]
    fn test_empty_filter_removes_orphans() {
        let mut ctx = sample_context();
        let allowed: Vec<String> = vec![];
        filter_context_by_document_ids(&mut ctx, Some(&allowed));
        assert_eq!(ctx.chunks.len(), 0);
        assert_eq!(ctx.entities.len(), 0);
        assert_eq!(ctx.relationships.len(), 0);
    }

    #[test]
    fn test_filter_single_document() {
        let mut ctx = sample_context();
        let allowed = vec!["doc-c".to_string()];
        filter_context_by_document_ids(&mut ctx, Some(&allowed));

        assert_eq!(ctx.chunks.len(), 1);
        assert_eq!(ctx.entities.len(), 0); // Charlie has no lineage
        assert_eq!(ctx.relationships.len(), 1);
        assert_eq!(ctx.relationships[0].source, "Bob");
    }

    #[test]
    fn test_spec031_multi_doc_entity_kept_if_any_id_matches() {
        let mut ctx = QueryContext::new();
        ctx.entities = vec![
            make_entity_multi("MultiDoc", &["doc-a", "doc-b"]),
            make_entity_multi("WrongDoc", &["doc-x", "doc-y"]),
        ];
        let allowed = vec!["doc-a".to_string()];
        filter_context_by_document_ids(&mut ctx, Some(&allowed));
        assert_eq!(ctx.entities.len(), 1);
        assert_eq!(ctx.entities[0].name, "MultiDoc");
    }

    #[test]
    fn test_spec031_multi_doc_entity_excluded_if_no_id_matches() {
        let mut ctx = QueryContext::new();
        ctx.entities = vec![make_entity_multi("CrossDoc", &["doc-x", "doc-z"])];
        let allowed = vec!["doc-a".to_string()];
        filter_context_by_document_ids(&mut ctx, Some(&allowed));
        assert_eq!(ctx.entities.len(), 0);
    }

    #[test]
    fn test_spec031_source_document_ids_takes_priority_over_singular() {
        let entity = RetrievedEntity::new("Conflict", "PERSON", "desc")
            .with_source_document_id("doc-x")
            .with_source_document_ids(vec!["doc-a".to_string()]);
        let mut ctx = QueryContext::new();
        ctx.entities = vec![entity];
        let allowed = vec!["doc-a".to_string()];
        filter_context_by_document_ids(&mut ctx, Some(&allowed));
        assert_eq!(ctx.entities.len(), 1);
    }

    #[test]
    fn test_spec031_singular_fallback_when_plural_empty() {
        let entity =
            RetrievedEntity::new("Fallback", "PERSON", "desc").with_source_document_id("doc-a");
        let mut ctx = QueryContext::new();
        ctx.entities = vec![entity];
        let allowed = vec!["doc-a".to_string()];
        filter_context_by_document_ids(&mut ctx, Some(&allowed));
        assert_eq!(ctx.entities.len(), 1);
    }

    #[test]
    fn test_021_no_lineage_data_dropped_under_scope() {
        let entity = RetrievedEntity::new("Global", "CONCEPT", "global concept");
        let mut ctx = QueryContext::new();
        ctx.entities = vec![entity];
        let allowed = vec!["doc-x".to_string()];
        filter_context_by_document_ids(&mut ctx, Some(&allowed));
        assert_eq!(ctx.entities.len(), 0, "021 L4: unknown provenance drops");
    }

    #[test]
    fn test_021_derive_doc_from_chunk_ids() {
        let entity = RetrievedEntity::new("FromChunk", "ORG", "d")
            .with_source_chunk_ids(vec!["doc-a-chunk-7".into(), "doc-b-chunk-1".into()]);
        let mut ctx = QueryContext::new();
        ctx.entities = vec![entity];
        let allowed = vec!["doc-b".to_string()];
        filter_context_by_document_ids(&mut ctx, Some(&allowed));
        assert_eq!(ctx.entities.len(), 1);
        assert_eq!(ctx.entities[0].name, "FromChunk");
    }

    #[test]
    fn test_021_foreign_chunk_lineage_excluded() {
        let entity = RetrievedEntity::new("Foreign", "ORG", "d")
            .with_source_chunk_ids(vec!["doc-z-chunk-0".into()]);
        let mut ctx = QueryContext::new();
        ctx.entities = vec![entity];
        let allowed = vec!["doc-a".to_string()];
        filter_context_by_document_ids(&mut ctx, Some(&allowed));
        assert_eq!(ctx.entities.len(), 0);
    }

    #[test]
    fn test_spec031_strict_when_any_lineage_present() {
        let entity = make_entity("TypicalEntity", Some("doc-z"));
        let mut ctx = QueryContext::new();
        ctx.entities = vec![entity];
        let allowed = vec!["doc-a".to_string()];
        filter_context_by_document_ids(&mut ctx, Some(&allowed));
        assert_eq!(ctx.entities.len(), 0);
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

    #[test]
    fn spec146_hub_description_drops_unauthorized_fragment() {
        let entity = RetrievedEntity::new(
            "HUB",
            "CONCEPT",
            "public occ<SEP>SECRET_TOKEN_SPEC146_UNAUTHORIZED",
        )
        .with_score(0.8)
        .with_source_chunk_ids(vec!["doc-a-chunk-0".into(), "doc-b-chunk-0".into()])
        .with_source_document_ids(vec!["doc-a".into(), "doc-b".into()]);
        let mut ctx = QueryContext::new();
        ctx.entities = vec![entity];
        let allowed = vec!["doc-a".to_string()];
        filter_context_by_document_ids(&mut ctx, Some(&allowed));
        assert_eq!(ctx.entities.len(), 1);
        assert!(
            !ctx.entities[0].description.contains("SECRET_TOKEN"),
            "unauthorized occurrence leaked: {}",
            ctx.entities[0].description
        );
        assert!(ctx.entities[0].description.contains("public occ"));
    }

    #[test]
    fn test_021_rel_derive_from_source_chunk_id() {
        let rel = RetrievedRelationship::new("A", "B", "REL").with_source_chunk_id("doc-a-chunk-2");
        let mut ctx = QueryContext::new();
        ctx.relationships = vec![rel];
        let allowed = vec!["doc-a".to_string()];
        filter_context_by_document_ids(&mut ctx, Some(&allowed));
        assert_eq!(ctx.relationships.len(), 1);
    }
}
