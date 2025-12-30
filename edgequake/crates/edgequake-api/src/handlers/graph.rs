//! Knowledge graph handlers.

use axum::{
    extract::{Path, Query, State},
    response::sse::{Event, Sse},
    Json,
};
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::debug;
use utoipa::ToSchema;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
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
    tenant_ctx: TenantContext,
    Query(params): Query<GraphQueryParams>,
) -> ApiResult<Json<KnowledgeGraphResponse>> {
    debug!(
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        "Getting graph with tenant context"
    );

    let total_nodes = state.graph_storage.node_count().await?;
    let total_edges = state.graph_storage.edge_count().await?;

    // Helper closure to check if a node matches the tenant context
    let matches_tenant_context =
        |properties: &std::collections::HashMap<String, serde_json::Value>| {
            // If no tenant context is set, allow all nodes
            if tenant_ctx.tenant_id.is_none() {
                return true;
            }

            // Check if node has matching tenant_id
            if let Some(ref ctx_tenant_id) = tenant_ctx.tenant_id {
                if let Some(node_tenant_id) = properties.get("tenant_id").and_then(|v| v.as_str()) {
                    if node_tenant_id != ctx_tenant_id {
                        return false;
                    }
                }
                // If node has no tenant_id but context has one, still include it for backward compatibility
            }

            // Check workspace_id if set
            if let Some(ref ctx_workspace_id) = tenant_ctx.workspace_id {
                if let Some(node_workspace_id) =
                    properties.get("workspace_id").and_then(|v| v.as_str())
                {
                    if node_workspace_id != ctx_workspace_id {
                        return false;
                    }
                }
            }

            true
        };

    let (nodes, edges, is_truncated) = if let Some(start) = &params.start_node {
        let kg = state
            .graph_storage
            .get_knowledge_graph(start, params.depth, params.max_nodes)
            .await?;

        let nodes: Vec<GraphNodeResponse> = kg
            .nodes
            .into_iter()
            .filter(|n| matches_tenant_context(&n.properties))
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

        // Also filter edges by tenant context
        let node_ids: std::collections::HashSet<_> = nodes.iter().map(|n| &n.id).collect();
        let edges: Vec<GraphEdgeResponse> = kg
            .edges
            .into_iter()
            .filter(|e| {
                matches_tenant_context(&e.properties)
                    && node_ids.contains(&e.source)
                    && node_ids.contains(&e.target)
            })
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
        // OPTIMIZED: Use batch query to get popular nodes with degrees
        // This eliminates the N+1 query pattern (was 400+ queries, now 2)
        let nodes_with_degrees = state
            .graph_storage
            .get_popular_nodes_with_degree(
                params.max_nodes,
                None, // No min_degree filter
                None, // No entity_type filter
                tenant_ctx.tenant_id.as_deref(),
                tenant_ctx.workspace_id.as_deref(),
            )
            .await?;

        // Convert to response format
        let nodes: Vec<GraphNodeResponse> = nodes_with_degrees
            .into_iter()
            .map(|(node, degree)| GraphNodeResponse {
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
            })
            .collect();

        // OPTIMIZED: Use filtered edge query instead of get_all_edges
        let node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
        let filtered_edges = state
            .graph_storage
            .get_edges_for_node_set(
                &node_ids,
                tenant_ctx.tenant_id.as_deref(),
                tenant_ctx.workspace_id.as_deref(),
            )
            .await?;

        let edges: Vec<GraphEdgeResponse> = filtered_edges
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

        (nodes, edges, total_nodes > params.max_nodes)
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

// ============================================
// GAP-036: Popular Labels / Entities
// ============================================

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

fn default_popular_limit() -> usize {
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

/// Get popular entities/labels sorted by connection count.
#[utoipa::path(
    get,
    path = "/api/v1/graph/labels/popular",
    tag = "Graph",
    params(
        ("limit" = usize, Query, description = "Max results (default 50)"),
        ("min_degree" = Option<usize>, Query, description = "Minimum connections"),
        ("entity_type" = Option<String>, Query, description = "Filter by type")
    ),
    responses(
        (status = 200, description = "Popular labels retrieved", body = PopularLabelsResponse)
    )
)]
pub async fn get_popular_labels(
    State(state): State<AppState>,
    Query(params): Query<PopularLabelsQuery>,
) -> ApiResult<Json<PopularLabelsResponse>> {
    let total_entities = state.graph_storage.node_count().await?;

    // Get popular labels from storage
    let popular_ids = state
        .graph_storage
        .get_popular_labels(params.limit * 2) // Get more to allow filtering
        .await?;

    let mut labels = Vec::new();

    for id in popular_ids {
        if labels.len() >= params.limit {
            break;
        }

        if let Some(node) = state.graph_storage.get_node(&id).await? {
            let degree = state.graph_storage.node_degree(&id).await?;

            // Apply min_degree filter
            if let Some(min) = params.min_degree {
                if degree < min {
                    continue;
                }
            }

            let entity_type = node
                .properties
                .get("entity_type")
                .and_then(|v| v.as_str())
                .unwrap_or("UNKNOWN")
                .to_string();

            // Apply entity_type filter
            if let Some(ref type_filter) = params.entity_type {
                if !entity_type.eq_ignore_ascii_case(type_filter) {
                    continue;
                }
            }

            let description = node
                .properties
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            labels.push(PopularLabel {
                label: id,
                entity_type,
                degree,
                description,
            });
        }
    }

    // Sort by degree descending
    labels.sort_by(|a, b| b.degree.cmp(&a.degree));

    Ok(Json(PopularLabelsResponse {
        labels,
        total_entities,
    }))
}

// ============================================================================
// Graph Streaming Types and Handler
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

fn default_stream_max_nodes() -> usize {
    200
}

fn default_stream_batch_size() -> usize {
    50
}

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

/// Stream graph data progressively via SSE.
///
/// This endpoint streams graph nodes and edges in batches, making it suitable
/// for very large graphs where loading everything at once would be too slow.
///
/// Events are sent in order:
/// 1. `metadata` - Initial graph statistics
/// 2. `nodes` - Multiple batches of nodes (batch_size per event)
/// 3. `edges` - Edges between streamed nodes
/// 4. `done` - Completion summary
#[utoipa::path(
    get,
    path = "/api/v1/graph/stream",
    tag = "Graph",
    params(
        ("start_node" = Option<String>, Query, description = "Starting node ID"),
        ("max_nodes" = usize, Query, description = "Max nodes to stream (default 200)"),
        ("batch_size" = usize, Query, description = "Nodes per batch (default 50)")
    ),
    responses(
        (status = 200, description = "SSE stream of graph data")
    )
)]
pub async fn stream_graph(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Query(params): Query<GraphStreamQueryParams>,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>, ApiError> {
    debug!(
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        max_nodes = params.max_nodes,
        batch_size = params.batch_size,
        "Starting graph stream"
    );

    // Create channel for SSE events
    let (tx, rx) = mpsc::channel::<GraphStreamEvent>(100);

    // Clone for async task
    let state_clone = state.clone();
    let params_clone = params.clone();
    let tenant_ctx_clone = tenant_ctx.clone();

    // Spawn background task for streaming
    tokio::spawn(async move {
        let start_time = std::time::Instant::now();

        // Get total counts
        let total_nodes = state_clone.graph_storage.node_count().await.unwrap_or(0);
        let total_edges = state_clone.graph_storage.edge_count().await.unwrap_or(0);

        // Get nodes with degrees (optimized batch query)
        let nodes_with_degrees = match state_clone
            .graph_storage
            .get_popular_nodes_with_degree(
                params_clone.max_nodes,
                None,
                None,
                tenant_ctx_clone.tenant_id.as_deref(),
                tenant_ctx_clone.workspace_id.as_deref(),
            )
            .await
        {
            Ok(nodes) => nodes,
            Err(e) => {
                let _ = tx
                    .send(GraphStreamEvent::Error {
                        message: format!("Failed to fetch nodes: {}", e),
                    })
                    .await;
                return;
            }
        };

        let nodes_to_stream = nodes_with_degrees.len();
        let total_batches =
            (nodes_to_stream + params_clone.batch_size - 1) / params_clone.batch_size;

        // Send metadata event
        if tx
            .send(GraphStreamEvent::Metadata {
                total_nodes,
                total_edges,
                nodes_to_stream,
                edges_to_stream: 0, // Will be determined after node streaming
            })
            .await
            .is_err()
        {
            return; // Client disconnected
        }

        // Collect all node IDs for edge fetching
        let all_node_ids: Vec<String> = nodes_with_degrees
            .iter()
            .map(|(n, _)| n.id.clone())
            .collect();

        // Stream nodes in batches
        for (batch_idx, chunk) in nodes_with_degrees
            .chunks(params_clone.batch_size)
            .enumerate()
        {
            let batch_nodes: Vec<GraphNodeResponse> = chunk
                .iter()
                .map(|(node, degree)| GraphNodeResponse {
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
                    degree: *degree,
                    properties: serde_json::to_value(&node.properties).unwrap_or_default(),
                })
                .collect();

            if tx
                .send(GraphStreamEvent::Nodes {
                    batch: batch_idx + 1,
                    total_batches,
                    nodes: batch_nodes,
                })
                .await
                .is_err()
            {
                return; // Client disconnected
            }

            // Small yield to prevent blocking
            tokio::task::yield_now().await;
        }

        // Fetch and stream edges (optimized batch query)
        let edges = match state_clone
            .graph_storage
            .get_edges_for_node_set(
                &all_node_ids,
                tenant_ctx_clone.tenant_id.as_deref(),
                tenant_ctx_clone.workspace_id.as_deref(),
            )
            .await
        {
            Ok(e) => e,
            Err(e) => {
                let _ = tx
                    .send(GraphStreamEvent::Error {
                        message: format!("Failed to fetch edges: {}", e),
                    })
                    .await;
                return;
            }
        };

        let edge_responses: Vec<GraphEdgeResponse> = edges
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

        let edges_count = edge_responses.len();

        if tx
            .send(GraphStreamEvent::Edges {
                edges: edge_responses,
            })
            .await
            .is_err()
        {
            return;
        }

        // Send completion event
        let duration_ms = start_time.elapsed().as_millis() as u64;
        let _ = tx
            .send(GraphStreamEvent::Done {
                nodes_count: nodes_to_stream,
                edges_count,
                duration_ms,
            })
            .await;
    });

    // Convert channel to SSE stream
    let sse_stream = ReceiverStream::new(rx).map(|event| {
        let json = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
        Ok::<_, Infallible>(Event::default().data(json))
    });

    Ok(Sse::new(sse_stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keep-alive"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_graph_empty() {
        let state = AppState::test_state();
        let tenant_ctx = TenantContext::default();
        let params = GraphQueryParams {
            start_node: None,
            depth: 2,
            max_nodes: 100,
        };

        let result = get_graph(State(state), tenant_ctx, Query(params)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert!(response.nodes.is_empty());
    }
}
