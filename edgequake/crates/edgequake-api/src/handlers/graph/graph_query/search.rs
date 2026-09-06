//! Label and node search handlers.
//!
//! - `search_labels` — fuzzy label search
//! - `search_nodes` — full node search with optional neighbor expansion

use axum::{
    extract::{Query, State},
    Json,
};

use crate::error::ApiResult;
use crate::handlers::auth::OptionalAuth;
use crate::handlers::graph_types::*;
use crate::middleware::TenantContext;
use crate::services::run_timed_graph_query;
use crate::state::{AppState, GraphQueryRuntime, StorageRuntime};

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
    State(storage): State<StorageRuntime>,
    tenant_ctx: TenantContext,
    OptionalAuth(auth_user): OptionalAuth,
    Query(params): Query<SearchLabelsQuery>,
) -> ApiResult<Json<SearchLabelsResponse>> {
    let user_id = crate::services::spec146_authz::optional_auth_user_id(
        auth_user.as_ref(),
        &tenant_ctx,
    );
    let allow_ids = crate::services::spec146_authz::resolve_optional_allow_ids(
        &state,
        &tenant_ctx,
        user_id.as_deref(),
    )
    .await?;

    // SPEC-146: over-fetch when ABAC on so filtered autocomplete still fills.
    let fetch_limit = if allow_ids.is_some() {
        params.limit.saturating_mul(4).max(params.limit).min(200)
    } else {
        params.limit
    };

    let raw_labels = storage
        .graph_storage
        .search_labels(
            &params.q,
            fetch_limit,
            tenant_ctx.tenant_id.as_deref(),
            tenant_ctx.workspace_id.as_deref(),
        )
        .await?;

    // 072: never surface bare opaque machine IDs in autocomplete.
    // SPEC-146: drop labels whose node provenance is outside allow-set.
    let mut labels = Vec::with_capacity(params.limit);
    for raw in raw_labels {
        if labels.len() >= params.limit {
            break;
        }
        let (display, node_opt) = if edgequake_storage::is_opaque_identifier(&raw) {
            match storage.graph_storage.get_node(&raw).await {
                Ok(Some(node)) => {
                    let label = crate::handlers::graph::graph_node_label(&node);
                    (label, Some(node))
                }
                _ => (edgequake_pipeline::soft_label_opaque(None, None), None),
            }
        } else {
            let bare = edgequake_pipeline::bare_entity_id(&raw);
            if edgequake_storage::is_opaque_identifier(bare) {
                match storage.graph_storage.get_node(&raw).await {
                    Ok(Some(node)) => {
                        let label = crate::handlers::graph::graph_node_label(&node);
                        (label, Some(node))
                    }
                    _ => (edgequake_pipeline::soft_label_opaque(None, None), None),
                }
            } else {
                let node = storage.graph_storage.get_node(&raw).await.ok().flatten();
                (raw, node)
            }
        };

        if let Some(ref allow) = allow_ids {
            let Some(ref node) = node_opt else {
                // Fail-closed: cannot prove provenance → omit under ABAC.
                continue;
            };
            if !crate::services::spec146_authz::graph_properties_in_allow(
                &node.properties,
                Some(allow.as_slice()),
            ) {
                continue;
            }
        }

        labels.push(display);
    }

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
    State(storage): State<StorageRuntime>,
    State(graph): State<GraphQueryRuntime>,
    tenant_ctx: TenantContext,
    OptionalAuth(auth_user): OptionalAuth,
    Query(params): Query<SearchNodesQuery>,
) -> ApiResult<Json<SearchNodesResponse>> {
    use std::collections::HashSet;

    // WHY no materialization guard here (SPEC-053 B1):
    //   search_nodes is an O(log N) indexed btree lookup (idx_node_prop_node_id_btree,
    //   idx_node_tenant_id, etc.). It uses 1 DB connection for ≤50ms — a fundamentally
    //   different resource class from full-graph O(V+E) materializations (3 parallel DB
    //   connections held for 5-10s). Gating search on the materialization semaphore
    //   caused 503s on every keystroke whenever the graph was loading.
    //   The DB statement_timeout already provides backpressure for pathological queries.;

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

    let user_id = crate::services::spec146_authz::optional_auth_user_id(
        auth_user.as_ref(),
        &tenant_ctx,
    );
    let allow_ids = crate::services::spec146_authz::resolve_optional_allow_ids(
        &state,
        &tenant_ctx,
        user_id.as_deref(),
    )
    .await?;
    let matching_nodes: Vec<_> = matching_nodes
        .into_iter()
        .filter(|(node, _)| {
            crate::services::spec146_authz::graph_properties_in_allow(
                &node.properties,
                allow_ids.as_deref(),
            )
        })
        .collect();

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
                    if !crate::services::spec146_authz::graph_properties_in_allow(
                        &neighbor.properties,
                        allow_ids.as_deref(),
                    ) {
                        continue;
                    }
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

            let description = crate::services::spec146_authz::sanitize_graph_description(
                &node.properties,
                allow_ids.as_deref(),
            );

            GraphNodeResponse {
                id: node.id.clone(),
                label: crate::handlers::graph::graph_node_label(&node),
                node_type: entity_type,
                description,
                degree,
                properties: crate::services::spec146_authz::sanitize_graph_properties(
                    &node.properties,
                    allow_ids.as_deref(),
                ),
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
