//! Entity neighborhood BFS — SPEC-027 IMP-029 (extracted from entity_ops).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use edgequake_storage::traits::GraphStorage;

use crate::error::ApiResult;
use crate::handlers::entities_types::{NeighborhoodEdge, NeighborhoodNode};
use crate::handlers::isolation::{filter_edges_by_tenant_context, load_node_for_tenant_context};
use crate::middleware::TenantContext;

/// Collect connected nodes and edges within `depth` hops (batch incident edges per frontier).
pub async fn build_entity_neighborhood(
    graph_storage: &Arc<dyn GraphStorage>,
    tenant_ctx: &TenantContext,
    root_entity_id: &str,
    depth: u32,
) -> ApiResult<(Vec<NeighborhoodNode>, Vec<NeighborhoodEdge>)> {
    let mut visited_nodes = HashSet::new();
    let mut frontier = vec![root_entity_id.to_string()];
    visited_nodes.insert(root_entity_id.to_string());

    let mut all_edges = Vec::new();
    let mut seen_edge_ids = HashSet::new();

    for _ in 0..depth {
        if frontier.is_empty() {
            break;
        }

        let batch_edges = filter_edges_by_tenant_context(
            graph_storage.get_incident_edges_batch(&frontier).await?,
            tenant_ctx,
        );

        let frontier_set: HashSet<&str> = frontier.iter().map(String::as_str).collect();
        let mut next_frontier = Vec::new();

        for edge in batch_edges {
            let edge_id = format!("{}_{}", edge.source, edge.target);
            if seen_edge_ids.insert(edge_id.clone()) {
                all_edges.push((edge_id, edge.clone()));
            }

            let neighbor = if frontier_set.contains(edge.source.as_str()) {
                &edge.target
            } else if frontier_set.contains(edge.target.as_str()) {
                &edge.source
            } else {
                continue;
            };

            if visited_nodes.insert(neighbor.clone()) {
                next_frontier.push(neighbor.clone());
            }
        }

        frontier = next_frontier;
    }

    let visited: Vec<String> = visited_nodes.iter().cloned().collect();
    let degree_map: HashMap<String, usize> = graph_storage
        .node_degrees_batch(&visited)
        .await
        .unwrap_or_default()
        .into_iter()
        .collect();

    let mut nodes = Vec::with_capacity(visited.len());
    for node_id in &visited {
        if let Ok(node) =
            load_node_for_tenant_context(graph_storage.as_ref(), node_id, tenant_ctx).await
        {
            let degree = degree_map.get(node_id).copied().unwrap_or(0);
            nodes.push(NeighborhoodNode {
                id: node.id.clone(),
                entity_type: node
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
            });
        }
    }

    let edges: Vec<NeighborhoodEdge> = all_edges
        .into_iter()
        .map(|(id, edge)| NeighborhoodEdge {
            id,
            source: edge.source,
            target: edge.target,
            relation_type: edge
                .properties
                .get("relation_type")
                .and_then(|v| v.as_str())
                .unwrap_or("RELATED_TO")
                .to_string(),
            weight: edge
                .properties
                .get("weight")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0),
        })
        .collect();

    Ok((nodes, edges))
}
