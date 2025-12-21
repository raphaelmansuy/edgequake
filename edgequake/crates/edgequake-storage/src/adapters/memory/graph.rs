//! In-memory graph storage.

use async_trait::async_trait;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::RwLock;

use crate::error::{Result, StorageError};
use crate::traits::{GraphEdge, GraphNode, GraphStorage, KnowledgeGraph};

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

    async fn has_node(&self, node_id: &str) -> Result<bool> {
        let nodes = self.nodes.read().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;
        Ok(nodes.contains_key(node_id))
    }

    async fn get_node(&self, node_id: &str) -> Result<Option<GraphNode>> {
        let nodes = self.nodes.read().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;

        Ok(nodes.get(node_id).map(|props| GraphNode {
            id: node_id.to_string(),
            properties: props.clone(),
        }))
    }

    async fn upsert_node(
        &self,
        node_id: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let mut nodes = self.nodes.write().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;
        let mut adjacency = self.adjacency.write().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;

        nodes.insert(node_id.to_string(), properties);
        adjacency.entry(node_id.to_string()).or_default();

        Ok(())
    }

    async fn delete_node(&self, node_id: &str) -> Result<()> {
        let mut nodes = self.nodes.write().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;
        let mut edges = self.edges.write().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;
        let mut adjacency = self.adjacency.write().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;

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

    async fn node_degree(&self, node_id: &str) -> Result<usize> {
        let adjacency = self.adjacency.read().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;

        Ok(adjacency.get(node_id).map(|n| n.len()).unwrap_or(0))
    }

    async fn get_all_nodes(&self) -> Result<Vec<GraphNode>> {
        let nodes = self.nodes.read().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;

        Ok(nodes
            .iter()
            .map(|(id, props)| GraphNode {
                id: id.clone(),
                properties: props.clone(),
            })
            .collect())
    }

    async fn get_nodes_by_ids(&self, node_ids: &[String]) -> Result<Vec<GraphNode>> {
        let nodes = self.nodes.read().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;

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

    async fn has_edge(&self, source: &str, target: &str) -> Result<bool> {
        let edges = self.edges.read().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;
        let key = Self::edge_key(source, target);
        Ok(edges.contains_key(&key))
    }

    async fn get_edge(&self, source: &str, target: &str) -> Result<Option<GraphEdge>> {
        let edges = self.edges.read().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;
        let key = Self::edge_key(source, target);

        Ok(edges.get(&key).map(|props| GraphEdge {
            source: key.0.clone(),
            target: key.1.clone(),
            properties: props.clone(),
        }))
    }

    async fn upsert_edge(
        &self,
        source: &str,
        target: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let mut edges = self.edges.write().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;
        let mut adjacency = self.adjacency.write().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;

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

    async fn delete_edge(&self, source: &str, target: &str) -> Result<()> {
        let mut edges = self.edges.write().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;
        let mut adjacency = self.adjacency.write().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;

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

    async fn get_node_edges(&self, node_id: &str) -> Result<Vec<GraphEdge>> {
        let edges = self.edges.read().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;

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

    async fn get_all_edges(&self) -> Result<Vec<GraphEdge>> {
        let edges = self.edges.read().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;

        Ok(edges
            .iter()
            .map(|((s, t), props)| GraphEdge {
                source: s.clone(),
                target: t.clone(),
                properties: props.clone(),
            })
            .collect())
    }

    async fn get_knowledge_graph(
        &self,
        start_node: &str,
        max_depth: usize,
        max_nodes: usize,
    ) -> Result<KnowledgeGraph> {
        let nodes_map = self.nodes.read().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;
        let edges_map = self.edges.read().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;
        let adjacency = self.adjacency.read().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;

        let mut visited: HashSet<String> = HashSet::new();
        let mut result_nodes: Vec<GraphNode> = Vec::new();
        let mut result_edges: Vec<GraphEdge> = Vec::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();

        queue.push_back((start_node.to_string(), 0));

        while let Some((node_id, depth)) = queue.pop_front() {
            if visited.contains(&node_id) || depth > max_depth || result_nodes.len() >= max_nodes {
                continue;
            }

            visited.insert(node_id.clone());

            if let Some(props) = nodes_map.get(&node_id) {
                result_nodes.push(GraphNode {
                    id: node_id.clone(),
                    properties: props.clone(),
                });
            }

            if let Some(neighbors) = adjacency.get(&node_id) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        queue.push_back((neighbor.clone(), depth + 1));
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

    async fn get_popular_labels(&self, limit: usize) -> Result<Vec<String>> {
        let adjacency = self.adjacency.read().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;

        let mut node_degrees: Vec<(String, usize)> = adjacency
            .iter()
            .map(|(id, neighbors)| (id.clone(), neighbors.len()))
            .collect();

        node_degrees.sort_by(|a, b| b.1.cmp(&a.1));

        Ok(node_degrees.into_iter().take(limit).map(|(id, _)| id).collect())
    }

    async fn search_labels(&self, query: &str, limit: usize) -> Result<Vec<String>> {
        let nodes = self.nodes.read().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;

        let query_lower = query.to_lowercase();

        Ok(nodes
            .keys()
            .filter(|id| id.to_lowercase().contains(&query_lower))
            .take(limit)
            .cloned()
            .collect())
    }

    async fn get_neighbors(&self, node_id: &str, depth: usize) -> Result<Vec<GraphNode>> {
        let kg = self.get_knowledge_graph(node_id, depth, 1000).await?;
        Ok(kg.nodes.into_iter().filter(|n| n.id != node_id).collect())
    }

    async fn node_count(&self) -> Result<usize> {
        let nodes = self.nodes.read().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;
        Ok(nodes.len())
    }

    async fn edge_count(&self) -> Result<usize> {
        let edges = self.edges.read().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;
        Ok(edges.len())
    }

    async fn clear(&self) -> Result<()> {
        let mut nodes = self.nodes.write().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;
        let mut edges = self.edges.write().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;
        let mut adjacency = self.adjacency.write().map_err(|e| {
            StorageError::Database(format!("Lock error: {}", e))
        })?;

        nodes.clear();
        edges.clear();
        adjacency.clear();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let kg = storage.get_knowledge_graph("A", 2, 10).await.unwrap();

        assert_eq!(kg.node_count(), 3);
        assert_eq!(kg.edge_count(), 2);
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
