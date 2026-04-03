//! SSE streaming handler for progressive graph data loading.
//!
//! Contains: `stream_graph`.

use axum::{
    extract::{Query, State},
    response::sse::{Event, Sse},
};
use futures::stream::StreamExt;
use std::convert::Infallible;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, warn};

use crate::error::ApiError;
use crate::handlers::graph_types::*;
use crate::handlers::isolation::properties_match_tenant_context;
use crate::middleware::TenantContext;
use crate::state::AppState;

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
    // WHY: Defense in depth - clamp params to safe ranges even if client sends invalid values
    let params = params.validated();

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

        // Get nodes with degrees (optimized batch query with timeout)
        const QUERY_TIMEOUT_SECS: u64 = 15;

        debug!("About to query nodes with timeout wrapper");

        let query_future = state_clone.graph_storage.get_popular_nodes_with_degree(
            params_clone.max_nodes,
            None,
            None,
            tenant_ctx_clone.tenant_id.as_deref(),
            tenant_ctx_clone.workspace_id.as_deref(),
        );

        let nodes_with_degrees =
            match tokio::time::timeout(Duration::from_secs(QUERY_TIMEOUT_SECS), query_future).await
            {
                Ok(Ok(nodes)) => {
                    debug!("Query succeeded with {} nodes", nodes.len());
                    nodes
                }
                Ok(Err(e)) => {
                    // Check if this is a statement timeout error - if so, fall back
                    let error_msg = format!("{}", e);
                    debug!("Query returned error: {}", error_msg);
                    if error_msg.contains("statement timeout")
                        || error_msg.contains("canceling statement")
                    {
                        warn!(
                            max_nodes = params_clone.max_nodes,
                            "Database query timed out, falling back to simple node fetch"
                        );

                        match state_clone.graph_storage.get_all_nodes().await {
                            Ok(all_nodes) => all_nodes
                                .into_iter()
                                .filter(|n| {
                                    properties_match_tenant_context(
                                        &n.properties,
                                        &tenant_ctx_clone,
                                    )
                                })
                                .take(params_clone.max_nodes)
                                .map(|n| (n, 0usize)) // Degree unknown, use 0
                                .collect(),
                            Err(e) => {
                                let _ = tx
                                    .send(GraphStreamEvent::Error {
                                        message: format!(
                                            "Failed to fetch nodes after timeout: {}",
                                            e
                                        ),
                                    })
                                    .await;
                                return;
                            }
                        }
                    } else {
                        // Some other error, not a timeout
                        let _ = tx
                            .send(GraphStreamEvent::Error {
                                message: format!("Failed to fetch nodes: {}", e),
                            })
                            .await;
                        return;
                    }
                }
                Err(_) => {
                    // Timeout: Fall back to simple node list
                    warn!(
                        timeout_secs = QUERY_TIMEOUT_SECS,
                        max_nodes = params_clone.max_nodes,
                        "Stream query timed out, falling back to simple node fetch"
                    );

                    match state_clone.graph_storage.get_all_nodes().await {
                        Ok(all_nodes) => all_nodes
                            .into_iter()
                            .filter(|n| {
                                properties_match_tenant_context(&n.properties, &tenant_ctx_clone)
                            })
                            .take(params_clone.max_nodes)
                            .map(|n| (n, 0usize)) // Degree unknown, use 0
                            .collect(),
                        Err(e) => {
                            let _ = tx
                                .send(GraphStreamEvent::Error {
                                    message: format!("Failed to fetch nodes after timeout: {}", e),
                                })
                                .await;
                            return;
                        }
                    }
                }
            };

        let nodes_to_stream = nodes_with_degrees.len();
        let total_batches = nodes_to_stream.div_ceil(params_clone.batch_size);

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
