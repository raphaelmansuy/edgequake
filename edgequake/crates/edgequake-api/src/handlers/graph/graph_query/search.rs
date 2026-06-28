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
use crate::services::{admit_graph_materialization, run_timed_graph_query};
use crate::state::{GraphQueryRuntime, StorageRuntime};

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
    State(storage): State<StorageRuntime>,
    tenant_ctx: TenantContext,
    Query(params): Query<SearchLabelsQuery>,
) -> ApiResult<Json<SearchLabelsResponse>> {
    let labels = storage
        .graph_storage
        .search_labels(
            &params.q,
            params.limit,
            tenant_ctx.tenant_id.as_deref(),
            tenant_ctx.workspace_id.as_deref(),
        )
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
    State(storage): State<StorageRuntime>,
    State(graph): State<GraphQueryRuntime>,
    tenant_ctx: TenantContext,
    Query(params): Query<SearchNodesQuery>,
) -> ApiResult<Json<SearchNodesResponse>> {
    use std::collections::HashSet;

    let _materialize_guard = admit_graph_materialization(&graph)?;

    // Get tenant/workspace context from middleware
    let tenant_id = tenant_ctx.tenant_id.clone();
    let workspace_id = tenant_ctx.workspace_id.clone();

    let q = params.q.clone();
    let limit = params.limit;
    let entity_type = params.entity_type.clone();
    let tenant_for_search = tenant_id.clone();
    let workspace_for_search = workspace_id.clone();
    let graph_storage = storage.graph_storage.clone();
    let matching_nodes = run_timed_graph_query(&graph.budget, "search_nodes", async move {
        graph_storage
            .search_nodes(
                &q,
                limit,
                entity_type.as_deref(),
                tenant_for_search.as_deref(),
                workspace_for_search.as_deref(),
            )
            .await
    })
    .await?;

    let total_matches = matching_nodes.len();
    let is_truncated = total_matches >= params.limit;

    // Collect node IDs for edge lookup
    let mut node_ids: HashSet<String> = matching_nodes.iter().map(|(n, _)| n.id.clone()).collect();

    // Optionally include neighbors (SPEC-027 IMP-015: batch degree lookup for expansions)
    let mut all_nodes = matching_nodes;
    if params.include_neighbors && !all_nodes.is_empty() {
        // Clone the node IDs to iterate on (avoid borrow conflict)
        let initial_node_ids: Vec<String> = all_nodes
            .iter()
            .take(10)
            .map(|(n, _)| n.id.clone())
            .collect();

        let mut expanded_neighbors = Vec::new();
        for node_id in initial_node_ids {
            if let Ok(neighbors) = storage
                .graph_storage
                .get_neighbors(
                    &node_id,
                    params.neighbor_depth,
                    tenant_id.as_deref(),
                    workspace_id.as_deref(),
                )
                .await
            {
                for neighbor in neighbors {
                    if node_ids.insert(neighbor.id.clone()) {
                        expanded_neighbors.push(neighbor);
                    }
                }
            }
        }

        let expanded_ids: Vec<String> = expanded_neighbors
            .iter()
            .map(|neighbor| neighbor.id.clone())
            .collect();
        let degree_map: std::collections::HashMap<String, usize> = if expanded_ids.is_empty() {
            std::collections::HashMap::new()
        } else {
            storage
                .graph_storage
                .node_degrees_batch(&expanded_ids)
                .await
                .unwrap_or_default()
                .into_iter()
                .collect()
        };

        for neighbor in expanded_neighbors {
            let degree = degree_map.get(&neighbor.id).copied().unwrap_or(0);
            all_nodes.push((neighbor, degree));
        }
    }

    // Get edges between all collected nodes
    let edges = if all_nodes.len() > 1 {
        let node_id_vec: Vec<String> = node_ids.into_iter().collect();
        let tenant_for_edges = tenant_id.clone();
        let workspace_for_edges = workspace_id.clone();
        let graph_storage_edges = storage.graph_storage.clone();
        run_timed_graph_query(&graph.budget, "edges_for_node_set", async move {
            graph_storage_edges
                .get_edges_for_node_set(
                    &node_id_vec,
                    tenant_for_edges.as_deref(),
                    workspace_for_edges.as_deref(),
                )
                .await
        })
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
        .map(GraphEdgeResponse::from_storage_edge)
        .collect();

    Ok(Json(SearchNodesResponse {
        nodes: nodes_response,
        edges: edges_response,
        total_matches,
        is_truncated,
    }))
}
