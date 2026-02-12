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

    /// Get degrees for multiple nodes in a single query (batch operation).
    ///
    /// This is significantly more efficient than calling `node_degree` multiple times.
    /// Default implementation calls `node_degree` sequentially (N queries).
    /// Implementations should override for performance (1 query with IN clause).
    ///
    /// # Arguments
    ///
    /// * `node_ids` - List of node IDs to query
    ///
    /// # Returns
    ///
    /// Vector of (node_id, degree) tuples in unspecified order
    async fn node_degrees_batch(&self, node_ids: &[String]) -> Result<Vec<(String, usize)>> {
        let mut results = Vec::new();
        for node_id in node_ids {
            let degree = self.node_degree(node_id).await?;
            results.push((node_id.clone(), degree));
        }
        Ok(results)
    }

    /// Get all nodes.
    async fn get_all_nodes(&self) -> Result<Vec<GraphNode>>;

    /// Get nodes by a list of IDs.
    async fn get_nodes_by_ids(&self, node_ids: &[String]) -> Result<Vec<GraphNode>>;

    /// Get nodes as a HashMap keyed by node_id (LightRAG-inspired batch pattern).
    ///
    /// This method is optimized for O(1) lookups after retrieval.
    /// Uses UNNEST with ORDINALITY for efficient batch SQL queries.
    ///
    /// Default implementation wraps `get_nodes_by_ids`.
    /// PostgreSQL implementation uses optimized batch SQL.
    ///
    /// # Arguments
    /// * `node_ids` - List of node IDs to fetch
    ///
    /// # Returns
    /// HashMap mapping node_id -> GraphNode for found nodes
    async fn get_nodes_batch(&self, node_ids: &[String]) -> Result<HashMap<String, GraphNode>> {
        let nodes = self.get_nodes_by_ids(node_ids).await?;
        Ok(nodes.into_iter().map(|n| (n.id.clone(), n)).collect())
    }

    /// Get edges where BOTH endpoints are in the specified node set.
    ///
    /// This is the LightRAG-inspired pattern that eliminates the
    /// "fetch-all-edges-then-filter" anti-pattern by using SQL JOINs.
    ///
    /// Default implementation falls back to `get_edges_for_node_set`.
    /// PostgreSQL implementation uses optimized batch SQL.
    ///
    /// # Arguments
    /// * `node_ids` - Set of node IDs to filter edges
    ///
    /// # Returns
    /// Edges where both source and target are in the node set
    async fn get_edges_for_nodes_batch(&self, node_ids: &[String]) -> Result<Vec<GraphEdge>> {
        self.get_edges_for_node_set(node_ids, None, None).await
    }

    /// Get nodes with their in/out degrees in a single batch query.
    ///
    /// Combines node retrieval with degree calculation for efficiency.
    /// Returns (node, in_degree, out_degree) tuples.
    ///
    /// Default implementation combines two separate queries.
    /// PostgreSQL implementation uses optimized single query.
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
            // Default: assume symmetric (no in/out distinction)
            result.push((node, total_degree, total_degree));
        }
        Ok(result)
    }

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

    /// Search for nodes with full text matching on label and description.
    ///
    /// Returns nodes with their degree, optionally filtered by entity type.
    /// Searches both label and description fields.
    ///
    /// # Arguments
    ///
    /// * `query` - Search text (matches label or description)
    /// * `limit` - Maximum nodes to return
    /// * `entity_type` - Optional filter by entity type
    /// * `tenant_id` - Tenant context for multi-tenancy (optional)
    /// * `workspace_id` - Workspace context (optional)
    ///
    /// # Returns
    ///
    /// Vector of (GraphNode, degree) tuples matching the search
    async fn search_nodes(
        &self,
        query: &str,
        limit: usize,
        entity_type: Option<&str>,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<(GraphNode, usize)>>;

    /// Get neighbors of a node at a specific depth.
    async fn get_neighbors(&self, node_id: &str, depth: usize) -> Result<Vec<GraphNode>>;

    // ========== Optimized Batch Operations ==========

    /// Get popular nodes with their degrees in a single query.
    ///
    /// This method eliminates N+1 query patterns by returning nodes
    /// with their connection counts in one database round-trip.
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum nodes to return
    /// * `min_degree` - Minimum connection count (optional filter)
    /// * `entity_type` - Filter by entity type (optional)
    /// * `tenant_id` - Tenant context for multi-tenancy (optional)
    /// * `workspace_id` - Workspace context (optional)
    ///
    /// # Returns
    ///
    /// Vector of (GraphNode, degree) tuples, ordered by degree descending
    async fn get_popular_nodes_with_degree(
        &self,
        limit: usize,
        min_degree: Option<usize>,
        entity_type: Option<&str>,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<(GraphNode, usize)>> {
        // Default implementation uses existing methods (N+1 pattern)
        // Implementations should override for performance
        let labels = self.get_popular_labels(limit * 2).await?;
        let mut results = Vec::new();

        for label in labels {
            if results.len() >= limit {
                break;
            }
            if let Some(node) = self.get_node(&label).await? {
                let degree = self.node_degree(&label).await?;

                // Apply min_degree filter
                if let Some(min) = min_degree {
                    if degree < min {
                        continue;
                    }
                }

                // Apply entity_type filter
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

                // Apply tenant filter
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

                // Apply workspace filter
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

    /// Get edges between nodes in a specified set.
    ///
    /// This method eliminates the "fetch-all-then-filter" pattern by
    /// querying only edges that connect nodes in the given set.
    ///
    /// # Arguments
    ///
    /// * `node_ids` - Set of node IDs to filter edges
    /// * `tenant_id` - Tenant context (optional)
    /// * `workspace_id` - Workspace context (optional)
    ///
    /// # Returns
    ///
    /// Edges where both source and target are in the node set
    async fn get_edges_for_node_set(
        &self,
        node_ids: &[String],
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<GraphEdge>> {
        // Default implementation uses existing methods
        // Implementations should override for performance
        let all_edges = self.get_all_edges().await?;
        let node_set: std::collections::HashSet<&str> =
            node_ids.iter().map(|s| s.as_str()).collect();

        let filtered: Vec<GraphEdge> = all_edges
            .into_iter()
            .filter(|e| {
                // Both endpoints must be in the node set
                if !node_set.contains(e.source.as_str()) || !node_set.contains(e.target.as_str()) {
                    return false;
                }

                // Apply tenant filter
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

                // Apply workspace filter
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

    // ========== Utility Operations ==========

    /// Get node count.
    async fn node_count(&self) -> Result<usize>;

    /// Get edge count.
    async fn edge_count(&self) -> Result<usize>;

    /// Get node count for a specific workspace.
    ///
    /// WHY: Dashboard and workspace pages need accurate per-workspace statistics.
    /// This enables multi-tenant environments to show isolated entity counts.
    ///
    /// Default implementation falls back to global count (not workspace-scoped).
    /// Implementations should override for accurate workspace statistics.
    async fn node_count_by_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
        let _ = workspace_id;
        self.node_count().await
    }

    /// Get edge count for a specific workspace.
    ///
    /// WHY: Dashboard needs accurate relationship counts per workspace.
    /// This complements node_count_by_workspace for complete statistics.
    ///
    /// Default implementation falls back to global count (not workspace-scoped).
    /// Implementations should override for accurate workspace statistics.
    async fn edge_count_by_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
        let _ = workspace_id;
        self.edge_count().await
    }

    /// Get the number of distinct entity types for a specific workspace.
    ///
    /// WHY: The dashboard EntityTypes KPI card needs this count. Previously the
    /// frontend fetched ALL graph nodes just to compute
    /// `new Set(nodes.map(n => n.node_type)).size`, which is O(N) in nodes and
    /// extremely slow for large workspaces (8000+ nodes). This trait method
    /// allows PostgreSQL to answer with a single `COUNT(DISTINCT ...)` query.
    ///
    /// Default implementation fetches all nodes and counts unique types in
    /// memory — still faster than the old frontend approach because data
    /// doesn't cross the network, but implementations should override with
    /// a native aggregate query.
    async fn distinct_node_type_count_by_workspace(
        &self,
        workspace_id: &uuid::Uuid,
    ) -> Result<usize> {
        let _ = workspace_id;
        let nodes = self.get_all_nodes().await?;
        let types: std::collections::HashSet<&str> = nodes
            .iter()
            .filter_map(|n| n.properties.get("entity_type").and_then(|v| v.as_str()))
            .collect();
        Ok(types.len())
    }

    /// Clear all nodes and edges.
    async fn clear(&self) -> Result<()>;

    /// Clear nodes and edges for a specific workspace.
    ///
    /// This method removes only data belonging to the specified workspace,
    /// allowing multi-tenant environments to rebuild one workspace without
    /// affecting others.
    ///
    /// Returns a tuple of (nodes_deleted, edges_deleted).
    ///
    /// Default implementation returns (0, 0) - implementations should override.
    async fn clear_workspace(&self, workspace_id: &uuid::Uuid) -> Result<(usize, usize)> {
        let _ = workspace_id;
        Ok((0, 0))
    }
}
