//! Bounded graph scan operations — SPEC-006 TR-006-001 (ISP).

use async_trait::async_trait;
use std::collections::HashMap;

use crate::error::Result;

use super::graph::{GraphEdge, GraphNode};

/// Returns true when node properties match strict tenant/workspace filter.
pub fn node_matches_tenant_workspace(
    properties: &HashMap<String, serde_json::Value>,
    tenant_id: Option<&str>,
    workspace_id: Option<&str>,
) -> bool {
    let (Some(tid), Some(wid)) = (tenant_id, workspace_id) else {
        return false;
    };
    let prop_tid = properties.get("tenant_id").and_then(|v| v.as_str());
    let prop_wid = properties.get("workspace_id").and_then(|v| v.as_str());
    matches!((prop_tid, prop_wid), (Some(t), Some(w)) if t == tid && w == wid)
}

/// Returns true when edge properties match strict tenant/workspace filter.
pub fn edge_matches_tenant_workspace(
    properties: &HashMap<String, serde_json::Value>,
    tenant_id: Option<&str>,
    workspace_id: Option<&str>,
) -> bool {
    node_matches_tenant_workspace(properties, tenant_id, workspace_id)
}

/// SPEC-058: match SQL `edge_and_clause` Strict — each provided dimension must
/// equal the property (missing property → exclude). Allows tenant-only or
/// workspace-only filters used by RAG expand.
pub fn edge_matches_scope_dims(
    properties: &HashMap<String, serde_json::Value>,
    tenant_id: Option<&str>,
    workspace_id: Option<&str>,
) -> bool {
    if let Some(tid) = tenant_id {
        match properties.get("tenant_id").and_then(|v| v.as_str()) {
            Some(t) if t == tid => {}
            _ => return false,
        }
    }
    if let Some(wid) = workspace_id {
        match properties.get("workspace_id").and_then(|v| v.as_str()) {
            Some(w) if w == wid => {}
            _ => return false,
        }
    }
    true
}

/// Per-dimension scope match with legacy-null compatibility.
///
/// Missing filter dimension = wildcard. When a dimension is set, property may be
/// absent/NULL (legacy rows) or equal; an explicit *different* value never matches.
/// Used by list/discovery filters (issue #305 workspace-only discovery).
pub fn scope_dim_matches_legacy_null(
    properties: &HashMap<String, serde_json::Value>,
    key: &str,
    expected: Option<&str>,
) -> bool {
    let Some(exp) = expected else {
        return true;
    };
    match properties.get(key).and_then(|v| v.as_str()) {
        None => true,
        Some(actual) => actual == exp,
    }
}

/// Returns true when a node satisfies list filter criteria.
pub fn node_matches_list_filter(node: &GraphNode, filter: &NodeListFilter) -> bool {
    if !scope_dim_matches_legacy_null(&node.properties, "tenant_id", filter.tenant_id.as_deref()) {
        return false;
    }
    if !scope_dim_matches_legacy_null(
        &node.properties,
        "workspace_id",
        filter.workspace_id.as_deref(),
    ) {
        return false;
    }
    if let Some(ref entity_type) = filter.entity_type {
        let node_type = node
            .properties
            .get("entity_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !node_type.eq_ignore_ascii_case(entity_type) {
            return false;
        }
    }
    if let Some(ref search) = filter.search {
        let search_lower = search.to_lowercase();
        let name_matches = node.id.to_lowercase().contains(&search_lower);
        let desc_matches = node
            .properties
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase()
            .contains(&search_lower);
        if !name_matches && !desc_matches {
            return false;
        }
    }
    if let Some(ref community_ids) = filter.community_ids {
        let Some(cid) = node.properties.get("community_id").and_then(|v| v.as_u64()) else {
            return false;
        };
        if !community_ids.contains(&cid) {
            return false;
        }
    }
    true
}

/// Returns true when an edge satisfies list filter criteria.
pub fn edge_matches_list_filter(edge: &GraphEdge, filter: &EdgeListFilter) -> bool {
    if !scope_dim_matches_legacy_null(&edge.properties, "tenant_id", filter.tenant_id.as_deref()) {
        return false;
    }
    if !scope_dim_matches_legacy_null(
        &edge.properties,
        "workspace_id",
        filter.workspace_id.as_deref(),
    ) {
        return false;
    }
    if let Some(ref rel_type) = filter.relationship_type {
        let edge_type = edge
            .properties
            .get("relation_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !edge_type.eq_ignore_ascii_case(rel_type) {
            return false;
        }
    }
    true
}

/// True when a string looks like a graph endpoint / entity id (`workspace::NAME`),
/// not document provenance. SPEC-098 Symptom F: edge property `source_id` is the
/// start-node id and must never enter cascade remaining-sources.
pub fn is_topology_entity_ref(value: &str) -> bool {
    value.contains("::")
}

/// Loose UUID shape (`8-4-4-4-12` hex) used for bare `source_document_id` values.
fn looks_like_document_uuid(value: &str) -> bool {
    let b = value.as_bytes();
    if b.len() != 36 {
        return false;
    }
    let is_hex = |c: u8| c.is_ascii_hexdigit();
    let dash = |i: usize| b[i] == b'-';
    b.iter().enumerate().all(|(i, &c)| match i {
        8 | 13 | 18 | 23 => dash(i),
        _ => is_hex(c),
    })
}

fn push_provenance_ref(refs: &mut Vec<String>, value: &str) {
    let trimmed = value.trim();
    if trimmed.is_empty() || is_topology_entity_ref(trimmed) {
        return;
    }
    refs.push(trimmed.to_string());
}

/// Collect **document provenance** refs from node/edge properties (SPEC-045 / SPEC-098).
///
/// SSOT for cascade delete, post-proof, and analytics reconcile.
///
/// Includes: `source_ids`, `source_chunk_ids`, singular `source_chunk_id`,
/// singular `source_document_id`, and legacy **node** pipe-joined `source_id`
/// when provenance-shaped.
///
/// Does **not** fold `source_document_ids[]` into the chunk-slot list — that
/// array is maintained separately in rebuild (avoids writing bare doc UUIDs
/// into `source_ids`). Singular `source_document_id` is still collected so
/// orphan citation rows remain cascade-visible.
///
/// Never treats edge topology `source_id` / `target_id` (`workspace::ENTITY`) as
/// provenance — that bug poisoned arrays and blocked exclusive edge delete.
pub fn collect_source_references(properties: &HashMap<String, serde_json::Value>) -> Vec<String> {
    let mut refs = Vec::new();

    // Legacy node field only — edge topology `source_id` is `ws::ENTITY` and skipped.
    if let Some(source_id) = properties.get("source_id").and_then(|v| v.as_str()) {
        let pipe_joined = source_id.contains('|');
        for part in source_id.split('|') {
            let part = part.trim();
            if part.is_empty() || is_topology_entity_ref(part) {
                continue;
            }
            if pipe_joined || part.contains("-chunk-") || looks_like_document_uuid(part) {
                refs.push(part.to_string());
            }
        }
    }

    for key in ["source_ids", "source_chunk_ids"] {
        if let Some(arr) = properties.get(key).and_then(|v| v.as_array()) {
            for item in arr {
                if let Some(s) = item.as_str() {
                    push_provenance_ref(&mut refs, s);
                }
            }
        }
    }

    if let Some(chunk) = properties.get("source_chunk_id").and_then(|v| v.as_str()) {
        push_provenance_ref(&mut refs, chunk);
    }
    if let Some(doc) = properties
        .get("source_document_id")
        .and_then(|v| v.as_str())
    {
        push_provenance_ref(&mut refs, doc);
    }

    refs
}

/// Canonical relationship id used by list endpoint: `{source}_{target}`.
pub fn edge_relationship_id(edge: &GraphEdge) -> String {
    format!("{}_{}", edge.source, edge.target)
}

/// True when edge matches API relationship id (property `id` or composite key).
pub fn edge_matches_relationship_id(edge: &GraphEdge, relationship_id: &str) -> bool {
    if relationship_id.is_empty() {
        return false;
    }
    if edge_relationship_id(edge) == relationship_id {
        return true;
    }
    edge.properties
        .get("id")
        .and_then(|v| v.as_str())
        .is_some_and(|id| id == relationship_id)
}

/// True if any source reference starts with one of the prefixes.
pub fn sources_match_prefixes(
    properties: &HashMap<String, serde_json::Value>,
    prefixes: &[String],
) -> bool {
    if prefixes.is_empty() {
        return false;
    }
    collect_source_references(properties).iter().any(|s| {
        prefixes
            .iter()
            .any(|p| s.starts_with(p.as_str()) || s == p.as_str())
    })
}

/// Filter for paged node listing (push-down to storage).
#[derive(Debug, Clone, Default)]
pub struct NodeListFilter {
    pub tenant_id: Option<String>,
    pub workspace_id: Option<String>,
    pub entity_type: Option<String>,
    pub search: Option<String>,
    /// Index-time Louvain community labels (SPEC-025 6.3 push-down filter).
    pub community_ids: Option<Vec<u64>>,
}

/// Filter for paged edge listing.
#[derive(Debug, Clone, Default)]
pub struct EdgeListFilter {
    pub tenant_id: Option<String>,
    pub workspace_id: Option<String>,
    pub relationship_type: Option<String>,
}

/// Paged graph query result.
#[derive(Debug, Clone)]
pub struct PagedGraphResult<T> {
    pub items: Vec<T>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}

impl<T> PagedGraphResult<T> {
    pub fn empty(limit: usize, offset: usize) -> Self {
        Self {
            items: Vec::new(),
            total: 0,
            offset,
            limit,
        }
    }
}

/// Keyset (seek) page for community snapshot scans (GH-404).
///
/// `next_after` is an opaque cursor (`id::text` on AGE / node id in memory).
/// Absent when the scan is exhausted. No `COUNT(*)` — stop when a page is short.
#[derive(Debug, Clone)]
pub struct KeysetPage<T> {
    pub items: Vec<T>,
    pub next_after: Option<String>,
}

impl<T> KeysetPage<T> {
    pub fn empty() -> Self {
        Self {
            items: Vec::new(),
            next_after: None,
        }
    }
}

/// Bounded graph reads — never require loading the full graph into the caller.
#[async_trait]
pub trait GraphScanOps: Send + Sync {
    /// List nodes with tenant/filter push-down and pagination.
    async fn list_nodes_filtered(
        &self,
        filter: &NodeListFilter,
        offset: usize,
        limit: usize,
    ) -> Result<PagedGraphResult<GraphNode>>;

    /// List edges with tenant/filter push-down and pagination.
    async fn list_edges_filtered(
        &self,
        filter: &EdgeListFilter,
        offset: usize,
        limit: usize,
    ) -> Result<PagedGraphResult<GraphEdge>>;

    /// Keyset scan of nodes after `after` cursor (community snapshot / GH-404).
    ///
    /// Stable order by storage id. No `COUNT(*)`. When `after` is `None`, starts
    /// from the beginning.
    async fn scan_nodes_after(
        &self,
        filter: &NodeListFilter,
        after: Option<&str>,
        limit: usize,
    ) -> Result<KeysetPage<GraphNode>>;

    /// Keyset scan of edges after `after` cursor (community snapshot / GH-404).
    ///
    /// Resolves endpoints via stored `eq_*` / properties — never AGE parent
    /// vertex text-cast joins. No `COUNT(*)` / `OFFSET`.
    async fn scan_edges_after(
        &self,
        filter: &EdgeListFilter,
        after: Option<&str>,
        limit: usize,
    ) -> Result<KeysetPage<GraphEdge>>;

    /// Exact community / entity-type / relationship-type counts for one scope.
    ///
    /// Covers the whole scope, not a prefix. The default sweeps with the keyset
    /// scans; backends that can aggregate server-side should override it.
    async fn facet_counts(
        &self,
        nodes: &NodeListFilter,
        edges: &EdgeListFilter,
    ) -> Result<super::graph_facets::GraphFacetCounts> {
        super::graph_facets::facet_counts_by_scan(self, nodes, edges).await
    }

    /// Find nodes whose source_ids reference any of the given prefixes.
    async fn find_nodes_by_source_prefixes(
        &self,
        filter: &NodeListFilter,
        source_prefixes: &[String],
    ) -> Result<Vec<GraphNode>>;

    /// Find edges whose source_id references any of the given prefixes.
    async fn find_edges_by_source_prefixes(
        &self,
        filter: &EdgeListFilter,
        source_prefixes: &[String],
    ) -> Result<Vec<GraphEdge>>;

    /// O(1) SQL / single-pass edge lookup by relationship id (SPEC-006 P2).
    async fn find_edge_by_relationship_id(
        &self,
        filter: &EdgeListFilter,
        relationship_id: &str,
    ) -> Result<Option<GraphEdge>>;
}

#[cfg(test)]
mod list_filter_tests {
    use super::*;
    use std::collections::HashMap;

    fn node_with(workspace: Option<&str>, tenant: Option<&str>) -> GraphNode {
        let mut properties = HashMap::new();
        if let Some(w) = workspace {
            properties.insert("workspace_id".into(), serde_json::json!(w));
        }
        if let Some(t) = tenant {
            properties.insert("tenant_id".into(), serde_json::json!(t));
        }
        GraphNode {
            id: "N1".into(),
            properties,
        }
    }

    #[test]
    fn workspace_only_filter_matches_legacy_null_and_equal() {
        let filter = NodeListFilter {
            tenant_id: None,
            workspace_id: Some("ws-a".into()),
            entity_type: None,
            search: None,
            community_ids: None,
        };
        assert!(node_matches_list_filter(&node_with(None, None), &filter));
        assert!(node_matches_list_filter(
            &node_with(Some("ws-a"), None),
            &filter
        ));
        assert!(!node_matches_list_filter(
            &node_with(Some("ws-b"), None),
            &filter
        ));
    }
}
