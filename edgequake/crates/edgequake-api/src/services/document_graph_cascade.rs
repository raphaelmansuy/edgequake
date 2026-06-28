//! Document-scoped graph cascade — SPEC-006 P1 (SRP/DRY).
//!
//! Bounded graph mutations and lineage reads keyed by document source prefixes.
//! Never loads the full workspace graph into handler memory.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use edgequake_storage::traits::{
    collect_source_references, EdgeListFilter, GraphEdge, GraphNode, GraphStorage, NodeListFilter,
    VectorStorage,
};

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;

/// Document source scope for bounded graph operations.
#[derive(Debug, Clone)]
pub struct DocumentSourceScope {
    pub document_id: String,
    pub key_prefix: String,
    pub source_prefixes: Vec<String>,
}

impl DocumentSourceScope {
    pub fn from_document_id(document_id: impl Into<String>) -> Self {
        let document_id = document_id.into();
        Self {
            key_prefix: document_id.clone(),
            source_prefixes: vec![document_id.clone()],
            document_id,
        }
    }

    pub fn with_key_prefix(document_id: String, key_prefix: String) -> Self {
        let source_prefixes = if key_prefix != document_id {
            vec![key_prefix.clone(), document_id.clone()]
        } else {
            vec![document_id.clone()]
        };
        Self {
            document_id,
            key_prefix,
            source_prefixes,
        }
    }

    pub fn chunk_prefix(&self) -> String {
        format!("{}-chunk-", self.key_prefix)
    }
}

/// Statistics from cascade remove or impact analysis.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CascadeStats {
    pub entities_removed: usize,
    pub entities_updated: usize,
    pub relationships_removed: usize,
    pub relationships_updated: usize,
    pub embeddings_deleted: usize,
}

pub fn node_list_filter(tenant_ctx: Option<&TenantContext>) -> NodeListFilter {
    match tenant_ctx {
        Some(ctx) => NodeListFilter {
            tenant_id: ctx.tenant_id.clone(),
            workspace_id: ctx.workspace_id.clone(),
            entity_type: None,
            search: None,
            community_ids: None,
        },
        None => NodeListFilter::default(),
    }
}

pub fn edge_list_filter(tenant_ctx: Option<&TenantContext>) -> EdgeListFilter {
    match tenant_ctx {
        Some(ctx) => EdgeListFilter {
            tenant_id: ctx.tenant_id.clone(),
            workspace_id: ctx.workspace_id.clone(),
            relationship_type: None,
        },
        None => EdgeListFilter::default(),
    }
}

pub fn source_belongs_to_document(source: &str, scope: &DocumentSourceScope) -> bool {
    scope.source_prefixes.iter().any(|p| {
        source.starts_with(p.as_str())
            || source.starts_with(&edgequake_storage::kv_keys::doc_chunk_prefix(p))
            || source == p.as_str()
    })
}

pub fn remaining_sources_after_removal(
    properties: &HashMap<String, serde_json::Value>,
    scope: &DocumentSourceScope,
) -> Vec<String> {
    collect_source_references(properties)
        .into_iter()
        .filter(|s| !source_belongs_to_document(s, scope))
        .collect()
}

pub fn sources_for_document(
    properties: &HashMap<String, serde_json::Value>,
    scope: &DocumentSourceScope,
) -> Vec<String> {
    collect_source_references(properties)
        .into_iter()
        .filter(|s| source_belongs_to_document(s, scope))
        .collect()
}

/// Find nodes whose sources reference this document (bounded push-down).
pub async fn find_document_nodes(
    graph: &Arc<dyn GraphStorage>,
    tenant_ctx: Option<&TenantContext>,
    scope: &DocumentSourceScope,
) -> ApiResult<Vec<GraphNode>> {
    let filter = node_list_filter(tenant_ctx);
    graph
        .find_nodes_by_source_prefixes(&filter, &scope.source_prefixes)
        .await
        .map_err(ApiError::from)
}

/// Find edges whose sources reference this document (bounded push-down).
pub async fn find_document_edges(
    graph: &Arc<dyn GraphStorage>,
    tenant_ctx: Option<&TenantContext>,
    scope: &DocumentSourceScope,
) -> ApiResult<Vec<GraphEdge>> {
    let filter = edge_list_filter(tenant_ctx);
    graph
        .find_edges_by_source_prefixes(&filter, &scope.source_prefixes)
        .await
        .map_err(ApiError::from)
}

fn edge_key(edge: &GraphEdge) -> (String, String) {
    (edge.source.clone(), edge.target.clone())
}

/// Cascade remove document sources from graph entities and relationships.
pub async fn cascade_remove_document_sources(
    graph: &Arc<dyn GraphStorage>,
    vector_storage: Option<&Arc<dyn VectorStorage>>,
    tenant_ctx: Option<&TenantContext>,
    scope: &DocumentSourceScope,
) -> ApiResult<CascadeStats> {
    let mut stats = CascadeStats::default();
    let affected_nodes = find_document_nodes(graph, tenant_ctx, scope).await?;

    let mut deleted_node_ids = HashSet::new();

    for node in affected_nodes {
        let sources = collect_source_references(&node.properties);
        if sources.is_empty() {
            continue;
        }
        let remaining = remaining_sources_after_removal(&node.properties, scope);
        if remaining.is_empty() {
            graph.delete_node(&node.id).await.map_err(ApiError::from)?;
            if let Some(vs) = vector_storage {
                let _ = vs.delete_entity(&node.id).await;
                stats.embeddings_deleted += 1;
            }
            deleted_node_ids.insert(node.id.clone());
            stats.entities_removed += 1;
        } else if remaining.len() < sources.len() {
            let mut updated_props = node.properties.clone();
            updated_props.insert("source_ids".to_string(), serde_json::json!(remaining));
            // Legacy pipe-separated source_id must not shadow updated source_ids.
            updated_props.remove("source_id");
            graph
                .upsert_node(&node.id, updated_props)
                .await
                .map_err(ApiError::from)?;
            stats.entities_updated += 1;
        }
    }

    let mut edges_to_process: HashMap<(String, String), GraphEdge> = HashMap::new();
    for edge in find_document_edges(graph, tenant_ctx, scope).await? {
        edges_to_process.insert(edge_key(&edge), edge);
    }

    if !deleted_node_ids.is_empty() {
        let ids: Vec<String> = deleted_node_ids.iter().cloned().collect();
        for edge in graph
            .get_edges_for_nodes_batch(&ids)
            .await
            .map_err(ApiError::from)?
        {
            edges_to_process.insert(edge_key(&edge), edge);
        }
    }

    for edge in edges_to_process.into_values() {
        let source_exists = graph.has_node(&edge.source).await.map_err(ApiError::from)?;
        let target_exists = graph.has_node(&edge.target).await.map_err(ApiError::from)?;
        if !source_exists || !target_exists {
            graph
                .delete_edge(&edge.source, &edge.target)
                .await
                .map_err(ApiError::from)?;
            stats.relationships_removed += 1;
            continue;
        }

        let sources = collect_source_references(&edge.properties);
        if sources.is_empty() {
            continue;
        }
        let remaining = remaining_sources_after_removal(&edge.properties, scope);
        if remaining.is_empty() {
            graph
                .delete_edge(&edge.source, &edge.target)
                .await
                .map_err(ApiError::from)?;
            stats.relationships_removed += 1;
        } else if remaining.len() < sources.len() {
            let mut updated_props = edge.properties.clone();
            updated_props.insert("source_ids".to_string(), serde_json::json!(remaining));
            updated_props.remove("source_id");
            graph
                .upsert_edge(&edge.source, &edge.target, updated_props)
                .await
                .map_err(ApiError::from)?;
            stats.relationships_updated += 1;
        }
    }

    Ok(stats)
}

/// Read-only impact preview (same bounded scope as cascade).
pub async fn analyze_deletion_impact_stats(
    graph: &Arc<dyn GraphStorage>,
    tenant_ctx: Option<&TenantContext>,
    scope: &DocumentSourceScope,
) -> ApiResult<CascadeStats> {
    let mut stats = CascadeStats::default();

    for node in find_document_nodes(graph, tenant_ctx, scope).await? {
        let sources = collect_source_references(&node.properties);
        if sources.is_empty() {
            continue;
        }
        let remaining = remaining_sources_after_removal(&node.properties, scope);
        if remaining.is_empty() {
            stats.entities_removed += 1;
        } else if remaining.len() < sources.len() {
            stats.entities_updated += 1;
        }
    }

    for edge in find_document_edges(graph, tenant_ctx, scope).await? {
        let sources = collect_source_references(&edge.properties);
        if sources.is_empty() {
            continue;
        }
        let remaining = remaining_sources_after_removal(&edge.properties, scope);
        if remaining.is_empty() {
            stats.relationships_removed += 1;
        } else if remaining.len() < sources.len() {
            stats.relationships_updated += 1;
        }
    }

    Ok(stats)
}

/// Statistics from document graph data cleanup.
#[derive(Debug, Default, Clone)]
pub struct CleanupStats {
    pub entities_removed: usize,
    pub entities_updated: usize,
    pub relationships_removed: usize,
    pub relationships_updated: usize,
    pub embeddings_deleted: usize,
}

/// Clean up graph data for a document without deleting KV entries.
pub async fn cleanup_document_graph_data(
    document_id: &str,
    graph_storage: &Arc<dyn GraphStorage>,
    vector_storage: Option<&Arc<dyn VectorStorage>>,
) -> ApiResult<CleanupStats> {
    let scope = DocumentSourceScope::from_document_id(document_id);
    let cascade_stats =
        cascade_remove_document_sources(graph_storage, vector_storage, None, &scope).await?;

    tracing::info!(
        document_id = %document_id,
        entities_removed = cascade_stats.entities_removed,
        entities_updated = cascade_stats.entities_updated,
        relationships_removed = cascade_stats.relationships_removed,
        relationships_updated = cascade_stats.relationships_updated,
        embeddings_deleted = cascade_stats.embeddings_deleted,
        "Document graph data cleanup completed"
    );

    Ok(CleanupStats {
        entities_removed: cascade_stats.entities_removed,
        entities_updated: cascade_stats.entities_updated,
        relationships_removed: cascade_stats.relationships_removed,
        relationships_updated: cascade_stats.relationships_updated,
        embeddings_deleted: cascade_stats.embeddings_deleted,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_belongs_matches_chunk_and_doc_id() {
        let scope = DocumentSourceScope::from_document_id("doc-abc");
        assert!(source_belongs_to_document("doc-abc", &scope));
        assert!(source_belongs_to_document("doc-abc-chunk-0", &scope));
        assert!(!source_belongs_to_document("other-doc", &scope));
    }

    #[test]
    fn remaining_sources_filters_document_refs() {
        let scope = DocumentSourceScope::from_document_id("doc-1");
        let mut props = HashMap::new();
        props.insert(
            "source_ids".to_string(),
            serde_json::json!(["doc-1-chunk-0", "doc-2-chunk-1"]),
        );
        let remaining = remaining_sources_after_removal(&props, &scope);
        assert_eq!(remaining, vec!["doc-2-chunk-1"]);
    }

    #[test]
    fn legacy_pipe_source_id_matches_document_scope() {
        let scope = DocumentSourceScope::from_document_id("doc-legacy");
        let mut props = HashMap::new();
        props.insert(
            "source_id".to_string(),
            serde_json::json!("doc-legacy-chunk-0|other-doc-chunk-1"),
        );
        let remaining = remaining_sources_after_removal(&props, &scope);
        assert_eq!(remaining, vec!["other-doc-chunk-1"]);
    }

    #[test]
    fn key_prefix_scope_includes_both_prefixes() {
        let scope = DocumentSourceScope::with_key_prefix(
            "doc-uuid".to_string(),
            "kv-key-prefix".to_string(),
        );
        assert_eq!(scope.source_prefixes.len(), 2);
        assert!(source_belongs_to_document("kv-key-prefix-chunk-0", &scope));
        assert!(source_belongs_to_document("doc-uuid-chunk-1", &scope));
    }
}
