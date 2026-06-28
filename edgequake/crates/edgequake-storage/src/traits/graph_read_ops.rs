//! Read-only graph storage operations (SPEC-017 ISP Phase 2b).

use async_trait::async_trait;
use std::collections::HashMap;

use crate::error::Result;

use super::graph::{GraphEdge, GraphNode, KnowledgeGraph};

/// Read, search, and traverse graph data without mutation.
#[async_trait]
pub trait GraphStorageReadOps: Send + Sync {
    async fn has_node(&self, node_id: &str) -> Result<bool>;

    async fn get_node(&self, node_id: &str) -> Result<Option<GraphNode>>;

    async fn node_degree(&self, node_id: &str) -> Result<usize>;

    async fn node_degrees_batch(&self, node_ids: &[String]) -> Result<Vec<(String, usize)>> {
        let mut results = Vec::new();
        for node_id in node_ids {
            let degree = self.node_degree(node_id).await?;
            results.push((node_id.clone(), degree));
        }
        Ok(results)
    }

    /// Legacy full-graph load — **not for API hot paths** (SPEC-006). Prefer `GraphScanOps`.
    #[deprecated(
        note = "SPEC-006: use bounded GraphScanOps / list_nodes_filtered instead of full-graph load"
    )]
    async fn get_all_nodes(&self) -> Result<Vec<GraphNode>>;

    async fn get_nodes_by_ids(&self, node_ids: &[String]) -> Result<Vec<GraphNode>>;

    async fn get_nodes_batch(&self, node_ids: &[String]) -> Result<HashMap<String, GraphNode>> {
        let nodes = self.get_nodes_by_ids(node_ids).await?;
        Ok(nodes.into_iter().map(|n| (n.id.clone(), n)).collect())
    }

    async fn get_edges_for_nodes_batch(&self, node_ids: &[String]) -> Result<Vec<GraphEdge>> {
        self.get_edges_for_node_set(node_ids, None, None).await
    }

    async fn get_nodes_with_degrees_batch(
        &self,
        node_ids: &[String],
    ) -> Result<Vec<(GraphNode, usize, usize)>> {
        let nodes = self.get_nodes_batch(node_ids).await?;
        let degrees: HashMap<String, usize> = self
            .node_degrees_batch(node_ids)
            .await?
            .into_iter()
            .collect();

        let mut result = Vec::new();
        for (id, node) in nodes {
            let total_degree = degrees.get(&id).copied().unwrap_or(0);
            result.push((node, total_degree, total_degree));
        }
        Ok(result)
    }

    async fn has_edge(&self, source: &str, target: &str) -> Result<bool>;

    async fn get_edge(&self, source: &str, target: &str) -> Result<Option<GraphEdge>>;

    async fn get_node_edges(&self, node_id: &str) -> Result<Vec<GraphEdge>>;

    /// Incident edges for many nodes in one round-trip (SPEC-025 6.2).
    ///
    /// Returns edges where **either** endpoint is in `node_ids` (same semantics as
    /// repeated `get_node_edges`, without N+1 per frontier node).
    async fn get_incident_edges_batch(&self, node_ids: &[String]) -> Result<Vec<GraphEdge>> {
        let mut collected = Vec::new();
        for node_id in node_ids {
            collected.extend(self.get_node_edges(node_id).await?);
        }
        Ok(collected)
    }

    /// Legacy full-graph load — **not for API hot paths** (SPEC-006).
    #[deprecated(note = "SPEC-006: use bounded edge queries instead of full-graph load")]
    async fn get_all_edges(&self) -> Result<Vec<GraphEdge>>;

    async fn get_knowledge_graph(
        &self,
        start_node: &str,
        max_depth: usize,
        max_nodes: usize,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<KnowledgeGraph>;

    async fn get_popular_labels(
        &self,
        limit: usize,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<String>>;

    async fn search_labels(
        &self,
        query: &str,
        limit: usize,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<String>>;

    async fn search_nodes(
        &self,
        query: &str,
        limit: usize,
        entity_type: Option<&str>,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<(GraphNode, usize)>>;

    async fn get_neighbors(
        &self,
        node_id: &str,
        depth: usize,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<GraphNode>>;

    async fn get_popular_nodes_with_degree(
        &self,
        limit: usize,
        min_degree: Option<usize>,
        entity_type: Option<&str>,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<(GraphNode, usize)>> {
        let labels = self
            .get_popular_labels(limit * 2, tenant_id, workspace_id)
            .await?;
        let mut results = Vec::new();

        for label in labels {
            if results.len() >= limit {
                break;
            }
            if let Some(node) = self.get_node(&label).await? {
                let degree = self.node_degree(&label).await?;

                if let Some(min) = min_degree {
                    if degree < min {
                        continue;
                    }
                }

                if let Some(et) = entity_type {
                    let node_type = node
                        .properties
                        .get("entity_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if node_type != et {
                        continue;
                    }
                }

                if let Some(tid) = tenant_id {
                    let node_tenant = node
                        .properties
                        .get("tenant_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if !node_tenant.is_empty() && node_tenant != tid {
                        continue;
                    }
                }

                if let Some(wid) = workspace_id {
                    let node_workspace = node
                        .properties
                        .get("workspace_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if !node_workspace.is_empty() && node_workspace != wid {
                        continue;
                    }
                }

                results.push((node, degree));
            }
        }

        Ok(results)
    }

    #[allow(deprecated)]
    async fn get_edges_for_node_set(
        &self,
        node_ids: &[String],
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<GraphEdge>> {
        let all_edges = self.get_all_edges().await?;
        let node_set: std::collections::HashSet<&str> =
            node_ids.iter().map(|s| s.as_str()).collect();

        let filtered: Vec<GraphEdge> = all_edges
            .into_iter()
            .filter(|e| {
                if !node_set.contains(e.source.as_str()) || !node_set.contains(e.target.as_str()) {
                    return false;
                }

                if let Some(tid) = tenant_id {
                    let edge_tenant = e
                        .properties
                        .get("tenant_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if !edge_tenant.is_empty() && edge_tenant != tid {
                        return false;
                    }
                }

                if let Some(wid) = workspace_id {
                    let edge_workspace = e
                        .properties
                        .get("workspace_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if !edge_workspace.is_empty() && edge_workspace != wid {
                        return false;
                    }
                }

                true
            })
            .collect();

        Ok(filtered)
    }
}
