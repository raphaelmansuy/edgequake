//! Graph storage trait.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::Result;

/// A node in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// Node identifier (typically the entity name)
    pub id: String,
    /// Node properties
    pub properties: HashMap<String, serde_json::Value>,
}

impl GraphNode {
    /// Create a new graph node.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            properties: HashMap::new(),
        }
    }

    /// Create a node with properties.
    pub fn with_properties(
        id: impl Into<String>,
        properties: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            id: id.into(),
            properties,
        }
    }

    /// Add a property to the node.
    pub fn set_property(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.properties.insert(key.into(), value);
    }

    /// Get a property value.
    pub fn get_property(&self, key: &str) -> Option<&serde_json::Value> {
        self.properties.get(key)
    }
}

/// An edge in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Source node identifier
    pub source: String,
    /// Target node identifier
    pub target: String,
    /// Edge properties
    pub properties: HashMap<String, serde_json::Value>,
}

impl GraphEdge {
    /// Create a new graph edge.
    pub fn new(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            properties: HashMap::new(),
        }
    }

    /// Create an edge with properties.
    pub fn with_properties(
        source: impl Into<String>,
        target: impl Into<String>,
        properties: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            properties,
        }
    }

    /// Add a property to the edge.
    pub fn set_property(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.properties.insert(key.into(), value);
    }

    /// Get a property value.
    pub fn get_property(&self, key: &str) -> Option<&serde_json::Value> {
        self.properties.get(key)
    }
}

/// A subgraph extracted from the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    /// Nodes in the subgraph
    pub nodes: Vec<GraphNode>,
    /// Edges in the subgraph
    pub edges: Vec<GraphEdge>,
    /// Whether the result was truncated due to size limits
    pub is_truncated: bool,
}

impl KnowledgeGraph {
    /// Create an empty knowledge graph.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            is_truncated: false,
        }
    }

    /// Get the number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the number of edges.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Check if the graph is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty() && self.edges.is_empty()
    }

    /// Add a node to the graph.
    pub fn add_node(&mut self, node: GraphNode) {
        self.nodes.push(node);
    }

    /// Add an edge to the graph.
    pub fn add_edge(&mut self, edge: GraphEdge) {
        self.edges.push(edge);
    }
}

impl Default for KnowledgeGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Graph storage interface for the knowledge graph.
///
/// Provides storage and querying of nodes (entities) and
/// edges (relationships) in the knowledge graph.
///
/// # Implementations
///
/// - `MemoryGraphStorage` - In-memory graph (testing)
/// - `PostgresAGEStorage` - PostgreSQL with Apache AGE extension
/// - `SurrealDBGraphStorage` - SurrealDB graph relations
#[async_trait]
pub trait GraphStorage: Send + Sync {
    /// Get the storage namespace.
    fn namespace(&self) -> &str;

    /// Initialize the graph storage.
    async fn initialize(&self) -> Result<()>;

    /// Flush pending changes.
    async fn finalize(&self) -> Result<()>;

    // ========== Node Operations ==========

    /// Check if a node exists.
    async fn has_node(&self, node_id: &str) -> Result<bool>;

    /// Get a node by ID.
    async fn get_node(&self, node_id: &str) -> Result<Option<GraphNode>>;

    /// Insert or update a node.
    async fn upsert_node(
        &self,
        node_id: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<()>;

    /// Insert or update multiple nodes in batch.
    ///
    /// Default implementation calls `upsert_node` sequentially.
    /// Implementations should override this for better performance.
    async fn upsert_nodes_batch(
        &self,
        nodes: &[(String, HashMap<String, serde_json::Value>)],
    ) -> Result<()> {
        for (node_id, properties) in nodes {
            self.upsert_node(node_id, properties.clone()).await?;
        }
        Ok(())
    }

    /// Delete a node and its connected edges.
    async fn delete_node(&self, node_id: &str) -> Result<()>;

    /// Get the degree (number of edges) of a node.
    async fn node_degree(&self, node_id: &str) -> Result<usize>;

    /// Get all nodes.
    async fn get_all_nodes(&self) -> Result<Vec<GraphNode>>;

    /// Get nodes by a list of IDs.
    async fn get_nodes_by_ids(&self, node_ids: &[String]) -> Result<Vec<GraphNode>>;

    // ========== Edge Operations ==========

    /// Check if an edge exists between two nodes.
    async fn has_edge(&self, source: &str, target: &str) -> Result<bool>;

    /// Get an edge between two nodes.
    async fn get_edge(&self, source: &str, target: &str) -> Result<Option<GraphEdge>>;

    /// Insert or update an edge.
    async fn upsert_edge(
        &self,
        source: &str,
        target: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<()>;

    /// Insert or update multiple edges in batch.
    ///
    /// Default implementation calls `upsert_edge` sequentially.
    /// Implementations should override this for better performance.
    async fn upsert_edges_batch(
        &self,
        edges: &[(String, String, HashMap<String, serde_json::Value>)],
    ) -> Result<()> {
        for (source, target, properties) in edges {
            self.upsert_edge(source, target, properties.clone()).await?;
        }
        Ok(())
    }

    /// Delete an edge.
    async fn delete_edge(&self, source: &str, target: &str) -> Result<()>;

    /// Get all edges connected to a node.
    async fn get_node_edges(&self, node_id: &str) -> Result<Vec<GraphEdge>>;

    /// Get all edges.
    async fn get_all_edges(&self) -> Result<Vec<GraphEdge>>;

    // ========== Graph Queries ==========

    /// Extract a subgraph starting from a node.
    ///
    /// # Arguments
    ///
    /// * `start_node` - Starting node for traversal
    /// * `max_depth` - Maximum traversal depth
    /// * `max_nodes` - Maximum nodes to return
    async fn get_knowledge_graph(
        &self,
        start_node: &str,
        max_depth: usize,
        max_nodes: usize,
    ) -> Result<KnowledgeGraph>;

    /// Get the most connected (popular) node labels.
    async fn get_popular_labels(&self, limit: usize) -> Result<Vec<String>>;

    /// Search for nodes by label prefix.
    async fn search_labels(&self, query: &str, limit: usize) -> Result<Vec<String>>;

    /// Get neighbors of a node at a specific depth.
    async fn get_neighbors(&self, node_id: &str, depth: usize) -> Result<Vec<GraphNode>>;

    // ========== Utility Operations ==========

    /// Get node count.
    async fn node_count(&self) -> Result<usize>;

    /// Get edge count.
    async fn edge_count(&self) -> Result<usize>;

    /// Clear all nodes and edges.
    async fn clear(&self) -> Result<()>;
}
