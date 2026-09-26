//! Document graph lineage builder — SPEC-045 SSOT (SRP/DRY).
//!
//! # First principle
//!
//! Document-scoped **relationships** are edges whose **both endpoints** belong
//! to the document's extracted entity set — even when edge properties lack
//! `source_ids` after graph merge (nodes retain provenance; edges often do not).
//!
//! # SPEC-149 primary path
//!
//! New-data-layer documents are discovered via AGE GIN probes on
//! `source_ids` **and** `source_chunk_ids` (projection canonicalizes both).
//! `chunk_entity_links` / `chunk_relation_links` remain a legacy-only fallback
//! for pre-projection merger writes; durable committer ingest does not populate
//! those tables.

use std::sync::Arc;

use edgequake_storage::traits::{collect_source_references, GraphEdge, GraphNode, GraphStorage};

use edgequake_pipeline::bare_entity_id;
use edgequake_storage::is_opaque_identifier;

use crate::error::ApiResult;
use crate::handlers::graph::graph_node_label;
use crate::handlers::lineage_types::{EntitySummaryResponse, RelationshipSummaryResponse};
use crate::middleware::TenantContext;
use crate::services::{
    find_document_nodes, find_relationships_for_document_lineage, sources_for_document,
    DocumentSourceScope,
};

/// Built entity/relationship summaries for `GET /lineage/documents/:id`.
#[derive(Debug, Clone, Default)]
pub struct DocumentGraphLineageBuild {
    pub entities: Vec<EntitySummaryResponse>,
    pub relationships: Vec<RelationshipSummaryResponse>,
}

/// Map a graph node to a lineage entity summary (document-scoped sources only).
pub fn entity_summary_from_node(
    node: &GraphNode,
    scope: &DocumentSourceScope,
) -> Option<EntitySummaryResponse> {
    let doc_sources = sources_for_document(&node.properties, scope);
    if doc_sources.is_empty() {
        return None;
    }
    let all_sources = collect_source_references(&node.properties);
    let entity_type = node
        .properties
        .get("entity_type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let label = graph_node_label(node);
    let bare = bare_entity_id(&node.id);
    let name = if is_opaque_identifier(bare) {
        label.clone()
    } else {
        bare.to_string()
    };
    let description = node
        .properties
        .get("description")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    Some(EntitySummaryResponse {
        id: node.id.clone(),
        name,
        label,
        entity_type,
        source_chunks: doc_sources,
        is_shared: all_sources.len() > 1,
        description,
    })
}

/// Map a graph edge to a lineage relationship summary.
pub fn relationship_summary_from_edge(
    edge: &GraphEdge,
    scope: &DocumentSourceScope,
) -> RelationshipSummaryResponse {
    let doc_sources = sources_for_document(&edge.properties, scope);
    let keywords = edge
        .properties
        .get("keywords")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    RelationshipSummaryResponse {
        source: edge.source.clone(),
        target: edge.target.clone(),
        keywords,
        source_chunks: doc_sources,
    }
}

/// Build document-scoped graph lineage from AGE + optional PG chunk-link fallback.
pub async fn build_document_graph_lineage(
    graph: &Arc<dyn GraphStorage>,
    tenant_ctx: &TenantContext,
    document_id: &str,
    #[cfg(feature = "postgres")] pg_pool: crate::services::OptionalPgPool<'_>,
) -> ApiResult<DocumentGraphLineageBuild> {
    let scope = DocumentSourceScope::from_document_id(document_id.to_string());

    let entities: Vec<EntitySummaryResponse> = find_document_nodes(graph, Some(tenant_ctx), &scope)
        .await?
        .iter()
        .filter_map(|node| entity_summary_from_node(node, &scope))
        .collect();

    let entity_ids: Vec<String> = entities.iter().map(|e| e.id.clone()).collect();

    let relationships: Vec<RelationshipSummaryResponse> =
        find_relationships_for_document_lineage(graph, Some(tenant_ctx), &scope, &entity_ids)
            .await?
            .iter()
            .map(|edge| relationship_summary_from_edge(edge, &scope))
            .collect();

    let (entities, relationships) = merge_chunk_link_lineage_fallback(
        entities,
        relationships,
        tenant_ctx,
        document_id,
        graph,
        #[cfg(feature = "postgres")]
        pg_pool,
    )
    .await;

    Ok(DocumentGraphLineageBuild {
        entities,
        relationships,
    })
}

async fn merge_chunk_link_lineage_fallback(
    entities: Vec<EntitySummaryResponse>,
    relationships: Vec<RelationshipSummaryResponse>,
    tenant_ctx: &TenantContext,
    document_id: &str,
    graph: &Arc<dyn GraphStorage>,
    #[cfg(feature = "postgres")] pg_pool: crate::services::OptionalPgPool<'_>,
) -> (Vec<EntitySummaryResponse>, Vec<RelationshipSummaryResponse>) {
    #[cfg(feature = "postgres")]
    {
        if entities.is_empty() || relationships.is_empty() {
            if let Some(pool) = pg_pool {
                match super::postgres_chunk_lineage::load_document_lineage_from_chunk_links(
                    pool,
                    tenant_ctx,
                    document_id,
                    graph.as_ref(),
                )
                .await
                {
                    Ok((link_entities, link_relationships)) => {
                        let entities = if entities.is_empty() && !link_entities.is_empty() {
                            link_entities
                        } else {
                            entities
                        };
                        let relationships =
                            if relationships.is_empty() && !link_relationships.is_empty() {
                                link_relationships
                            } else {
                                relationships
                            };
                        return (entities, relationships);
                    }
                    Err(e) => {
                        tracing::warn!(
                            document_id = %document_id,
                            error = %e,
                            "chunk link lineage fallback failed"
                        );
                    }
                }
            }
        }
    }

    (entities, relationships)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn relationship_summary_reads_keywords() {
        let scope = DocumentSourceScope::from_document_id("doc-1");
        let mut props = HashMap::new();
        props.insert("keywords".to_string(), serde_json::json!("uses"));
        let edge = GraphEdge {
            source: "A".into(),
            target: "B".into(),
            properties: props,
        };
        let summary = relationship_summary_from_edge(&edge, &scope);
        assert_eq!(summary.source, "A");
        assert_eq!(summary.target, "B");
        assert_eq!(summary.keywords, "uses");
    }

    #[test]
    fn entity_summary_soft_labels_opaque_uuid() {
        let scope = DocumentSourceScope::from_document_id("doc-1");
        let mut props = HashMap::new();
        props.insert("entity_type".to_string(), serde_json::json!("CONCEPT"));
        props.insert(
            "description".to_string(),
            serde_json::json!("Future of work theme from the agenda"),
        );
        props.insert(
            "source_ids".to_string(),
            serde_json::json!(["doc-1-chunk-0"]),
        );
        props.insert(
            "label".to_string(),
            serde_json::json!("84B69E27-E38B-444A-83DD-5E6A537C6F12"),
        );
        let node = GraphNode {
            id: "84B69E27-E38B-444A-83DD-5E6A537C6F12".into(),
            properties: props,
        };
        let summary = entity_summary_from_node(&node, &scope).expect("summary");
        assert_eq!(summary.id, "84B69E27-E38B-444A-83DD-5E6A537C6F12");
        assert!(
            summary.label.contains("Future of work"),
            "got {}",
            summary.label
        );
        assert!(!summary.label.contains("84B69E27"));
        assert_eq!(summary.name, summary.label);
        assert!(summary.description.is_some());
    }

    #[test]
    fn entity_summary_opaque_without_description_uses_type_badge() {
        let scope = DocumentSourceScope::from_document_id("doc-1");
        let mut props = HashMap::new();
        props.insert("entity_type".to_string(), serde_json::json!("CONCEPT"));
        props.insert(
            "source_ids".to_string(),
            serde_json::json!(["doc-1-chunk-0"]),
        );
        let node = GraphNode {
            id: "84b69e27-e38b-444a-83dd-5e6a537c6f12".into(),
            properties: props,
        };
        let summary = entity_summary_from_node(&node, &scope).expect("summary");
        assert_eq!(summary.label, "Opaque ID · CONCEPT");
        assert_eq!(summary.name, "Opaque ID · CONCEPT");
    }

    #[test]
    fn entity_summary_human_name_keeps_bare_identity() {
        let scope = DocumentSourceScope::from_document_id("doc-1");
        let mut props = HashMap::new();
        props.insert("entity_type".to_string(), serde_json::json!("PERSON"));
        props.insert("label".to_string(), serde_json::json!("SARAH_CHEN"));
        props.insert(
            "source_ids".to_string(),
            serde_json::json!(["doc-1-chunk-0"]),
        );
        let node = GraphNode {
            id: "00000000-0000-0000-0000-000000000003::SARAH_CHEN".into(),
            properties: props,
        };
        let summary = entity_summary_from_node(&node, &scope).expect("summary");
        assert_eq!(
            summary.id,
            "00000000-0000-0000-0000-000000000003::SARAH_CHEN"
        );
        assert_eq!(summary.name, "SARAH_CHEN");
        assert_eq!(summary.label, "SARAH_CHEN");
    }
}
