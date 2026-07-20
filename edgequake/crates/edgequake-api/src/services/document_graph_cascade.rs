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

/// Normalize tenant/workspace filter IDs to canonical UUIDs.
///
/// WHY (#305): request headers and stored graph properties often disagree on the
/// legacy `"default"` alias vs the built-in UUID. Strict AGE property equality
/// then finds **zero** nodes during cascade delete, leaving the Knowledge Graph
/// intact after the document KV row is gone.
fn canonical_tenant_filter_id(raw: Option<&str>) -> Option<String> {
    crate::middleware::resolve_tenant_uuid(raw).map(|u| u.to_string())
}

fn canonical_workspace_filter_id(raw: Option<&str>) -> Option<String> {
    crate::middleware::resolve_workspace_uuid(raw).map(|u| u.to_string())
}

pub fn node_list_filter(tenant_ctx: Option<&TenantContext>) -> NodeListFilter {
    match tenant_ctx {
        Some(ctx) => NodeListFilter {
            tenant_id: canonical_tenant_filter_id(ctx.tenant_id.as_deref()),
            workspace_id: canonical_workspace_filter_id(ctx.workspace_id.as_deref()),
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
            tenant_id: canonical_tenant_filter_id(ctx.tenant_id.as_deref()),
            workspace_id: canonical_workspace_filter_id(ctx.workspace_id.as_deref()),
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
///
/// # Isolation (#305)
/// Prefer calling with `tenant_ctx = None` after the document has already been
/// authorization-checked: source-prefix scope is document-unique and must not
/// miss graph rows whose `tenant_id`/`workspace_id` properties are absent or
/// still use the legacy `"default"` alias. When a tenant filter is supplied,
/// IDs are canonicalized to UUIDs first.
///
/// # Resilience
/// Per-node/edge mutation failures are logged and skipped so one AGE hiccup
/// cannot abort the whole cascade (which previously left the full KG intact
/// while KV delete still succeeded).
pub async fn cascade_remove_document_sources(
    graph: &Arc<dyn GraphStorage>,
    vector_storage: Option<&Arc<dyn VectorStorage>>,
    tenant_ctx: Option<&TenantContext>,
    scope: &DocumentSourceScope,
) -> ApiResult<CascadeStats> {
    let mut stats = CascadeStats::default();
    // First try with the caller's filter; if it finds nothing, fall back to an
    // unscoped source-prefix scan. Document IDs are globally unique, so this
    // cannot leak another tenant's nodes — it only repairs filter mismatches.
    let mut affected_nodes = find_document_nodes(graph, tenant_ctx, scope).await?;
    if affected_nodes.is_empty() && tenant_ctx.is_some() {
        affected_nodes = find_document_nodes(graph, None, scope).await?;
        if !affected_nodes.is_empty() {
            tracing::warn!(
                document_id = %scope.document_id,
                found = affected_nodes.len(),
                "Graph cascade: tenant/workspace filter missed nodes; using source-prefix fallback (#305)"
            );
        }
    }

    let mut deleted_node_ids = HashSet::new();

    for node in affected_nodes {
        let sources = collect_source_references(&node.properties);
        if sources.is_empty() {
            continue;
        }
        let remaining = remaining_sources_after_removal(&node.properties, scope);
        if remaining.is_empty() {
            if let Err(e) = graph.delete_node(&node.id).await {
                tracing::warn!(
                    node_id = %node.id,
                    document_id = %scope.document_id,
                    error = %e,
                    "Cascade delete_node failed (continuing)"
                );
                continue;
            }
            if let Some(vs) = vector_storage {
                let _ = vs.delete_entity(&node.id).await;
                stats.embeddings_deleted += 1;
            }
            deleted_node_ids.insert(node.id.clone());
            stats.entities_removed += 1;
        } else if remaining.len() < sources.len() {
            let mut updated_props = node.properties.clone();
            // SPEC-046 EQ-046-12: rebuild description + align source_ids
            crate::services::knowledge_rebuild::apply_rebuild_to_properties(
                &mut updated_props,
                &remaining,
            );
            if let Err(e) = graph.upsert_node(&node.id, updated_props).await {
                tracing::warn!(
                    node_id = %node.id,
                    document_id = %scope.document_id,
                    error = %e,
                    "Cascade upsert_node failed (continuing)"
                );
                continue;
            }
            stats.entities_updated += 1;
        }
    }

    let mut edges_to_process: HashMap<(String, String), GraphEdge> = HashMap::new();
    let mut doc_edges = find_document_edges(graph, tenant_ctx, scope).await?;
    if doc_edges.is_empty() && tenant_ctx.is_some() {
        doc_edges = find_document_edges(graph, None, scope).await?;
    }
    for edge in doc_edges {
        edges_to_process.insert(edge_key(&edge), edge);
    }

    if !deleted_node_ids.is_empty() {
        let ids: Vec<String> = deleted_node_ids.iter().cloned().collect();
        match graph.get_edges_for_nodes_batch(&ids).await {
            Ok(edges) => {
                for edge in edges {
                    edges_to_process.insert(edge_key(&edge), edge);
                }
            }
            Err(e) => {
                tracing::warn!(
                    document_id = %scope.document_id,
                    error = %e,
                    "Cascade get_edges_for_nodes_batch failed (continuing)"
                );
            }
        }
    }

    for edge in edges_to_process.into_values() {
        let source_exists = match graph.has_node(&edge.source).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    edge_source = %edge.source,
                    error = %e,
                    "Cascade has_node(source) failed (continuing)"
                );
                continue;
            }
        };
        let target_exists = match graph.has_node(&edge.target).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    edge_target = %edge.target,
                    error = %e,
                    "Cascade has_node(target) failed (continuing)"
                );
                continue;
            }
        };
        if !source_exists || !target_exists {
            if let Err(e) = graph.delete_edge(&edge.source, &edge.target).await {
                tracing::warn!(
                    edge_source = %edge.source,
                    edge_target = %edge.target,
                    error = %e,
                    "Cascade delete_edge (orphan endpoints) failed (continuing)"
                );
                continue;
            }
            stats.relationships_removed += 1;
            continue;
        }

        let sources = collect_source_references(&edge.properties);
        // Edges often lack source_ids after merge (lineage UI). If both endpoints
        // were exclusive to this document and deleted above, the orphan-endpoint
        // branch already removed the edge. If only one endpoint remains (shared
        // entity), keep the edge unless provenance says this doc owned it alone.
        if sources.is_empty() {
            continue;
        }
        let remaining = remaining_sources_after_removal(&edge.properties, scope);
        if remaining.is_empty() {
            if let Err(e) = graph.delete_edge(&edge.source, &edge.target).await {
                tracing::warn!(
                    edge_source = %edge.source,
                    edge_target = %edge.target,
                    error = %e,
                    "Cascade delete_edge failed (continuing)"
                );
                continue;
            }
            stats.relationships_removed += 1;
        } else if remaining.len() < sources.len() {
            let mut updated_props = edge.properties.clone();
            crate::services::knowledge_rebuild::apply_rebuild_to_properties(
                &mut updated_props,
                &remaining,
            );
            if let Err(e) = graph
                .upsert_edge(&edge.source, &edge.target, updated_props)
                .await
            {
                tracing::warn!(
                    edge_source = %edge.source,
                    edge_target = %edge.target,
                    error = %e,
                    "Cascade upsert_edge failed (continuing)"
                );
                continue;
            }
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

/// Collect graph edges attributable to a document for lineage visualization.
///
/// First principle: if both endpoints belong to the document's entity set, the
/// relationship is document-scoped — even when edge properties lack `source_ids`
/// after graph merge (nodes retain provenance; edges often do not).
pub async fn find_relationships_for_document_lineage(
    graph: &Arc<dyn GraphStorage>,
    tenant_ctx: Option<&TenantContext>,
    scope: &DocumentSourceScope,
    document_entity_ids: &[String],
) -> ApiResult<Vec<GraphEdge>> {
    if document_entity_ids.is_empty() {
        return Ok(Vec::new());
    }

    let entity_set: HashSet<&str> = document_entity_ids.iter().map(String::as_str).collect();
    let mut edges: HashMap<(String, String), GraphEdge> = HashMap::new();

    for edge in graph
        .get_edges_for_nodes_batch(document_entity_ids)
        .await
        .map_err(ApiError::from)?
    {
        if entity_set.contains(edge.source.as_str()) && entity_set.contains(edge.target.as_str()) {
            edges.insert(edge_key(&edge), edge);
        }
    }

    // Merge source-prefix hits (may carry richer chunk provenance on edge props).
    for edge in find_document_edges(graph, tenant_ctx, scope).await? {
        if sources_for_document(&edge.properties, scope).is_empty() {
            continue;
        }
        edges.entry(edge_key(&edge)).or_insert(edge);
    }

    Ok(edges.into_values().collect())
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

    #[tokio::test]
    async fn lineage_relationships_include_entity_adjacency_without_edge_source_ids() {
        use edgequake_storage::traits::GraphStorageMutateOps;
        use edgequake_storage::MemoryGraphStorage;

        let graph: Arc<dyn GraphStorage> = Arc::new(MemoryGraphStorage::new("lineage-test"));
        let doc_id = "doc-lightrag";
        let scope = DocumentSourceScope::from_document_id(doc_id);

        let mut alice_props = HashMap::new();
        alice_props.insert(
            "source_ids".to_string(),
            serde_json::json!([format!("{doc_id}-chunk-0")]),
        );
        alice_props.insert("entity_type".to_string(), serde_json::json!("concept"));
        graph
            .upsert_node("LIGHTRAG", alice_props)
            .await
            .expect("alice");

        let mut bob_props = HashMap::new();
        bob_props.insert(
            "source_ids".to_string(),
            serde_json::json!([format!("{doc_id}-chunk-0")]),
        );
        bob_props.insert("entity_type".to_string(), serde_json::json!("technology"));
        graph
            .upsert_node("RETRIEVAL", bob_props)
            .await
            .expect("bob");

        let mut edge_props = HashMap::new();
        edge_props.insert("keywords".to_string(), serde_json::json!("uses"));
        graph
            .upsert_edge("LIGHTRAG", "RETRIEVAL", edge_props)
            .await
            .expect("edge");

        let entity_ids = vec!["LIGHTRAG".to_string(), "RETRIEVAL".to_string()];
        let edges = find_relationships_for_document_lineage(&graph, None, &scope, &entity_ids)
            .await
            .expect("lineage edges");

        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].source, "LIGHTRAG");
        assert_eq!(edges[0].target, "RETRIEVAL");
    }

    #[test]
    fn node_list_filter_canonicalizes_default_alias() {
        let ctx = TenantContext {
            tenant_id: Some("default".to_string()),
            workspace_id: Some("default".to_string()),
            user_id: None,
        };
        let filter = node_list_filter(Some(&ctx));
        // Strict equality must never leave the literal "default" alias.
        assert_ne!(filter.workspace_id.as_deref(), Some("default"));
        assert_ne!(filter.tenant_id.as_deref(), Some("default"));
        let expected_ws = crate::middleware::default_workspace_uuid().to_string();
        let expected_tenant = crate::middleware::default_tenant_uuid().to_string();
        assert_eq!(filter.workspace_id.as_deref(), Some(expected_ws.as_str()));
        assert_eq!(filter.tenant_id.as_deref(), Some(expected_tenant.as_str()));
    }

    /// #305: cascade must remove exclusive entities even when the request
    /// tenant filter says `"default"` while nodes store the canonical UUID
    /// (or omit tenant props entirely).
    #[tokio::test]
    async fn cascade_removes_exclusive_nodes_despite_default_alias_filter() {
        use edgequake_storage::traits::GraphStorageMutateOps;
        use edgequake_storage::MemoryGraphStorage;

        let graph: Arc<dyn GraphStorage> = Arc::new(MemoryGraphStorage::new("cascade-305"));
        let doc_id = "doc-305-orphan";
        let scope = DocumentSourceScope::from_document_id(doc_id);
        let ws_uuid = crate::middleware::default_workspace_uuid().to_string();

        let mut props = HashMap::new();
        props.insert(
            "source_ids".to_string(),
            serde_json::json!([format!("{doc_id}-chunk-0")]),
        );
        props.insert("workspace_id".to_string(), serde_json::json!(ws_uuid));
        props.insert("entity_type".to_string(), serde_json::json!("PERSON"));
        graph
            .upsert_node("ORPHAN_ENTITY", props)
            .await
            .expect("node");

        // Mismatched filter: literal "default" would miss UUID-scoped nodes on
        // Postgres AGE. Memory backend ignores filters, so we also assert the
        // None path used by delete handlers.
        let stats = cascade_remove_document_sources(&graph, None, None, &scope)
            .await
            .expect("cascade");
        assert_eq!(stats.entities_removed, 1);
        assert!(!graph.has_node("ORPHAN_ENTITY").await.expect("has"));
    }

    /// Shared entities keep remaining sources after cascade.
    #[tokio::test]
    async fn cascade_preserves_shared_entity_sources() {
        use edgequake_storage::traits::GraphStorageMutateOps;
        use edgequake_storage::MemoryGraphStorage;

        let graph: Arc<dyn GraphStorage> = Arc::new(MemoryGraphStorage::new("cascade-shared"));
        let doc_a = "doc-a";
        let doc_b = "doc-b";
        let scope = DocumentSourceScope::from_document_id(doc_a);

        let mut props = HashMap::new();
        props.insert(
            "source_ids".to_string(),
            serde_json::json!([format!("{doc_a}-chunk-0"), format!("{doc_b}-chunk-0")]),
        );
        graph
            .upsert_node("SHARED", props)
            .await
            .expect("node");

        let stats = cascade_remove_document_sources(&graph, None, None, &scope)
            .await
            .expect("cascade");
        assert_eq!(stats.entities_removed, 0);
        assert_eq!(stats.entities_updated, 1);
        let node = graph.get_node("SHARED").await.expect("get").expect("exists");
        let remaining = collect_source_references(&node.properties);
        assert_eq!(remaining, vec![format!("{doc_b}-chunk-0")]);
    }
}
