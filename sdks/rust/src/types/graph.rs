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
    pub id: String,
    #[serde(default)]
    pub entity_name: String,
    #[serde(default)]
    pub entity_type: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub source_id: Option<String>,
    #[serde(default)]
    pub properties: Option<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub degree: Option<u32>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

/// Paginated entity list response.
#[derive(Debug, Clone, Deserialize)]
pub struct EntityListResponse {
    #[serde(default)]
    pub items: Vec<Entity>,
    #[serde(default)]
    pub total: u32,
    #[serde(default)]
    pub page: u32,
    #[serde(default)]
    pub page_size: u32,
    #[serde(default)]
    pub total_pages: u32,
}

/// Entity detail response from GET /api/v1/graph/entities/{name}.
#[derive(Debug, Clone, Deserialize)]
pub struct EntityDetailResponse {
    pub entity: Entity,
    #[serde(default)]
    pub relationships: Option<EntityRelationships>,
    #[serde(default)]
    pub statistics: Option<EntityStatistics>,
}

/// Relationships wrapper in entity detail response.
#[derive(Debug, Clone, Deserialize)]
pub struct EntityRelationships {
    #[serde(default)]
    pub outgoing: Vec<Relationship>,
    #[serde(default)]
    pub incoming: Vec<Relationship>,
}

/// Statistics wrapper in entity detail response.
#[derive(Debug, Clone, Deserialize)]
pub struct EntityStatistics {
    #[serde(default)]
    pub total_relationships: u32,
    #[serde(default)]
    pub outgoing_count: u32,
    #[serde(default)]
    pub incoming_count: u32,
    #[serde(default)]
    pub document_references: u32,
}

/// Request to create an entity.
#[derive(Debug, Clone, Serialize)]
pub struct CreateEntityRequest {
    pub entity_name: String,
    pub entity_type: String,
    pub description: String,
    pub source_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Response from POST /api/v1/graph/entities.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateEntityResponse {
    pub status: String,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub entity: Option<Entity>,
}

/// Entity exists check.
#[derive(Debug, Clone, Deserialize)]
pub struct EntityExistsResponse {
    pub exists: bool,
    #[serde(default)]
    pub entity_id: Option<String>,
    #[serde(default)]
    pub entity_type: Option<String>,
    #[serde(default)]
    pub degree: Option<u32>,
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
    pub source_entity: String,
    pub target_entity: String,
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

/// Request body for `POST /api/v1/graph/degrees/batch`.
#[derive(Debug, Clone, Serialize)]
pub struct BatchDegreeRequest {
    pub node_ids: Vec<String>,
}

/// One node degree entry from the batch API.
#[derive(Debug, Clone, Deserialize)]
pub struct NodeDegree {
    pub node_id: String,
    pub degree: u32,
}

/// Response from `POST /api/v1/graph/degrees/batch`.
#[derive(Debug, Clone, Deserialize)]
pub struct DegreesBatchResponse {
    #[serde(default)]
    pub degrees: Vec<NodeDegree>,
    #[serde(default)]
    pub count: u32,
}

/// Paginated relationship list response.
#[derive(Debug, Clone, Deserialize)]
pub struct RelationshipListResponse {
    #[serde(default)]
    pub items: Vec<Relationship>,
    #[serde(default)]
    pub total: u32,
    #[serde(default)]
    pub page: u32,
    #[serde(default)]
    pub page_size: u32,
    #[serde(default)]
    pub total_pages: u32,
}
