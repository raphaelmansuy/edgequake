//! Graph-related types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A node in the knowledge graph.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub node_type: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub properties: Option<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub degree: Option<u32>,
}

/// An edge in the knowledge graph.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub edge_type: Option<String>,
    #[serde(default)]
    pub weight: Option<f64>,
    #[serde(default)]
    pub properties: Option<HashMap<String, serde_json::Value>>,
}

/// Full graph response.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GraphResponse {
    #[serde(default)]
    pub nodes: Vec<GraphNode>,
    #[serde(default)]
    pub edges: Vec<GraphEdge>,
    #[serde(default)]
    pub total_nodes: Option<u32>,
    #[serde(default)]
    pub total_edges: Option<u32>,
}

/// Search nodes response.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchNodesResponse {
    #[serde(default)]
    pub nodes: Vec<GraphNode>,
    #[serde(default)]
    pub edges: Vec<GraphEdge>,
    #[serde(default)]
    pub total_matches: Option<u32>,
}

/// Entity.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Entity {
    pub name: String,
    #[serde(default)]
    pub entity_type: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub properties: Option<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub degree: Option<u32>,
    #[serde(default)]
    pub created_at: Option<String>,
}

/// Request to create an entity.
#[derive(Debug, Clone, Serialize)]
pub struct CreateEntityRequest {
    pub name: String,
    pub entity_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

/// Entity exists check.
#[derive(Debug, Clone, Deserialize)]
pub struct EntityExistsResponse {
    pub exists: bool,
    #[serde(default)]
    pub entity_name: Option<String>,
}

/// Merge entities response.
#[derive(Debug, Clone, Deserialize)]
pub struct MergeEntitiesResponse {
    #[serde(default)]
    pub merged_entity: Option<Entity>,
    #[serde(default)]
    pub merged_count: u32,
    #[serde(default)]
    pub message: Option<String>,
}

/// Merge entities request.
#[derive(Debug, Clone, Serialize)]
pub struct MergeEntitiesRequest {
    pub source: String,
    pub target: String,
}

/// Neighborhood response.
#[derive(Debug, Clone, Deserialize)]
pub struct NeighborhoodResponse {
    #[serde(default)]
    pub center: Option<Entity>,
    #[serde(default)]
    pub nodes: Vec<GraphNode>,
    #[serde(default)]
    pub edges: Vec<GraphEdge>,
    #[serde(default = "default_depth")]
    pub depth: u32,
}

fn default_depth() -> u32 {
    1
}

/// Relationship.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Relationship {
    #[serde(default)]
    pub id: Option<String>,
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub relationship_type: Option<String>,
    #[serde(default)]
    pub weight: Option<f64>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub properties: Option<HashMap<String, serde_json::Value>>,
}

/// Request to create a relationship.
#[derive(Debug, Clone, Serialize)]
pub struct CreateRelationshipRequest {
    pub source: String,
    pub target: String,
    pub relationship_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Degrees batch response.
#[derive(Debug, Clone, Deserialize)]
pub struct DegreesBatchResponse {
    #[serde(default)]
    pub degrees: HashMap<String, u32>,
}
