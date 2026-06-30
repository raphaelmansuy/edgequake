//! Map structured retrieval subgraph → chat `MessageContext` (SPEC-028 FP-028-11 DRY).

use edgequake_core::types::{
    MessageContext, MessageContextEntity, MessageContextRelationship, MessageSource,
};
use edgequake_query::QueryContext;

use crate::handlers::context_types::{ContentGranularity, SubgraphBundle};
use crate::handlers::query_types::SourceReference;
use crate::services::context_bundle_mapper::{map_query_context_to_subgraph, MappingOptions};

/// Build chat message context from engine retrieval (no lossy flat-source re-parsing).
pub fn build_message_context_from_engine(
    context: &QueryContext,
    chunk_sources: &[SourceReference],
) -> MessageContext {
    let subgraph = map_query_context_to_subgraph(
        context,
        &MappingOptions {
            granularity: ContentGranularity::Citation,
            include_lineage: true,
            include_documents: false,
            include_agent_hints: false,
            include_subgraph: true,
            rerank_top_k: None,
            reranked: false,
        },
    );
    message_context_from_subgraph(&subgraph, chunk_sources)
}

/// Map pre-built subgraph + chunk citation sources to chat `MessageContext`.
pub fn message_context_from_subgraph(
    subgraph: &SubgraphBundle,
    chunk_sources: &[SourceReference],
) -> MessageContext {
    MessageContext {
        sources: chunk_sources
            .iter()
            .filter(|s| s.source_type == "chunk")
            .map(|s| MessageSource {
                id: s.id.clone(),
                title: s.file_path.clone().or_else(|| s.document_id.clone()),
                content: Some(s.snippet.clone().unwrap_or_default()),
                score: s.score,
                document_id: s.document_id.clone(),
                // SPEC-033 FP-033-1: Persist page attribution so the citation panel
                // can group passages by page even after conversation reload.
                // Without these fields, page_start is silently dropped when the
                // MessageContext is serialised to the database, breaking page
                // grouping for all conversations viewed after streaming completes.
                source_type: Some(s.source_type.clone()),
                page_start: s.page_start,
                page_end: s.page_end,
            })
            .collect(),
        entities: subgraph
            .entities
            .iter()
            .map(|e| MessageContextEntity {
                name: e.name.clone(),
                entity_type: e.entity_type.clone(),
                description: Some(e.description.clone()),
                score: e.score,
                source_document_id: e
                    .lineage
                    .as_ref()
                    .and_then(|l| l.source_document_id.clone()),
                source_file_path: e.lineage.as_ref().and_then(|l| l.source_file_path.clone()),
                source_chunk_ids: e
                    .lineage
                    .as_ref()
                    .map(|l| l.source_chunk_ids.clone())
                    .unwrap_or_default(),
            })
            .collect(),
        relationships: subgraph
            .relationships
            .iter()
            .map(|r| MessageContextRelationship {
                source: r.source.clone(),
                target: r.target.clone(),
                relation_type: r.relation_type.clone(),
                description: Some(r.description.clone()),
                score: r.score,
                source_document_id: r
                    .lineage
                    .as_ref()
                    .and_then(|l| l.source_document_id.clone()),
                source_file_path: r.lineage.as_ref().and_then(|l| l.source_file_path.clone()),
            })
            .collect(),
    }
}

// ============================================================================
// SPEC-033 FP-033-1: Unit tests for page attribution persistence
// ============================================================================

#[cfg(test)]
mod tests {
    use crate::handlers::context_types::SubgraphBundle;
    use crate::handlers::query_types::SourceReference;

    use super::*;

    fn make_chunk_source(page_start: Option<u32>) -> SourceReference {
        SourceReference {
            source_type: "chunk".to_string(),
            id: "doc-1-chunk-0".to_string(),
            score: 0.9,
            rerank_score: None,
            snippet: Some("test content".to_string()),
            reference_id: Some(1),
            document_id: Some("doc-1".to_string()),
            file_path: Some("test.pdf".to_string()),
            start_line: None,
            end_line: None,
            chunk_index: Some(0),
            entity_type: None,
            degree: None,
            source_chunk_ids: None,
            page_start,
            page_end: page_start, // always equals page_start
        }
    }

    /// SPEC-033 FP-033-1: page_start IS preserved in the persisted MessageContext.
    /// This is the critical regression test — before the fix, page_start was
    /// silently dropped here, causing flat citation rendering after reload.
    #[test]
    fn page_start_is_persisted_in_message_context() {
        let sources = vec![make_chunk_source(Some(7))];
        let ctx = message_context_from_subgraph(&SubgraphBundle::default(), &sources);

        assert_eq!(ctx.sources.len(), 1);
        assert_eq!(
            ctx.sources[0].page_start,
            Some(7),
            "page_start must survive message_context_from_subgraph to prevent flat citations after reload"
        );
        assert_eq!(ctx.sources[0].page_end, Some(7));
    }

    /// SPEC-033: source_type is persisted so the frontend can filter chunk sources.
    #[test]
    fn source_type_is_persisted_in_message_context() {
        let sources = vec![make_chunk_source(Some(3))];
        let ctx = message_context_from_subgraph(&SubgraphBundle::default(), &sources);

        assert_eq!(
            ctx.sources[0].source_type.as_deref(),
            Some("chunk"),
            "source_type must be persisted for correct filtering in the frontend"
        );
    }

    /// Non-PDF chunks (no page_start) produce None in MessageSource — no regression.
    #[test]
    fn page_start_absent_for_non_pdf_chunks() {
        let sources = vec![make_chunk_source(None)];
        let ctx = message_context_from_subgraph(&SubgraphBundle::default(), &sources);

        assert_eq!(ctx.sources.len(), 1);
        assert_eq!(
            ctx.sources[0].page_start, None,
            "Non-PDF chunks should produce None page_start"
        );
    }

    /// Multiple chunks from different pages are all persisted correctly.
    #[test]
    fn multiple_pages_all_persisted() {
        let sources = vec![
            make_chunk_source(Some(1)),
            make_chunk_source(Some(5)),
            make_chunk_source(Some(12)),
        ];
        let ctx = message_context_from_subgraph(&SubgraphBundle::default(), &sources);

        let pages: Vec<Option<u32>> = ctx.sources.iter().map(|s| s.page_start).collect();
        assert_eq!(pages, vec![Some(1), Some(5), Some(12)]);
    }
}
