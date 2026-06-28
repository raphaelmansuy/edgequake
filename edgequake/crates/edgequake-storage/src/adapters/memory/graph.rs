//! In-memory graph storage.
//!
//! Provides graph storage using adjacency lists for efficient traversal.
//!
//! ## Implements
//!
//! - [`FEAT0210`]: In-memory graph storage
//! - [`FEAT0211`]: Entity node management
//! - [`FEAT0212`]: Relationship edge management
//!
//! ## Use Cases
//!
//! - [`UC0602`]: System stores entities and relationships
//! - [`UC0701`]: System traverses knowledge graph
//!
//! ## Enforces
//!
//! - [`BR0210`]: Thread-safe concurrent access via RwLock
//! - [`BR0211`]: Consistent edge key normalization

use async_trait::async_trait;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::RwLock;

use crate::error::Result;
use crate::traits::{
    edge_matches_list_filter, edge_matches_relationship_id, node_matches_list_filter,
    sources_match_prefixes, EdgeListFilter, GraphEdge, GraphNode, GraphScanOps, GraphStorage,
    GraphStorageAnalyticsOps, GraphStorageMutateOps, GraphStorageReadOps, KnowledgeGraph,
    NodeListFilter, PagedGraphResult,
};

/// In-memory graph storage implementation.
///
/// Uses adjacency lists for efficient traversal.
/// Suitable for testing and small graphs.
pub struct MemoryGraphStorage {
    namespace: String,
    nodes: RwLock<HashMap<String, HashMap<String, serde_json::Value>>>,
    // edges stored as (source, target) -> properties
    edges: RwLock<HashMap<(String, String), HashMap<String, serde_json::Value>>>,
    // adjacency list: node -> set of neighbors
    adjacency: RwLock<HashMap<String, HashSet<String>>>,
}

impl MemoryGraphStorage {
    /// Create a new in-memory graph storage.
    pub fn new(namespace: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            nodes: RwLock::new(HashMap::new()),
            edges: RwLock::new(HashMap::new()),
            adjacency: RwLock::new(HashMap::new()),
        }
    }

    /// Normalize edge key (alphabetically sorted for consistency).
    fn edge_key(source: &str, target: &str) -> (String, String) {
        if source <= target {
            (source.to_string(), target.to_string())
        } else {
            (target.to_string(), source.to_string())
        }
    }
}

#[async_trait]
impl GraphStorage for MemoryGraphStorage {
    fn namespace(&self) -> &str {
        &self.namespace
    }

    async fn initialize(&self) -> Result<()> {
        Ok(())
    }

    async fn finalize(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
#[allow(deprecated)]
impl GraphStorageReadOps for MemoryGraphStorage {
    async fn has_node(&self, node_id: &str) -> Result<bool> {
        let nodes = self.nodes.read().map_err(super::lock::map_lock_err)?;
        Ok(nodes.contains_key(node_id))
    }

    async fn get_node(&self, node_id: &str) -> Result<Option<GraphNode>> {
        let nodes = self.nodes.read().map_err(super::lock::map_lock_err)?;

        Ok(nodes.get(node_id).map(|props| GraphNode {
            id: node_id.to_string(),
            properties: props.clone(),
        }))
    }

    async fn node_degree(&self, node_id: &str) -> Result<usize> {
        let adjacency = self.adjacency.read().map_err(super::lock::map_lock_err)?;

        Ok(adjacency.get(node_id).map(|n| n.len()).unwrap_or(0))
    }

    async fn node_degrees_batch(&self, node_ids: &[String]) -> Result<Vec<(String, usize)>> {
        let adjacency = self.adjacency.read().map_err(super::lock::map_lock_err)?;

        Ok(node_ids
            .iter()
            .map(|id| {
                let degree = adjacency.get(id).map(|n| n.len()).unwrap_or(0);
                (id.clone(), degree)
            })
            .collect())
    }

    async fn get_all_nodes(&self) -> Result<Vec<GraphNode>> {
        let nodes = self.nodes.read().map_err(super::lock::map_lock_err)?;

        Ok(nodes
            .iter()
            .map(|(id, props)| GraphNode {
                id: id.clone(),
                properties: props.clone(),
            })
            .collect())
    }

    async fn get_nodes_by_ids(&self, node_ids: &[String]) -> Result<Vec<GraphNode>> {
        let nodes = self.nodes.read().map_err(super::lock::map_lock_err)?;

        Ok(node_ids
            .iter()
            .filter_map(|id| {
                nodes.get(id).map(|props| GraphNode {
                    id: id.clone(),
                    properties: props.clone(),
                })
            })
            .collect())
    }

    /// Optimized batch node retrieval returning HashMap for O(1) lookups.
    async fn get_nodes_batch(&self, node_ids: &[String]) -> Result<HashMap<String, GraphNode>> {
        let nodes = self.nodes.read().map_err(super::lock::map_lock_err)?;

        let mut result = HashMap::new();
        for id in node_ids {
            if let Some(props) = nodes.get(id) {
                result.insert(
                    id.clone(),
                    GraphNode {
                        id: id.clone(),
                        properties: props.clone(),
                    },
                );
            }
        }
        Ok(result)
    }

    /// Get edges where both endpoints are in the specified node set.
    async fn get_edges_for_nodes_batch(&self, node_ids: &[String]) -> Result<Vec<GraphEdge>> {
        let edges = self.edges.read().map_err(super::lock::map_lock_err)?;

        let node_set: HashSet<&str> = node_ids.iter().map(|s| s.as_str()).collect();

        Ok(edges
            .iter()
            .filter(|((s, t), _)| node_set.contains(s.as_str()) && node_set.contains(t.as_str()))
            .map(|((s, t), props)| GraphEdge {
                source: s.clone(),
                target: t.clone(),
                properties: props.clone(),
            })
            .collect())
    }

    /// Get nodes with their degrees in a single batch operation.
    async fn get_nodes_with_degrees_batch(
        &self,
        node_ids: &[String],
    ) -> Result<Vec<(GraphNode, usize, usize)>> {
        let nodes = self.nodes.read().map_err(super::lock::map_lock_err)?;
        let adjacency = self.adjacency.read().map_err(super::lock::map_lock_err)?;

        let mut result = Vec::new();
        for id in node_ids {
            if let Some(props) = nodes.get(id) {
                let degree = adjacency.get(id).map(|n| n.len()).unwrap_or(0);
                result.push((
                    GraphNode {
                        id: id.clone(),
                        properties: props.clone(),
                    },
                    degree, // in_degree (symmetric graph, so same)
                    degree, // out_degree
                ));
            }
        }
        Ok(result)
    }

    async fn has_edge(&self, source: &str, target: &str) -> Result<bool> {
        let edges = self.edges.read().map_err(super::lock::map_lock_err)?;
        let key = Self::edge_key(source, target);
        Ok(edges.contains_key(&key))
    }

    async fn get_edge(&self, source: &str, target: &str) -> Result<Option<GraphEdge>> {
        let edges = self.edges.read().map_err(super::lock::map_lock_err)?;
        let key = Self::edge_key(source, target);

        Ok(edges.get(&key).map(|props| GraphEdge {
            source: key.0.clone(),
            target: key.1.clone(),
            properties: props.clone(),
        }))
    }

    async fn get_node_edges(&self, node_id: &str) -> Result<Vec<GraphEdge>> {
        let edges = self.edges.read().map_err(super::lock::map_lock_err)?;

        Ok(edges
            .iter()
            .filter(|((s, t), _)| s == node_id || t == node_id)
            .map(|((s, t), props)| GraphEdge {
                source: s.clone(),
                target: t.clone(),
                properties: props.clone(),
            })
            .collect())
    }

    async fn get_incident_edges_batch(&self, node_ids: &[String]) -> Result<Vec<GraphEdge>> {
        if node_ids.is_empty() {
            return Ok(Vec::new());
        }

        let node_set: std::collections::HashSet<&str> =
            node_ids.iter().map(|s| s.as_str()).collect();
        let edges = self.edges.read().map_err(super::lock::map_lock_err)?;

        Ok(edges
            .iter()
            .filter(|((s, t), _)| node_set.contains(s.as_str()) || node_set.contains(t.as_str()))
            .map(|((s, t), props)| GraphEdge {
                source: s.clone(),
                target: t.clone(),
                properties: props.clone(),
            })
            .collect())
    }

    async fn get_all_edges(&self) -> Result<Vec<GraphEdge>> {
        let edges = self.edges.read().map_err(super::lock::map_lock_err)?;

        Ok(edges
            .iter()
            .map(|((s, t), props)| GraphEdge {
                source: s.clone(),
                target: t.clone(),
                properties: props.clone(),
            })
            .collect())
    }

    async fn get_edges_for_node_set(
        &self,
        node_ids: &[String],
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<GraphEdge>> {
        use std::collections::HashSet;

        let node_set: HashSet<&str> = node_ids.iter().map(|s| s.as_str()).collect();
        if node_set.is_empty() {
            return Ok(Vec::new());
        }

        let edges = self.edges.read().map_err(super::lock::map_lock_err)?;
        Ok(edges
            .iter()
            .filter_map(|((source, target), props)| {
                if !node_set.contains(source.as_str()) || !node_set.contains(target.as_str()) {
                    return None;
                }
                let edge = GraphEdge {
                    source: source.clone(),
                    target: target.clone(),
                    properties: props.clone(),
                };
                if let Some(tid) = tenant_id {
                    let edge_tenant = edge
                        .properties
                        .get("tenant_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if !edge_tenant.is_empty() && edge_tenant != tid {
                        return None;
                    }
                }
                if let Some(wid) = workspace_id {
                    let edge_workspace = edge
                        .properties
                        .get("workspace_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if !edge_workspace.is_empty() && edge_workspace != wid {
                        return None;
                    }
                }
                Some(edge)
            })
            .collect())
    }

    async fn get_knowledge_graph(
        &self,
        start_node: &str,
        max_depth: usize,
        max_nodes: usize,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<KnowledgeGraph> {
        use crate::traits::node_matches_list_filter;
        use crate::traits::NodeListFilter;

        let node_filter = match (tenant_id, workspace_id) {
            (Some(tid), Some(wid)) => Some(NodeListFilter {
                tenant_id: Some(tid.to_string()),
                workspace_id: Some(wid.to_string()),
                ..Default::default()
            }),
            _ => None,
        };

        let nodes_map = self.nodes.read().map_err(super::lock::map_lock_err)?;
        let edges_map = self.edges.read().map_err(super::lock::map_lock_err)?;
        let adjacency = self.adjacency.read().map_err(super::lock::map_lock_err)?;

        let mut visited: HashSet<String> = HashSet::new();
        let mut result_nodes: Vec<GraphNode> = Vec::new();
        let mut result_edges: Vec<GraphEdge> = Vec::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();

        if node_filter.as_ref().is_some_and(|f| {
            nodes_map.get(start_node).is_none_or(|props| {
                !node_matches_list_filter(
                    &GraphNode {
                        id: start_node.to_string(),
                        properties: props.clone(),
                    },
                    f,
                )
            })
        }) {
            return Ok(KnowledgeGraph::new());
        }

        queue.push_back((start_node.to_string(), 0));

        while let Some((node_id, depth)) = queue.pop_front() {
            if visited.contains(&node_id) || depth > max_depth || result_nodes.len() >= max_nodes {
                continue;
            }

            visited.insert(node_id.clone());

            if let Some(props) = nodes_map.get(&node_id) {
                let node = GraphNode {
                    id: node_id.clone(),
                    properties: props.clone(),
                };
                if node_filter
                    .as_ref()
                    .is_some_and(|f| !node_matches_list_filter(&node, f))
                {
                    continue;
                }
                result_nodes.push(node);
            }

            if let Some(neighbors) = adjacency.get(&node_id) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        if let Some(props) = nodes_map.get(neighbor) {
                            if node_filter.as_ref().is_none_or(|f| {
                                node_matches_list_filter(
                                    &GraphNode {
                                        id: neighbor.clone(),
                                        properties: props.clone(),
                                    },
                                    f,
                                )
                            }) {
                                queue.push_back((neighbor.clone(), depth + 1));
                            }
                        }
                    }
                }
            }
        }

        // Collect edges between visited nodes
        for ((s, t), props) in edges_map.iter() {
            if visited.contains(s) && visited.contains(t) {
                result_edges.push(GraphEdge {
                    source: s.clone(),
                    target: t.clone(),
                    properties: props.clone(),
                });
            }
        }

        Ok(KnowledgeGraph {
            nodes: result_nodes,
            edges: result_edges,
            is_truncated: visited.len() >= max_nodes,
        })
    }

    async fn get_popular_labels(
        &self,
        limit: usize,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<String>> {
        let nodes = self.nodes.read().map_err(super::lock::map_lock_err)?;
        let adjacency = self.adjacency.read().map_err(super::lock::map_lock_err)?;

        let mut node_degrees: Vec<(String, usize)> = adjacency
            .iter()
            .filter(|(id, _)| {
                nodes.get(*id).is_some_and(|props| {
                    if let Some(tid) = tenant_id {
                        let node_tid = props
                            .get("tenant_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        if node_tid != tid {
                            return false;
                        }
                    }
                    if let Some(wid) = workspace_id {
                        let node_wid = props
                            .get("workspace_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        if node_wid != wid {
                            return false;
                        }
                    }
                    true
                })
            })
            .map(|(id, neighbors)| (id.clone(), neighbors.len()))
            .collect();

        node_degrees.sort_by_key(|entry| std::cmp::Reverse(entry.1));

        Ok(node_degrees
            .into_iter()
            .take(limit)
            .map(|(id, _)| id)
            .collect())
    }

    async fn search_labels(
        &self,
        query: &str,
        limit: usize,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<String>> {
        let nodes = self.nodes.read().map_err(super::lock::map_lock_err)?;

        let query_lower = query.to_lowercase();

        Ok(nodes
            .iter()
            .filter(|(id, properties)| {
                if let Some(tid) = tenant_id {
                    let node_tid = properties
                        .get("tenant_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if node_tid != tid {
                        return false;
                    }
                }
                if let Some(wid) = workspace_id {
                    let node_wid = properties
                        .get("workspace_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if node_wid != wid {
                        return false;
                    }
                }
                id.to_lowercase().contains(&query_lower)
            })
            .take(limit)
            .map(|(id, _)| id.clone())
            .collect())
    }

    async fn search_nodes(
        &self,
        query: &str,
        limit: usize,
        entity_type: Option<&str>,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<(GraphNode, usize)>> {
        let nodes = self.nodes.read().map_err(super::lock::map_lock_err)?;

        let adjacency = self.adjacency.read().map_err(super::lock::map_lock_err)?;

        let query_lower = query.to_lowercase();

        let mut results: Vec<(GraphNode, usize)> = nodes
            .iter()
            .filter(|(node_id, props)| {
                // Text search on label (node_id) and description
                let label_match = node_id.to_lowercase().contains(&query_lower);
                let desc_match = props
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|d| d.to_lowercase().contains(&query_lower))
                    .unwrap_or(false);

                if !label_match && !desc_match {
                    return false;
                }

                // Apply entity_type filter
                if let Some(etype) = entity_type {
                    let node_type = props
                        .get("entity_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if node_type != etype {
                        return false;
                    }
                }

                // Apply tenant filter
                if let Some(tid) = tenant_id {
                    let node_tenant = props
                        .get("tenant_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if node_tenant != tid {
                        return false;
                    }
                }

                // Apply workspace filter
                if let Some(wid) = workspace_id {
                    let node_workspace = props
                        .get("workspace_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if node_workspace != wid {
                        return false;
                    }
                }

                true
            })
            .map(|(node_id, props)| {
                // Calculate degree from adjacency list
                let degree = adjacency.get(node_id).map(|n| n.len()).unwrap_or(0);
                let node = GraphNode {
                    id: node_id.clone(),
                    properties: props.clone(),
                };
                (node, degree)
            })
            .collect();

        // Sort by degree descending
        results.sort_by_key(|entry| std::cmp::Reverse(entry.1));
        results.truncate(limit);

        Ok(results)
    }

    async fn get_neighbors(
        &self,
        node_id: &str,
        depth: usize,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<GraphNode>> {
        let kg = self
            .get_knowledge_graph(node_id, depth, 1000, None, None)
            .await?;
        Ok(kg
            .nodes
            .into_iter()
            .filter(|n| {
                if n.id == node_id {
                    return false;
                }
                if let Some(tid) = tenant_id {
                    let node_tid = n
                        .properties
                        .get("tenant_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if node_tid != tid {
                        return false;
                    }
                }
                if let Some(wid) = workspace_id {
                    let node_wid = n
                        .properties
                        .get("workspace_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if node_wid != wid {
                        return false;
                    }
                }
                true
            })
            .collect())
    }
}

#[async_trait]
impl GraphStorageMutateOps for MemoryGraphStorage {
    async fn upsert_node(
        &self,
        node_id: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let mut nodes = self.nodes.write().map_err(super::lock::map_lock_err)?;
        let mut adjacency = self.adjacency.write().map_err(super::lock::map_lock_err)?;

        nodes.insert(node_id.to_string(), properties);
        adjacency.entry(node_id.to_string()).or_default();

        Ok(())
    }

    /// P-G10 / RC-15: real batch — ONE lock acquisition for all nodes, not N.
    /// Matches the contract documented on `GraphStorageMutateOps`.
    async fn upsert_nodes_batch(
        &self,
        nodes: &[(String, HashMap<String, serde_json::Value>)],
    ) -> Result<()> {
        let mut nodes_map = self.nodes.write().map_err(super::lock::map_lock_err)?;
        let mut adjacency = self.adjacency.write().map_err(super::lock::map_lock_err)?;
        for (node_id, properties) in nodes {
            nodes_map.insert(node_id.clone(), properties.clone());
            adjacency.entry(node_id.clone()).or_default();
        }
        Ok(())
    }

    async fn delete_node(&self, node_id: &str) -> Result<()> {
        let mut nodes = self.nodes.write().map_err(super::lock::map_lock_err)?;
        let mut edges = self.edges.write().map_err(super::lock::map_lock_err)?;
        let mut adjacency = self.adjacency.write().map_err(super::lock::map_lock_err)?;

        nodes.remove(node_id);

        // Remove all edges involving this node
        let to_remove: Vec<(String, String)> = edges
            .keys()
            .filter(|(s, t)| s == node_id || t == node_id)
            .cloned()
            .collect();

        for key in to_remove {
            edges.remove(&key);
        }

        // Update adjacency
        adjacency.remove(node_id);
        for neighbors in adjacency.values_mut() {
            neighbors.remove(node_id);
        }

        Ok(())
    }

    async fn delete_node_scoped(
        &self,
        node_id: &str,
        tenant_id: &str,
        workspace_id: &str,
    ) -> Result<bool> {
        let matches = {
            let nodes = self.nodes.read().map_err(super::lock::map_lock_err)?;
            nodes.get(node_id).is_some_and(|props| {
                props
                    .get("tenant_id")
                    .and_then(|v| v.as_str())
                    .is_some_and(|t| t == tenant_id)
                    && props
                        .get("workspace_id")
                        .and_then(|v| v.as_str())
                        .is_some_and(|w| w == workspace_id)
            })
        };
        if !matches {
            return Ok(false);
        }
        self.delete_node(node_id).await?;
        Ok(true)
    }

    async fn upsert_edge(
        &self,
        source: &str,
        target: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let mut edges = self.edges.write().map_err(super::lock::map_lock_err)?;
        let mut adjacency = self.adjacency.write().map_err(super::lock::map_lock_err)?;

        let key = Self::edge_key(source, target);
        edges.insert(key, properties);

        // Update adjacency (bidirectional)
        adjacency
            .entry(source.to_string())
            .or_default()
            .insert(target.to_string());
        adjacency
            .entry(target.to_string())
            .or_default()
            .insert(source.to_string());

        Ok(())
    }

    /// P-G10 / RC-15: real batch — ONE lock acquisition for all edges, not N.
    async fn upsert_edges_batch(
        &self,
        edges: &[(String, String, HashMap<String, serde_json::Value>)],
    ) -> Result<()> {
        let mut edges_map = self.edges.write().map_err(super::lock::map_lock_err)?;
        let mut adjacency = self.adjacency.write().map_err(super::lock::map_lock_err)?;
        for (source, target, properties) in edges {
            let key = Self::edge_key(source, target);
            edges_map.insert(key, properties.clone());
            adjacency
                .entry(source.clone())
                .or_default()
                .insert(target.clone());
            adjacency
                .entry(target.clone())
                .or_default()
                .insert(source.clone());
        }
        Ok(())
    }

    async fn delete_edge(&self, source: &str, target: &str) -> Result<()> {
        let mut edges = self.edges.write().map_err(super::lock::map_lock_err)?;
        let mut adjacency = self.adjacency.write().map_err(super::lock::map_lock_err)?;

        let key = Self::edge_key(source, target);
        edges.remove(&key);

        // Update adjacency
        if let Some(neighbors) = adjacency.get_mut(source) {
            neighbors.remove(target);
        }
        if let Some(neighbors) = adjacency.get_mut(target) {
            neighbors.remove(source);
        }

        Ok(())
    }

    async fn delete_edge_scoped(
        &self,
        source: &str,
        target: &str,
        tenant_id: &str,
        workspace_id: &str,
    ) -> Result<bool> {
        let key = Self::edge_key(source, target);
        let matches = {
            let edges = self.edges.read().map_err(super::lock::map_lock_err)?;
            edges.get(&key).is_some_and(|props| {
                props
                    .get("tenant_id")
                    .and_then(|v| v.as_str())
                    .is_some_and(|t| t == tenant_id)
                    && props
                        .get("workspace_id")
                        .and_then(|v| v.as_str())
                        .is_some_and(|w| w == workspace_id)
            })
        };
        if !matches {
            return Ok(false);
        }
        self.delete_edge(source, target).await?;
        Ok(true)
    }

    async fn clear(&self) -> Result<()> {
        let mut nodes = self.nodes.write().map_err(super::lock::map_lock_err)?;
        let mut edges = self.edges.write().map_err(super::lock::map_lock_err)?;
        let mut adjacency = self.adjacency.write().map_err(super::lock::map_lock_err)?;

        nodes.clear();
        edges.clear();
        adjacency.clear();

        Ok(())
    }

    /// Clear nodes and edges for a specific workspace.
    ///
    /// Filters by `workspace_id` property in node/edge data.
    /// Returns (nodes_deleted, edges_deleted).
    async fn clear_workspace(&self, workspace_id: &uuid::Uuid) -> Result<(usize, usize)> {
        let mut nodes = self.nodes.write().map_err(super::lock::map_lock_err)?;
        let mut edges = self.edges.write().map_err(super::lock::map_lock_err)?;
        let mut adjacency = self.adjacency.write().map_err(super::lock::map_lock_err)?;

        let workspace_id_str = workspace_id.to_string();

        // Collect node IDs to remove (nodes are HashMap<String, Value>)
        let node_ids_to_remove: Vec<String> = nodes
            .iter()
            .filter_map(|(id, props)| {
                if let Some(ws_id) = props.get("workspace_id").and_then(|v| v.as_str()) {
                    if ws_id == workspace_id_str {
                        return Some(id.clone());
                    }
                }
                None
            })
            .collect();

        let nodes_deleted = node_ids_to_remove.len();

        // Remove nodes
        for id in &node_ids_to_remove {
            nodes.remove(id);
            adjacency.remove(id);
        }

        // Collect edge keys to remove (edges where either endpoint was in workspace)
        let node_set: std::collections::HashSet<&str> =
            node_ids_to_remove.iter().map(|s| s.as_str()).collect();

        let edge_keys_to_remove: Vec<(String, String)> = edges
            .iter()
            .filter_map(|((src, tgt), edge_props)| {
                // Remove if either endpoint was deleted OR if edge has workspace_id property
                let endpoint_deleted =
                    node_set.contains(src.as_str()) || node_set.contains(tgt.as_str());
                let edge_workspace_match = edge_props
                    .get("workspace_id")
                    .and_then(|v| v.as_str())
                    .map(|ws| ws == workspace_id_str)
                    .unwrap_or(false);

                if endpoint_deleted || edge_workspace_match {
                    Some((src.clone(), tgt.clone()))
                } else {
                    None
                }
            })
            .collect();

        let edges_deleted = edge_keys_to_remove.len();

        // Remove edges
        for key in &edge_keys_to_remove {
            edges.remove(key);
        }

        // Update adjacency for remaining nodes (remove edges to deleted nodes)
        for neighbors in adjacency.values_mut() {
            neighbors.retain(|neighbor| !node_set.contains(neighbor.as_str()));
        }

        Ok((nodes_deleted, edges_deleted))
    }
}

#[async_trait]
impl GraphStorageAnalyticsOps for MemoryGraphStorage {
    async fn node_count(&self) -> Result<usize> {
        let nodes = self.nodes.read().map_err(super::lock::map_lock_err)?;
        Ok(nodes.len())
    }

    async fn edge_count(&self) -> Result<usize> {
        let edges = self.edges.read().map_err(super::lock::map_lock_err)?;
        Ok(edges.len())
    }

    async fn node_count_by_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
        let nodes = self.nodes.read().map_err(super::lock::map_lock_err)?;
        let workspace_id_str = workspace_id.to_string();
        Ok(nodes
            .values()
            .filter(|props| {
                props
                    .get("workspace_id")
                    .and_then(|v| v.as_str())
                    .is_some_and(|ws| ws == workspace_id_str)
            })
            .count())
    }

    async fn edge_count_by_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
        let edges = self.edges.read().map_err(super::lock::map_lock_err)?;
        let workspace_id_str = workspace_id.to_string();
        Ok(edges
            .values()
            .filter(|props| {
                props
                    .get("workspace_id")
                    .and_then(|v| v.as_str())
                    .is_some_and(|ws| ws == workspace_id_str)
            })
            .count())
    }

    async fn distinct_node_type_count_by_workspace(
        &self,
        workspace_id: &uuid::Uuid,
    ) -> Result<usize> {
        let nodes = self.nodes.read().map_err(super::lock::map_lock_err)?;
        let workspace_id_str = workspace_id.to_string();
        let types: std::collections::HashSet<&str> = nodes
            .values()
            .filter(|props| {
                props
                    .get("workspace_id")
                    .and_then(|v| v.as_str())
                    .is_some_and(|ws| ws == workspace_id_str)
            })
            .filter_map(|props| props.get("entity_type").and_then(|v| v.as_str()))
            .collect();
        Ok(types.len())
    }
}

#[async_trait]
impl GraphScanOps for MemoryGraphStorage {
    async fn list_nodes_filtered(
        &self,
        filter: &NodeListFilter,
        offset: usize,
        limit: usize,
    ) -> Result<PagedGraphResult<GraphNode>> {
        let nodes = self.nodes.read().map_err(super::lock::map_lock_err)?;

        let mut matching_ids: Vec<String> = nodes
            .iter()
            .filter_map(|(id, props)| {
                let node = GraphNode {
                    id: id.clone(),
                    properties: props.clone(),
                };
                node_matches_list_filter(&node, filter).then_some(id.clone())
            })
            .collect();

        matching_ids.sort();
        let total = matching_ids.len();
        let page_ids: Vec<String> = matching_ids.into_iter().skip(offset).take(limit).collect();

        let items: Vec<GraphNode> = page_ids
            .iter()
            .filter_map(|id| {
                nodes.get(id).map(|props| GraphNode {
                    id: id.clone(),
                    properties: props.clone(),
                })
            })
            .collect();

        Ok(PagedGraphResult {
            items,
            total,
            offset,
            limit,
        })
    }

    async fn list_edges_filtered(
        &self,
        filter: &EdgeListFilter,
        offset: usize,
        limit: usize,
    ) -> Result<PagedGraphResult<GraphEdge>> {
        let edges = self.edges.read().map_err(super::lock::map_lock_err)?;

        let mut matching: Vec<GraphEdge> = edges
            .iter()
            .map(|((source, target), props)| GraphEdge {
                source: source.clone(),
                target: target.clone(),
                properties: props.clone(),
            })
            .filter(|edge| edge_matches_list_filter(edge, filter))
            .collect();

        matching.sort_by(|a, b| {
            let a_id = format!("{}_{}", a.source, a.target);
            let b_id = format!("{}_{}", b.source, b.target);
            a_id.cmp(&b_id)
        });

        let total = matching.len();
        let items: Vec<GraphEdge> = matching.into_iter().skip(offset).take(limit).collect();

        Ok(PagedGraphResult {
            items,
            total,
            offset,
            limit,
        })
    }

    async fn find_nodes_by_source_prefixes(
        &self,
        filter: &NodeListFilter,
        source_prefixes: &[String],
    ) -> Result<Vec<GraphNode>> {
        let nodes = self.nodes.read().map_err(super::lock::map_lock_err)?;
        let mut matched: Vec<GraphNode> = nodes
            .iter()
            .filter_map(|(id, props)| {
                let node = GraphNode {
                    id: id.clone(),
                    properties: props.clone(),
                };
                if node_matches_list_filter(&node, filter)
                    && sources_match_prefixes(&node.properties, source_prefixes)
                {
                    Some(node)
                } else {
                    None
                }
            })
            .collect();
        matched.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(matched)
    }

    async fn find_edges_by_source_prefixes(
        &self,
        filter: &EdgeListFilter,
        source_prefixes: &[String],
    ) -> Result<Vec<GraphEdge>> {
        let edges = self.edges.read().map_err(super::lock::map_lock_err)?;
        let mut matched: Vec<GraphEdge> = edges
            .iter()
            .map(|((source, target), props)| GraphEdge {
                source: source.clone(),
                target: target.clone(),
                properties: props.clone(),
            })
            .filter(|edge| {
                edge_matches_list_filter(edge, filter)
                    && sources_match_prefixes(&edge.properties, source_prefixes)
            })
            .collect();
        matched.sort_by(|a, b| {
            let a_id = format!("{}_{}", a.source, a.target);
            let b_id = format!("{}_{}", b.source, b.target);
            a_id.cmp(&b_id)
        });
        Ok(matched)
    }

    async fn find_edge_by_relationship_id(
        &self,
        filter: &EdgeListFilter,
        relationship_id: &str,
    ) -> Result<Option<GraphEdge>> {
        let edges = self.edges.read().map_err(super::lock::map_lock_err)?;
        for ((source, target), props) in edges.iter() {
            let edge = GraphEdge {
                source: source.clone(),
                target: target.clone(),
                properties: props.clone(),
            };
            if edge_matches_list_filter(&edge, filter)
                && edge_matches_relationship_id(&edge, relationship_id)
            {
                return Ok(Some(edge));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{GraphStorageMutateOps, GraphStorageReadOps};

    #[tokio::test]
    async fn test_graph_node_operations() {
        let storage = MemoryGraphStorage::new("test");
        storage.initialize().await.unwrap();

        // Create nodes
        let mut props = HashMap::new();
        props.insert("name".to_string(), serde_json::json!("Alice"));
        storage.upsert_node("alice", props).await.unwrap();

        assert!(storage.has_node("alice").await.unwrap());
        assert!(!storage.has_node("bob").await.unwrap());

        let node = storage.get_node("alice").await.unwrap().unwrap();
        assert_eq!(node.id, "alice");
    }

    #[tokio::test]
    async fn test_graph_edge_operations() {
        let storage = MemoryGraphStorage::new("test");

        storage.upsert_node("alice", HashMap::new()).await.unwrap();
        storage.upsert_node("bob", HashMap::new()).await.unwrap();

        let mut props = HashMap::new();
        props.insert("relation".to_string(), serde_json::json!("knows"));
        storage.upsert_edge("alice", "bob", props).await.unwrap();

        assert!(storage.has_edge("alice", "bob").await.unwrap());
        assert!(storage.has_edge("bob", "alice").await.unwrap()); // Symmetric

        assert_eq!(storage.node_degree("alice").await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_graph_traversal() {
        let storage = MemoryGraphStorage::new("test");

        // Create a small graph: A -- B -- C
        storage.upsert_node("A", HashMap::new()).await.unwrap();
        storage.upsert_node("B", HashMap::new()).await.unwrap();
        storage.upsert_node("C", HashMap::new()).await.unwrap();

        storage.upsert_edge("A", "B", HashMap::new()).await.unwrap();
        storage.upsert_edge("B", "C", HashMap::new()).await.unwrap();

        // Traverse from A
        let kg = storage
            .get_knowledge_graph("A", 2, 10, None, None)
            .await
            .unwrap();

        assert_eq!(kg.node_count(), 3);
        assert_eq!(kg.edge_count(), 2);
    }

    #[tokio::test]
    async fn test_graph_traversal_scoped_by_tenant_workspace() {
        let storage = MemoryGraphStorage::new("test");

        let mut props_a = HashMap::new();
        props_a.insert("tenant_id".to_string(), serde_json::json!("t1"));
        props_a.insert("workspace_id".to_string(), serde_json::json!("w1"));
        storage.upsert_node("A", props_a.clone()).await.unwrap();

        let mut props_b = HashMap::new();
        props_b.insert("tenant_id".to_string(), serde_json::json!("t1"));
        props_b.insert("workspace_id".to_string(), serde_json::json!("w1"));
        storage.upsert_node("B", props_b).await.unwrap();

        let mut props_c = HashMap::new();
        props_c.insert("tenant_id".to_string(), serde_json::json!("t2"));
        props_c.insert("workspace_id".to_string(), serde_json::json!("w2"));
        storage.upsert_node("C", props_c).await.unwrap();

        storage.upsert_edge("A", "B", HashMap::new()).await.unwrap();
        storage.upsert_edge("B", "C", HashMap::new()).await.unwrap();

        let scoped = storage
            .get_knowledge_graph("A", 2, 10, Some("t1"), Some("w1"))
            .await
            .unwrap();
        assert_eq!(scoped.node_count(), 2);
        assert!(scoped.nodes.iter().all(|n| n.id == "A" || n.id == "B"));

        let cross_tenant = storage
            .get_knowledge_graph("C", 2, 10, Some("t1"), Some("w1"))
            .await
            .unwrap();
        assert_eq!(cross_tenant.node_count(), 0);
    }

    #[tokio::test]
    async fn test_graph_delete_cascade() {
        let storage = MemoryGraphStorage::new("test");

        storage.upsert_node("A", HashMap::new()).await.unwrap();
        storage.upsert_node("B", HashMap::new()).await.unwrap();
        storage.upsert_edge("A", "B", HashMap::new()).await.unwrap();

        assert_eq!(storage.edge_count().await.unwrap(), 1);

        storage.delete_node("A").await.unwrap();

        assert!(!storage.has_node("A").await.unwrap());
        assert_eq!(storage.edge_count().await.unwrap(), 0);
    }
}
