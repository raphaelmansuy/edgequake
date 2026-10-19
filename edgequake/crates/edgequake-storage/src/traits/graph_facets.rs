//! Exact facet counts over one workspace's graph.
//!
//! Backends that can aggregate server-side (Postgres) override
//! [`GraphScanOps::facet_counts`]. Everything else uses
//! [`facet_counts_by_scan`], a keyset sweep that tallies in memory. Both
//! cover the whole workspace; neither samples a prefix of it.

use std::collections::HashMap;

use super::graph_scan_ops::{EdgeListFilter, GraphScanOps, NodeListFilter};
use crate::error::Result;
use crate::traits::{GraphEdge, GraphNode};

/// Bucket used when a node carries no entity type.
pub const UNKNOWN_ENTITY_TYPE: &str = "UNKNOWN";
/// Bucket used when an edge carries no relationship type.
pub const DEFAULT_RELATIONSHIP_TYPE: &str = "RELATED_TO";

const SWEEP_PAGE: usize = 1_000;

/// `(bucket, count)` pairs, unordered. Callers sort for display.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GraphFacetCounts {
    /// Node count per `community_id` (nodes without one are not counted).
    pub communities: Vec<(String, usize)>,
    /// Node count per entity type.
    pub entity_types: Vec<(String, usize)>,
    /// Edge count per relationship type.
    pub relationship_types: Vec<(String, usize)>,
}

impl GraphFacetCounts {
    /// Each list sorted by count descending, then key. Stable for display and
    /// for comparing two backends.
    pub fn sorted(mut self) -> Self {
        for v in [
            &mut self.communities,
            &mut self.entity_types,
            &mut self.relationship_types,
        ] {
            v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        }
        self
    }
}

/// `community_id` property as text; accepts a string or an integer.
fn node_community_id(node: &GraphNode) -> Option<String> {
    node.properties.get("community_id").and_then(|v| {
        v.as_str()
            .map(str::to_string)
            .or_else(|| v.as_u64().map(|n| n.to_string()))
            .or_else(|| v.as_i64().map(|n| n.to_string()))
    })
}

fn non_empty_str<'a>(props: &'a HashMap<String, serde_json::Value>, key: &str) -> Option<&'a str> {
    props
        .get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
}

fn edge_relationship_type(edge: &GraphEdge) -> &str {
    non_empty_str(&edge.properties, "relationship_type")
        .or_else(|| non_empty_str(&edge.properties, "relation_type"))
        .unwrap_or(DEFAULT_RELATIONSHIP_TYPE)
}

fn into_pairs(map: HashMap<String, usize>) -> Vec<(String, usize)> {
    map.into_iter().collect()
}

/// Keyset sweep of the whole workspace, counting as it goes.
///
/// Memory is bounded by the number of distinct buckets, not by graph size.
pub async fn facet_counts_by_scan<G: GraphScanOps + ?Sized>(
    graph: &G,
    nodes: &NodeListFilter,
    edges: &EdgeListFilter,
) -> Result<GraphFacetCounts> {
    let mut communities: HashMap<String, usize> = HashMap::new();
    let mut entity_types: HashMap<String, usize> = HashMap::new();
    let mut after: Option<String> = None;
    loop {
        let page = graph
            .scan_nodes_after(nodes, after.as_deref(), SWEEP_PAGE)
            .await?;
        for node in &page.items {
            if let Some(cid) = node_community_id(node) {
                *communities.entry(cid).or_insert(0) += 1;
            }
            let et = non_empty_str(&node.properties, "entity_type").unwrap_or(UNKNOWN_ENTITY_TYPE);
            *entity_types.entry(et.to_string()).or_insert(0) += 1;
        }
        match page.next_after {
            Some(next) => after = Some(next),
            None => break,
        }
    }

    let mut relationship_types: HashMap<String, usize> = HashMap::new();
    let mut after: Option<String> = None;
    loop {
        let page = graph
            .scan_edges_after(edges, after.as_deref(), SWEEP_PAGE)
            .await?;
        for edge in &page.items {
            *relationship_types
                .entry(edge_relationship_type(edge).to_string())
                .or_insert(0) += 1;
        }
        match page.next_after {
            Some(next) => after = Some(next),
            None => break,
        }
    }

    Ok(GraphFacetCounts {
        communities: into_pairs(communities),
        entity_types: into_pairs(entity_types),
        relationship_types: into_pairs(relationship_types),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn node(props: serde_json::Value) -> GraphNode {
        GraphNode {
            id: "n".into(),
            properties: serde_json::from_value(props).unwrap(),
        }
    }

    fn edge(props: serde_json::Value) -> GraphEdge {
        GraphEdge {
            source: "a".into(),
            target: "b".into(),
            properties: serde_json::from_value(props).unwrap(),
        }
    }

    #[test]
    fn community_id_accepts_string_and_integers() {
        assert_eq!(
            node_community_id(&node(json!({"community_id": "c1"}))),
            Some("c1".into())
        );
        assert_eq!(
            node_community_id(&node(json!({"community_id": 7}))),
            Some("7".into())
        );
        assert_eq!(
            node_community_id(&node(json!({"community_id": -2}))),
            Some("-2".into())
        );
        assert_eq!(
            node_community_id(&node(json!({"community_id": null}))),
            None
        );
        assert_eq!(node_community_id(&node(json!({}))), None);
    }

    #[test]
    fn relationship_type_prefers_relationship_then_relation_then_default() {
        let both = edge(json!({"relationship_type": "A", "relation_type": "B"}));
        assert_eq!(edge_relationship_type(&both), "A");
        let only_relation = edge(json!({"relation_type": "B"}));
        assert_eq!(edge_relationship_type(&only_relation), "B");
        let empty = edge(json!({"relationship_type": "", "relation_type": ""}));
        assert_eq!(edge_relationship_type(&empty), DEFAULT_RELATIONSHIP_TYPE);
        assert_eq!(
            edge_relationship_type(&edge(json!({}))),
            DEFAULT_RELATIONSHIP_TYPE
        );
    }

    #[test]
    fn sorted_orders_by_count_then_key() {
        let counts = GraphFacetCounts {
            communities: vec![("b".into(), 2), ("a".into(), 2), ("z".into(), 9)],
            ..Default::default()
        }
        .sorted();
        assert_eq!(
            counts.communities,
            vec![("z".into(), 9), ("a".into(), 2), ("b".into(), 2)]
        );
    }
}
