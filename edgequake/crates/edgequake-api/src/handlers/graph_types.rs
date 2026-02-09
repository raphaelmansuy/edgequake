//! Graph handler DTOs and request/response types.
//!
//! This module contains all Data Transfer Objects (DTOs) for graph operations,
//! extracted from the main graph.rs handler for better modularity.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ============================================================================
// Graph Core DTOs
// ============================================================================

/// Graph node response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GraphNodeResponse {
    /// Node ID.
    pub id: String,

    /// Node label/name.
    pub label: String,

    /// Node type.
    pub node_type: String,

    /// Node description.
    pub description: String,

    /// Number of connections.
    pub degree: usize,

    /// Additional properties.
    pub properties: serde_json::Value,
}

/// Graph edge response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GraphEdgeResponse {
    /// Source node ID.
    pub source: String,

    /// Target node ID.
    pub target: String,

    /// Edge type.
    pub edge_type: String,

    /// Edge weight.
    pub weight: f32,

    /// Additional properties.
    pub properties: serde_json::Value,
}

/// Knowledge graph response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct KnowledgeGraphResponse {
    /// Nodes in the graph.
    pub nodes: Vec<GraphNodeResponse>,

    /// Edges in the graph.
    pub edges: Vec<GraphEdgeResponse>,

    /// Whether the graph was truncated.
    pub is_truncated: bool,

    /// Total node count in storage.
    pub total_nodes: usize,

    /// Total edge count in storage.
    pub total_edges: usize,
}

/// Graph query parameters.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GraphQueryParams {
    /// Starting node ID.
    pub start_node: Option<String>,

    /// Maximum traversal depth.
    #[serde(default = "default_depth")]
    pub depth: usize,

    /// Maximum nodes to return.
    #[serde(default = "default_max_nodes")]
    pub max_nodes: usize,
}

/// WHY: Maximum allowed nodes per graph request - prevents performance issues
/// even if frontend validation is bypassed. Matches frontend MAX_DISPLAY_NODES.
pub const MAX_GRAPH_NODES: usize = 500;

/// Maximum traversal depth to prevent exponential explosion.
pub const MAX_GRAPH_DEPTH: usize = 5;

/// Default traversal depth.
pub fn default_depth() -> usize {
    2
}

/// Default max nodes.
pub fn default_max_nodes() -> usize {
    100
}

impl GraphQueryParams {
    /// WHY: Defense in depth - clamp parameters to safe ranges even if client sends invalid values
    /// This ensures server stability regardless of frontend validation.
    pub fn validated(mut self) -> Self {
        self.max_nodes = self.max_nodes.clamp(1, MAX_GRAPH_NODES);
        self.depth = self.depth.clamp(1, MAX_GRAPH_DEPTH);
        self
    }
}

// ============================================================================
// Label Search DTOs
// ============================================================================

/// Search labels query.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SearchLabelsQuery {
    /// Search query.
    pub q: String,

    /// Maximum results.
    #[serde(default = "graph_default_limit")]
    pub limit: usize,
}

/// Default search limit for graph operations.
pub fn graph_default_limit() -> usize {
    20
}

/// Search labels response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SearchLabelsResponse {
    /// Matching labels.
    pub labels: Vec<String>,
}

// ============================================================================
// Search Nodes DTOs (Full Node Search)
// ============================================================================

/// Search nodes query - returns full node data with degree.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SearchNodesQuery {
    /// Search query string (searches label and description).
    pub q: String,

    /// Maximum results to return.
    #[serde(default = "default_search_nodes_limit")]
    pub limit: usize,

    /// Whether to include neighbors of matching nodes.
    #[serde(default)]
    pub include_neighbors: bool,

    /// Neighbor depth when include_neighbors is true.
    #[serde(default = "default_neighbor_depth")]
    pub neighbor_depth: usize,

    /// Filter by entity type (optional).
    pub entity_type: Option<String>,
}

/// Default search nodes limit.
pub fn default_search_nodes_limit() -> usize {
    50
}

/// Default neighbor depth for search.
pub fn default_neighbor_depth() -> usize {
    1
}

/// Search nodes response - full node data for graph display.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SearchNodesResponse {
    /// Matching nodes with full data.
    pub nodes: Vec<GraphNodeResponse>,

    /// Edges connecting the returned nodes.
    pub edges: Vec<GraphEdgeResponse>,

    /// Total matches in database (before limit).
    pub total_matches: usize,

    /// Whether results were truncated.
    pub is_truncated: bool,
}

// ============================================================================
// Popular Labels DTOs
// ============================================================================

/// Query parameters for popular labels.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PopularLabelsQuery {
    /// Maximum number of labels to return.
    #[serde(default = "default_popular_limit")]
    pub limit: usize,

    /// Minimum degree (connections) to include.
    #[serde(default)]
    pub min_degree: Option<usize>,

    /// Filter by entity type.
    #[serde(default)]
    pub entity_type: Option<String>,
}

/// Default popular labels limit.
pub fn default_popular_limit() -> usize {
    50
}

/// Popular label with metadata.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PopularLabel {
    /// Label/entity name.
    pub label: String,

    /// Entity type.
    pub entity_type: String,

    /// Number of connections (degree).
    pub degree: usize,

    /// Brief description.
    pub description: String,
}

/// Response with popular labels.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PopularLabelsResponse {
    /// List of popular labels sorted by degree.
    pub labels: Vec<PopularLabel>,

    /// Total entity count in graph.
    pub total_entities: usize,
}

// ============================================================================
// Batch Operations DTOs
// ============================================================================

/// Request body for batch degree query.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BatchDegreeRequest {
    /// List of node IDs to query.
    pub node_ids: Vec<String>,
}

/// Response for a single node degree.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NodeDegree {
    /// Node ID.
    pub node_id: String,

    /// Number of connections.
    pub degree: usize,
}

/// Response for batch degree query.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BatchDegreeResponse {
    /// Degrees for each requested node.
    pub degrees: Vec<NodeDegree>,

    /// Number of nodes queried.
    pub count: usize,
}

// ============================================================================
// Streaming Graph DTOs
// ============================================================================

/// Query parameters for streaming graph endpoint.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GraphStreamQueryParams {
    /// Starting node ID for traversal.
    pub start_node: Option<String>,

    /// Maximum nodes to return.
    #[serde(default = "default_stream_max_nodes")]
    pub max_nodes: usize,

    /// Batch size for streaming (how many nodes per chunk).
    #[serde(default = "default_stream_batch_size")]
    pub batch_size: usize,
}

impl GraphStreamQueryParams {
    /// WHY: Defense in depth - clamp streaming params to safe ranges
    pub fn validated(mut self) -> Self {
        self.max_nodes = self.max_nodes.clamp(1, MAX_GRAPH_NODES);
        self.batch_size = self.batch_size.clamp(10, 100);
        self
    }
}

/// Default streaming depth.
pub fn default_stream_depth() -> usize {
    2
}

/// Default batch size for streaming.
pub fn default_batch_size() -> usize {
    50
}

/// Default stream max nodes (for original SSE endpoint).
pub fn default_stream_max_nodes() -> usize {
    200
}

/// Default stream batch size.
pub fn default_stream_batch_size() -> usize {
    50
}

// ============================================================================
// Streaming Event Types
// ============================================================================

/// Events sent during graph streaming.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(tag = "type")]
pub enum GraphStreamEvent {
    /// Initial metadata about the graph.
    #[serde(rename = "metadata")]
    Metadata {
        /// Total nodes in graph.
        total_nodes: usize,
        /// Total edges in graph.
        total_edges: usize,
        /// Nodes to be streamed.
        nodes_to_stream: usize,
        /// Edges to be streamed (estimated).
        edges_to_stream: usize,
    },

    /// Batch of nodes.
    #[serde(rename = "nodes")]
    Nodes {
        /// Current batch number.
        batch: usize,
        /// Total batches expected.
        total_batches: usize,
        /// Nodes in this batch.
        nodes: Vec<GraphNodeResponse>,
    },

    /// Batch of edges.
    #[serde(rename = "edges")]
    Edges {
        /// Edges in this batch.
        edges: Vec<GraphEdgeResponse>,
    },

    /// Stream complete.
    #[serde(rename = "done")]
    Done {
        /// Total nodes streamed.
        nodes_count: usize,
        /// Total edges streamed.
        edges_count: usize,
        /// Duration in milliseconds.
        duration_ms: u64,
    },

    /// Error during streaming.
    #[serde(rename = "error")]
    Error {
        /// Error message.
        message: String,
    },
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_node_response_serialization() {
        let node = GraphNodeResponse {
            id: "test_node".to_string(),
            label: "Test Node".to_string(),
            node_type: "PERSON".to_string(),
            description: "A test node".to_string(),
            degree: 5,
            properties: serde_json::json!({"custom": "value"}),
        };

        let json = serde_json::to_string(&node).unwrap();
        assert!(json.contains("test_node"));
        assert!(json.contains("PERSON"));
    }

    #[test]
    fn test_graph_edge_response_serialization() {
        let edge = GraphEdgeResponse {
            source: "node_a".to_string(),
            target: "node_b".to_string(),
            edge_type: "RELATED_TO".to_string(),
            weight: 0.8,
            properties: serde_json::json!({}),
        };

        let json = serde_json::to_string(&edge).unwrap();
        assert!(json.contains("node_a"));
        assert!(json.contains("RELATED_TO"));
    }

    #[test]
    fn test_graph_query_params_defaults() {
        let json = r#"{"start_node": "test"}"#;
        let params: GraphQueryParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.depth, 2);
        assert_eq!(params.max_nodes, 100);
    }

    #[test]
    fn test_search_labels_query_defaults() {
        let json = r#"{"q": "test"}"#;
        let query: SearchLabelsQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.limit, 20);
    }

    #[test]
    fn test_popular_labels_query_defaults() {
        let json = r#"{}"#;
        let query: PopularLabelsQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.limit, 50);
        assert!(query.min_degree.is_none());
        assert!(query.entity_type.is_none());
    }

    #[test]
    fn test_batch_degree_request_deserialization() {
        let json = r#"{"node_ids": ["a", "b", "c"]}"#;
        let request: BatchDegreeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.node_ids.len(), 3);
    }

    #[test]
    fn test_knowledge_graph_response_serialization() {
        let response = KnowledgeGraphResponse {
            nodes: vec![],
            edges: vec![],
            is_truncated: false,
            total_nodes: 10,
            total_edges: 20,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("total_nodes"));
        assert!(json.contains("is_truncated"));
    }

    #[test]
    fn test_graph_stream_query_params_defaults() {
        let json = r#"{}"#;
        let params: GraphStreamQueryParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.max_nodes, 200);
        assert_eq!(params.batch_size, 50);
    }

    #[test]
    fn test_graph_stream_event_serialization() {
        let event = GraphStreamEvent::Metadata {
            total_nodes: 100,
            total_edges: 200,
            nodes_to_stream: 50,
            edges_to_stream: 100,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("metadata"));
        assert!(json.contains("total_nodes"));
    }
}
