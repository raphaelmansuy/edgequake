//! Label and node search handlers.
//!
//! - `search_labels` — fuzzy label search
//! - `search_nodes` — full node search with optional neighbor expansion

use axum::{
    extract::{Query, State},
    Json,
};

use crate::error::ApiResult;
use crate::handlers::graph_types::*;
use crate::middleware::TenantContext;
use crate::state::AppState;

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

/// Search for nodes with full data (label and description search).
///
/// Returns matching nodes with their degrees, optionally with edges.
/// Searches both label and description fields for comprehensive results.
#[utoipa::path(
    get,
    path = "/api/v1/graph/nodes/search",
    tag = "Graph",
    params(
        ("q" = String, Query, description = "Search query (searches label and description)"),
        ("limit" = usize, Query, description = "Max results (default 50)"),
        ("include_neighbors" = bool, Query, description = "Include neighbor nodes"),
        ("neighbor_depth" = usize, Query, description = "Depth for neighbor traversal"),
        ("entity_type" = Option<String>, Query, description = "Filter by entity type")
    ),
    responses(
        (status = 200, description = "Nodes found", body = SearchNodesResponse)
    )
)]
pub async fn search_nodes(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Query(params): Query<SearchNodesQuery>,
) -> ApiResult<Json<SearchNodesResponse>> {
    use std::collections::HashSet;

    // Get tenant/workspace context from middleware
    let tenant_id = tenant_ctx.tenant_id.clone();
    let workspace_id = tenant_ctx.workspace_id.clone();

    // Search for matching nodes
    let matching_nodes = state
        .graph_storage
        .search_nodes(
            &params.q,
            params.limit,
            params.entity_type.as_deref(),
            tenant_id.as_deref(),
            workspace_id.as_deref(),
        )
        .await?;

    let total_matches = matching_nodes.len();
    let is_truncated = total_matches >= params.limit;

    // Collect node IDs for edge lookup
    let mut node_ids: HashSet<String> = matching_nodes.iter().map(|(n, _)| n.id.clone()).collect();

    // Optionally include neighbors
    let mut all_nodes = matching_nodes;
    if params.include_neighbors && !all_nodes.is_empty() {
        // Clone the node IDs to iterate on (avoid borrow conflict)
        let initial_node_ids: Vec<String> = all_nodes
            .iter()
            .take(10)
            .map(|(n, _)| n.id.clone())
            .collect();

        for node_id in initial_node_ids {
            // Limit neighbor lookups
            if let Ok(neighbors) = state
                .graph_storage
                .get_neighbors(&node_id, params.neighbor_depth)
                .await
            {
                for neighbor in neighbors {
                    if !node_ids.contains(&neighbor.id) {
                        node_ids.insert(neighbor.id.clone());
                        // Get degree for neighbor
                        let degree = state
                            .graph_storage
                            .node_degree(&neighbor.id)
                            .await
                            .unwrap_or(0);
                        all_nodes.push((neighbor, degree));
                    }
                }
            }
        }
    }

    // Get edges between all collected nodes
    let edges = if all_nodes.len() > 1 {
        let node_id_vec: Vec<String> = node_ids.into_iter().collect();
        state
            .graph_storage
            .get_edges_for_node_set(&node_id_vec, tenant_id.as_deref(), workspace_id.as_deref())
            .await
            .unwrap_or_default()
    } else {
        vec![]
    };

    // Convert to response format
    let nodes_response: Vec<GraphNodeResponse> = all_nodes
        .into_iter()
        .map(|(node, degree)| {
            let entity_type = node
                .properties
                .get("entity_type")
                .and_then(|v| v.as_str())
                .unwrap_or("UNKNOWN")
                .to_string();

            let description = node
                .properties
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            GraphNodeResponse {
                id: node.id.clone(),
                label: node.id,
                node_type: entity_type,
                description,
                degree,
                properties: serde_json::to_value(&node.properties).unwrap_or_default(),
            }
        })
        .collect();

    let edges_response: Vec<GraphEdgeResponse> = edges
        .into_iter()
        .map(|edge| GraphEdgeResponse {
            source: edge.source,
            target: edge.target,
            relationship_type: edge
                .properties
                .get("relationship_type")
                .or_else(|| edge.properties.get("relation_type"))
                .and_then(|v| v.as_str())
                .unwrap_or("RELATED_TO")
                .to_string(),
            weight: edge
                .properties
                .get("weight")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0) as f32,
            properties: serde_json::to_value(&edge.properties).unwrap_or_default(),
        })
        .collect();

    Ok(Json(SearchNodesResponse {
        nodes: nodes_response,
        edges: edges_response,
        total_matches,
        is_truncated,
    }))
}
