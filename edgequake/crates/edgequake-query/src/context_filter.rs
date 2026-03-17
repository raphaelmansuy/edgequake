//! Post-retrieval context filtering by document IDs.
//!
//! Filters a `QueryContext` to only include items from allowed documents.
//! Applied after vector search / mode-specific retrieval but BEFORE
//! truncation and LLM answer generation.
//!
//! @implements SPEC-005: Document date and pattern filters (Tier 1)

use std::collections::HashSet;

use crate::context::QueryContext;

/// Filter a `QueryContext` to only keep items from the allowed document set.
///
/// - **Chunks**: kept if `document_id` is in `allowed_ids`; excluded if missing.
/// - **Entities**: kept if `source_document_id` is in `allowed_ids` or absent
///   (entities without provenance are kept to preserve cross-document knowledge).
/// - **Relationships**: same rule as entities.
///
/// This is a no-op when `allowed_ids` is `None` (no filter active).
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

    // Entities: lenient — keep if source_document_id matches OR is absent
    context.entities.retain(|entity| {
        entity
            .source_document_id
            .as_deref()
            .map(|id| id_set.contains(id))
            .unwrap_or(true)
    });

    // Relationships: lenient — same as entities
    context.relationships.retain(|rel| {
        rel.source_document_id
            .as_deref()
            .map(|id| id_set.contains(id))
            .unwrap_or(true)
    });
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

    fn make_relationship(src: &str, tgt: &str, doc_id: Option<&str>) -> RetrievedRelationship {
        let mut rel = RetrievedRelationship::new(src, tgt, "KNOWS").with_score(0.7);
        if let Some(d) = doc_id {
            rel = rel.with_source_document_id(d);
        }
        rel
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

        // Entities: Alice (doc-a), Bob (doc-b), Charlie (no provenance) kept
        assert_eq!(ctx.entities.len(), 3);

        // Relationships: Alice→Bob (doc-a) kept, Bob→Charlie (doc-c) excluded, X→Y (no prov) kept
        assert_eq!(ctx.relationships.len(), 2);
    }

    #[test]
    fn test_empty_filter_removes_all_chunks() {
        let mut ctx = sample_context();
        let allowed: Vec<String> = vec![];

        filter_context_by_document_ids(&mut ctx, Some(&allowed));

        // All chunks removed (none match empty set)
        assert_eq!(ctx.chunks.len(), 0);

        // Entities without provenance still kept
        assert_eq!(ctx.entities.len(), 1); // only Charlie (no doc_id)
        assert_eq!(ctx.entities[0].name, "Charlie");

        // Relationships without provenance still kept
        assert_eq!(ctx.relationships.len(), 1); // only X→Y
    }

    #[test]
    fn test_filter_single_document() {
        let mut ctx = sample_context();
        let allowed = vec!["doc-c".to_string()];

        filter_context_by_document_ids(&mut ctx, Some(&allowed));

        assert_eq!(ctx.chunks.len(), 1);
        assert_eq!(ctx.chunks[0].document_id.as_deref(), Some("doc-c"));

        // Entities: Charlie (no prov) kept; Alice (doc-a) and Bob (doc-b) excluded
        assert_eq!(ctx.entities.len(), 1);

        // Relationships: Bob→Charlie (doc-c) kept; Alice→Bob (doc-a) excluded; X→Y (no prov) kept
        assert_eq!(ctx.relationships.len(), 2);
    }
}
