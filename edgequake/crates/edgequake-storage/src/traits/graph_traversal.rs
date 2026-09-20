//! Provider-neutral bounded graph traversal.

use std::collections::HashSet;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{Result, StorageError};

use super::{GraphEdge, GraphStorageReadOps};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphTruncationReason {
    MaxDepth,
    MaxNodes,
    MaxEdges,
    MaxBytes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphTraversalBudget {
    pub max_depth: usize,
    pub max_nodes: usize,
    pub max_edges: usize,
    pub max_bytes: usize,
}

#[derive(Debug, Clone)]
pub struct BoundedGraphTraversal {
    pub node_ids: Vec<String>,
    pub edges: Vec<GraphEdge>,
    pub bytes: usize,
    pub truncation_reason: Option<GraphTruncationReason>,
}

#[async_trait]
pub trait GraphExpansionAuthorizer: Send + Sync {
    /// Positional authorization result for candidate edges. Implementations
    /// must validate the edge revision, both endpoints, and contributing
    /// document visibility before returning `true`.
    async fn authorize_edges(&self, edges: &[GraphEdge]) -> Result<Vec<bool>>;
}

pub async fn traverse_graph_bounded(
    graph: &dyn GraphStorageReadOps,
    authorizer: &dyn GraphExpansionAuthorizer,
    seeds: &[String],
    budget: GraphTraversalBudget,
    tenant_id: Option<&str>,
    workspace_id: Option<&str>,
) -> Result<BoundedGraphTraversal> {
    if budget.max_nodes == 0
        || budget.max_edges == 0
        || budget.max_bytes == 0
        || budget.max_depth == 0
        || seeds.is_empty()
    {
        return Ok(BoundedGraphTraversal {
            node_ids: Vec::new(),
            edges: Vec::new(),
            bytes: 0,
            truncation_reason: seeds.is_empty().then_some(GraphTruncationReason::MaxDepth),
        });
    }

    let mut seen_nodes = HashSet::new();
    let mut nodes = Vec::new();
    let mut bytes = 0usize;
    for seed in seeds {
        if seen_nodes.insert(seed.clone()) {
            if nodes.len() == budget.max_nodes {
                return Ok(result(
                    nodes,
                    Vec::new(),
                    bytes,
                    GraphTruncationReason::MaxNodes,
                ));
            }
            bytes = bytes.saturating_add(seed.len());
            if bytes > budget.max_bytes {
                return Ok(result(
                    Vec::new(),
                    Vec::new(),
                    0,
                    GraphTruncationReason::MaxBytes,
                ));
            }
            nodes.push(seed.clone());
        }
    }

    let mut frontier = nodes.clone();
    let mut seen_edges = HashSet::new();
    let mut edges = Vec::new();

    for depth in 0..budget.max_depth {
        if frontier.is_empty() {
            break;
        }
        let candidates = graph
            .get_incident_edges_batch(&frontier, tenant_id, workspace_id)
            .await?;
        let authorized = authorizer.authorize_edges(&candidates).await?;
        if authorized.len() != candidates.len() {
            return Err(StorageError::InvalidData(
                "graph authorizer returned a non-positional result".into(),
            ));
        }

        let mut next_frontier = Vec::new();
        for (edge, authorized) in candidates.into_iter().zip(authorized) {
            if !authorized || !seen_edges.insert(edge_identity(&edge)) {
                continue;
            }
            if edges.len() == budget.max_edges {
                return Ok(result(nodes, edges, bytes, GraphTruncationReason::MaxEdges));
            }

            let edge_bytes = serialized_edge_bytes(&edge)?;
            if bytes.saturating_add(edge_bytes) > budget.max_bytes {
                return Ok(result(nodes, edges, bytes, GraphTruncationReason::MaxBytes));
            }

            let new_nodes: Vec<String> = [&edge.source, &edge.target]
                .into_iter()
                .filter(|id| !seen_nodes.contains(id.as_str()))
                .cloned()
                .collect();
            if seen_nodes.len().saturating_add(new_nodes.len()) > budget.max_nodes {
                return Ok(result(nodes, edges, bytes, GraphTruncationReason::MaxNodes));
            }

            bytes += edge_bytes;
            for id in new_nodes {
                if seen_nodes.insert(id.clone()) {
                    bytes = bytes.saturating_add(id.len());
                    next_frontier.push(id.clone());
                    nodes.push(id);
                }
            }
            edges.push(edge);
        }
        frontier = next_frontier;

        if depth + 1 == budget.max_depth && !frontier.is_empty() {
            return Ok(result(nodes, edges, bytes, GraphTruncationReason::MaxDepth));
        }
    }

    Ok(BoundedGraphTraversal {
        node_ids: nodes,
        edges,
        bytes,
        truncation_reason: None,
    })
}

fn result(
    node_ids: Vec<String>,
    edges: Vec<GraphEdge>,
    bytes: usize,
    reason: GraphTruncationReason,
) -> BoundedGraphTraversal {
    BoundedGraphTraversal {
        node_ids,
        edges,
        bytes,
        truncation_reason: Some(reason),
    }
}

fn edge_identity(edge: &GraphEdge) -> String {
    let explicit = ["relationship_id", "edge_id", "id"]
        .into_iter()
        .find_map(|key| edge.properties.get(key).and_then(|value| value.as_str()));
    if let Some(id) = explicit {
        return format!("id:{id}");
    }
    let relation = edge
        .properties
        .get("relation_type")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let mut properties: Vec<(&String, &serde_json::Value)> = edge.properties.iter().collect();
    properties.sort_by(|left, right| left.0.cmp(right.0));
    format!(
        "{}\u{0}{}\u{0}{}\u{0}{properties:?}",
        edge.source, edge.target, relation
    )
}

fn serialized_edge_bytes(edge: &GraphEdge) -> Result<usize> {
    serde_json::to_vec(edge)
        .map(|bytes| bytes.len())
        .map_err(|error| StorageError::Serialization(error.to_string()))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::adapters::memory::MemoryGraphStorage;
    use crate::traits::{GraphStorage, GraphStorageMutateOps};

    use super::*;

    struct RejectDeletedBridge;

    #[async_trait]
    impl GraphExpansionAuthorizer for RejectDeletedBridge {
        async fn authorize_edges(&self, edges: &[GraphEdge]) -> Result<Vec<bool>> {
            Ok(edges
                .iter()
                .map(|edge| {
                    edge.properties
                        .get("document_serving")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false)
                })
                .collect())
        }
    }

    #[tokio::test]
    async fn authorizes_before_frontier_expansion() {
        let graph = MemoryGraphStorage::new("bounded");
        graph.initialize().await.unwrap();
        graph
            .upsert_edge(
                "A",
                "B",
                HashMap::from([("document_serving".into(), serde_json::json!(false))]),
            )
            .await
            .unwrap();
        graph
            .upsert_edge(
                "B",
                "C",
                HashMap::from([("document_serving".into(), serde_json::json!(true))]),
            )
            .await
            .unwrap();

        let result = traverse_graph_bounded(
            &graph,
            &RejectDeletedBridge,
            &["A".into()],
            GraphTraversalBudget {
                max_depth: 3,
                max_nodes: 10,
                max_edges: 10,
                max_bytes: 10_000,
            },
            None,
            None,
        )
        .await
        .unwrap();

        assert_eq!(result.node_ids, vec!["A"]);
        assert!(result.edges.is_empty());
    }

    #[tokio::test]
    async fn reports_truthful_edge_budget_truncation() {
        let graph = MemoryGraphStorage::new("bounded");
        graph.initialize().await.unwrap();
        for target in ["B", "C"] {
            graph
                .upsert_edge(
                    "A",
                    target,
                    HashMap::from([("document_serving".into(), serde_json::json!(true))]),
                )
                .await
                .unwrap();
        }
        let result = traverse_graph_bounded(
            &graph,
            &RejectDeletedBridge,
            &["A".into()],
            GraphTraversalBudget {
                max_depth: 2,
                max_nodes: 10,
                max_edges: 1,
                max_bytes: 10_000,
            },
            None,
            None,
        )
        .await
        .unwrap();

        assert_eq!(result.edges.len(), 1);
        assert_eq!(
            result.truncation_reason,
            Some(GraphTruncationReason::MaxEdges)
        );
    }
}
