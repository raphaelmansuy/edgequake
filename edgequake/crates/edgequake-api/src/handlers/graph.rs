//! Knowledge graph handlers.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use edgequake_storage::GraphStorage;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

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

fn default_depth() -> usize {
    2
}

fn default_max_nodes() -> usize {
    100
}

/// Get knowledge graph.
#[utoipa::path(
    get,
    path = "/api/v1/graph",
    tag = "Graph",
    params(
        ("start_node" = Option<String>, Query, description = "Starting node ID"),
        ("depth" = usize, Query, description = "Max traversal depth"),
        ("max_nodes" = usize, Query, description = "Max nodes to return")
    ),
    responses(
        (status = 200, description = "Graph retrieved", body = KnowledgeGraphResponse)
    )
)]
pub async fn get_graph(
    State(state): State<AppState>,
    Query(params): Query<GraphQueryParams>,
) -> ApiResult<Json<KnowledgeGraphResponse>> {
    let total_nodes = state.graph_storage.node_count().await?;
    let total_edges = state.graph_storage.edge_count().await?;

    let (nodes, edges, is_truncated) = if let Some(start) = &params.start_node {
        let kg = state
            .graph_storage
            .get_knowledge_graph(start, params.depth, params.max_nodes)
            .await?;

        let nodes: Vec<GraphNodeResponse> = kg
            .nodes
            .into_iter()
            .map(|n| GraphNodeResponse {
                id: n.id.clone(),
                label: n.id.clone(),
                node_type: n
                    .properties
                    .get("entity_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UNKNOWN")
                    .to_string(),
                description: n
                    .properties
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                degree: 0,
                properties: serde_json::to_value(&n.properties).unwrap_or_default(),
            })
            .collect();

        let edges: Vec<GraphEdgeResponse> = kg
            .edges
            .into_iter()
            .map(|e| GraphEdgeResponse {
                source: e.source,
                target: e.target,
                edge_type: e
                    .properties
                    .get("relation_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("RELATED_TO")
                    .to_string(),
                weight: e
                    .properties
                    .get("weight")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1.0) as f32,
                properties: serde_json::to_value(&e.properties).unwrap_or_default(),
            })
            .collect();

        (nodes, edges, kg.is_truncated)
    } else {
        // Return popular nodes
        let popular = state.graph_storage.get_popular_labels(params.max_nodes).await?;

        let mut nodes = Vec::new();
        for id in popular {
            if let Some(node) = state.graph_storage.get_node(&id).await? {
                let degree = state.graph_storage.node_degree(&id).await?;
                nodes.push(GraphNodeResponse {
                    id: node.id.clone(),
                    label: node.id.clone(),
                    node_type: node
                        .properties
                        .get("entity_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("UNKNOWN")
                        .to_string(),
                    description: node
                        .properties
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    degree,
                    properties: serde_json::to_value(&node.properties).unwrap_or_default(),
                });
            }
        }

        (nodes, vec![], total_nodes > params.max_nodes)
    };

    Ok(Json(KnowledgeGraphResponse {
        nodes,
        edges,
        is_truncated,
        total_nodes,
        total_edges,
    }))
}

/// Get a specific node.
#[utoipa::path(
    get,
    path = "/api/v1/graph/nodes/{node_id}",
    tag = "Graph",
    params(
        ("node_id" = String, Path, description = "Node ID")
    ),
    responses(
        (status = 200, description = "Node retrieved", body = GraphNodeResponse),
        (status = 404, description = "Node not found")
    )
)]
pub async fn get_node(
    State(state): State<AppState>,
    Path(node_id): Path<String>,
) -> ApiResult<Json<GraphNodeResponse>> {
    let node = state
        .graph_storage
        .get_node(&node_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Node '{}' not found", node_id)))?;

    let degree = state.graph_storage.node_degree(&node_id).await?;

    Ok(Json(GraphNodeResponse {
        id: node.id.clone(),
        label: node.id.clone(),
        node_type: node
            .properties
            .get("entity_type")
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN")
            .to_string(),
        description: node
            .properties
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        degree,
        properties: serde_json::to_value(&node.properties).unwrap_or_default(),
    }))
}

/// Search labels query.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SearchLabelsQuery {
    /// Search query.
    pub q: String,

    /// Maximum results.
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    20
}

/// Search labels response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SearchLabelsResponse {
    /// Matching labels.
    pub labels: Vec<String>,
}

/// Search for node labels.
#[utoipa::path(
    get,
    path = "/api/v1/graph/labels/search",
    tag = "Graph",
    params(
        ("q" = String, Query, description = "Search query"),
        ("limit" = usize, Query, description = "Max results")
    ),
    responses(
        (status = 200, description = "Labels found", body = SearchLabelsResponse)
    )
)]
pub async fn search_labels(
    State(state): State<AppState>,
    Query(params): Query<SearchLabelsQuery>,
) -> ApiResult<Json<SearchLabelsResponse>> {
    let labels = state
        .graph_storage
        .search_labels(&params.q, params.limit)
        .await?;

    Ok(Json(SearchLabelsResponse { labels }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_graph_empty() {
        let state = AppState::test_state();
        let params = GraphQueryParams {
            start_node: None,
            depth: 2,
            max_nodes: 100,
        };

        let result = get_graph(State(state), Query(params)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert!(response.nodes.is_empty());
    }
}
