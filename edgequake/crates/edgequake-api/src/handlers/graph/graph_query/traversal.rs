//! Graph traversal handler (`GET /api/v1/graph`).
//!
//! Returns knowledge graph data with optional BFS traversal from a
//! starting node, including timeout-guarded fallback paths.

use axum::{
    extract::{Query, State},
    Json,
};
use std::time::Duration;
use tracing::{debug, warn};

use crate::error::ApiResult;
use crate::handlers::graph_types::*;
use crate::handlers::isolation::properties_match_tenant_context;
use crate::middleware::TenantContext;
use crate::state::AppState;

/// Get knowledge graph with traversal from optional starting node.
///
/// # Implements
///
/// - **UC0101**: Explore Entity Neighborhood
/// - **FEAT0601**: Knowledge Graph Visualization
///
/// # Enforces
///
/// - **BR0201**: Tenant isolation (filters by workspace)
/// - **BR0009**: Node limit enforcement via `max_nodes`
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
    let request_start = std::time::Instant::now();

    // WHY: Defense in depth - clamp params to safe ranges even if client sends invalid values
    let params = params.validated();

    debug!(
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        "Getting graph with tenant context"
    );

    // SECURITY: Enforce strict tenant context requirement - NO EXCEPTIONS
    // This matches the strict filtering in entities.rs and relationships.rs (commit d11edba8)
    if tenant_ctx.tenant_id.is_none() || tenant_ctx.workspace_id.is_none() {
        warn!(
            tenant_id = ?tenant_ctx.tenant_id,
            workspace_id = ?tenant_ctx.workspace_id,
            "Tenant context missing - returning empty graph for security"
        );
        return Ok(Json(KnowledgeGraphResponse {
            nodes: vec![],
            edges: vec![],
            is_truncated: false,
            total_nodes: 0,
            total_edges: 0,
        }));
    }

    let (nodes, edges, is_truncated) = if let Some(start) = &params.start_node {
        let kg = state
            .graph_storage
            .get_knowledge_graph(start, params.depth, params.max_nodes)
            .await?;

        let nodes: Vec<GraphNodeResponse> = kg
            .nodes
            .into_iter()
            .filter(|n| properties_match_tenant_context(&n.properties, &tenant_ctx))
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
                properties_match_tenant_context(&e.properties, &tenant_ctx)
                    && node_ids.contains(&e.source)
                    && node_ids.contains(&e.target)
            })
            .map(|e| GraphEdgeResponse {
                source: e.source,
                target: e.target,
                relationship_type: e
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
        // Added 15-second timeout to prevent indefinite hangs on large graphs

        const QUERY_TIMEOUT_SECS: u64 = 15;

        let query_future = state.graph_storage.get_popular_nodes_with_degree(
            params.max_nodes,
            None, // No min_degree filter
            None, // No entity_type filter
            tenant_ctx.tenant_id.as_deref(),
            tenant_ctx.workspace_id.as_deref(),
        );

        let nodes_with_degrees =
            match tokio::time::timeout(Duration::from_secs(QUERY_TIMEOUT_SECS), query_future).await
            {
                Ok(Ok(nodes)) => nodes,
                Ok(Err(e)) => {
                    // Check if this is a statement timeout - if so, fall back
                    let error_msg = format!("{}", e);
                    if error_msg.contains("statement timeout")
                        || error_msg.contains("canceling statement")
                    {
                        warn!(
                            max_nodes = params.max_nodes,
                            "Database query timed out, falling back to simple node fetch"
                        );

                        // Fall back to simple node list
                        state
                            .graph_storage
                            .get_all_nodes()
                            .await?
                            .into_iter()
                            .filter(|n| properties_match_tenant_context(&n.properties, &tenant_ctx))
                            .take(params.max_nodes)
                            .map(|n| (n, 0usize)) // Degree unknown in fallback
                            .collect()
                    } else {
                        return Err(e.into());
                    }
                }
                Err(_) => {
                    // Tokio timeout: Fall back to simple node list without degree calculation
                    warn!(
                        timeout_secs = QUERY_TIMEOUT_SECS,
                        max_nodes = params.max_nodes,
                        "Graph query timed out (tokio), falling back to simple node fetch"
                    );

                    // Use get_all_nodes with limit as fallback (no degree calculation)
                    let all_nodes = state.graph_storage.get_all_nodes().await?;
                    let filtered_nodes: Vec<_> = all_nodes
                        .into_iter()
                        .filter(|n| properties_match_tenant_context(&n.properties, &tenant_ctx))
                        .take(params.max_nodes)
                        .map(|n| (n, 0usize)) // Degree unknown, use 0
                        .collect();

                    filtered_nodes
                }
            };

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
                relationship_type: e
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

        (nodes, edges, false) // is_truncated calculated after counts arrive
    };

    // WHY: Run node_count/edge_count concurrently AFTER main query completes.
    // These are cheap COUNT(*) queries but still save ~50ms by running in parallel.
    let (total_nodes_result, total_edges_result) = tokio::join!(
        state.graph_storage.node_count(),
        state.graph_storage.edge_count(),
    );
    let total_nodes = total_nodes_result.unwrap_or(nodes.len());
    let total_edges = total_edges_result.unwrap_or(edges.len());
    let is_truncated = is_truncated || total_nodes > params.max_nodes;

    let elapsed_ms = request_start.elapsed().as_millis();
    debug!(
        elapsed_ms,
        total_nodes,
        total_edges,
        node_count = nodes.len(),
        edge_count = edges.len(),
        "Graph query completed"
    );

    Ok(Json(KnowledgeGraphResponse {
        nodes,
        edges,
        is_truncated,
        total_nodes,
        total_edges,
    }))
}
