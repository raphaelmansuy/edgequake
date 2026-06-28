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
