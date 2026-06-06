//! Graph storage trait for knowledge graph operations.
//!
//! # Implements
//!
//! - **FEAT0202**: Graph Traversal (get_node_edges, get_neighbors)
//! - **FEAT0203**: Graph Mutation (upsert_node, upsert_edge, delete_*)
//! - **FEAT0204**: Graph Analytics (node_count, edge_count)
//!
//! # Enforces
//!
//! - **BR0008**: Entity names normalized (via caller, not trait)
//! - **BR0201**: Namespace-based tenant isolation
//!
//! # WHY: Property Graph Model
//!
//! We use a property graph (nodes + edges with arbitrary properties) because:
//! - Entities have varying attributes (type, description, source_id)
//! - Relationships have metadata (weight, keywords, timestamps)
//! - Flexible schema accommodates different domains
//!
//! This model is compatible with:
//! - Apache AGE (PostgreSQL graph extension)
//! - Neo4j, Neptune, and other graph databases

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
/// Composite of read / mutate / analytics operation traits (SPEC-017 ISP Phase 2b).
/// Adapters implement subtraits in separate `impl` blocks; callers may use
/// [`GraphReadView`](super::graph_read_view::GraphReadView) for read-only paths.
///
/// # Implementations
///
/// - `MemoryGraphStorage` - In-memory graph (testing)
/// - `PostgresAGEStorage` - PostgreSQL with Apache AGE extension
#[async_trait]
pub trait GraphStorage:
    super::graph_read_ops::GraphStorageReadOps
    + super::graph_scan_ops::GraphScanOps
    + super::graph_mutate_ops::GraphStorageMutateOps
    + super::graph_analytics_ops::GraphStorageAnalyticsOps
{
    /// Get the storage namespace.
    fn namespace(&self) -> &str;

    /// Initialize the graph storage.
    async fn initialize(&self) -> Result<()>;

    /// Flush pending changes.
    async fn finalize(&self) -> Result<()>;
}
