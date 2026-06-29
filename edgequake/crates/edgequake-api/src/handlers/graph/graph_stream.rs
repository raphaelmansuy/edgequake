//! SSE streaming handler for progressive graph data loading.
//!
//! Contains: `stream_graph`.

use axum::{
    extract::{Query, State},
    response::sse::{Event, Sse},
};
use futures::stream::StreamExt;
use std::convert::Infallible;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::debug;

use crate::error::{ApiError, TransientCongestion};
use crate::handlers::graph_types::*;
use crate::middleware::TenantContext;
use crate::services::{admit_graph_materialization, run_timed_graph_query};
use crate::state::{GraphQueryRuntime, StorageRuntime};

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
        (status = 200, description = "SSE stream of GraphStreamEvent payloads",
            content(
                (GraphStreamEvent = "text/event-stream")
            )
        )
    )
)]
pub async fn stream_graph(
    State(storage): State<StorageRuntime>,
    State(graph): State<GraphQueryRuntime>,
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
    let graph_storage = storage.graph_storage.clone();
    let graph_clone = graph.clone();
    let params_clone = params.clone();
    let tenant_ctx_clone = tenant_ctx.clone();

    // Spawn background task for streaming
    tokio::spawn(async move {
        let start_time = std::time::Instant::now();

        let _materialize_guard = match admit_graph_materialization(&graph_clone) {
            Ok(guard) => guard,
            Err(ApiError::ServiceUnavailable {
                message,
                retry_after_secs,
            }) => {
                // WHY: reuse the single TransientCongestion SSOT so the SSE
                // error event carries the same reason + retry_after_secs as
                // the HTTP 503 would. Previously the SSE path sent only the
                // bare string and the client could not retry intelligently.
                let payload = TransientCongestion {
                    reason: "transient_congestion",
                    retry_after_secs,
                };
                let (msg, reason, retry) = payload.sse_error_fields(message);
                let _ = tx
                    .send(GraphStreamEvent::Error {
                        message: msg,
                        reason,
                        retry_after_secs: retry,
                    })
                    .await;
                return;
            }
            Err(other) => {
                let _ = tx
                    .send(GraphStreamEvent::Error {
                        message: other.to_string(),
                        reason: None,
                        retry_after_secs: None,
                    })
                    .await;
                return;
            }
        };

        debug!("About to query counts + nodes in parallel");

        let max_nodes = params_clone.max_nodes;
        let tenant_id = tenant_ctx_clone.tenant_id.clone();
        let workspace_id = tenant_ctx_clone.workspace_id.clone();
        let graph_for_query = graph_clone.clone();
        let graph_for_node_count = graph_storage.clone();
        let graph_for_edge_count = graph_storage.clone();
        let graph_for_popular = graph_storage.clone();
        let graph_for_edges = graph_storage.clone();

        let (total_nodes, total_edges, nodes_result) = tokio::join!(
            async move { graph_for_node_count.node_count_fast().await.unwrap_or(0) },
            async move { graph_for_edge_count.edge_count_fast().await.unwrap_or(0) },
            async move {
                run_timed_graph_query(&graph_for_query.budget, "graph_stream", async move {
                    graph_for_popular
                        .get_popular_nodes_with_degree(
                            max_nodes,
                            None,
                            None,
                            tenant_id.as_deref(),
                            workspace_id.as_deref(),
                        )
                        .await
                })
                .await
            }
        );

        let nodes_with_degrees = match nodes_result {
            Ok(nodes) => {
                debug!("Query succeeded with {} nodes", nodes.len());
                nodes
            }
            Err(e) => {
                let _ = tx
                    .send(GraphStreamEvent::Error {
                        message: e.to_string(),
                        reason: None,
                        retry_after_secs: None,
                    })
                    .await;
                return;
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
        let edges = match graph_for_edges
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
                        reason: None,
                        retry_after_secs: None,
                    })
                    .await;
                return;
            }
        };

        let edge_responses: Vec<GraphEdgeResponse> = edges
            .into_iter()
            .map(GraphEdgeResponse::from_storage_edge)
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
