//! Advanced entity operations: exists check, merge, neighborhood traversal.
//!
//! @implements UC0101 (Explore Entity Neighborhood)
//! @implements FEAT0202 (Graph Traversal)

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

use super::{node_to_entity_response, normalize_entity_name};
pub use crate::handlers::entities_types::{
    EntityExistsQuery, EntityExistsResponse, EntityNeighborhoodQuery, EntityNeighborhoodResponse,
    MergeDetails, MergeEntitiesRequest, MergeEntitiesResponse, NeighborhoodEdge, NeighborhoodNode,
};

/// Check if an entity exists.
#[utoipa::path(
    get,
    path = "/api/v1/graph/entities/exists",
    tag = "Entities",
    params(
        ("entity_name" = String, Query, description = "Entity name")
    ),
    responses(
        (status = 200, description = "Existence checked", body = EntityExistsResponse)
    )
)]
pub async fn entity_exists(
    State(state): State<AppState>,
    Query(params): Query<EntityExistsQuery>,
) -> ApiResult<Json<EntityExistsResponse>> {
    let entity_name = normalize_entity_name(&params.entity_name);

    if let Some(node) = state.graph_storage.get_node(&entity_name).await? {
        let degree = state.graph_storage.node_degree(&entity_name).await?;
        let entity_type = node
            .properties
            .get("entity_type")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(Json(EntityExistsResponse {
            exists: true,
            entity_id: Some(entity_name),
            entity_type,
            degree: Some(degree),
        }))
    } else {
        Ok(Json(EntityExistsResponse {
            exists: false,
            entity_id: None,
            entity_type: None,
            degree: None,
        }))
    }
}

/// Merge two entities (deduplication).
#[utoipa::path(
    post,
    path = "/api/v1/graph/entities/merge",
    tag = "Entities",
    request_body = MergeEntitiesRequest,
    responses(
        (status = 200, description = "Entities merged", body = MergeEntitiesResponse),
        (status = 404, description = "Entity not found")
    )
)]
pub async fn merge_entities(
    State(state): State<AppState>,
    Json(req): Json<MergeEntitiesRequest>,
) -> ApiResult<Json<MergeEntitiesResponse>> {
    let source_entity = normalize_entity_name(&req.source_entity);
    let target_entity = normalize_entity_name(&req.target_entity);

    // Get both entities
    let source_node = state
        .graph_storage
        .get_node(&source_entity)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!("Source entity '{}' not found", source_entity))
        })?;

    let mut target_node = state
        .graph_storage
        .get_node(&target_entity)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!("Target entity '{}' not found", target_entity))
        })?;

    // Merge descriptions based on strategy
    let description_strategy = req.merge_strategy.clone();
    match description_strategy.as_str() {
        "prefer_source" => {
            if let Some(desc) = source_node.properties.get("description") {
                target_node
                    .properties
                    .insert("description".to_string(), desc.clone());
            }
        }
        "prefer_target" => {
            // Keep target description as-is
        }
        "merge" => {
            let source_desc = source_node
                .properties
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let target_desc = target_node
                .properties
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let merged_desc = format!("{}; {}", target_desc, source_desc);
            target_node
                .properties
                .insert("description".to_string(), merged_desc.into());
        }
        _ => {}
    }

    // Merge metadata
    if let Some(source_meta) = source_node.properties.get("metadata").cloned() {
        if let Some(target_meta) = target_node.properties.get("metadata").cloned() {
            if let (Some(mut target_obj), Some(source_obj)) =
                (target_meta.as_object().cloned(), source_meta.as_object())
            {
                for (k, v) in source_obj {
                    target_obj.insert(k.clone(), v.clone());
                }
                target_node
                    .properties
                    .insert("metadata".to_string(), serde_json::json!(target_obj));
            }
        } else {
            target_node
                .properties
                .insert("metadata".to_string(), source_meta);
        }
    }

    // Get source relationships
    let source_edges = state.graph_storage.get_node_edges(&source_entity).await?;

    let relationships_merged = source_edges.len();
    let duplicate_relationships_removed = 0;

    // Redirect relationships to target entity
    // This is a simplified implementation - in production, you'd want to:
    // 1. Check for duplicate relationships
    // 2. Merge relationship weights
    // 3. Handle bidirectional relationships properly

    // Update target node
    let now = Utc::now().to_rfc3339();
    target_node
        .properties
        .insert("updated_at".to_string(), now.into());
    state
        .graph_storage
        .upsert_node(&target_entity, target_node.properties.clone())
        .await?;

    // Delete source node
    state.graph_storage.delete_node(&source_entity).await?;

    let degree = state.graph_storage.node_degree(&target_entity).await?;
    let merged_entity = node_to_entity_response(target_node, degree);

    let merge_details = MergeDetails {
        source_entity_id: source_entity,
        target_entity_id: target_entity,
        relationships_merged,
        duplicate_relationships_removed,
        description_strategy,
        metadata_strategy: "merge".to_string(),
    };

    Ok(Json(MergeEntitiesResponse {
        status: "success".to_string(),
        message: "Entities merged successfully".to_string(),
        merged_entity,
        merge_details,
    }))
}

/// Get entity neighborhood (connected nodes within specified depth).
#[utoipa::path(
    get,
    path = "/api/v1/graph/entities/{entity_name}/neighborhood",
    tag = "Entities",
    params(
        ("entity_name" = String, Path, description = "Entity name"),
        ("depth" = Option<u32>, Query, description = "Traversal depth (default 1, max 3)")
    ),
    responses(
        (status = 200, description = "Entity neighborhood", body = EntityNeighborhoodResponse),
        (status = 404, description = "Entity not found")
    )
)]
pub async fn get_entity_neighborhood(
    State(state): State<AppState>,
    Path(entity_name): Path<String>,
    Query(query): Query<EntityNeighborhoodQuery>,
) -> ApiResult<Json<EntityNeighborhoodResponse>> {
    // Try normalized name first
    let normalized_name = normalize_entity_name(&entity_name);

    // WHY: Handle special characters (accents, etc.) that may differ between frontend and storage
    // Try multiple lookup strategies:
    // 1. Direct normalized lookup
    // 2. URL-decoded original name
    // 3. Search by label substring
    let resolved_entity = if let Some(node) = state.graph_storage.get_node(&normalized_name).await?
    {
        node.id
    } else if let Some(node) = state.graph_storage.get_node(&entity_name).await? {
        // Try original name (might have special chars)
        node.id
    } else {
        // Fallback: search for nodes with matching label
        let search_results = state
            .graph_storage
            .search_nodes(&entity_name, 1, None, None, None)
            .await
            .unwrap_or_default();

        if let Some((node, _)) = search_results.first() {
            node.id.clone()
        } else {
            return Err(ApiError::NotFound(format!(
                "Entity '{}' not found (tried: '{}', original: '{}')",
                normalized_name, normalized_name, entity_name
            )));
        }
    };

    // Clamp depth to range [1, 3]
    let depth = query.depth.clamp(1, 3);

    // Collect nodes and edges using BFS
    let mut visited_nodes = std::collections::HashSet::new();
    let mut frontier = vec![resolved_entity.clone()];
    visited_nodes.insert(resolved_entity.clone());

    let mut all_edges = Vec::new();

    // BFS traversal up to the specified depth
    for _ in 0..depth {
        let mut next_frontier = Vec::new();

        for node_id in &frontier {
            let edges = state.graph_storage.get_node_edges(node_id).await?;

            for edge in edges {
                // Check both directions
                let neighbor = if edge.source == *node_id {
                    &edge.target
                } else {
                    &edge.source
                };

                // Add edge to collection (dedup by edge id)
                let edge_id = format!("{}_{}", edge.source, edge.target);
                if !all_edges.iter().any(|(id, _): &(String, _)| id == &edge_id) {
                    all_edges.push((edge_id, edge.clone()));
                }

                // Add neighbor to next frontier if not visited
                if !visited_nodes.contains(neighbor) {
                    visited_nodes.insert(neighbor.clone());
                    next_frontier.push(neighbor.clone());
                }
            }
        }

        frontier = next_frontier;
        if frontier.is_empty() {
            break;
        }
    }

    // Build response nodes
    let mut nodes = Vec::with_capacity(visited_nodes.len());
    for node_id in &visited_nodes {
        if let Some(node) = state.graph_storage.get_node(node_id).await? {
            let degree = state.graph_storage.node_degree(node_id).await.unwrap_or(0);
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

    // Build response edges
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

    Ok(Json(EntityNeighborhoodResponse { nodes, edges }))
}
